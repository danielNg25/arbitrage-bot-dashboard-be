use actix_cors::Cors;
use actix_web::{middleware::Logger, web, App, HttpServer};
use log::info;
use std::sync::Arc;
use tokio::sync::mpsc;

mod config;
mod contract;
mod database;
mod errors;
mod handlers;
mod indexer;
mod models;
mod notification_handler;
mod pool_indexer;
mod routes;
mod utils;

use config::Config;
use database::init_database;
use indexer::Indexer;
use notification_handler::NotificationHandler;
use pool_indexer::PoolIndexer;
use routes::configure_routes;

use crate::indexer::NotificationConfig;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load configuration
    let config = Config::load().expect("Failed to load configuration");

    // Initialize logging
    env_logger::init_from_env(env_logger::Env::new().default_filter_or(&config.server.log_level));

    info!("Starting Arbitrage Bot API...");
    info!("Configuration loaded: {:?}", config);

    // Initialize database connection
    let db = init_database(&config.database)
        .await
        .expect("Failed to initialize database");

    // Create shared database connection
    let db_arc = Arc::new(db.clone());
    let (notification_tx, mut notification_rx) = mpsc::channel(100);

    // Start the background indexer
    let indexer = Indexer::new(
        db_arc.clone(),
        config.indexer.interval_minutes,
        config.indexer.hourly_data_retention_hours,
        Arc::new(NotificationConfig::new(
            notification_tx,
            config.telegram.min_profit_usd,
        )),
    );
    indexer.start().await;
    info!(
        "Background indexer started ({} minute interval)",
        config.indexer.interval_minutes
    );

    // Initialize notification handler if configured
    let notification_handler_option = NotificationHandler::from_config(&config, db_arc.clone());

    if let Some(notification_handler) = notification_handler_option {
        let notification_handler = Arc::new(notification_handler);

        if notification_handler.is_configured() {
            info!("Telegram notification handler initialized");

            // Clone for the notification receiver task
            let notification_handler_for_rx = notification_handler.clone();

            tokio::spawn(async move {
                while let Some(opportunity) = notification_rx.recv().await {
                    let handler_clone = notification_handler_for_rx.clone();
                    tokio::spawn(async move { handler_clone.send_notification(opportunity).await });
                }
            });

            // Start the pool indexer if enabled
            if config.pool_indexer.enabled {
                let pool_indexer =
                    PoolIndexer::from_config(db_arc.clone(), notification_handler.clone(), &config);
                pool_indexer.start().await;
                info!(
                    "Pool indexer started ({} second interval)",
                    config.pool_indexer.interval_seconds
                );
            }
        } else {
            info!("Telegram notification handler is not properly configured");
        }
    } else {
        info!("Telegram notification handler not initialized (missing configuration)");
    }

    // Build bind address from config
    let bind_addr = format!("{}:{}", config.server.host, config.server.port);

    info!("Server will be available at http://{}", bind_addr);

    HttpServer::new(move || {
        // Configure CORS from config
        let mut cors = Cors::default();

        for origin in &config.cors.allowed_origins {
            cors = cors.allowed_origin(origin);
        }

        // Convert string methods to HTTP methods
        let methods: Vec<actix_web::http::Method> = config
            .cors
            .allowed_methods
            .iter()
            .filter_map(|m| m.parse().ok())
            .collect();

        // Add WebSocket-specific headers and methods
        let mut all_headers = config.cors.allowed_headers.clone();
        all_headers.extend_from_slice(&[
            "Upgrade".to_string(),
            "Connection".to_string(),
            "Sec-WebSocket-Key".to_string(),
            "Sec-WebSocket-Version".to_string(),
            "Sec-WebSocket-Protocol".to_string(),
        ]);

        let mut all_methods = methods;
        all_methods.push(actix_web::http::Method::from_bytes(b"OPTIONS").unwrap());

        cors = cors
            .allowed_methods(all_methods)
            .allowed_headers(all_headers)
            .expose_headers(vec![
                "Upgrade".to_string(),
                "Connection".to_string(),
                "Sec-WebSocket-Accept".to_string(),
            ]);

        if config.cors.supports_credentials {
            cors = cors.supports_credentials();
        }

        App::new()
            .app_data(web::Data::new(db.clone()))
            .wrap(cors)
            .wrap(Logger::default())
            .configure(configure_routes)
    })
    .bind(&bind_addr)?
    .run()
    .await
}
