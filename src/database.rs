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

    Ok(db)
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
            id: network.id.map(|id| id.to_hex()).unwrap_or_default(),
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
    min_created_at: Option<String>,
    max_created_at: Option<String>,
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

    // Handle created_at timestamp filtering
    if min_created_at.is_some() || max_created_at.is_some() {
        let mut created_at_filter = doc! {};
        let mut has_filter = false;

        if let Some(min_ts) = &min_created_at {
            // Try to parse as Unix timestamp first, then as ISO 8601
            if let Ok(unix_ts) = min_ts.parse::<u64>() {
                // Already a Unix timestamp
                created_at_filter.insert("$gte", unix_ts as i64);
                has_filter = true;
                info!(
                    "Applied min_created_at (Unix): {} -> filter: $gte: {}",
                    min_ts, unix_ts
                );
            } else if let Ok(dt) = DateTime::parse_from_rfc3339(min_ts) {
                // Parse ISO 8601 timestamp and convert to Unix timestamp
                let unix_ts = dt.timestamp() as u64;
                created_at_filter.insert("$gte", unix_ts as i64);
                has_filter = true;
                info!(
                    "Applied min_created_at (ISO): {} -> filter: $gte: {}",
                    min_ts, unix_ts
                );
            } else {
                info!("Invalid min_created_at format: {}", min_ts);
            }
        }

        if let Some(max_ts) = &max_created_at {
            // Try to parse as Unix timestamp first, then as ISO 8601
            if let Ok(unix_ts) = max_ts.parse::<u64>() {
                // Already a Unix timestamp
                created_at_filter.insert("$lte", unix_ts as i64);
                has_filter = true;
                info!(
                    "Applied max_created_at (Unix): {} -> filter: $lte: {}",
                    max_ts, unix_ts
                );
            } else if let Ok(dt) = DateTime::parse_from_rfc3339(max_ts) {
                // Parse ISO 8601 timestamp and convert to Unix timestamp
                let unix_ts = dt.timestamp() as u64;
                created_at_filter.insert("$lte", unix_ts as i64);
                has_filter = true;
                info!(
                    "Applied max_created_at (ISO): {} -> filter: $lte: {}",
                    max_ts, unix_ts
                );
            } else {
                info!("Invalid max_created_at format: {}", max_ts);
            }
        }

        if has_filter {
            info!("Final created_at filter: {:?}", created_at_filter);
            filter.insert("created_at", created_at_filter);
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
            id: doc
                .get_object_id("_id")
                .map(|id| id.to_hex())
                .unwrap_or_default(),
            network_id,
            status,
            profit_usd,
            profit_amount: doc.get_str("profit").ok().map(|s| s.to_string()),
            gas_usd,
            created_at: created_at_str,
            source_tx,
            source_block_number,
            execute_block_number: doc.get_i64("execute_block_number").ok().map(|n| n as u64),
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

pub async fn get_opportunity_details(
    db: &Database,
    opportunity_id: &str,
) -> DbResult<Option<crate::models::OpportunityDetailsResponse>> {
    let opportunities_collection: Collection<Opportunity> = db.collection("opportunities");
    let networks_collection: Collection<Network> = db.collection("networks");
    let tokens_collection: Collection<crate::models::Token> = db.collection("tokens");
    let pools_collection: Collection<crate::models::Pool> = db.collection("pools");
    let debug_collection: Collection<crate::models::OpportunityDebug> =
        db.collection("opportunity_debug");

    // Parse the opportunity ID
    let object_id = match bson::oid::ObjectId::parse_str(opportunity_id) {
        Ok(id) => id,
        Err(_) => return Ok(None), // Invalid ID format
    };

    // Get the opportunity
    let opportunity = match opportunities_collection
        .find_one(doc! { "_id": object_id }, None)
        .await?
    {
        Some(opp) => opp,
        None => return Ok(None), // Opportunity not found
    };

    // Get the network
    let network = match networks_collection
        .find_one(doc! { "chain_id": opportunity.network_id as i64 }, None)
        .await?
    {
        Some(net) => net,
        None => return Ok(None), // Network not found
    };

    // Get the debug info to extract the path
    let debug_info = debug_collection
        .find_one(doc! { "_id": object_id }, None)
        .await?;

    let mut path_tokens = Vec::new();
    let mut path_pools = Vec::new();

    if let Some(debug) = debug_info {
        if let Some(path) = debug.path {
            // Process the path: even indices are tokens, odd indices are pools
            info!("Processing path with {} elements", path.len());
            for (index, address) in path.iter().enumerate() {
                info!(
                    "Path element {}: {} (type: {})",
                    index,
                    address,
                    if index % 2 == 0 { "token" } else { "pool" }
                );
                if index % 2 == 0 {
                    // Token - fetch token details
                    let address_lower = address.to_lowercase();
                    info!(
                        "Querying token with network_id: {}, address: {} (lowercase: {})",
                        opportunity.network_id, address, address_lower
                    );
                    if let Some(token) = tokens_collection
                        .find_one(
                            doc! {
                                "network_id": opportunity.network_id as i64,
                                "address": &address_lower
                            },
                            None,
                        )
                        .await?
                    {
                        info!(
                            "Found token: {} - name: {:?}, symbol: {:?}",
                            token.address, token.name, token.symbol
                        );
                        path_tokens.push(crate::models::TokenResponse {
                            id: token.id.map(|id| id.to_hex()).unwrap_or_default(),
                            address: token.address,
                            name: token.name,
                            symbol: token.symbol,
                            decimals: token.decimals,
                            price: token.price,
                        });
                    } else {
                        info!("Token not found for address: {}", address_lower);
                        // Token not found, create a basic response
                        path_tokens.push(crate::models::TokenResponse {
                            id: "".to_string(), // No ID for tokens not found in database
                            address: address.clone(),
                            name: None,
                            symbol: None,
                            decimals: None,
                            price: None,
                        });
                    }
                } else {
                    // Pool - fetch pool details
                    let address_lower = address.to_lowercase();
                    info!(
                        "Querying pool with network_id: {}, address: {} (lowercase: {})",
                        opportunity.network_id, address, address_lower
                    );
                    if let Some(pool) = pools_collection
                        .find_one(
                            doc! {
                                "network_id": opportunity.network_id as i64,
                                "address": &address_lower
                            },
                            None,
                        )
                        .await?
                    {
                        info!(
                            "Found pool: {} - type: {}, tokens: {:?}",
                            pool.address, pool.pool_type, pool.tokens
                        );
                        path_pools.push(crate::models::PoolResponse {
                            id: pool.id.map(|id| id.to_hex()).unwrap_or_default(),
                            address: pool.address,
                            pool_type: pool.pool_type,
                            tokens: pool.tokens,
                        });
                    } else {
                        info!("Pool not found for address: {}", address_lower);
                        // Pool not found, create a basic response
                        path_pools.push(crate::models::PoolResponse {
                            id: "".to_string(), // No ID for pools not found in database
                            address: address.clone(),
                            pool_type: "Unknown".to_string(),
                            tokens: Vec::new(),
                        });
                    }
                }
            }
        }
    }

    // Build the opportunity response
    let opportunity_response = crate::models::OpportunityResponse {
        id: opportunity.id.map(|id| id.to_hex()).unwrap_or_default(),
        network_id: opportunity.network_id,
        status: opportunity.status.clone(),
        profit_usd: opportunity.profit_usd,
        profit_amount: opportunity.profit.clone(),
        gas_usd: opportunity.gas_usd,
        created_at: DateTime::from_timestamp(opportunity.created_at as i64, 0)
            .unwrap_or_else(|| Utc::now())
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string(),
        source_tx: opportunity.source_tx.clone(),
        source_block_number: opportunity.source_block_number,
        execute_block_number: opportunity.execute_block_number,
        profit_token: opportunity.profit_token.clone(),
        profit_token_name: None, // Will be populated if profit_token is in path_tokens
        profit_token_symbol: None, // Will be populated if profit_token is in path_tokens
        profit_token_decimals: None, // Will be populated if profit_token is in path_tokens
    };

    // Build the network response
    let success_rate = if let (Some(executed), Some(success)) = (network.executed, network.success)
    {
        if executed > 0 {
            Some(success as f64 / executed as f64)
        } else {
            None
        }
    } else {
        None
    };

    let network_response = crate::models::NetworkResponse {
        id: network.id.map(|id| id.to_hex()).unwrap_or_default(),
        chain_id: network.chain_id,
        name: network.name.clone(),
        rpc: network.rpc.clone(),
        block_explorer: network.block_explorer.clone(),
        executed: network.executed,
        success: network.success,
        failed: network.failed,
        total_profit_usd: network.total_profit_usd,
        total_gas_usd: network.total_gas_usd,
        last_proccesed_created_at: network.last_proccesed_created_at,
        created_at: network.created_at,
        success_rate,
    };

    // Populate profit token details if it's in the path
    let mut final_opportunity_response = opportunity_response;
    if let Some(token_info) = path_tokens
        .iter()
        .find(|t| t.address == opportunity.profit_token)
    {
        final_opportunity_response.profit_token_name = token_info.name.clone();
        final_opportunity_response.profit_token_symbol = token_info.symbol.clone();
        final_opportunity_response.profit_token_decimals = token_info.decimals;
    }

    Ok(Some(crate::models::OpportunityDetailsResponse {
        opportunity: final_opportunity_response,
        network: network_response,
        path_tokens,
        path_pools,
    }))
}
