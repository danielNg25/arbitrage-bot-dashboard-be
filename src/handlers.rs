use actix_web::{web, HttpResponse, Result};
use log::{error, info};
use mongodb::Database;

use crate::{
    database::{get_networks_with_stats, get_opportunities, get_profit_over_time},
    errors::ApiError,
    indexer::Indexer,
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

/// GET /opportunities - Returns opportunities with optional filtering and pagination
pub async fn get_opportunities_handler(
    db: web::Data<Database>,
    query: web::Query<OpportunityQuery>,
) -> Result<HttpResponse, ApiError> {
    info!(
        "Handling GET /opportunities request with query: {:?}",
        query
    );

    match get_opportunities(
        &db,
        query.network_id,
        query.status.clone(),
        query.min_profit_usd,
        query.max_profit_usd,
        query.min_gas_usd,
        query.max_gas_usd,
        query.min_created_at.clone(),
        query.max_created_at.clone(),
        query.page,
        query.limit,
    )
    .await
    {
        Ok(response) => {
            info!(
                "Successfully retrieved {} opportunities (page {}, total: {})",
                response.opportunities.len(),
                response.pagination.page,
                response.pagination.total
            );
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            error!("Failed to retrieve opportunities: {}", e);
            Err(ApiError::DatabaseError(e.to_string()))
        }
    }
}

/// GET /opportunities/profit-over-time - Returns profit data over the last 30 days
pub async fn get_profit_over_time_handler(
    db: web::Data<Database>,
) -> Result<HttpResponse, ApiError> {
    info!("Handling GET /opportunities/profit-over-time request");

    match get_profit_over_time(&db).await {
        Ok(profit_data) => {
            info!(
                "Successfully retrieved profit data for {} days",
                profit_data.len()
            );
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

/// GET /opportunities/{id} - Returns detailed opportunity information
pub async fn get_opportunity_details_handler(
    path: web::Path<String>,
    db: web::Data<Database>,
) -> Result<HttpResponse, ApiError> {
    let opportunity_id = path.into_inner();
    info!("Handling GET /opportunities/{} request", opportunity_id);

    match crate::database::get_opportunity_details(&db, &opportunity_id).await {
        Ok(Some(details)) => {
            info!(
                "Successfully retrieved opportunity details for ID: {}",
                opportunity_id
            );
            Ok(HttpResponse::Ok().json(details))
        }
        Ok(None) => {
            info!("Opportunity not found for ID: {}", opportunity_id);
            Err(ApiError::NotFound("Opportunity not found".to_string()))
        }
        Err(e) => {
            error!("Database error while fetching opportunity details: {}", e);
            Err(ApiError::DatabaseError(e.to_string()))
        }
    }
}

/// GET /opportunities/tx/{tx_hash} - Returns detailed opportunity information by transaction hash
pub async fn get_opportunity_details_by_tx_handler(
    path: web::Path<String>,
    db: web::Data<Database>,
) -> Result<HttpResponse, ApiError> {
    let tx_hash = path.into_inner();
    info!("Handling GET /opportunities/tx/{} request", tx_hash);

    match crate::database::get_opportunity_details_by_tx_hash(&db, &tx_hash).await {
        Ok(Some(details)) => {
            info!(
                "Successfully retrieved opportunity details for tx hash: {}",
                tx_hash
            );
            Ok(HttpResponse::Ok().json(details))
        }
        Ok(None) => {
            info!("Opportunity not found for tx hash: {}", tx_hash);
            Err(ApiError::NotFound(
                "Opportunity not found for the given transaction hash".to_string(),
            ))
        }
        Err(e) => {
            error!(
                "Database error while fetching opportunity details by tx hash: {}",
                e
            );
            Err(ApiError::DatabaseError(e.to_string()))
        }
    }
}

/// POST /admin/index - Triggers manual indexing of network and token metrics
pub async fn trigger_indexing_handler(db: web::Data<Database>) -> Result<HttpResponse, ApiError> {
    info!("Handling POST /admin/index request - triggering manual indexing");

    // Create a temporary indexer instance for manual run
    let db_arc = std::sync::Arc::new(db.get_ref().clone());
    let _indexer = Indexer::new(db_arc.clone(), 5); // Default interval for manual runs

    match Indexer::run_manual_indexing(&db_arc).await {
        Ok(_) => {
            info!("Manual indexing completed successfully");
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "status": "success",
                "message": "Indexing completed successfully",
                "timestamp": chrono::Utc::now().to_rfc3339()
            })))
        }
        Err(e) => {
            error!("Manual indexing failed: {}", e);
            Err(ApiError::DatabaseError(e.to_string()))
        }
    }
}
