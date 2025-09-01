use actix_web::web;

use crate::handlers::{
    get_network_aggregations_handler, get_networks, get_opportunities_handler,
    get_opportunity_details_by_tx_handler, get_opportunity_details_handler,
    get_profit_over_time_handler, get_summary_aggregations_handler, get_time_aggregations_handler,
    get_token_performance_handler, health_check, trigger_indexing_handler,
};

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            .route("/health", web::get().to(health_check))
            .route("/networks", web::get().to(get_networks))
            .route(
                "/networks/{network_id}/aggregations",
                web::get().to(get_network_aggregations_handler),
            )
            .route(
                "/tokens/performance",
                web::get().to(get_token_performance_handler),
            )
            .route(
                "/time-aggregations",
                web::get().to(get_time_aggregations_handler),
            )
            .route(
                "/summary-aggregations",
                web::get().to(get_summary_aggregations_handler),
            )
            .route("/opportunities", web::get().to(get_opportunities_handler))
            .route(
                "/opportunities/profit-over-time",
                web::get().to(get_profit_over_time_handler),
            )
            .route(
                "/opportunities/tx/{tx_hash}",
                web::get().to(get_opportunity_details_by_tx_handler),
            )
            .route(
                "/opportunities/{id}",
                web::get().to(get_opportunity_details_handler),
            )
            .route("/admin/index", web::post().to(trigger_indexing_handler)),
    );
}
