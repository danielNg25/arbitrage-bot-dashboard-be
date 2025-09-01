use crate::database::DbResult;
use crate::models::{Network, Opportunity, Token};
use alloy::primitives::U256;
use chrono::Utc;
use futures::StreamExt;
use log::{error, info, warn};
use mongodb::{bson::doc, Collection, Database};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Duration;

pub struct Indexer {
    db: Arc<Database>,
    running: Arc<Mutex<bool>>,
    interval_minutes: u64,
}

impl Indexer {
    pub fn new(db: Arc<Database>, interval_minutes: u64) -> Self {
        Self {
            db,
            running: Arc::new(Mutex::new(false)),
            interval_minutes,
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
                if let Err(e) = Self::run_indexing_cycle(&db).await {
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

    async fn run_indexing_cycle(db: &Database) -> DbResult<()> {
        info!("Starting indexing cycle");
        let start_time = std::time::Instant::now();

        // Get all networks that need indexing
        let networks = Self::get_networks_to_index(db).await?;
        info!("Found {} networks to index", networks.len());

        for network in networks {
            if let Err(e) = Self::index_network(db, &network).await {
                error!("Failed to index network {}: {}", network.chain_id, e);
            }
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

    async fn index_network(db: &Database, network: &Network) -> DbResult<()> {
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
            return Ok(());
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

        info!(
            "Network {} indexed: {} opportunities, profit: ${:.2}, gas: ${:.2}, executed: {}",
            network_id,
            opportunities.len(),
            total_profit_usd,
            total_gas_usd,
            executed_count
        );

        Ok(())
    }

    pub async fn run_manual_indexing(db: &Database) -> DbResult<()> {
        info!("Running manual indexing cycle");
        Self::run_indexing_cycle(db).await
    }
}
