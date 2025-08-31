use actix_web::{web, HttpResponse, Result};
use log::{error, info};
use mongodb::Database;

use crate::{
    database::{get_networks_with_stats, get_opportunities, get_profit_over_time},
    errors::ApiError,
    models::OpportunityQuery,
};

/// GET /networks - Returns all networks with statistics
pub async fn get_networks(db: web::Data<Database>) -> Result<HttpResponse, ApiError> {
    info!("Handling GET /networks request");
    
    match get_networks_with_stats(&db).await {
        Ok(networks) => {
            info!("Successfully retrieved {} networks", networks.len());
            Ok(HttpResponse::Ok().json(networks))
        }
        Err(e) => {
            error!("Failed to retrieve networks: {}", e);
            Err(ApiError::DatabaseError(e.to_string()))
        }
    }
}

/// GET /opportunities - Returns opportunities with optional filtering
pub async fn get_opportunities_handler(
    db: web::Data<Database>,
    query: web::Query<OpportunityQuery>,
) -> Result<HttpResponse, ApiError> {
    info!("Handling GET /opportunities request with query: {:?}", query);
    
    match get_opportunities(&db, query.network_id, query.status.clone()).await {
        Ok(opportunities) => {
            info!("Successfully retrieved {} opportunities", opportunities.len());
            Ok(HttpResponse::Ok().json(opportunities))
        }
        Err(e) => {
            error!("Failed to retrieve opportunities: {}", e);
            Err(ApiError::DatabaseError(e.to_string()))
        }
    }
}

/// GET /opportunities/profit-over-time - Returns profit data over the last 30 days
pub async fn get_profit_over_time_handler(db: web::Data<Database>) -> Result<HttpResponse, ApiError> {
    info!("Handling GET /opportunities/profit-over-time request");
    
    match get_profit_over_time(&db).await {
        Ok(profit_data) => {
            info!("Successfully retrieved profit data for {} days", profit_data.len());
            Ok(HttpResponse::Ok().json(profit_data))
        }
        Err(e) => {
            error!("Failed to retrieve profit over time data: {}", e);
            Err(ApiError::DatabaseError(e.to_string()))
        }
    }
}

/// Health check endpoint
pub async fn health_check() -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "service": "arbitrage-bot-api",
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}
