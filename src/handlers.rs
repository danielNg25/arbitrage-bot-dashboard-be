use actix::{Actor, ActorContext, AsyncContext, Handler, Message, StreamHandler};
use actix_web::{web, HttpResponse, Result};
use actix_web_actors::ws;
use log::{debug, error, info, warn};
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

// ========================= WebSocket: Opportunities =========================

/// WebSocket session for streaming new opportunities
pub struct OpportunitiesWs {
    db: std::sync::Arc<mongodb::Database>,
    query: OpportunityQuery,
    change_stream_task: Option<tokio::task::JoinHandle<()>>,
}

impl OpportunitiesWs {
    pub fn new(db: std::sync::Arc<mongodb::Database>, query: OpportunityQuery) -> Self {
        Self {
            db,
            query,
            change_stream_task: None,
        }
    }
}

impl Actor for OpportunitiesWs {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!(
            "WebSocket actor started for opportunities with filters: {:?}",
            self.query
        );
        let db = self.db.clone();
        let query = self.query.clone();
        let addr = ctx.address();

        // Spawn MongoDB change stream task and store the handle
        let task = tokio::spawn(async move {
            use chrono::{DateTime, Utc};
            use futures::StreamExt;
            use mongodb::bson::doc;

            let opportunities: mongodb::Collection<crate::models::Opportunity> =
                db.collection("opportunities");
            let tokens: mongodb::Collection<crate::models::Token> = db.collection("tokens");

            info!("Starting MongoDB change stream for opportunities collection");
            let pipeline = vec![doc! {"$match": {"operationType": {"$in": ["insert"]}}}];
            let mut stream = match opportunities.watch(pipeline, None).await {
                Ok(s) => {
                    info!("Successfully opened MongoDB change stream");
                    s
                }
                Err(e) => {
                    log::error!("Failed to open change stream: {}", e);
                    return;
                }
            };

            info!(
                "Listening for new opportunity inserts with filters: {:?}",
                query
            );

            while let Some(Ok(event)) = stream.next().await {
                if let Some(full) = event.full_document {
                    debug!(
                        "Received new opportunity: network_id={}, status={}, profit_usd={:?}",
                        full.network_id, full.status, full.profit_usd
                    );

                    // Apply filters similar to get_opportunities
                    if let Some(nid) = query.network_id {
                        if (full.network_id as u64) != nid {
                            debug!(
                                "Filtered out opportunity: network_id {} != {}",
                                full.network_id, nid
                            );
                            continue;
                        }
                    }
                    if let Some(ref s) = query.status {
                        let status = full.status.to_lowercase();
                        if status != s.to_lowercase() {
                            debug!(
                                "Filtered out opportunity: status '{}' != '{}'",
                                full.status, s
                            );
                            continue;
                        }
                    }
                    if let Some(min) = query.min_profit_usd {
                        if full.profit_usd.unwrap_or(0.0) < min {
                            debug!(
                                "Filtered out opportunity: profit_usd {:.2} < {:.2}",
                                full.profit_usd.unwrap_or(0.0),
                                min
                            );
                            continue;
                        }
                    }
                    if let Some(max) = query.max_profit_usd {
                        if full.profit_usd.unwrap_or(0.0) > max {
                            debug!(
                                "Filtered out opportunity: profit_usd {:.2} > {:.2}",
                                full.profit_usd.unwrap_or(0.0),
                                max
                            );
                            continue;
                        }
                    }
                    if let Some(min) = query.min_estimate_profit_usd {
                        if full.estimate_profit_usd.unwrap_or(0.0) < min {
                            debug!(
                                "Filtered out opportunity: estimate_profit_usd {:.2} < {:.2}",
                                full.estimate_profit_usd.unwrap_or(0.0),
                                min
                            );
                            continue;
                        }
                    }
                    if let Some(max) = query.max_estimate_profit_usd {
                        if full.estimate_profit_usd.unwrap_or(0.0) > max {
                            debug!(
                                "Filtered out opportunity: estimate_profit_usd {:.2} > {:.2}",
                                full.estimate_profit_usd.unwrap_or(0.0),
                                max
                            );
                            continue;
                        }
                    }
                    if let Some(min) = query.min_gas_usd {
                        if full.gas_usd.unwrap_or(f64::MAX) < min {
                            debug!(
                                "Filtered out opportunity: gas_usd {:.2} < {:.2}",
                                full.gas_usd.unwrap_or(f64::MAX),
                                min
                            );
                            continue;
                        }
                    }
                    if let Some(max) = query.max_gas_usd {
                        if full.gas_usd.unwrap_or(0.0) > max {
                            debug!(
                                "Filtered out opportunity: gas_usd {:.2} > {:.2}",
                                full.gas_usd.unwrap_or(0.0),
                                max
                            );
                            continue;
                        }
                    }

                    // Get token information for enrichment (same as REST API)
                    let token_data = match tokens.find_one(doc! { "address": &full.profit_token, "network_id": full.network_id as i64 }, None).await {
                        Ok(Some(token)) => token,
                        Ok(None) => {
                            debug!("No token data found for address: {}", full.profit_token);
                            crate::models::Token {
                                id: None,
                                network_id: full.network_id,
                                address: full.profit_token.clone(),
                                name: None,
                                symbol: None,
                                decimals: None,
                                price: None,
                                total_profit: None,
                                total_profit_usd: 0.0,
                                created_at: 0,
                                updated_at: 0,
                            }
                        }
                        Err(e) => {
                            log::warn!("Failed to fetch token data: {}", e);
                            crate::models::Token {
                                id: None,
                                network_id: full.network_id,
                                address: full.profit_token.clone(),
                                name: None,
                                symbol: None,
                                decimals: None,
                                price: None,
                                total_profit: None,
                                total_profit_usd: 0.0,
                                created_at: 0,
                                updated_at: 0,
                            }
                        }
                    };

                    // Convert timestamp to ISO 8601 string (same as REST API)
                    let created_at_str = DateTime::from_timestamp(full.created_at as i64, 0)
                        .unwrap_or_else(|| Utc::now())
                        .format("%Y-%m-%dT%H:%M:%SZ")
                        .to_string();

                    // Create OpportunityResponse with same structure as REST API
                    let opportunity_response = crate::models::OpportunityResponse {
                        id: full.id.as_ref().map(|o| o.to_hex()).unwrap_or_default(),
                        network_id: full.network_id,
                        status: full.status,
                        profit_usd: full.profit_usd,
                        estimate_profit_usd: full.estimate_profit_usd,
                        estimate_profit: full.estimate_profit,
                        profit_amount: full.profit,
                        gas_usd: full.gas_usd,
                        created_at: created_at_str,
                        source_tx: full.source_tx,
                        source_block_number: full.source_block_number,
                        execute_block_number: full.execute_block_number,
                        profit_token: full.profit_token,
                        profit_token_name: token_data.name,
                        profit_token_symbol: token_data.symbol,
                        profit_token_decimals: token_data.decimals,
                        simulation_time: full.simulation_time,
                        error: full.error,
                    };

                    info!("Opportunity passed all filters, sending to client: id={:?}, network_id={}, status={}, profit_usd={:.2}", 
                          opportunity_response.id, opportunity_response.network_id, opportunity_response.status, opportunity_response.profit_usd.unwrap_or(0.0));

                    // Send the full OpportunityResponse as JSON (same structure as REST API)
                    let payload =
                        serde_json::to_string(&opportunity_response).unwrap_or_else(|e| {
                            log::error!("Failed to serialize opportunity response: {}", e);
                            "{}".to_string()
                        });

                    let _ = addr.do_send(NewOpportunity(payload));
                }
            }
            warn!("MongoDB change stream ended unexpectedly");
        });

        // Store the task handle for cleanup
        self.change_stream_task = Some(task);
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        info!("WebSocket actor stopped, cleaning up resources");

        // Cancel the MongoDB change stream task
        if let Some(task) = self.change_stream_task.take() {
            task.abort();
            info!("MongoDB change stream task aborted");
        }
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for OpportunitiesWs {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                debug!("Received ping, sending pong");
                ctx.pong(&msg);
            }
            Ok(ws::Message::Text(text)) => {
                debug!("Received text message from client: {}", text);
            }
            Ok(ws::Message::Binary(bin)) => {
                debug!("Received binary message from client ({} bytes)", bin.len());
            }
            Ok(ws::Message::Close(reason)) => {
                info!("Client requested close: {:?}", reason);
                ctx.close(reason);
                ctx.stop();
            }
            Ok(ws::Message::Pong(_)) => {
                debug!("Received pong from client");
            }
            Ok(ws::Message::Continuation(_)) => {
                debug!("Received continuation frame");
            }
            Ok(ws::Message::Nop) => {
                debug!("Received NOP frame");
            }
            Err(e) => {
                error!("WebSocket protocol error: {}", e);
                ctx.stop();
            }
        }
    }
}

// Message to push new opportunity text to client
struct NewOpportunity(String);
impl Message for NewOpportunity {
    type Result = ();
}

impl Handler<NewOpportunity> for OpportunitiesWs {
    type Result = ();

    fn handle(&mut self, msg: NewOpportunity, ctx: &mut Self::Context) -> Self::Result {
        debug!("Sending opportunity to WebSocket client: {}", msg.0);
        ctx.text(msg.0);
    }
}

/// GET /ws/opportunities - WebSocket endpoint for new opportunities stream
pub async fn ws_opportunities_handler(
    req: actix_web::HttpRequest,
    stream: actix_web::web::Payload,
    db: web::Data<Database>,
    query: web::Query<OpportunityQuery>,
) -> actix_web::Result<HttpResponse> {
    info!(
        "WebSocket connection request to /ws/opportunities with query: {:?}",
        query
    );

    // Log client information
    if let Some(addr) = req.connection_info().peer_addr() {
        info!("WebSocket client connecting from: {}", addr);
    }

    if let Some(user_agent) = req.headers().get("User-Agent") {
        if let Ok(ua) = user_agent.to_str() {
            info!("WebSocket client User-Agent: {}", ua);
        }
    }

    // Log all headers for debugging
    info!("WebSocket request headers:");
    for (name, value) in req.headers() {
        if let Ok(v) = value.to_str() {
            info!("  {}: {}", name, v);
        }
    }

    // Log origin specifically
    if let Some(origin) = req.headers().get("Origin") {
        if let Ok(o) = origin.to_str() {
            info!("Origin header: {}", o);
        }
    }

    let ws = OpportunitiesWs::new(
        std::sync::Arc::new(db.get_ref().clone()),
        query.into_inner(),
    );
    info!("Starting WebSocket connection for opportunities");
    ws::start(ws, &req, stream)
}

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
    let _indexer = Indexer::new(db_arc.clone(), 5, 168); // Default interval for manual runs

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
