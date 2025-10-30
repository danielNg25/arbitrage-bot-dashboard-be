use alloy::primitives::{Address, U256};
use bson::doc;
use chrono::Utc;
use log::{debug, error};
use mongodb::{bson, Database};
use serde::{Deserialize, Serialize};

use crate::utils::{address_to_string, u256_to_string, OpportunityStatus};

/// Network model for MongoDB
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Network {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<bson::oid::ObjectId>,
    pub chain_id: u64,
    pub name: String,
    pub rpc: Option<String>,
    pub block_explorer: Option<String>,
    pub router_address: Option<String>, // Router contract address for this network
    pub executors: Option<Vec<String>>,
    pub executed: Option<u64>,
    pub success: Option<u64>,
    pub failed: Option<u64>,
    pub total_profit_usd: f64,
    pub total_gas_usd: f64,
    pub last_proccesed_created_at: Option<u64>,
    pub last_pool_index_block: Option<u64>,
    pub last_processed_id: Option<String>,
    pub created_at: u64,
}

impl Network {
    pub fn new(
        chain_id: u64,
        name: String,
        rpc: Option<String>,
        router_address: Option<String>,
        executors: Option<Vec<String>>,
    ) -> Self {
        Self {
            id: None,
            chain_id,
            name,
            rpc,
            block_explorer: None,
            router_address,
            executors,
            executed: None,
            success: None,
            failed: None,
            total_profit_usd: 0.0,
            total_gas_usd: 0.0,
            last_proccesed_created_at: None,
            last_pool_index_block: None,
            last_processed_id: None,
            created_at: Utc::now().timestamp() as u64,
        }
    }

    pub async fn update_last_pool_index_block(&mut self, db: &Database, block_number: u64) {
        self.last_pool_index_block = Some(block_number);
        match Network::update_last_pool_index_block_inner(&db, self.chain_id, block_number).await {
            Ok(_) => {
                debug!(
                    "Updated last_pool_index_block for network {} to {}",
                    self.chain_id, block_number
                );
            }
            Err(e) => {
                error!(
                    "Failed to update last_pool_index_block for network {}: {}",
                    self.chain_id, e
                );
            }
        }
    }

    async fn update_last_pool_index_block_inner(
        db: &Database,
        network_id: u64,
        block_number: u64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let networks_collection = db.collection::<Network>("networks");
        networks_collection
            .update_one(
                doc! { "chain_id": network_id as i64 },
                doc! { "$set": { "last_pool_index_block": block_number as i64 } },
                None,
            )
            .await?;
        Ok(())
    }
}

/// Token model for MongoDB
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Token {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<bson::oid::ObjectId>,
    pub network_id: u64,
    pub address: String,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub decimals: Option<u8>,
    pub price: Option<f64>,
    pub total_profit: Option<String>,
    pub total_profit_usd: f64,
    pub created_at: u64,
    pub updated_at: u64,
}

impl Token {
    pub fn _new(
        network_id: u64,
        address: &Address,
        name: Option<String>,
        symbol: Option<String>,
        decimals: Option<u8>,
        price: Option<f64>,
    ) -> Self {
        Self {
            id: None,
            network_id,
            address: address_to_string(address),
            name,
            symbol,
            decimals,
            price,
            total_profit: None,
            total_profit_usd: 0.0,
            created_at: Utc::now().timestamp() as u64,
            updated_at: Utc::now().timestamp() as u64,
        }
    }
}

/// Pool model for MongoDB
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Pool {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<bson::oid::ObjectId>,
    pub network_id: u64,
    pub address: String,
    pub tokens: Vec<String>, // Token addresses
    pub pool_type: String,
    pub created_at: u64,
    pub updated_at: u64,
}

impl Pool {
    pub fn _new(network_id: u64, address: &Address, tokens: &[Address], pool_type: &str) -> Self {
        Self {
            id: None,
            network_id,
            address: address_to_string(address),
            tokens: tokens.iter().map(address_to_string).collect(),
            pool_type: pool_type.to_string(),
            created_at: Utc::now().timestamp() as u64,
            updated_at: Utc::now().timestamp() as u64,
        }
    }
}

/// Unified Opportunity model for MongoDB (combines important data and debug data)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Opportunity {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<bson::oid::ObjectId>,
    pub network_id: u64,
    pub source_block_number: Option<u64>,
    pub source_tx: Option<String>,
    pub source_log_index: Option<u64>,
    pub source_pool: Option<String>,
    pub status: String,
    pub execute_block_number: Option<u64>,
    pub execute_tx: Option<String>,
    pub profit_token: String,
    pub amount: String,
    pub profit: Option<String>,
    pub profit_usd: Option<f64>,
    pub gas_token_amount: Option<String>,
    pub gas_usd: Option<f64>,
    // Debug fields
    pub estimate_profit: Option<String>,
    pub estimate_profit_usd: Option<f64>,
    pub path: Option<Vec<String>>,
    pub received_at: Option<u64>,
    pub send_at: Option<u64>,
    pub simulation_time: Option<u64>,
    pub error: Option<String>,
    pub gas_amount: Option<u64>,
    pub gas_price: Option<u64>,
    pub created_at: u64,
    pub updated_at: u64,
}

impl Opportunity {
    pub fn _new(
        network_id: u64,
        profit_token: &Address,
        amount: &U256,
        status: OpportunityStatus,
        source_block_number: Option<u64>,
        source_tx: Option<String>,
        source_log_index: Option<u64>,
        source_pool: Option<String>,
        execute_block_number: Option<u64>,
        execute_tx: Option<String>,
        profit: Option<String>,
        profit_usd: Option<f64>,
        gas_token_amount: Option<String>,
        gas_usd: Option<f64>,
    ) -> Self {
        let now = Utc::now().timestamp() as u64;
        Self {
            id: None,
            network_id,
            source_block_number,
            source_tx,
            source_log_index,
            source_pool,
            status: status.to_string(),
            execute_block_number,
            execute_tx,
            profit_token: address_to_string(profit_token),
            profit,
            profit_usd,
            gas_token_amount,
            gas_usd,
            amount: u256_to_string(amount),
            // Debug fields initialized to None
            estimate_profit: None,
            estimate_profit_usd: None,
            path: None,
            received_at: None,
            send_at: None,
            simulation_time: None,
            error: None,
            gas_amount: None,
            gas_price: None,
            created_at: now,
            updated_at: now,
        }
    }
}

// OpportunityDebug model removed - now integrated into unified Opportunity model

// pub enum PoolType {
//     /// Uniswap V2-compatible pool (constant product formula)
//     UniswapV2,
//     /// Uniswap V3-compatible pool (concentrated liquidity)
//     UniswapV3,
//     /// ERC4626-compatible pool
//     ERC4626,
// }

// Response models for API endpoints
#[derive(Debug, Serialize)]
pub struct NetworkResponse {
    pub id: String, // MongoDB ObjectId as string
    pub chain_id: u64,
    pub name: String,
    pub rpc: Option<String>,
    pub block_explorer: Option<String>,
    pub router_address: Option<String>, // Router contract address for this network
    pub executors: Option<Vec<String>>, // Executor addresses for this network
    pub executed: Option<u64>,
    pub success: Option<u64>,
    pub failed: Option<u64>,
    pub total_profit_usd: f64,
    pub total_gas_usd: f64,
    pub last_proccesed_created_at: Option<u64>,
    pub created_at: u64,
    pub success_rate: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct OpportunityResponse {
    pub id: String, // MongoDB ObjectId as string
    pub network_id: u64,
    pub status: String,
    pub profit_usd: Option<f64>,
    pub estimate_profit_usd: Option<f64>, // Estimated profit from debug data
    pub estimate_profit: Option<String>,  // Raw profit amount from debug data
    pub profit_amount: Option<String>,    // Profit amount in token's native units
    pub gas_usd: Option<f64>,
    pub created_at: String, // ISO 8601 formatted
    pub source_tx: Option<String>,
    pub source_block_number: Option<u64>,
    pub execute_block_number: Option<u64>,
    pub profit_token: String,
    pub profit_token_name: Option<String>,
    pub profit_token_symbol: Option<String>,
    pub profit_token_decimals: Option<u8>,
    pub simulation_time: Option<u64>, // Simulation time in milliseconds
    pub error: Option<String>,        // Error message if any
}

#[derive(Debug, Serialize)]
pub struct PaginatedOpportunitiesResponse {
    pub opportunities: Vec<OpportunityResponse>,
    pub pagination: PaginationInfo,
}

#[derive(Debug, Serialize)]
pub struct PaginationInfo {
    pub page: u32,
    pub limit: u32,
    pub total: u64,
    pub total_pages: u32,
    pub has_next: bool,
    pub has_prev: bool,
}

#[derive(Debug, Serialize)]
pub struct ProfitOverTimeResponse {
    pub date: String, // YYYY-MM-DD format
    pub profit_usd: f64,
}

// Query parameters for filtering
#[derive(Debug, Deserialize, Clone)]
pub struct OpportunityQuery {
    pub network_id: Option<u64>,
    pub status: Option<String>,
    pub min_profit_usd: Option<f64>,
    pub max_profit_usd: Option<f64>,
    pub min_estimate_profit_usd: Option<f64>,
    pub max_estimate_profit_usd: Option<f64>,
    pub min_gas_usd: Option<f64>,
    pub max_gas_usd: Option<f64>,
    pub min_created_at: Option<String>, // ISO 8601 timestamp (e.g., "2024-01-01T00:00:00Z") or Unix timestamp (e.g., "1704067200")
    pub max_created_at: Option<String>, // ISO 8601 timestamp (e.g., "2024-01-31T23:59:59Z") or Unix timestamp (e.g., "1706745599")
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

/// Opportunity Debug model for MongoDB (debug data)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CountedTx {
    pub tx_hash: String,
}

impl CountedTx {
    pub fn new(_opportunity_id: &str, tx_hash: String) -> Self {
        Self { tx_hash }
    }
}

/// Opportunity Details Response - includes opportunity, network, tokens, and pools
#[derive(Debug, Serialize)]
pub struct OpportunityDetailsResponse {
    pub opportunity: OpportunityDetailsData,
    pub network: NetworkResponse,
    pub path_tokens: Vec<TokenResponse>,
    pub path_pools: Vec<PoolResponse>,
}

/// Opportunity Details Data - includes all fields from both Opportunity and OpportunityDebug
#[derive(Debug, Serialize)]
pub struct OpportunityDetailsData {
    pub id: String, // MongoDB ObjectId as string
    pub network_id: u64,
    pub status: String,
    pub profit_usd: Option<f64>,
    pub profit_amount: Option<String>, // Profit amount in token's native units
    pub gas_usd: Option<f64>,
    pub created_at: String, // ISO 8601 formatted
    pub source_tx: Option<String>,
    pub source_block_number: Option<u64>,
    pub source_log_index: Option<u64>,
    pub source_pool: Option<String>,
    pub execute_block_number: Option<u64>,
    pub execute_tx: Option<String>,
    pub profit_token: String,
    pub profit_token_name: Option<String>,
    pub profit_token_symbol: Option<String>,
    pub profit_token_decimals: Option<u8>,
    pub amount: Option<String>, // Original amount in token's native units
    pub gas_token_amount: Option<String>, // Gas amount in token's native units
    pub updated_at: String,     // ISO 8601 formatted

    // Debug fields from OpportunityDebug
    pub estimate_profit: Option<String>,
    pub estimate_profit_usd: Option<f64>,
    pub path: Option<Vec<String>>,
    pub received_at: Option<String>, // ISO 8601 formatted
    pub send_at: Option<String>,     // ISO 8601 formatted
    pub simulation_time: Option<u64>,
    pub error: Option<String>,
    pub gas_amount: Option<u64>,
    pub gas_price: Option<u64>,
}

/// Token Response for opportunity details
#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub id: String, // MongoDB ObjectId as string
    pub address: String,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub decimals: Option<u8>,
    pub price: Option<f64>,
}

/// Pool Response for opportunity details
#[derive(Debug, Serialize)]
pub struct PoolResponse {
    pub id: String, // MongoDB ObjectId as string
    pub address: String,
    pub pool_type: String,
    pub tokens: Vec<String>,
}

/// Token Performance Query Parameters
#[derive(Debug, Deserialize)]
pub struct TokenPerformanceQuery {
    pub network_id: Option<u64>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

/// Token Performance Response
#[derive(Debug, Serialize)]
pub struct TokenPerformanceResponse {
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub total_profit_usd: f64,
    pub total_profit: String,
    pub price: Option<f64>,
    pub address: String,
    pub network_id: u64,
    pub network_name: String,
}

/// Time Aggregation Period
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeAggregationPeriod {
    Hourly,
    Daily,
    Monthly,
}

/// Time Aggregation Data for MongoDB
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TimeAggregation {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<bson::oid::ObjectId>,
    pub network_id: u64,
    pub period: String,       // "hourly", "daily", "monthly"
    pub timestamp: u64,       // Unix timestamp for the period start
    pub period_start: String, // ISO 8601 formatted period start
    pub period_end: String,   // ISO 8601 formatted period end

    // Aggregated metrics
    pub total_opportunities: u64,
    pub executed_opportunities: u64,
    pub successful_opportunities: u64,
    pub failed_opportunities: u64,
    pub total_profit_usd: f64,
    pub total_gas_usd: f64,

    // Token-specific aggregations
    pub top_profit_tokens: Vec<TokenAggregation>,

    // Time series data
    pub created_at: u64,
    pub updated_at: u64,
}

/// Token Aggregation within Time Period
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TokenAggregation {
    pub address: String,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub total_profit_usd: f64,
    pub total_profit: String, // U256 string
    pub opportunity_count: u64,
    pub avg_profit_usd: f64,
}

/// Time Aggregation Query Parameters
#[derive(Debug, Deserialize)]
pub struct TimeAggregationQuery {
    pub network_id: Option<u64>,
    pub period: Option<String>,     // "hourly", "daily", "monthly"
    pub start_time: Option<String>, // ISO 8601 or Unix timestamp
    pub end_time: Option<String>,   // ISO 8601 or Unix timestamp
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

/// Time Aggregation Response
#[derive(Debug, Serialize)]
pub struct TimeAggregationResponse {
    pub network_id: u64,
    pub network_name: String,
    pub period: String,
    pub timestamp: u64,
    pub period_start: String,
    pub period_end: String,
    pub total_opportunities: u64,
    pub executed_opportunities: u64,
    pub successful_opportunities: u64,
    pub failed_opportunities: u64,
    pub total_profit_usd: f64,
    pub total_gas_usd: f64,
    pub avg_profit_usd: f64,
    pub avg_gas_usd: f64,
    pub success_rate: f64,
    pub top_profit_tokens: Vec<TokenAggregationResponse>,
}

/// Token Aggregation Response
#[derive(Debug, Serialize)]
pub struct TokenAggregationResponse {
    pub address: String,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub total_profit_usd: f64,
    pub total_profit: String,
    pub opportunity_count: u64,
    pub avg_profit_usd: f64,
}

/// Summary Aggregation Data for MongoDB (cross-network totals)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SummaryAggregation {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<bson::oid::ObjectId>,
    pub period: String,       // "hourly", "daily", "monthly"
    pub timestamp: u64,       // Unix timestamp for the period start
    pub period_start: String, // ISO 8601 formatted period start
    pub period_end: String,   // ISO 8601 formatted period end

    // Cross-network aggregated metrics
    pub total_opportunities: u64,
    pub executed_opportunities: u64,
    pub successful_opportunities: u64,
    pub failed_opportunities: u64,
    pub total_profit_usd: f64,
    pub total_gas_usd: f64,
    // Time series data
    pub created_at: u64,
    pub updated_at: u64,
}

/// Network Summary within Summary Aggregation
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NetworkSummary {
    pub network_id: u64,
    pub network_name: String,
    pub total_opportunities: u64,
    pub executed_opportunities: u64,
    pub successful_opportunities: u64,
    pub failed_opportunities: u64,
    pub total_profit_usd: f64,
    pub total_gas_usd: f64,
    pub success_rate: f64,
}

/// Summary Aggregation Query Parameters
#[derive(Debug, Deserialize)]
pub struct SummaryAggregationQuery {
    pub period: Option<String>,     // "hourly", "daily", "monthly"
    pub start_time: Option<String>, // ISO 8601 or Unix timestamp
    pub end_time: Option<String>,   // ISO 8601 or Unix timestamp
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

/// Network Aggregation Query Parameters (for specific network aggregations)
#[derive(Debug, Deserialize)]
pub struct NetworkAggregationQuery {
    pub _network_id: Option<u64>, // Optional - will be set from path parameter
    pub period: Option<String>,   // "hourly", "daily", "monthly"
    pub start_time: Option<String>, // ISO 8601 or Unix timestamp
    pub end_time: Option<String>, // ISO 8601 or Unix timestamp
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

/// Debug Opportunity Request
#[derive(Debug, Deserialize)]
pub struct DebugOpportunityRequest {
    pub opportunity: Opportunity,
    pub block_number: Option<String>, // Block number to simulate at (optional, defaults to latest)
    pub from_address: Option<String>, // Address to simulate from (optional, defaults to zero address)
    pub to: Option<String>, // Router contract address to call (optional, defaults to a standard router)
}

/// Debug Opportunity Response
#[derive(Debug, Serialize)]
pub struct DebugOpportunityResponse {
    pub success: bool,
    pub error: Option<String>,
    pub trace: Option<String>,
    pub gas_used: Option<String>,
    pub block_number: Option<String>,
    pub transaction_data: Option<String>,
}

/// Summary Aggregation Response
#[derive(Debug, Serialize)]
pub struct SummaryAggregationResponse {
    pub period: String,
    pub timestamp: u64,
    pub period_start: String,
    pub period_end: String,
    pub total_opportunities: u64,
    pub executed_opportunities: u64,
    pub successful_opportunities: u64,
    pub failed_opportunities: u64,
    pub total_profit_usd: f64,
    pub total_gas_usd: f64,
}

//// Network Summary Response
// #[derive(Debug, Serialize)]
// pub struct NetworkSummaryResponse {
//     pub network_id: u64,
//     pub network_name: String,
//     pub total_opportunities: u64,
//     pub executed_opportunities: u64,
//     pub successful_opportunities: u64,
//     pub failed_opportunities: u64,
//     pub total_profit_usd: f64,
//     pub total_gas_usd: f64,
//     pub success_rate: f64,
// }
