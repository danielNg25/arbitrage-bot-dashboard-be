use alloy::primitives::{Address, U256};
use chrono::{DateTime, Utc};
use futures::StreamExt;
use log::info;
use mongodb::{bson::doc, Client, Collection, Database};

use crate::{
    config::DatabaseConfig,
    models::{Network, Opportunity},
    utils::OpportunityStatus,
};

pub type DbResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub async fn init_database(db_config: &DatabaseConfig) -> DbResult<Database> {
    let client_options = mongodb::options::ClientOptions::parse(&db_config.uri).await?;

    // Apply custom options
    let mut client_options = client_options;
    client_options.connect_timeout = Some(std::time::Duration::from_millis(
        db_config.connection_timeout_ms,
    ));

    if let Some(max_pool_size) = db_config.max_pool_size {
        client_options.max_pool_size = Some(max_pool_size);
    }

    let client = Client::with_options(client_options)?;
    let db = client.database(&db_config.database_name);

    info!("Connected to MongoDB database: {}", db_config.database_name);

    // Initialize collections with dummy data if they're empty
    init_dummy_data(&db).await?;

    Ok(db)
}

async fn init_dummy_data(db: &Database) -> DbResult<()> {
    let networks_collection: Collection<Network> = db.collection("networks");
    let opportunities_collection: Collection<Opportunity> = db.collection("opportunities");

    // Check if networks collection is empty
    let network_count = networks_collection.count_documents(doc! {}, None).await?;

    if network_count == 0 {
        info!("Initializing dummy networks data...");

        // Create dummy networks
        let networks = vec![
            Network::new(
                1,
                "Ethereum Mainnet".to_string(),
                Some("https://eth-mainnet.alchemyapi.io/v2/your-api-key".to_string()),
            ),
            Network::new(
                137,
                "Polygon".to_string(),
                Some("https://polygon-rpc.com".to_string()),
            ),
            Network::new(
                56,
                "Binance Smart Chain".to_string(),
                Some("https://bsc-dataseed.binance.org".to_string()),
            ),
        ];

        for mut network in networks {
            // Add some dummy profit and gas data
            network.total_profit_usd = rand::random::<f64>() * 10000.0;
            network.total_gas_usd = rand::random::<f64>() * 1000.0;

            // Add dummy metrics data
            network.executed = Some(rand::random::<u64>() % 1000);
            network.success = Some(rand::random::<u64>() % 800);
            network.failed = Some(rand::random::<u64>() % 200);
            network.last_proccesed_created_at =
                Some(Utc::now().timestamp() as u64 - rand::random::<u64>() % 86400);

            let result = networks_collection.insert_one(&network, None).await?;
            if let Some(id) = result.inserted_id.as_object_id() {
                info!("Created network: {} (ID: {})", network.name, id);
            }
        }
    }

    // Check if opportunities collection is empty
    let opportunity_count = opportunities_collection
        .count_documents(doc! {}, None)
        .await?;

    if opportunity_count == 0 {
        info!("Initializing dummy opportunities data...");

        // Create dummy opportunities
        let mut opportunities = Vec::new();
        let statuses = vec![
            OpportunityStatus::Succeeded,
            OpportunityStatus::PartiallySucceeded,
            OpportunityStatus::Reverted,
            OpportunityStatus::Error,
            OpportunityStatus::Skipped,
        ];

        for i in 0..50 {
            let network_id = match i % 3 {
                0 => 1,   // Ethereum
                1 => 137, // Polygon
                _ => 56,  // BSC
            };

            let status = statuses[i % statuses.len()];
            let profit_usd = if status == OpportunityStatus::Succeeded
                || status == OpportunityStatus::PartiallySucceeded
            {
                Some(rand::random::<f64>() * 500.0)
            } else {
                None
            };

            let gas_usd = if status == OpportunityStatus::Succeeded
                || status == OpportunityStatus::PartiallySucceeded
            {
                Some(rand::random::<f64>() * 50.0)
            } else {
                None
            };

            // Create dummy addresses and amounts
            let dummy_address = Address::from_slice(&[i as u8; 20]);
            let dummy_amount = U256::from(i as u64 + 1);

            let opportunity = Opportunity::new(
                network_id,
                &dummy_address,
                &dummy_amount,
                status,
                Some(rand::random::<u64>()),
                Some(format!("0x{:064x}", rand::random::<u64>())),
                Some(rand::random::<u64>()),
                Some(format!("0x{:040x}", rand::random::<u64>())),
                Some(rand::random::<u64>()),
                Some(format!("0x{:064x}", rand::random::<u64>())),
                Some(format!("{}", rand::random::<u64>())),
                profit_usd,
                Some(format!("{}", rand::random::<u64>())),
                gas_usd,
            );

            opportunities.push(opportunity);
        }

        // Insert opportunities in batches
        let result = opportunities_collection
            .insert_many(opportunities, None)
            .await?;
        info!("Created {} dummy opportunities", result.inserted_ids.len());
    }

    Ok(())
}

pub async fn get_networks_with_stats(
    db: &Database,
) -> DbResult<Vec<crate::models::NetworkResponse>> {
    let networks_collection: Collection<Network> = db.collection("networks");

    let mut networks = Vec::new();
    let mut cursor = networks_collection.find(doc! {}, None).await?;

    while let Some(network) = cursor.next().await {
        let network = network?;

        // Count executed opportunities for this network

        // Calculate success rate
        let success_rate =
            if let (Some(executed), Some(success)) = (network.executed, network.success) {
                if executed > 0 {
                    Some(success as f64 / executed as f64)
                } else {
                    None
                }
            } else {
                None
            };

        let response = crate::models::NetworkResponse {
            chain_id: network.chain_id,
            name: network.name,
            rpc: network.rpc,
            block_explorer: network.block_explorer,
            executed: network.executed,
            success: network.success,
            failed: network.failed,
            total_profit_usd: network.total_profit_usd,
            total_gas_usd: network.total_gas_usd,
            last_proccesed_created_at: network.last_proccesed_created_at,
            created_at: network.created_at,
            success_rate,
        };

        networks.push(response);
    }

    Ok(networks)
}

pub async fn get_opportunities(
    db: &Database,
    network_id: Option<u64>,
    status: Option<String>,
    min_profit_usd: Option<f64>,
    max_profit_usd: Option<f64>,
    min_gas_usd: Option<f64>,
    max_gas_usd: Option<f64>,
    min_source_timestamp: Option<String>,
    max_source_timestamp: Option<String>,
    page: Option<u32>,
    limit: Option<u32>,
) -> DbResult<crate::models::PaginatedOpportunitiesResponse> {
    let opportunities_collection: Collection<Opportunity> = db.collection("opportunities");
    let _tokens_collection: Collection<crate::models::Token> = db.collection("tokens");

    let mut filter = doc! {};

    // Apply filters
    if let Some(nid) = network_id {
        filter.insert("network_id", nid as i64);
    }
    // Handle status filtering - support both single status and multiple statuses with case-insensitive matching
    else if let Some(s) = &status {
        // Single status - use case-insensitive regex matching
        let status_regex = format!("^{}$", regex::escape(s));
        filter.insert(
            "status",
            doc! {
                "$regex": status_regex,
                "$options": "i"
            },
        );
        info!("Applied single status filter (case-insensitive): {}", s);
    }

    info!("Final filter: {:?}", filter);

    // Profit USD range filter
    if min_profit_usd.is_some() || max_profit_usd.is_some() {
        let mut profit_filter = doc! {};
        if let Some(min) = min_profit_usd {
            profit_filter.insert("$gte", min);
        }
        if let Some(max) = max_profit_usd {
            profit_filter.insert("$lte", max);
        }
        filter.insert("profit_usd", profit_filter);
    }

    // Gas USD range filter
    if min_gas_usd.is_some() || max_gas_usd.is_some() {
        let mut gas_filter = doc! {};
        if let Some(min) = min_gas_usd {
            gas_filter.insert("$gte", min);
        }
        if let Some(max) = max_gas_usd {
            gas_filter.insert("$lte", max);
        }
        filter.insert("gas_usd", gas_filter);
    }

    // Handle source timestamp filtering
    if min_source_timestamp.is_some() || max_source_timestamp.is_some() {
        let mut source_timestamp_filter = doc! {};
        let mut has_filter = false;

        if let Some(min_ts) = &min_source_timestamp {
            // Try to parse as Unix timestamp first, then as ISO 8601
            if let Ok(unix_ts) = min_ts.parse::<u64>() {
                // Already a Unix timestamp
                source_timestamp_filter.insert("$gte", unix_ts as i64);
                has_filter = true;
                info!(
                    "Applied min_source_timestamp (Unix): {} -> filter: $gte: {}",
                    min_ts, unix_ts
                );
            } else if let Ok(dt) = DateTime::parse_from_rfc3339(min_ts) {
                // Parse ISO 8601 timestamp and convert to Unix timestamp
                let unix_ts = dt.timestamp() as u64;
                source_timestamp_filter.insert("$gte", unix_ts as i64);
                has_filter = true;
                info!(
                    "Applied min_source_timestamp (ISO): {} -> filter: $gte: {}",
                    min_ts, unix_ts
                );
            } else {
                info!("Invalid min_source_timestamp format: {}", min_ts);
            }
        }

        if let Some(max_ts) = &max_source_timestamp {
            // Try to parse as Unix timestamp first, then as ISO 8601
            if let Ok(unix_ts) = max_ts.parse::<u64>() {
                // Already a Unix timestamp
                source_timestamp_filter.insert("$lte", unix_ts as i64);
                has_filter = true;
                info!(
                    "Applied max_source_timestamp (Unix): {} -> filter: $lte: {}",
                    max_ts, unix_ts
                );
            } else if let Ok(dt) = DateTime::parse_from_rfc3339(max_ts) {
                // Parse ISO 8601 timestamp and convert to Unix timestamp
                let unix_ts = dt.timestamp() as u64;
                source_timestamp_filter.insert("$lte", unix_ts as i64);
                has_filter = true;
                info!(
                    "Applied max_source_timestamp (ISO): {} -> filter: $lte: {}",
                    max_ts, unix_ts
                );
            } else {
                info!("Invalid max_source_timestamp format: {}", max_ts);
            }
        }

        if has_filter {
            info!(
                "Final source timestamp filter: {:?}",
                source_timestamp_filter
            );
            filter.insert("source_block_timestamp", source_timestamp_filter);
            info!("Final complete filter: {:?}", filter);
        }
    }

    // Pagination parameters
    let page = page.unwrap_or(1);
    let limit = limit.unwrap_or(100).min(1000); // Cap at 1000
    let skip = (page - 1) * limit;

    // Get total count for pagination
    let total = opportunities_collection
        .count_documents(filter.clone(), None)
        .await?;
    let total_pages = ((total as f64) / (limit as f64)).ceil() as u32;

    // Build aggregation pipeline for joining opportunities with tokens
    let pipeline = vec![
        doc! { "$match": filter },
        doc! {
            "$lookup": {
                "from": "tokens",
                "let": {
                    "opp_network_id": "$network_id",
                    "opp_profit_token": "$profit_token"
                },
                "pipeline": [
                    doc! {
                        "$match": {
                            "$expr": {
                                "$and": [
                                    { "$eq": ["$network_id", "$$opp_network_id"] },
                                    { "$eq": ["$address", "$$opp_profit_token"] }
                                ]
                            }
                        }
                    }
                ],
                "as": "token_info"
            }
        },
        doc! {
            "$addFields": {
                "token_data": {
                    "$ifNull": [
                        { "$arrayElemAt": ["$token_info", 0] },
                        {}
                    ]
                }
            }
        },
        doc! {
            "$sort": { "created_at": -1 }
        },
        doc! {
            "$skip": skip as i64
        },
        doc! {
            "$limit": limit as i64
        },
    ];

    // Execute aggregation pipeline
    let mut opportunities = Vec::new();
    let mut cursor = opportunities_collection.aggregate(pipeline, None).await?;

    while let Some(doc) = cursor.next().await {
        let doc = doc?;

        // Extract opportunity data
        let network_id = doc.get_i64("network_id")? as u64;
        let status = doc.get_str("status")?.to_string();
        let profit_usd = doc.get_f64("profit_usd").ok();
        let gas_usd = doc.get_f64("gas_usd").ok();
        let created_at = doc.get_i64("created_at")? as u64;
        let source_tx = doc.get_str("source_tx").ok().map(|s| s.to_string());
        let source_block_number = doc.get_i64("source_block_number").ok().map(|n| n as u64);
        let profit_token = doc.get_str("profit_token")?.to_string();

        // Extract token data from joined result
        let empty_doc = doc! {};
        let token_data = doc.get_document("token_data").unwrap_or(&empty_doc);
        let profit_token_name = token_data.get_str("name").ok().map(|s| s.to_string());
        let profit_token_symbol = token_data.get_str("symbol").ok().map(|s| s.to_string());
        let profit_token_decimals = if let Some(bson::Bson::Int32(n)) = token_data.get("decimals") {
            Some(*n as u8)
        } else {
            None
        };

        // Convert timestamp to ISO 8601 string
        let created_at_str = DateTime::from_timestamp(created_at as i64, 0)
            .unwrap_or_else(|| Utc::now())
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();

        let response = crate::models::OpportunityResponse {
            network_id,
            status,
            profit_usd,
            profit_amount: doc.get_str("profit").ok().map(|s| s.to_string()),
            gas_usd,
            created_at: created_at_str,
            source_tx,
            source_block_number,
            source_block_timestamp: doc.get_i64("source_block_timestamp").ok().and_then(|ts| {
                DateTime::from_timestamp(ts, 0)
                    .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
            }),
            execute_block_number: doc.get_i64("execute_block_number").ok().map(|n| n as u64),
            execute_block_timestamp: doc.get_i64("execute_block_timestamp").ok().and_then(
                |block_num| {
                    // Convert block number to approximate timestamp (assuming 12 second block time)
                    // This is a rough approximation - in production you might want to use a block timestamp service
                    let block_time = block_num * 12; // 12 seconds per block
                    DateTime::from_timestamp(block_time, 0)
                        .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
                },
            ),
            profit_token,
            profit_token_name,
            profit_token_symbol,
            profit_token_decimals,
        };

        opportunities.push(response);
    }

    let pagination = crate::models::PaginationInfo {
        page,
        limit,
        total,
        total_pages,
        has_next: page < total_pages,
        has_prev: page > 1,
    };

    Ok(crate::models::PaginatedOpportunitiesResponse {
        opportunities,
        pagination,
    })
}

pub async fn get_profit_over_time(
    db: &Database,
) -> DbResult<Vec<crate::models::ProfitOverTimeResponse>> {
    let opportunities_collection: Collection<Opportunity> = db.collection("opportunities");

    // Get opportunities from the last 30 days
    let thirty_days_ago = Utc::now().timestamp() as u64 - (30 * 24 * 60 * 60);

    let pipeline = vec![
        doc! {
            "$match": {
                "created_at": { "$gte": thirty_days_ago as i64 },
                "profit_usd": { "$exists": true, "$ne": null }
            }
        },
        doc! {
            "$group": {
                "_id": {
                    "$dateToString": {
                        "format": "%Y-%m-%d",
                        "date": { "$toDate": { "$multiply": ["$created_at", 1000] } }
                    }
                },
                "total_profit": { "$sum": "$profit_usd" }
            }
        },
        doc! {
            "$sort": { "_id": 1 }
        },
    ];

    let mut cursor = opportunities_collection.aggregate(pipeline, None).await?;
    let mut results = Vec::new();

    while let Some(result) = cursor.next().await {
        let doc = result?;

        if let (Some(date), Some(profit)) =
            (doc.get_str("_id").ok(), doc.get_f64("total_profit").ok())
        {
            results.push(crate::models::ProfitOverTimeResponse {
                date: date.to_string(),
                profit_usd: profit,
            });
        }
    }

    Ok(results)
}
