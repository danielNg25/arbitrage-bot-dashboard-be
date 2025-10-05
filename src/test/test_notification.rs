use arbitrage_bot_api::{
    config::Config,
    database::init_database,
    models::Opportunity,
    notification_handler::{NotificationHandler, NotificationType},
};
use log::info;
use mongodb::bson::oid::ObjectId;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("debug"));

    // Load configuration
    let config = Config::load().expect("Failed to load configuration");

    // Initialize database connection
    let db = init_database(&config.database)
        .await
        .expect("Failed to initialize database");

    // Create shared database connection
    let db_arc = Arc::new(db);

    // Initialize notification handler
    let notification_handler = match NotificationHandler::from_config(&config, db_arc.clone()) {
        Some(handler) => handler,
        None => {
            eprintln!(
                "Failed to initialize notification handler. Check your telegram configuration."
            );
            return Ok(());
        }
    };

    if !notification_handler.is_configured() {
        eprintln!("Notification handler is not properly configured.");
        return Ok(());
    }

    info!("Notification handler initialized successfully");

    // Create a test opportunity for each status
    let statuses = vec!["succeeded", "partially_succeeded", "reverted", "error"];

    for (i, status) in statuses.iter().enumerate() {
        // Create a test opportunity
        let opportunity = create_test_opportunity(status, i as u64);

        info!("Sending test notification for status: {}", status);

        // Send notification
        notification_handler
            .send_notification(NotificationType::HighOpportunity(opportunity))
            .await;

        // Wait a bit between notifications
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }

    info!("All test notifications sent");

    Ok(())
}

fn create_test_opportunity(status: &str, id: u64) -> Opportunity {
    let network_id = 14; // Ethereum mainnet
    let profit_token = "0x1D80c49BbBCd1C0911346656B529DF9E5c2F783d".to_string(); // WETH

    let mut opportunity = Opportunity {
        id: Some(ObjectId::new()),
        network_id,
        source_block_number: Some(18000000 + id),
        source_tx: Some(format!(
            "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcde{}",
            id
        )),
        source_log_index: Some(1),
        source_pool: Some("0xabcdef1234567890abcdef1234567890abcdef12".to_string()),
        status: status.to_string(),
        execute_block_number: Some(18000001 + id),
        execute_tx: Some(format!(
            "0xfedcba0987654321fedcba0987654321fedcba0987654321fedcba098765432{}",
            id
        )),
        profit_token,
        amount: "1000000000000000000".to_string(), // 1 ETH
        profit: Some("1000000000000000000".to_string()), // 1 ETH
        profit_usd: Some(3500.0),                  // $3500
        gas_token_amount: Some("50000000000000000".to_string()), // 0.05 ETH
        gas_usd: Some(175.0),                      // $175
        estimate_profit: Some("1200000000000000000".to_string()), // 1.2 ETH
        estimate_profit_usd: Some(4200.0),         // $4200
        path: Some(vec![
            "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2".to_string(), // WETH
            "0x1f9840a85d5af5bf1d1762f925bdaddc4201f984".to_string(), // UNI
            "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48".to_string(), // USDC
        ]),
        received_at: Some(1633042800000),
        send_at: Some(1633042801000),
        simulation_time: Some(500), // 500ms
        error: if status == "error" || status == "reverted" {
            Some("Transaction reverted: insufficient output amount".to_string())
        } else {
            None
        },
        gas_amount: Some(250000),
        gas_price: Some(50000000000), // 50 gwei
        created_at: chrono::Utc::now().timestamp() as u64,
        updated_at: chrono::Utc::now().timestamp() as u64,
    };

    // Modify opportunity based on status
    match status {
        "succeeded" => {
            // Keep default values
        }
        "partially_succeeded" => {
            opportunity.profit = Some("500000000000000000".to_string()); // 0.5 ETH
            opportunity.profit_usd = Some(1750.0); // $1750
        }
        "reverted" => {
            opportunity.profit = None;
            opportunity.profit_usd = None;
        }
        "error" => {
            opportunity.profit = None;
            opportunity.profit_usd = None;
            opportunity.execute_tx = None;
            opportunity.execute_block_number = None;
            opportunity.gas_usd = None;
        }
        _ => {}
    }

    opportunity
}
