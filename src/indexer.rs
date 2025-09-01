use crate::database::{
    prune_old_hourly_data, upsert_summary_aggregation, upsert_time_aggregation, DbResult,
};
use crate::models::{
    Network, Opportunity, SummaryAggregation, TimeAggregation, TimeAggregationPeriod, Token,
    TokenAggregation,
};
use alloy::primitives::U256;
use chrono::{DateTime, Datelike, TimeZone, Timelike, Utc};
use futures::StreamExt;
use log::{debug, error, info, warn};
use mongodb::{bson::doc, Collection, Database};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Duration;

pub struct Indexer {
    db: Arc<Database>,
    running: Arc<Mutex<bool>>,
    interval_minutes: u64,
    hourly_data_retention_hours: u64,
}

impl Indexer {
    pub fn new(db: Arc<Database>, interval_minutes: u64, hourly_data_retention_hours: u64) -> Self {
        Self {
            db,
            running: Arc::new(Mutex::new(false)),
            interval_minutes,
            hourly_data_retention_hours,
        }
    }

    pub async fn start(&self) {
        let mut running = self.running.lock().await;
        if *running {
            warn!("Indexer is already running");
            return;
        }
        *running = true;
        drop(running);

        info!(
            "Starting indexer with {} minute interval",
            self.interval_minutes
        );

        let db = Arc::clone(&self.db);
        let running = Arc::clone(&self.running);
        let interval_minutes = self.interval_minutes;
        let hourly_data_retention_hours = self.hourly_data_retention_hours;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60 * interval_minutes));

            loop {
                // Check if we should stop
                {
                    let running_guard = running.lock().await;
                    if !*running_guard {
                        info!("Indexer stopped");
                        break;
                    }
                }

                // Wait for next interval
                interval.tick().await;

                // Run indexing
                if let Err(e) = Self::run_indexing_cycle(&db, hourly_data_retention_hours).await {
                    error!("Indexing cycle failed: {}", e);
                }
            }
        });
    }

    pub async fn stop(&self) {
        let mut running = self.running.lock().await;
        *running = false;
        info!("Stopping indexer...");
    }

    async fn run_indexing_cycle(db: &Database, hourly_data_retention_hours: u64) -> DbResult<()> {
        info!("Starting indexing cycle");
        let start_time = std::time::Instant::now();

        // Get all networks that need indexing
        let networks = Self::get_networks_to_index(db).await?;
        info!("Found {} networks to index", networks.len());

        // Collect all opportunities from all networks for summary processing
        let mut all_opportunities: Vec<Opportunity> = Vec::new();

        for network in networks {
            match Self::index_network(db, &network).await {
                Ok(opportunities) => {
                    let opportunity_count = opportunities.len();
                    // Add opportunities from this network to our collection
                    all_opportunities.extend(opportunities);
                    info!(
                        "Collected {} opportunities from network {} (total: {})",
                        opportunity_count,
                        network.chain_id,
                        all_opportunities.len()
                    );
                }
                Err(e) => {
                    error!("Failed to index network {}: {}", network.chain_id, e);
                }
            }
        }

        // Process all collected opportunities for summary aggregation
        if !all_opportunities.is_empty() {
            info!(
                "Processing {} total opportunities for summary aggregation",
                all_opportunities.len()
            );

            if let Err(e) = Self::index_summary_network(db, &all_opportunities).await {
                error!("Failed to index summary network: {}", e);
            } else {
                info!("Successfully completed summary aggregation");
            }
        } else {
            info!("No opportunities found for summary aggregation");
        }

        // Prune old hourly data
        if let Err(e) = prune_old_hourly_data(db, hourly_data_retention_hours).await {
            warn!("Failed to prune old hourly data: {}", e);
        } else {
            info!("Successfully pruned old hourly data");
        }

        let duration = start_time.elapsed();
        info!("Indexing cycle completed in {:?}", duration);
        Ok(())
    }

    async fn get_networks_to_index(db: &Database) -> DbResult<Vec<Network>> {
        let networks_collection: Collection<Network> = db.collection("networks");

        // Get all networks
        let mut cursor = networks_collection.find(None, None).await?;
        let mut networks = Vec::new();

        while let Some(network) = cursor.next().await {
            match network {
                Ok(net) => networks.push(net),
                Err(e) => warn!("Failed to read network: {}", e),
            }
        }

        Ok(networks)
    }

    async fn index_network(db: &Database, network: &Network) -> DbResult<Vec<Opportunity>> {
        let network_id = network.chain_id;
        info!("Indexing network {} ({})", network_id, network.name);

        let opportunities_collection: Collection<Opportunity> = db.collection("opportunities");
        let tokens_collection: Collection<Token> = db.collection("tokens");
        let networks_collection: Collection<Network> = db.collection("networks");

        // Build query for new opportunities since last indexing
        let mut query = doc! { "network_id": network_id as i64 };

        if let Some(last_created_at) = network.last_proccesed_created_at {
            query.insert("created_at", doc! { "$gt": last_created_at as i64 });
        }

        // Get new opportunities for this network
        let mut cursor = opportunities_collection.find(query.clone(), None).await?;
        let mut opportunities = Vec::new();
        let mut last_opportunity_id = None;
        let mut last_created_at = network.last_proccesed_created_at.unwrap_or(0);

        while let Some(opp) = cursor.next().await {
            match opp {
                Ok(opportunity) => {
                    opportunities.push(opportunity.clone());
                    if opportunity.created_at > last_created_at {
                        last_created_at = opportunity.created_at;
                    }
                    last_opportunity_id = opportunity.id.map(|id| id.to_hex());
                }
                Err(e) => warn!("Failed to read opportunity: {}", e),
            }
        }

        if opportunities.is_empty() {
            info!("No new opportunities for network {}", network_id);
            return Ok(Vec::new());
        }

        info!(
            "Found {} new opportunities for network {}",
            opportunities.len(),
            network_id
        );

        // Aggregate metrics from opportunities - accumulate with existing values
        let mut total_profit_usd = network.total_profit_usd;
        let mut total_gas_usd = network.total_gas_usd;
        let mut executed_count = network.executed.unwrap_or(0);
        let mut success_count = network.success.unwrap_or(0);
        let mut failed_count = network.failed.unwrap_or(0);

        // Track token metrics
        let mut token_metrics: HashMap<String, (f64, U256)> = HashMap::new(); // (profit_usd, total_profit_u256)

        for opportunity in &opportunities {
            // Update network metrics
            if let Some(profit_usd) = opportunity.profit_usd {
                total_profit_usd += profit_usd;
            }
            if let Some(gas_usd) = opportunity.gas_usd {
                total_gas_usd += gas_usd;
            }

            // Update status counts
            match opportunity.status.as_str() {
                "Succeeded" | "PartiallySucceeded" => {
                    executed_count += 1;
                    success_count += 1;
                }
                "Reverted" | "Error" => {
                    executed_count += 1;
                    failed_count += 1;
                }
                _ => {}
            }

            // Update token metrics
            if opportunity.profit_usd.is_some() || opportunity.profit.is_some() {
                let entry = token_metrics
                    .entry(opportunity.profit_token.clone())
                    .or_insert((0.0, U256::ZERO));

                // Aggregate USD profit
                if let Some(profit_usd) = opportunity.profit_usd {
                    entry.0 += profit_usd;
                }

                // Aggregate raw profit using U256 arithmetic
                if let Some(ref profit_str) = opportunity.profit {
                    match profit_str.parse::<U256>() {
                        Ok(profit_u256) => {
                            entry.1 = entry.1.saturating_add(profit_u256);
                        }
                        Err(e) => {
                            warn!("Failed to parse profit '{}' as U256: {}", profit_str, e);
                        }
                    }
                }
            }
        }

        // Update network document
        let network_update = doc! {
            "$set": {
                "total_profit_usd": total_profit_usd,
                "total_gas_usd": total_gas_usd,
                "executed": executed_count as i64,
                "success": success_count as i64,
                "failed": failed_count as i64,
                "last_proccesed_created_at": last_created_at as i64,
                "last_processed_id": last_opportunity_id.clone(),
                "updated_at": Utc::now().timestamp() as i64
            }
        };

        networks_collection
            .update_one(doc! { "chain_id": network_id as i64 }, network_update, None)
            .await?;

        // Update token metrics
        for (token_address, (profit_usd, total_profit_u256)) in token_metrics {
            // First, get the existing token to read current total_profit
            let existing_token = tokens_collection
                .find_one(
                    doc! {
                        "network_id": network_id as i64,
                        "address": token_address.to_lowercase()
                    },
                    None,
                )
                .await?;

            // Calculate new total_profit by adding to existing value
            let new_total_profit = if let Some(token) = existing_token {
                if let Some(existing_profit_str) = &token.total_profit {
                    match existing_profit_str.parse::<U256>() {
                        Ok(existing_u256) => existing_u256.saturating_add(total_profit_u256),
                        Err(_) => total_profit_u256, // If parsing fails, use new value only
                    }
                } else {
                    total_profit_u256
                }
            } else {
                total_profit_u256
            };

            let mut token_update = doc! {
                "$inc": {
                    "total_profit_usd": profit_usd
                },
                "$set": {
                    "updated_at": Utc::now().timestamp() as i64
                }
            };

            // Update total_profit with the aggregated U256 value (converted to string)
            if new_total_profit > U256::ZERO {
                token_update
                    .get_document_mut("$set")
                    .unwrap()
                    .insert("total_profit", new_total_profit.to_string());
            }

            tokens_collection
                .update_one(
                    doc! {
                        "network_id": network_id as i64,
                        "address": token_address.to_lowercase()
                    },
                    token_update,
                    None,
                )
                .await?;
        }

        // Create time aggregations for hourly, daily, and monthly periods
        info!(
            "Creating time aggregations for network {} with {} opportunities",
            network_id,
            opportunities.len()
        );

        // Create time aggregations for this network
        if let Err(e) = Self::create_time_aggregations(
            db,
            network_id,
            &opportunities,
            total_profit_usd,
            total_gas_usd,
            executed_count,
            success_count,
            failed_count,
        )
        .await
        {
            warn!(
                "Failed to create time aggregations for network {}: {}",
                network_id, e
            );
        } else {
            info!(
                "Successfully created time aggregations for network {}",
                network_id
            );
        }

        info!(
            "Network {} indexed: {} opportunities, profit: ${:.2}, gas: ${:.2}, executed: {}",
            network_id,
            opportunities.len(),
            total_profit_usd,
            total_gas_usd,
            executed_count
        );

        // Return the opportunities for summary processing
        Ok(opportunities)
    }

    /// Index summary network - treat summary as a special network that processes all opportunities
    async fn index_summary_network(
        db: &Database,
        all_opportunities: &[Opportunity],
    ) -> DbResult<()> {
        if all_opportunities.is_empty() {
            info!("No opportunities to process for summary network");
            return Ok(());
        }

        info!(
            "Indexing summary network with {} total opportunities from all networks",
            all_opportunities.len()
        );

        // Calculate total metrics across all opportunities
        let total_profit_usd: f64 = all_opportunities
            .iter()
            .filter_map(|opp| opp.profit_usd)
            .sum();

        let total_gas_usd: f64 = all_opportunities.iter().filter_map(|opp| opp.gas_usd).sum();

        let executed_count = all_opportunities
            .iter()
            .filter(|opp| opp.status.as_str() == "executed")
            .count() as u64;

        let success_count = all_opportunities
            .iter()
            .filter(|opp| opp.status.as_str() == "success")
            .count() as u64;

        let failed_count = all_opportunities
            .iter()
            .filter(|opp| opp.status.as_str() == "failed")
            .count() as u64;

        info!(
            "Summary network metrics: {} opportunities, ${:.2} profit, ${:.2} gas, {} executed, {} success, {} failed",
            all_opportunities.len(),
            total_profit_usd,
            total_gas_usd,
            executed_count,
            success_count,
            failed_count
        );

        // Create time aggregations for summary (treat as network_id = 0 for summary)
        let summary_network_id = 0u64;

        if let Err(e) = Self::create_time_aggregations(
            db,
            summary_network_id,
            all_opportunities,
            total_profit_usd,
            total_gas_usd,
            executed_count,
            success_count,
            failed_count,
        )
        .await
        {
            error!("Failed to create summary time aggregations: {}", e);
            return Err(e);
        }

        info!("Successfully created summary time aggregations");
        Ok(())
    }

    pub async fn run_manual_indexing(db: &Database) -> DbResult<()> {
        info!("Running manual indexing cycle");
        Self::run_indexing_cycle(db, 168).await
    }

    /// Create time aggregations for hourly, daily, and monthly periods
    async fn create_time_aggregations(
        db: &Database,
        network_id: u64,
        opportunities: &[Opportunity],
        _total_profit_usd: f64,
        _total_gas_usd: f64,
        _executed_count: u64,
        _success_count: u64,
        _failed_count: u64,
    ) -> DbResult<()> {
        if opportunities.is_empty() {
            info!("No opportunities to aggregate for network {}", network_id);
            return Ok(());
        }

        // Find the time range from first opportunity to now
        let first_opportunity_time = opportunities
            .iter()
            .map(|opp| opp.created_at)
            .min()
            .unwrap_or(Utc::now().timestamp() as u64);

        let now = Utc::now();
        let now_timestamp = now.timestamp() as u64;

        info!(
            "Creating time aggregations for network {} from {} to {} ({} opportunities)",
            network_id,
            first_opportunity_time,
            now_timestamp,
            opportunities.len()
        );

        // Create aggregations for different time periods
        let periods = [
            TimeAggregationPeriod::Hourly,
            TimeAggregationPeriod::Daily,
            TimeAggregationPeriod::Monthly,
        ];

        for period in periods {
            info!(
                "Processing {} aggregations for network {}",
                period_to_string(period),
                network_id
            );

            if let Err(e) = Self::create_period_aggregations_range(
                db,
                network_id,
                period,
                first_opportunity_time,
                now_timestamp,
                opportunities,
            )
            .await
            {
                warn!(
                    "Failed to create {} aggregations: {}",
                    period_to_string(period),
                    e
                );
            } else {
                info!(
                    "Successfully created {} aggregations for network {}",
                    period_to_string(period),
                    network_id
                );
            }
        }

        Ok(())
    }

    /// Create aggregations for all periods in a time range
    async fn create_period_aggregations_range(
        db: &Database,
        network_id: u64,
        period: TimeAggregationPeriod,
        start_time: u64,
        end_time: u64,
        opportunities: &[Opportunity],
    ) -> DbResult<()> {
        // Generate all period timestamps in the range
        let period_timestamps = Self::generate_period_timestamps(period, start_time, end_time);

        info!(
            "Generated {} {} periods from {} to {}",
            period_timestamps.len(),
            period_to_string(period),
            start_time,
            end_time
        );

        for timestamp in period_timestamps {
            // Filter opportunities for this specific period
            let period_opportunities =
                Self::filter_opportunities_for_period(opportunities, period, timestamp);

            if period_opportunities.is_empty() {
                debug!(
                    "No opportunities found for {} period at timestamp {}",
                    period_to_string(period),
                    timestamp
                );
                continue;
            }

            info!(
                "Processing {} opportunities for {} period at timestamp {}",
                period_opportunities.len(),
                period_to_string(period),
                timestamp
            );

            // Calculate metrics for this period
            let (total_profit_usd, total_gas_usd, executed_count, success_count, failed_count) =
                Self::calculate_period_metrics(&period_opportunities);

            // Create or update the aggregation
            let period_opportunities_refs: Vec<&Opportunity> =
                period_opportunities.iter().collect();

            if network_id == 0 {
                // This is a summary aggregation - store in summary_aggregations collection
                if let Err(e) = Self::create_or_update_summary_period_aggregation(
                    db,
                    period,
                    timestamp,
                    &period_opportunities_refs,
                    total_profit_usd,
                    total_gas_usd,
                    executed_count,
                    success_count,
                    failed_count,
                )
                .await
                {
                    warn!(
                        "Failed to create/update summary {} aggregation at timestamp {}: {}",
                        period_to_string(period),
                        timestamp,
                        e
                    );
                } else {
                    debug!(
                        "Successfully created/updated summary {} aggregation at timestamp {}",
                        period_to_string(period),
                        timestamp
                    );
                }
            } else {
                // This is a regular network aggregation - store in time_aggregations collection
                if let Err(e) = Self::create_or_update_period_aggregation(
                    db,
                    network_id,
                    period,
                    timestamp,
                    &period_opportunities_refs,
                    total_profit_usd,
                    total_gas_usd,
                    executed_count,
                    success_count,
                    failed_count,
                )
                .await
                {
                    warn!(
                        "Failed to create/update {} aggregation at timestamp {}: {}",
                        period_to_string(period),
                        timestamp,
                        e
                    );
                } else {
                    debug!(
                        "Successfully created/updated {} aggregation at timestamp {}",
                        period_to_string(period),
                        timestamp
                    );
                }
            }
        }

        Ok(())
    }

    /// Generate all period timestamps in a time range
    fn generate_period_timestamps(
        period: TimeAggregationPeriod,
        start_time: u64,
        end_time: u64,
    ) -> Vec<u64> {
        let mut timestamps = Vec::new();
        let start_dt = Utc.timestamp_opt(start_time as i64, 0).unwrap();

        let mut current = match period {
            TimeAggregationPeriod::Hourly => Self::get_hourly_timestamp(&start_dt),
            TimeAggregationPeriod::Daily => Self::get_daily_timestamp(&start_dt),
            TimeAggregationPeriod::Monthly => Self::get_monthly_timestamp(&start_dt),
        };

        let increment = match period {
            TimeAggregationPeriod::Hourly => 3600, // 1 hour in seconds
            TimeAggregationPeriod::Daily => 86400, // 1 day in seconds
            TimeAggregationPeriod::Monthly => 86400 * 30, // ~30 days in seconds
        };

        while current <= end_time {
            timestamps.push(current);
            current += increment;
        }

        timestamps
    }

    /// Filter opportunities for a specific time period
    fn filter_opportunities_for_period(
        opportunities: &[Opportunity],
        period: TimeAggregationPeriod,
        period_timestamp: u64,
    ) -> Vec<Opportunity> {
        let period_start = period_timestamp;
        let period_end = match period {
            TimeAggregationPeriod::Hourly => period_start + 3600,
            TimeAggregationPeriod::Daily => period_start + 86400,
            TimeAggregationPeriod::Monthly => period_start + (86400 * 30),
        };

        opportunities
            .iter()
            .filter(|opp| opp.created_at >= period_start && opp.created_at < period_end)
            .cloned()
            .collect()
    }

    /// Calculate metrics for a set of opportunities
    fn calculate_period_metrics(opportunities: &[Opportunity]) -> (f64, f64, u64, u64, u64) {
        let mut total_profit_usd = 0.0;
        let mut total_gas_usd = 0.0;
        let mut executed_count = 0;
        let mut success_count = 0;
        let mut failed_count = 0;

        for opp in opportunities {
            if let Some(profit) = opp.profit_usd {
                total_profit_usd += profit;
            }
            if let Some(gas) = opp.gas_usd {
                total_gas_usd += gas;
            }
            // Check if opportunity was executed based on status
            match opp.status.as_str() {
                "Succeeded" | "PartiallySucceeded" => {
                    executed_count += 1;
                    success_count += 1;
                }
                "Reverted" | "Error" => {
                    executed_count += 1;
                    failed_count += 1;
                }
                _ => {}
            }
        }

        (
            total_profit_usd,
            total_gas_usd,
            executed_count,
            success_count,
            failed_count,
        )
    }

    /// Create or update summary period aggregation for a specific time period
    async fn create_or_update_summary_period_aggregation(
        db: &Database,
        period: TimeAggregationPeriod,
        timestamp: u64,
        opportunities: &[&Opportunity],
        total_profit_usd: f64,
        total_gas_usd: f64,
        executed_count: u64,
        success_count: u64,
        failed_count: u64,
    ) -> DbResult<()> {
        // Calculate period start and end times
        let (period_start, period_end) = Self::get_period_bounds(period, timestamp);

        // Calculate averages and success rate
        let total_opportunities = opportunities.len() as u64;

        // Create the summary aggregation
        let summary_aggregation = SummaryAggregation {
            id: None,
            period: period_to_string(period),
            timestamp,
            period_start: period_start.parse().unwrap_or("0".to_string()),
            period_end: period_end.parse().unwrap_or("0".to_string()),
            total_opportunities,
            executed_opportunities: executed_count,
            successful_opportunities: success_count,
            failed_opportunities: failed_count,
            total_profit_usd,
            total_gas_usd,
            created_at: Utc::now().timestamp() as u64,
            updated_at: Utc::now().timestamp() as u64,
        };

        // Upsert the summary aggregation (create or update)
        upsert_summary_aggregation(db, period, timestamp, summary_aggregation).await
    }

    /// Create or update aggregation for a specific time period
    async fn create_or_update_period_aggregation(
        db: &Database,
        network_id: u64,
        period: TimeAggregationPeriod,
        timestamp: u64,
        opportunities: &[&Opportunity],
        total_profit_usd: f64,
        total_gas_usd: f64,
        executed_count: u64,
        success_count: u64,
        failed_count: u64,
    ) -> DbResult<()> {
        // Calculate period start and end times
        let (period_start, period_end) = Self::get_period_bounds(period, timestamp);

        // Calculate total opportunities
        let total_opportunities = opportunities.len() as u64;

        // Aggregate top profit tokens for this period
        let mut token_aggregations: HashMap<String, TokenAggregation> = HashMap::new();

        for opportunity in opportunities {
            if let Some(profit_usd) = opportunity.profit_usd {
                let token_address = opportunity.profit_token.clone();
                let entry = token_aggregations
                    .entry(token_address.clone())
                    .or_insert_with(|| TokenAggregation {
                        address: token_address,
                        name: None,   // Will be filled from token data if available
                        symbol: None, // Will be filled from token data if available
                        total_profit_usd: 0.0,
                        total_profit: "0".to_string(),
                        opportunity_count: 0,
                        avg_profit_usd: 0.0,
                    });

                entry.total_profit_usd += profit_usd;
                entry.opportunity_count += 1;

                // Aggregate raw profit using U256
                if let Some(ref profit_str) = opportunity.profit {
                    if let Ok(profit_u256) = profit_str.parse::<U256>() {
                        let current = entry.total_profit.parse::<U256>().unwrap_or(U256::ZERO);
                        entry.total_profit = current.saturating_add(profit_u256).to_string();
                    }
                }
            }
        }

        // Calculate averages for tokens
        for token_agg in token_aggregations.values_mut() {
            if token_agg.opportunity_count > 0 {
                token_agg.avg_profit_usd =
                    token_agg.total_profit_usd / token_agg.opportunity_count as f64;
            }
        }

        // Sort tokens by profit and take top 10
        let mut top_tokens: Vec<TokenAggregation> = token_aggregations.into_values().collect();
        top_tokens.sort_by(|a, b| {
            b.total_profit_usd
                .partial_cmp(&a.total_profit_usd)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        top_tokens.truncate(10);

        // Create the aggregation
        let aggregation = TimeAggregation {
            id: None,
            network_id,
            period: period_to_string(period),
            timestamp,
            period_start: period_start.parse().unwrap_or("0".to_string()),
            period_end: period_end.parse().unwrap_or("0".to_string()),
            total_profit_usd,
            total_gas_usd,
            total_opportunities,
            executed_opportunities: executed_count,
            successful_opportunities: success_count,
            failed_opportunities: failed_count,
            top_profit_tokens: top_tokens,
            created_at: Utc::now().timestamp() as u64,
            updated_at: Utc::now().timestamp() as u64,
        };

        // Upsert the aggregation (create or update)
        upsert_time_aggregation(db, network_id, period, timestamp, aggregation).await
    }

    /// Get timestamp for hourly period (rounded to hour)
    fn get_hourly_timestamp(dt: &DateTime<Utc>) -> u64 {
        let hour_dt = dt.date_naive().and_hms_opt(dt.hour(), 0, 0).unwrap();
        hour_dt.and_utc().timestamp() as u64
    }

    /// Get timestamp for daily period (rounded to day)
    fn get_daily_timestamp(dt: &DateTime<Utc>) -> u64 {
        let day_dt = dt.date_naive().and_hms_opt(0, 0, 0).unwrap();
        day_dt.and_utc().timestamp() as u64
    }

    /// Get timestamp for monthly period (rounded to first day of month)
    fn get_monthly_timestamp(dt: &DateTime<Utc>) -> u64 {
        let month_dt = dt
            .date_naive()
            .with_day(1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        month_dt.and_utc().timestamp() as u64
    }

    /// Get period start and end times as ISO strings
    fn get_period_bounds(period: TimeAggregationPeriod, timestamp: u64) -> (String, String) {
        let start_dt = Utc.timestamp_opt(timestamp as i64, 0).unwrap();

        let end_dt = match period {
            TimeAggregationPeriod::Hourly => start_dt + chrono::Duration::hours(1),
            TimeAggregationPeriod::Daily => start_dt + chrono::Duration::days(1),
            TimeAggregationPeriod::Monthly => {
                // Add one month (approximate)
                start_dt + chrono::Duration::days(31)
            }
        };

        (start_dt.to_rfc3339(), end_dt.to_rfc3339())
    }
}

/// Helper function to convert TimeAggregationPeriod to string
fn period_to_string(period: TimeAggregationPeriod) -> String {
    match period {
        TimeAggregationPeriod::Hourly => "hourly".to_string(),
        TimeAggregationPeriod::Daily => "daily".to_string(),
        TimeAggregationPeriod::Monthly => "monthly".to_string(),
    }
}
