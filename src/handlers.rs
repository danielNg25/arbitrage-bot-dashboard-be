use actix_web::{web, HttpResponse, Result};
use log::{error, info};
use mongodb::Database;

use crate::{
    database::{
        get_networks_with_stats, get_opportunities, get_profit_over_time, get_summary_aggregations,
        get_time_aggregations, get_token_performance,
    },
    errors::ApiError,
    indexer::Indexer,
    models::{
        NetworkAggregationQuery, OpportunityQuery, SummaryAggregationQuery, TimeAggregationQuery,
        TokenPerformanceQuery,
    },
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
        query.min_estimate_profit_usd,
        query.max_estimate_profit_usd,
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

/// GET /tokens/performance - Returns token performance data
pub async fn get_token_performance_handler(
    db: web::Data<Database>,
    query: web::Query<TokenPerformanceQuery>,
) -> Result<HttpResponse, ApiError> {
    info!("Handling GET /tokens/performance request");

    match get_token_performance(&db, query.network_id, query.limit, query.offset).await {
        Ok(tokens) => {
            info!(
                "Successfully retrieved {} token performance records",
                tokens.len()
            );
            Ok(HttpResponse::Ok().json(tokens))
        }
        Err(e) => {
            error!("Failed to retrieve token performance data: {}", e);
            Err(ApiError::DatabaseError(e.to_string()))
        }
    }
}

/// GET /time-aggregations - Returns time-based aggregations
pub async fn get_time_aggregations_handler(
    db: web::Data<Database>,
    query: web::Query<TimeAggregationQuery>,
) -> Result<HttpResponse, ApiError> {
    info!("Handling GET /time-aggregations request");

    match get_time_aggregations(
        &db,
        query.network_id,
        query.period.clone(),
        query.start_time.clone(),
        query.end_time.clone(),
        query.limit,
        query.offset,
    )
    .await
    {
        Ok(aggregations) => {
            info!(
                "Successfully retrieved {} time aggregation records",
                aggregations.len()
            );
            Ok(HttpResponse::Ok().json(aggregations))
        }
        Err(e) => {
            error!("Failed to retrieve time aggregations: {}", e);
            Err(ApiError::DatabaseError(e.to_string()))
        }
    }
}

/// GET /summary-aggregations - Returns cross-network summary aggregations
pub async fn get_summary_aggregations_handler(
    db: web::Data<Database>,
    query: web::Query<SummaryAggregationQuery>,
) -> Result<HttpResponse, ApiError> {
    info!("Handling GET /summary-aggregations request");

    match get_summary_aggregations(
        &db,
        query.period.clone(),
        query.start_time.clone(),
        query.end_time.clone(),
        query.limit,
        query.offset,
    )
    .await
    {
        Ok(aggregations) => {
            info!(
                "Successfully retrieved {} summary aggregation records",
                aggregations.len()
            );
            Ok(HttpResponse::Ok().json(aggregations))
        }
        Err(e) => {
            error!("Failed to retrieve summary aggregations: {}", e);
            Err(ApiError::DatabaseError(e.to_string()))
        }
    }
}

/// GET /networks/{network_id}/aggregations - Returns time aggregations for a specific network
pub async fn get_network_aggregations_handler(
    path: web::Path<u64>,
    db: web::Data<Database>,
    query: web::Query<NetworkAggregationQuery>,
) -> Result<HttpResponse, ApiError> {
    let network_id = path.into_inner();
    info!("Handling GET /networks/{}/aggregations request", network_id);

    let query_params = query.into_inner();

    match get_time_aggregations(
        &db,
        Some(network_id),
        query_params.period.clone(),
        query_params.start_time.clone(),
        query_params.end_time.clone(),
        query_params.limit,
        query_params.offset,
    )
    .await
    {
        Ok(aggregations) => {
            info!(
                "Successfully retrieved {} time aggregation records for network {}",
                aggregations.len(),
                network_id
            );
            Ok(HttpResponse::Ok().json(aggregations))
        }
        Err(e) => {
            error!(
                "Failed to retrieve time aggregations for network {}: {}",
                network_id, e
            );
            Err(ApiError::DatabaseError(e.to_string()))
        }
    }
}
