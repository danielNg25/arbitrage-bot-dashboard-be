use chrono::{DateTime, Utc};
use futures::StreamExt;
use log::{debug, info, warn};
use mongodb::{bson::doc, options::IndexOptions, Client, Collection, Database};
use regex;

use crate::{
    config::DatabaseConfig,
    models::{
        Network, Opportunity, SummaryAggregation, SummaryAggregationResponse, TimeAggregation,
        TimeAggregationPeriod, TimeAggregationResponse, Token, TokenAggregation,
        TokenAggregationResponse, TokenPerformanceResponse,
    },
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

    debug!("Connected to MongoDB database: {}", db_config.database_name);

    // Create indexes for better query performance
    if let Err(e) = create_database_indexes(&db).await {
        warn!("Failed to create database indexes: {}", e);
    }

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
            router_address: network.router_address,
            executors: network.executors,
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
    min_estimate_profit_usd: Option<f64>,
    max_estimate_profit_usd: Option<f64>,
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
        debug!("Applied single status filter (case-insensitive): {}", s);
    }

    debug!("Final filter: {:?}", filter);

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

    // Estimated profit USD range filter
    if min_estimate_profit_usd.is_some() || max_estimate_profit_usd.is_some() {
        let mut est_profit_filter = doc! {};
        if let Some(min) = min_estimate_profit_usd {
            est_profit_filter.insert("$gte", min);
        }
        if let Some(max) = max_estimate_profit_usd {
            est_profit_filter.insert("$lte", max);
        }
        filter.insert("estimate_profit_usd", est_profit_filter);
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
                debug!(
                    "Applied min_created_at (Unix): {} -> filter: $gte: {}",
                    min_ts, unix_ts
                );
            } else if let Ok(dt) = DateTime::parse_from_rfc3339(min_ts) {
                // Parse ISO 8601 timestamp and convert to Unix timestamp
                let unix_ts = dt.timestamp() as u64;
                created_at_filter.insert("$gte", unix_ts as i64);
                has_filter = true;
                debug!(
                    "Applied min_created_at (ISO): {} -> filter: $gte: {}",
                    min_ts, unix_ts
                );
            } else {
                debug!("Invalid min_created_at format: {}", min_ts);
            }
        }

        if let Some(max_ts) = &max_created_at {
            // Try to parse as Unix timestamp first, then as ISO 8601
            if let Ok(unix_ts) = max_ts.parse::<u64>() {
                // Already a Unix timestamp
                created_at_filter.insert("$lte", unix_ts as i64);
                has_filter = true;
                debug!(
                    "Applied max_created_at (Unix): {} -> filter: $lte: {}",
                    max_ts, unix_ts
                );
            } else if let Ok(dt) = DateTime::parse_from_rfc3339(max_ts) {
                // Parse ISO 8601 timestamp and convert to Unix timestamp
                let unix_ts = dt.timestamp() as u64;
                created_at_filter.insert("$lte", unix_ts as i64);
                has_filter = true;
                debug!(
                    "Applied max_created_at (ISO): {} -> filter: $lte: {}",
                    max_ts, unix_ts
                );
            } else {
                debug!("Invalid max_created_at format: {}", max_ts);
            }
        }

        if has_filter {
            debug!("Final created_at filter: {:?}", created_at_filter);
            filter.insert("created_at", created_at_filter);
            debug!("Final complete filter: {:?}", filter);
        }
    }

    // Pagination parameters
    let page = page.unwrap_or(1);
    let limit = limit.unwrap_or(100).min(1000); // Cap at 1000
    let skip = (page - 1) * limit;

    // Get total count for pagination - MongoDB will automatically use the best index
    let total = opportunities_collection
        .count_documents(filter.clone(), None)
        .await?;
    let total_pages = ((total as f64) / (limit as f64)).ceil() as u32;

    // Build optimized aggregation pipeline with projection and token lookup
    let mut pipeline = vec![doc! { "$match": filter }];

    // Add sorting, pagination, and projection
    pipeline.push(doc! {
        "$sort": { "created_at": -1 }
    });

    pipeline.push(doc! {
        "$skip": skip as i64
    });

    pipeline.push(doc! {
        "$limit": limit as i64
    });

    // Project only the fields we need before doing the lookup
    pipeline.push(doc! {
        "$project": {
            "_id": 1,
            "network_id": 1,
            "status": 1,
            "profit_usd": 1,
            "estimate_profit_usd": 1,
            "estimate_profit": 1,
            "profit": 1,
            "gas_usd": 1,
            "created_at": 1,
            "source_tx": 1,
            "source_block_number": 1,
            "execute_block_number": 1,
            "profit_token": 1,
            "simulation_time": 1,
            "error": 1
        }
    });

    // Optimized lookup with index hint
    pipeline.push(doc! {
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
                },
                // Project only needed token fields
                doc! {
                    "$project": {
                        "name": 1,
                        "symbol": 1,
                        "decimals": 1
                    }
                },
                // Project only needed token fields (already done above)
            ],
            "as": "token_info"
        }
    });

    pipeline.push(doc! {
        "$addFields": {
            "token_data": {
                "$ifNull": [
                    { "$arrayElemAt": ["$token_info", 0] },
                    {}
                ]
            }
        }
    });

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
            estimate_profit_usd: doc.get_f64("estimate_profit_usd").ok(),
            estimate_profit: doc.get_str("estimate_profit").ok().map(|s| s.to_string()),
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
            simulation_time: doc.get_i64("simulation_time").ok().map(|n| n as u64),
            error: doc.get_str("error").ok().map(|s| s.to_string()),
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

    // Use MongoDB aggregation to fetch all related data in one query (simplified for unified model)
    let pipeline = vec![
        doc! {
            "$match": { "_id": object_id }
        },
        doc! {
            "$lookup": {
                "from": "networks",
                "localField": "network_id",
                "foreignField": "chain_id",
                "as": "network"
            }
        },
        doc! {
            "$lookup": {
                "from": "tokens",
                "localField": "network_id",
                "foreignField": "network_id",
                "as": "all_tokens"
            }
        },
        doc! {
            "$lookup": {
                "from": "pools",
                "localField": "network_id",
                "foreignField": "network_id",
                "as": "all_pools"
            }
        },
        doc! {
            "$unwind": {
                "path": "$network",
                "preserveNullAndEmptyArrays": false
            }
        },
    ];

    let mut cursor = opportunities_collection.aggregate(pipeline, None).await?;
    let aggregated_doc = match cursor.next().await {
        Some(Ok(doc)) => doc,
        Some(Err(e)) => return Err(e.into()),
        None => return Ok(None), // No results from aggregation
    };

    // Extract the network data
    let network_doc = aggregated_doc.get_document("network")?;
    let network: Network = bson::from_document(network_doc.clone())?;

    // Debug info is now part of the unified opportunity model - no need to extract separately

    // Extract all tokens and pools
    let all_tokens: Vec<crate::models::Token> = aggregated_doc
        .get_array("all_tokens")?
        .iter()
        .filter_map(|token_bson| {
            if let Some(token_doc) = token_bson.as_document() {
                bson::from_document::<crate::models::Token>(token_doc.clone()).ok()
            } else {
                None
            }
        })
        .collect();

    let all_pools: Vec<crate::models::Pool> = aggregated_doc
        .get_array("all_pools")?
        .iter()
        .filter_map(|pool_bson| {
            if let Some(pool_doc) = pool_bson.as_document() {
                bson::from_document::<crate::models::Pool>(pool_doc.clone()).ok()
            } else {
                None
            }
        })
        .collect();

    let mut path_tokens = Vec::new();
    let mut path_pools = Vec::new();

    if let Some(ref path) = opportunity.path {
        // Process the path: even indices are tokens, odd indices are pools
        debug!("Processing path with {} elements", path.len());

        // Create lookup maps for fast access
        let token_map: std::collections::HashMap<String, &crate::models::Token> = all_tokens
            .iter()
            .map(|token| (token.address.to_lowercase(), token))
            .collect();

        let pool_map: std::collections::HashMap<String, &crate::models::Pool> = all_pools
            .iter()
            .map(|pool| (pool.address.to_lowercase(), pool))
            .collect();

        for (index, address) in path.iter().enumerate() {
            debug!(
                "Path element {}: {} (type: {})",
                index,
                address,
                if index % 2 == 0 { "token" } else { "pool" }
            );

            if index % 2 == 0 {
                // Token - lookup from batch fetched data
                let address_lower = address.to_lowercase();
                if let Some(token) = token_map.get(&address_lower) {
                    debug!(
                        "Found token: {} - name: {:?}, symbol: {:?}",
                        token.address, token.name, token.symbol
                    );
                    path_tokens.push(crate::models::TokenResponse {
                        id: token.id.map(|id| id.to_hex()).unwrap_or_default(),
                        address: token.address.clone(),
                        name: token.name.clone(),
                        symbol: token.symbol.clone(),
                        decimals: token.decimals,
                        price: token.price,
                    });
                } else {
                    debug!("Token not found for address: {}", address_lower);
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
                // Pool - lookup from batch fetched data
                let address_lower = address.to_lowercase();
                if let Some(pool) = pool_map.get(&address_lower) {
                    debug!(
                        "Found pool: {} - type: {}, tokens: {:?}",
                        pool.address, pool.pool_type, pool.tokens
                    );
                    path_pools.push(crate::models::PoolResponse {
                        id: pool.id.map(|id| id.to_hex()).unwrap_or_default(),
                        address: pool.address.clone(),
                        pool_type: pool.pool_type.clone(),
                        tokens: pool.tokens.clone(),
                    });
                } else {
                    debug!("Pool not found for address: {}", address_lower);
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

    // Build the opportunity details response with all fields from both Opportunity and OpportunityDebug
    let opportunity_details = crate::models::OpportunityDetailsData {
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
        source_log_index: opportunity.source_log_index,
        source_pool: opportunity.source_pool.clone(),
        execute_block_number: opportunity.execute_block_number,
        execute_tx: opportunity.execute_tx.clone(),
        profit_token: opportunity.profit_token.clone(),
        profit_token_name: None, // Will be populated if profit_token is in path_tokens
        profit_token_symbol: None, // Will be populated if profit_token is in path_tokens
        profit_token_decimals: None, // Will be populated if profit_token is in path_tokens
        amount: Some(opportunity.amount.clone()),
        gas_token_amount: opportunity.gas_token_amount.clone(),
        updated_at: DateTime::from_timestamp(opportunity.updated_at as i64, 0)
            .unwrap_or_else(|| Utc::now())
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string(),

        // Debug fields from OpportunityDebug
        estimate_profit: opportunity.estimate_profit.clone(),
        estimate_profit_usd: opportunity.estimate_profit_usd,
        path: opportunity.path.clone(),
        received_at: opportunity.received_at.map(|ts| {
            let seconds = ts as i64 / 1000;
            let milliseconds = (ts % 1000) as u32;
            DateTime::from_timestamp(seconds, milliseconds * 1_000_000)
                .unwrap_or_else(|| Utc::now())
                .format("%Y-%m-%dT%H:%M:%S%.3fZ")
                .to_string()
        }),
        send_at: opportunity.send_at.map(|ts| {
            let seconds = ts as i64 / 1000;
            let milliseconds = (ts % 1000) as u32;
            DateTime::from_timestamp(seconds, milliseconds * 1_000_000)
                .unwrap_or_else(|| Utc::now())
                .format("%Y-%m-%dT%H:%M:%S%.3fZ")
                .to_string()
        }),
        simulation_time: opportunity.simulation_time,
        error: opportunity.error.clone(),
        gas_amount: opportunity.gas_amount,
        gas_price: opportunity.gas_price,
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
        router_address: network.router_address.clone(),
        executors: network.executors.clone(),
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
    let mut final_opportunity_response = opportunity_details;
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

pub async fn get_opportunity_details_by_tx_hash(
    db: &Database,
    tx_hash: &str,
) -> DbResult<Option<crate::models::OpportunityDetailsResponse>> {
    let opportunities_collection: Collection<Opportunity> = db.collection("opportunities");
    let networks_collection: Collection<Network> = db.collection("networks");
    let tokens_collection: Collection<crate::models::Token> = db.collection("tokens");
    let pools_collection: Collection<crate::models::Pool> = db.collection("pools");
    // Try to find opportunity by source_tx first, then by execute_tx
    let opportunity = match opportunities_collection
        .find_one(
            doc! {
                "$or": [
                    { "execute_tx": tx_hash }
                ]
            },
            None,
        )
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

    // Debug info is now part of the unified opportunity model
    let debug_info = Some(&opportunity);

    let mut path_tokens = Vec::new();
    let mut path_pools = Vec::new();

    if let Some(debug) = debug_info {
        if let Some(path) = &debug.path {
            // Process the path: even indices are tokens, odd indices are pools
            debug!("Processing path with {} elements", path.len());
            for (index, address) in path.iter().enumerate() {
                debug!(
                    "Path element {}: {} (type: {})",
                    index,
                    address,
                    if index % 2 == 0 { "token" } else { "pool" }
                );
                if index % 2 == 0 {
                    // Token - fetch token details
                    let address_lower = address.to_lowercase();
                    debug!(
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
                        debug!(
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
                        debug!("Token not found for address: {}", address_lower);
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
                    debug!(
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
                        debug!(
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
                        debug!("Pool not found for address: {}", address_lower);
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

    // Build the opportunity details response with all fields from both Opportunity and OpportunityDebug
    let opportunity_details = crate::models::OpportunityDetailsData {
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
        source_log_index: opportunity.source_log_index,
        source_pool: opportunity.source_pool.clone(),
        execute_block_number: opportunity.execute_block_number,
        execute_tx: opportunity.execute_tx.clone(),
        profit_token: opportunity.profit_token.clone(),
        profit_token_name: None, // Will be populated if profit_token is in path_tokens
        profit_token_symbol: None, // Will be populated if profit_token is in path_tokens
        profit_token_decimals: None, // Will be populated if profit_token is in path_tokens
        amount: Some(opportunity.amount.clone()),
        gas_token_amount: opportunity.gas_token_amount.clone(),
        updated_at: DateTime::from_timestamp(opportunity.updated_at as i64, 0)
            .unwrap_or_else(|| Utc::now())
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string(),

        // Debug fields from OpportunityDebug
        estimate_profit: opportunity.estimate_profit.clone(),
        estimate_profit_usd: opportunity.estimate_profit_usd,
        path: opportunity.path.clone(),
        received_at: opportunity.received_at.map(|ts| {
            let seconds = ts as i64 / 1000;
            let milliseconds = (ts % 1000) as u32;
            DateTime::from_timestamp(seconds, milliseconds * 1_000_000)
                .unwrap_or_else(|| Utc::now())
                .format("%Y-%m-%dT%H:%M:%S%.3fZ")
                .to_string()
        }),
        send_at: opportunity.send_at.map(|ts| {
            let seconds = ts as i64 / 1000;
            let milliseconds = (ts % 1000) as u32;
            DateTime::from_timestamp(seconds, milliseconds * 1_000_000)
                .unwrap_or_else(|| Utc::now())
                .format("%Y-%m-%dT%H:%M:%S%.3fZ")
                .to_string()
        }),
        simulation_time: opportunity.simulation_time,
        error: opportunity.error.clone(),
        gas_amount: opportunity.gas_amount,
        gas_price: opportunity.gas_price,
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
        router_address: network.router_address.clone(),
        executors: network.executors.clone(),
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
    let mut final_opportunity_details = opportunity_details;
    if let Some(token_info) = path_tokens
        .iter()
        .find(|t| t.address == opportunity.profit_token)
    {
        final_opportunity_details.profit_token_name = token_info.name.clone();
        final_opportunity_details.profit_token_symbol = token_info.symbol.clone();
        final_opportunity_details.profit_token_decimals = token_info.decimals;
    }

    Ok(Some(crate::models::OpportunityDetailsResponse {
        opportunity: final_opportunity_details,
        network: network_response,
        path_tokens,
        path_pools,
    }))
}

/// Get token performance data with network information
pub async fn get_token_performance(
    db: &Database,
    network_id: Option<u64>,
    limit: Option<u64>,
    offset: Option<u64>,
) -> DbResult<Vec<TokenPerformanceResponse>> {
    let tokens_collection: Collection<Token> = db.collection("tokens");
    let networks_collection: Collection<Network> = db.collection("networks");

    // Build filter for network_id if provided
    let mut filter = doc! {};
    if let Some(net_id) = network_id {
        filter.insert("network_id", net_id as i64);
    }

    // Only include tokens that have profit data
    filter.insert("total_profit_usd", doc! { "$gt": 0.0 });

    // Set up pagination
    let limit = limit.unwrap_or(50).min(1000); // Max 1000 results
    let offset = offset.unwrap_or(0);

    // Create find options with pagination and sorting
    let find_options = mongodb::options::FindOptions::builder()
        .sort(doc! { "total_profit_usd": -1 }) // Sort by profit descending
        .skip(offset as u64)
        .limit(limit as i64)
        .build();

    // Find tokens with profit data
    let mut cursor = tokens_collection
        .find(filter.clone(), Some(find_options))
        .await?;

    let mut tokens = Vec::new();
    while let Some(token) = cursor.next().await {
        tokens.push(token?);
    }

    // Get network information for all unique network_ids
    let network_ids: Vec<i64> = tokens
        .iter()
        .map(|t| t.network_id as i64)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let mut networks = std::collections::HashMap::new();
    if !network_ids.is_empty() {
        let mut network_cursor = networks_collection
            .find(doc! { "chain_id": { "$in": network_ids } }, None)
            .await?;

        while let Some(network) = network_cursor.next().await {
            let network = network?;
            networks.insert(network.chain_id, network);
        }
    }

    // Apply pagination and build response
    let start = offset as usize;
    let end = (offset + limit) as usize;
    let paginated_tokens = if start < tokens.len() {
        &tokens[start..end.min(tokens.len())]
    } else {
        &[]
    };

    let mut result = Vec::new();
    for token in paginated_tokens {
        let network = networks.get(&token.network_id);
        let network_name = network
            .map(|n| n.name.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        result.push(TokenPerformanceResponse {
            name: token.name.clone(),
            symbol: token.symbol.clone(),
            total_profit_usd: token.total_profit_usd,
            total_profit: token
                .total_profit
                .clone()
                .unwrap_or_else(|| "0".to_string()),
            price: token.price,
            address: token.address.clone(),
            network_id: token.network_id,
            network_name,
        });
    }

    Ok(result)
}

/// Create or update time aggregation for a specific period
pub async fn upsert_time_aggregation(
    db: &Database,
    network_id: u64,
    period: TimeAggregationPeriod,
    timestamp: u64,
    aggregation: TimeAggregation,
) -> DbResult<()> {
    let collection: Collection<TimeAggregation> = db.collection("time_aggregations");

    let filter = doc! {
        "network_id": network_id as i64,
        "period": period_to_string(period),
        "timestamp": timestamp as i64,
    };

    // First, try to find existing aggregation to merge tokens
    let existing_aggregation = collection.find_one(filter.clone(), None).await?;

    let merged_tokens = if let Some(existing) = existing_aggregation {
        // Merge tokens from existing and new aggregation
        merge_token_aggregations(&existing.top_profit_tokens, &aggregation.top_profit_tokens)
    } else {
        // No existing aggregation, use new tokens as-is
        aggregation.top_profit_tokens.clone()
    };

    let update = doc! {
        "$set": {
            "network_id": network_id as i64,
            "period": period_to_string(period),
            "timestamp": timestamp as i64,
            "period_start": aggregation.period_start,
            "period_end": aggregation.period_end,
            "top_profit_tokens": bson::to_bson(&merged_tokens)?,
            "updated_at": Utc::now().timestamp() as i64,
        },
        "$inc": {
            "total_opportunities": aggregation.total_opportunities as i64,
            "executed_opportunities": aggregation.executed_opportunities as i64,
            "successful_opportunities": aggregation.successful_opportunities as i64,
            "failed_opportunities": aggregation.failed_opportunities as i64,
            "total_profit_usd": aggregation.total_profit_usd,
            "total_gas_usd": aggregation.total_gas_usd,
        },
        "$setOnInsert": {
            "created_at": Utc::now().timestamp() as i64,
        }
    };

    collection
        .update_one(
            filter,
            update,
            mongodb::options::UpdateOptions::builder()
                .upsert(true)
                .build(),
        )
        .await?;

    Ok(())
}

/// Prune old hourly data based on retention period
pub async fn prune_old_hourly_data(db: &Database, retention_hours: u64) -> DbResult<()> {
    let time_aggregations_collection: Collection<TimeAggregation> =
        db.collection("time_aggregations");
    let summary_aggregations_collection: Collection<SummaryAggregation> =
        db.collection("summary_aggregations");

    // Calculate cutoff timestamp (retention_hours ago)
    let cutoff_timestamp = Utc::now()
        .checked_sub_signed(chrono::Duration::hours(retention_hours as i64))
        .unwrap_or(Utc::now())
        .timestamp() as u64;

    info!(
        "Pruning hourly data older than {} hours (before timestamp {})",
        retention_hours, cutoff_timestamp
    );

    // Delete old hourly time aggregations
    let time_filter = doc! {
        "period": "hourly",
        "timestamp": { "$lt": cutoff_timestamp as i64 }
    };

    let time_result = time_aggregations_collection
        .delete_many(time_filter, None)
        .await?;
    info!(
        "Deleted {} old hourly time aggregations",
        time_result.deleted_count
    );

    // Delete old hourly summary aggregations
    let summary_filter = doc! {
        "period": "hourly",
        "timestamp": { "$lt": cutoff_timestamp as i64 }
    };

    let summary_result = summary_aggregations_collection
        .delete_many(summary_filter, None)
        .await?;
    info!(
        "Deleted {} old hourly summary aggregations",
        summary_result.deleted_count
    );

    let total_deleted = time_result.deleted_count + summary_result.deleted_count;
    info!("Total deleted: {} old hourly records", total_deleted);

    Ok(())
}

/// Prune old opportunities that are older than 7 days and have low profit values
pub async fn prune_old_opportunities(db: &Database) -> DbResult<()> {
    let opportunities_collection: Collection<Opportunity> = db.collection("opportunities");

    // Calculate cutoff timestamp (7 days ago)
    let cutoff_timestamp = Utc::now()
        .checked_sub_signed(chrono::Duration::days(7))
        .unwrap_or(Utc::now())
        .timestamp() as u64;

    info!(
        "Pruning opportunities older than 7 days (before timestamp {}) with low profit values",
        cutoff_timestamp
    );

    // Create filter for opportunities that are:
    // 1. Older than 7 days
    // 2. Have BOTH profit_usd AND estimate_profit_usd either null, missing, or < 1.0
    let filter = doc! {
        "created_at": { "$lt": cutoff_timestamp as i64 },
        "$and": [
            {
                "$or": [
                    { "profit_usd": { "$lt": 1.0 } },
                    { "profit_usd": null },
                    { "profit_usd": { "$exists": false } }
                ]
            },
            {
                "$or": [
                    { "estimate_profit_usd": { "$lt": 1.0 } },
                    { "estimate_profit_usd": null },
                    { "estimate_profit_usd": { "$exists": false } }
                ]
            }
        ]
    };

    // First, count how many records will be deleted for logging
    let count_result = opportunities_collection
        .count_documents(filter.clone(), None)
        .await?;
    info!("Found {} opportunities to prune", count_result);

    if count_result == 0 {
        info!("No opportunities found matching pruning criteria");
        return Ok(());
    }

    // Delete the matching opportunities
    let delete_result = opportunities_collection.delete_many(filter, None).await?;

    info!(
        "Successfully pruned {} old opportunities with low profit values",
        delete_result.deleted_count
    );

    Ok(())
}

/// Get time aggregations with filtering
pub async fn get_time_aggregations(
    db: &Database,
    network_id: Option<u64>,
    period: Option<String>,
    start_time: Option<String>,
    end_time: Option<String>,
    limit: Option<u64>,
    offset: Option<u64>,
) -> DbResult<Vec<TimeAggregationResponse>> {
    let collection: Collection<TimeAggregation> = db.collection("time_aggregations");

    // Build filter
    let mut filter = doc! {};
    if let Some(net_id) = network_id {
        filter.insert("network_id", net_id as i64);
    }
    if let Some(period_str) = period {
        filter.insert("period", period_str);
    }

    // Time range filter
    if start_time.is_some() || end_time.is_some() {
        let mut time_filter = doc! {};
        if let Some(start) = start_time {
            let start_ts = parse_time_to_timestamp(&start)?;
            time_filter.insert("$gte", start_ts as i64);
        }
        if let Some(end) = end_time {
            let end_ts = parse_time_to_timestamp(&end)?;
            time_filter.insert("$lte", end_ts as i64);
        }
        filter.insert("timestamp", time_filter);
    }

    // Set up pagination
    let limit = limit.unwrap_or(100).min(1000);
    let offset = offset.unwrap_or(0);

    // Build aggregation pipeline
    let pipeline = vec![
        // Match stage with our filter
        doc! { "$match": filter },
        // Sort by timestamp descending
        doc! { "$sort": { "timestamp": -1 } },
        // Skip for pagination
        doc! { "$skip": offset as i64 },
        // Limit for pagination
        doc! { "$limit": limit as i64 },
        // Lookup networks to get network names
        doc! {
            "$lookup": {
                "from": "networks",
                "let": { "network_id": "$network_id" },
                "pipeline": [
                    doc! {
                        "$match": {
                            "$expr": { "$eq": ["$chain_id", "$$network_id"] }
                        }
                    },
                    doc! { "$project": { "_id": 0, "name": 1, "chain_id": 1 } }
                ],
                "as": "network_info"
            }
        },
        // Add calculated fields and network name
        doc! {
            "$addFields": {
                "network_name": {
                    "$ifNull": [
                        { "$arrayElemAt": ["$network_info.name", 0] },
                        "Unknown"
                    ]
                }
            }
        },
    ];

    // Execute the aggregation pipeline
    let mut cursor = collection.aggregate(pipeline, None).await?;

    // Collect results directly from the aggregation
    let mut result = Vec::new();
    while let Some(doc) = cursor.next().await {
        let doc = doc?;

        // Extract basic fields
        let network_id = doc.get_i64("network_id").unwrap_or_default() as u64;
        let network_name = doc.get_str("network_name").unwrap_or("Unknown").to_string();
        let period = doc.get_str("period").unwrap_or_default().to_string();
        let timestamp = doc.get_i64("timestamp").unwrap_or_default() as u64;
        let period_start = doc.get_str("period_start").unwrap_or_default().to_string();
        let period_end = doc.get_str("period_end").unwrap_or_default().to_string();
        let total_opportunities = doc.get_i64("total_opportunities").unwrap_or_default() as u64;
        let executed_opportunities =
            doc.get_i64("executed_opportunities").unwrap_or_default() as u64;
        let successful_opportunities =
            doc.get_i64("successful_opportunities").unwrap_or_default() as u64;
        let failed_opportunities = doc.get_i64("failed_opportunities").unwrap_or_default() as u64;
        let total_profit_usd = doc.get_f64("total_profit_usd").unwrap_or_default();
        let total_gas_usd = doc.get_f64("total_gas_usd").unwrap_or_default();

        // Calculate derived fields
        let avg_profit_usd = if total_opportunities > 0 {
            total_profit_usd / total_opportunities as f64
        } else {
            0.0
        };

        let avg_gas_usd = if total_opportunities > 0 {
            total_gas_usd / total_opportunities as f64
        } else {
            0.0
        };

        let success_rate = if executed_opportunities > 0 {
            successful_opportunities as f64 / executed_opportunities as f64
        } else {
            0.0
        };

        // Parse token aggregations
        let mut top_profit_tokens = Vec::new();
        if let Ok(tokens) = doc.get_array("top_profit_tokens") {
            for token_bson in tokens {
                if let Ok(token_doc) =
                    bson::from_bson::<mongodb::bson::Document>(token_bson.clone())
                {
                    top_profit_tokens.push(TokenAggregationResponse {
                        address: token_doc.get_str("address").unwrap_or_default().to_string(),
                        name: token_doc.get_str("name").ok().map(|s| s.to_string()),
                        symbol: token_doc.get_str("symbol").ok().map(|s| s.to_string()),
                        total_profit_usd: token_doc.get_f64("profit_usd").unwrap_or_default(),
                        total_profit: token_doc.get_str("profit").unwrap_or_default().to_string(),
                        opportunity_count: token_doc.get_i64("count").unwrap_or_default() as u64,
                        avg_profit_usd: if token_doc.get_i64("count").unwrap_or_default() > 0 {
                            token_doc.get_f64("profit_usd").unwrap_or_default()
                                / token_doc.get_i64("count").unwrap_or_default() as f64
                        } else {
                            0.0
                        },
                    });
                }
            }
        }

        // Use the period_start and period_end directly since they're already ISO strings
        result.push(TimeAggregationResponse {
            network_id,
            network_name,
            period,
            timestamp,
            period_start,
            period_end,
            total_opportunities,
            executed_opportunities,
            successful_opportunities,
            failed_opportunities,
            total_profit_usd,
            total_gas_usd,
            avg_profit_usd,
            avg_gas_usd,
            success_rate,
            top_profit_tokens,
        });
    }

    Ok(result)
}

/// Merge two token aggregation lists by combining data for the same tokens
fn merge_token_aggregations(
    existing_tokens: &[TokenAggregation],
    new_tokens: &[TokenAggregation],
) -> Vec<TokenAggregation> {
    use alloy::primitives::U256;
    use std::collections::HashMap;

    // Create a map of existing tokens by address
    let mut token_map: HashMap<String, TokenAggregation> = existing_tokens
        .iter()
        .map(|t| (t.address.clone(), t.clone()))
        .collect();

    // Merge new tokens into the map
    for new_token in new_tokens {
        let entry = token_map
            .entry(new_token.address.clone())
            .or_insert_with(|| TokenAggregation {
                address: new_token.address.clone(),
                name: new_token.name.clone(),
                symbol: new_token.symbol.clone(),
                total_profit_usd: 0.0,
                total_profit: "0".to_string(),
                opportunity_count: 0,
                avg_profit_usd: 0.0,
            });

        // Add USD profit
        entry.total_profit_usd += new_token.total_profit_usd;
        entry.opportunity_count += new_token.opportunity_count;

        // Add raw profit using U256
        if let (Ok(existing_u256), Ok(new_u256)) = (
            entry.total_profit.parse::<U256>(),
            new_token.total_profit.parse::<U256>(),
        ) {
            entry.total_profit = existing_u256.saturating_add(new_u256).to_string();
        } else {
            // If parsing fails, just use the new value
            entry.total_profit = new_token.total_profit.clone();
        }

        // Update name/symbol if not set
        if entry.name.is_none() && new_token.name.is_some() {
            entry.name = new_token.name.clone();
        }
        if entry.symbol.is_none() && new_token.symbol.is_some() {
            entry.symbol = new_token.symbol.clone();
        }
    }

    // Convert back to vector and sort by total profit USD
    let mut merged_tokens: Vec<TokenAggregation> = token_map.into_values().collect();
    merged_tokens.sort_by(|a, b| {
        b.total_profit_usd
            .partial_cmp(&a.total_profit_usd)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Take top 10 tokens
    merged_tokens.truncate(10);

    // Calculate averages for the merged tokens
    for token in &mut merged_tokens {
        if token.opportunity_count > 0 {
            token.avg_profit_usd = token.total_profit_usd / token.opportunity_count as f64;
        }
    }

    merged_tokens
}

/// Helper function to convert TimeAggregationPeriod to string
fn period_to_string(period: TimeAggregationPeriod) -> String {
    match period {
        TimeAggregationPeriod::Hourly => "hourly".to_string(),
        TimeAggregationPeriod::Daily => "daily".to_string(),
        TimeAggregationPeriod::Monthly => "monthly".to_string(),
    }
}

/// Helper function to parse time string to timestamp
fn parse_time_to_timestamp(time_str: &str) -> DbResult<u64> {
    // Try parsing as Unix timestamp first
    if let Ok(ts) = time_str.parse::<u64>() {
        return Ok(ts);
    }

    // Try parsing as ISO 8601
    if let Ok(dt) = DateTime::parse_from_rfc3339(time_str) {
        return Ok(dt.timestamp() as u64);
    }

    // Try parsing as common date formats
    let formats = [
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d %H:%M",
        "%Y-%m-%d",
        "%Y/%m/%d %H:%M:%S",
        "%Y/%m/%d %H:%M",
        "%Y/%m/%d",
    ];

    for format in &formats {
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(time_str, format) {
            return Ok(dt.and_utc().timestamp() as u64);
        }
    }

    Err("Invalid time format".into())
}

/// Create or update summary aggregation for a specific period (cross-network totals)
pub async fn upsert_summary_aggregation(
    db: &Database,
    period: TimeAggregationPeriod,
    timestamp: u64,
    aggregation: SummaryAggregation,
) -> DbResult<()> {
    let collection: Collection<SummaryAggregation> = db.collection("summary_aggregations");

    let filter = doc! {
        "period": period_to_string(period),
        "timestamp": timestamp as i64,
    };

    let update = doc! {
        "$set": {
            "period": period_to_string(period),
            "timestamp": timestamp as i64,
            "period_start": aggregation.period_start,
            "period_end": aggregation.period_end,
            "updated_at": Utc::now().timestamp() as i64,
        },
        "$inc": {
            "total_opportunities": aggregation.total_opportunities as i64,
            "executed_opportunities": aggregation.executed_opportunities as i64,
            "successful_opportunities": aggregation.successful_opportunities as i64,
            "failed_opportunities": aggregation.failed_opportunities as i64,
            "total_profit_usd": aggregation.total_profit_usd,
            "total_gas_usd": aggregation.total_gas_usd,
        },
        "$setOnInsert": {
            "created_at": Utc::now().timestamp() as i64,
        }
    };

    collection
        .update_one(
            filter,
            update,
            mongodb::options::UpdateOptions::builder()
                .upsert(true)
                .build(),
        )
        .await?;

    Ok(())
}

/// Get summary aggregations with filtering
pub async fn get_summary_aggregations(
    db: &Database,
    period: Option<String>,
    start_time: Option<String>,
    end_time: Option<String>,
    limit: Option<u64>,
    offset: Option<u64>,
) -> DbResult<Vec<SummaryAggregationResponse>> {
    let collection: Collection<SummaryAggregation> = db.collection("summary_aggregations");

    // Build filter
    let mut filter = doc! {};
    if let Some(period_str) = period {
        filter.insert("period", period_str);
    }

    // Time range filter
    if start_time.is_some() || end_time.is_some() {
        let mut time_filter = doc! {};
        if let Some(start) = start_time {
            let start_ts = parse_time_to_timestamp(&start)?;
            time_filter.insert("$gte", start_ts as i64);
        }
        if let Some(end) = end_time {
            let end_ts = parse_time_to_timestamp(&end)?;
            time_filter.insert("$lte", end_ts as i64);
        }
        filter.insert("timestamp", time_filter);
    }

    // Set up pagination
    let limit = limit.unwrap_or(100).min(1000);
    let offset = offset.unwrap_or(0);

    // Create find options with pagination
    let find_options = mongodb::options::FindOptions::builder()
        .limit(limit as i64)
        .skip(offset as u64)
        .sort(doc! { "timestamp": -1 }) // Sort by timestamp descending (newest first)
        .build();

    // Find aggregations with pagination
    let mut cursor = collection.find(filter.clone(), Some(find_options)).await?;

    let mut aggregations = Vec::new();
    while let Some(agg) = cursor.next().await {
        aggregations.push(agg?);
    }

    // Build response
    let mut result = Vec::new();
    for agg in aggregations {
        result.push(SummaryAggregationResponse {
            period: agg.period.clone(),
            timestamp: agg.timestamp,
            period_start: agg.period_start.clone(),
            period_end: agg.period_end.clone(),
            total_opportunities: agg.total_opportunities,
            executed_opportunities: agg.executed_opportunities,
            successful_opportunities: agg.successful_opportunities,
            failed_opportunities: agg.failed_opportunities,
            total_profit_usd: agg.total_profit_usd,
            total_gas_usd: agg.total_gas_usd,
        });
    }

    Ok(result)
}

/// Create database indexes for better query performance
pub async fn create_database_indexes(db: &Database) -> DbResult<()> {
    info!("Creating database indexes for performance optimization...");

    // Create indexes with options
    let mut index_options = IndexOptions::default();
    index_options.background = Some(true);

    // ===== OPPORTUNITIES COLLECTION INDEXES =====
    info!("Creating indexes for opportunities collection...");
    let opportunities_collection: Collection<Opportunity> = db.collection("opportunities");

    // Create index models using the builder pattern
    let network_status_time_index = mongodb::IndexModel::builder()
        .keys(doc! {
            "network_id": 1,
            "status": 1,
            "created_at": -1
        })
        .options(Some(index_options.clone()))
        .build();

    // Compound index for profit filtering: network_id + profit_usd + created_at
    let network_profit_time_index = mongodb::IndexModel::builder()
        .keys(doc! {
            "network_id": 1,
            "profit_usd": 1,
            "created_at": -1
        })
        .options(Some(index_options.clone()))
        .build();

    // Compound index for estimate profit filtering: network_id + estimate_profit_usd + created_at
    let network_estimate_profit_time_index = mongodb::IndexModel::builder()
        .keys(doc! {
            "network_id": 1,
            "estimate_profit_usd": 1,
            "created_at": -1
        })
        .options(Some(index_options.clone()))
        .build();

    // Compound index for gas filtering: network_id + gas_usd + created_at
    let network_gas_time_index = mongodb::IndexModel::builder()
        .keys(doc! {
            "network_id": 1,
            "gas_usd": 1,
            "created_at": -1
        })
        .options(Some(index_options.clone()))
        .build();

    // Single field indexes for common filters
    let status_index = mongodb::IndexModel::builder()
        .keys(doc! { "status": 1 })
        .options(Some(index_options.clone()))
        .build();

    let created_at_index = mongodb::IndexModel::builder()
        .keys(doc! { "created_at": -1 })
        .options(Some(index_options.clone()))
        .build();

    let profit_token_index = mongodb::IndexModel::builder()
        .keys(doc! { "profit_token": 1 })
        .options(Some(index_options.clone()))
        .build();

    // Create all indexes
    let opportunity_indexes = vec![
        ("network_status_time", network_status_time_index),
        ("network_profit_time", network_profit_time_index),
        (
            "network_estimate_profit_time",
            network_estimate_profit_time_index,
        ),
        ("network_gas_time", network_gas_time_index),
        ("status", status_index),
        ("created_at", created_at_index),
        ("profit_token", profit_token_index),
    ];

    for (name, index_model) in opportunity_indexes {
        match opportunities_collection
            .create_index(index_model, None)
            .await
        {
            Ok(_) => info!("Created opportunity index: {}", name),
            Err(e) => {
                if e.to_string().contains("already exists") {
                    debug!("Opportunity index {} already exists", name);
                } else {
                    warn!("Failed to create opportunity index {}: {}", name, e);
                }
            }
        }
    }

    // ===== TIME AGGREGATIONS COLLECTION INDEXES =====
    info!("Creating indexes for time_aggregations collection...");
    let time_aggregations_collection: Collection<TimeAggregation> =
        db.collection("time_aggregations");

    // Compound index for time aggregation queries: network_id + period + timestamp
    let time_agg_network_period_time_index = mongodb::IndexModel::builder()
        .keys(doc! {
            "network_id": 1,
            "period": 1,
            "timestamp": -1
        })
        .options(Some(index_options.clone()))
        .build();

    // Index for timestamp-based queries
    let time_agg_timestamp_index = mongodb::IndexModel::builder()
        .keys(doc! { "timestamp": -1 })
        .options(Some(index_options.clone()))
        .build();

    // Index for period-based queries
    let time_agg_period_index = mongodb::IndexModel::builder()
        .keys(doc! { "period": 1 })
        .options(Some(index_options.clone()))
        .build();

    let time_agg_indexes = vec![
        (
            "network_period_timestamp",
            time_agg_network_period_time_index,
        ),
        ("timestamp", time_agg_timestamp_index),
        ("period", time_agg_period_index),
    ];

    for (name, index_model) in time_agg_indexes {
        match time_aggregations_collection
            .create_index(index_model, None)
            .await
        {
            Ok(_) => info!("Created time aggregation index: {}", name),
            Err(e) => {
                if e.to_string().contains("already exists") {
                    debug!("Time aggregation index {} already exists", name);
                } else {
                    warn!("Failed to create time aggregation index {}: {}", name, e);
                }
            }
        }
    }

    // ===== SUMMARY AGGREGATIONS COLLECTION INDEXES =====
    info!("Creating indexes for summary_aggregations collection...");
    let summary_aggregations_collection: Collection<SummaryAggregation> =
        db.collection("summary_aggregations");

    // Compound index for summary aggregation queries: period + timestamp
    let summary_agg_period_time_index = mongodb::IndexModel::builder()
        .keys(doc! {
            "period": 1,
            "timestamp": -1
        })
        .options(Some(index_options.clone()))
        .build();

    // Index for timestamp-based queries
    let summary_agg_timestamp_index = mongodb::IndexModel::builder()
        .keys(doc! { "timestamp": -1 })
        .options(Some(index_options.clone()))
        .build();

    let summary_agg_indexes = vec![
        ("period_timestamp", summary_agg_period_time_index),
        ("timestamp", summary_agg_timestamp_index),
    ];

    for (name, index_model) in summary_agg_indexes {
        match summary_aggregations_collection
            .create_index(index_model, None)
            .await
        {
            Ok(_) => info!("Created summary aggregation index: {}", name),
            Err(e) => {
                if e.to_string().contains("already exists") {
                    debug!("Summary aggregation index {} already exists", name);
                } else {
                    warn!("Failed to create summary aggregation index {}: {}", name, e);
                }
            }
        }
    }

    // ===== TOKENS COLLECTION INDEXES =====
    info!("Creating indexes for tokens collection...");
    let tokens_collection: Collection<crate::models::Token> = db.collection("tokens");

    // Compound index for token lookup by network and address
    let token_network_address_index = mongodb::IndexModel::builder()
        .keys(doc! {
            "network_id": 1,
            "address": 1
        })
        .options(Some(index_options.clone()))
        .build();

    // Index for token symbol searches
    let token_symbol_index = mongodb::IndexModel::builder()
        .keys(doc! { "symbol": 1 })
        .options(Some(index_options.clone()))
        .build();

    // Index for token name searches
    let token_name_index = mongodb::IndexModel::builder()
        .keys(doc! { "name": 1 })
        .options(Some(index_options.clone()))
        .build();

    let token_indexes = vec![
        ("network_address", token_network_address_index),
        ("symbol", token_symbol_index),
        ("name", token_name_index),
    ];

    for (name, index_model) in token_indexes {
        match tokens_collection.create_index(index_model, None).await {
            Ok(_) => info!("Created token index: {}", name),
            Err(e) => {
                if e.to_string().contains("already exists") {
                    debug!("Token index {} already exists", name);
                } else {
                    warn!("Failed to create token index {}: {}", name, e);
                }
            }
        }
    }

    info!("Database indexes creation completed");
    Ok(())
}
