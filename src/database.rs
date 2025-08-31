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
    let opportunities_collection: Collection<Opportunity> = db.collection("opportunities");

    let mut networks = Vec::new();
    let mut cursor = networks_collection.find(doc! {}, None).await?;

    while let Some(network) = cursor.next().await {
        let network = network?;

        // Count executed opportunities for this network
        let executed_count = opportunities_collection
            .count_documents(
                doc! {
                    "network_id": network.chain_id as i64,
                    "status": "succeeded"
                },
                None,
            )
            .await?;

        let response = crate::models::NetworkResponse {
            chain_id: network.chain_id,
            name: network.name,
            total_profit_usd: network.total_profit_usd,
            total_gas_usd: network.total_gas_usd,
            executed_opportunities: executed_count,
        };

        networks.push(response);
    }

    Ok(networks)
}

pub async fn get_opportunities(
    db: &Database,
    network_id: Option<u64>,
    status: Option<String>,
) -> DbResult<Vec<crate::models::OpportunityResponse>> {
    let opportunities_collection: Collection<Opportunity> = db.collection("opportunities");

    let mut filter = doc! {};

    if let Some(nid) = network_id {
        filter.insert("network_id", nid as i64);
    }

    if let Some(s) = status {
        filter.insert("status", s);
    }

    let mut opportunities = Vec::new();
    let mut cursor = opportunities_collection.find(filter, None).await?;

    while let Some(opportunity) = cursor.next().await {
        let opportunity = opportunity?;

        // Convert timestamp to ISO 8601 string
        let created_at = DateTime::from_timestamp(opportunity.created_at as i64, 0)
            .unwrap_or_else(|| Utc::now())
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();

        let response = crate::models::OpportunityResponse {
            network_id: opportunity.network_id,
            status: opportunity.status,
            profit_usd: opportunity.profit_usd,
            gas_usd: opportunity.gas_usd,
            created_at,
        };

        opportunities.push(response);
    }

    Ok(opportunities)
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
