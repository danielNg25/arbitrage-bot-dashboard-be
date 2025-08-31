use alloy::primitives::{Address, U256};
use chrono::Utc;
use mongodb::bson;
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
    pub executed: Option<u64>,
    pub success: Option<u64>,
    pub failed: Option<u64>,
    pub total_profit_usd: f64,
    pub total_gas_usd: f64,
    pub last_proccesed_created_at: Option<u64>,
    pub created_at: u64,
}

impl Network {
    pub fn new(chain_id: u64, name: String, rpc: Option<String>) -> Self {
        Self {
            id: None,
            chain_id,
            name,
            rpc,
            block_explorer: None,
            executed: None,
            success: None,
            failed: None,
            total_profit_usd: 0.0,
            total_gas_usd: 0.0,
            last_proccesed_created_at: None,
            created_at: Utc::now().timestamp() as u64,
        }
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
    pub fn new(
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
    pub fn new(network_id: u64, address: &Address, tokens: &[Address], pool_type: &str) -> Self {
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

/// Opportunity model for MongoDB (important data)
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
    pub created_at: u64,
    pub updated_at: u64,
}

impl Opportunity {
    pub fn new(
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
            created_at: now,
            updated_at: now,
        }
    }
}

/// Opportunity Debug model for MongoDB (debug data)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OpportunityDebug {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<bson::oid::ObjectId>,
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

impl OpportunityDebug {
    pub fn new(
        opportunity_id: &str,
        estimate_profit: Option<String>,
        estimate_profit_usd: Option<f64>,
        path: Option<Vec<String>>,
        error: Option<String>,
        gas_amount: Option<u64>,
        gas_price: Option<u64>,
        received_at: Option<u64>,
        send_at: Option<u64>,
        simulation_time: Option<u64>,
    ) -> Self {
        let now = Utc::now().timestamp() as u64;
        Self {
            id: Some(bson::oid::ObjectId::parse_str(opportunity_id).unwrap()),
            estimate_profit,
            estimate_profit_usd,
            path,
            received_at,
            send_at,
            simulation_time,
            error,
            gas_amount,
            gas_price,
            created_at: now,
            updated_at: now,
        }
    }
}

pub enum PoolType {
    /// Uniswap V2-compatible pool (constant product formula)
    UniswapV2,
    /// Uniswap V3-compatible pool (concentrated liquidity)
    UniswapV3,
    /// ERC4626-compatible pool
    ERC4626,
}

// Response models for API endpoints
#[derive(Debug, Serialize)]
pub struct NetworkResponse {
    pub id: String, // MongoDB ObjectId as string
    pub chain_id: u64,
    pub name: String,
    pub rpc: Option<String>,
    pub block_explorer: Option<String>,
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
    pub profit_amount: Option<String>, // Profit amount in token's native units
    pub gas_usd: Option<f64>,
    pub created_at: String, // ISO 8601 formatted
    pub source_tx: Option<String>,
    pub source_block_number: Option<u64>,
    pub execute_block_number: Option<u64>,
    pub profit_token: String,
    pub profit_token_name: Option<String>,
    pub profit_token_symbol: Option<String>,
    pub profit_token_decimals: Option<u8>,
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
#[derive(Debug, Deserialize)]
pub struct OpportunityQuery {
    pub network_id: Option<u64>,
    pub status: Option<String>,
    pub min_profit_usd: Option<f64>,
    pub max_profit_usd: Option<f64>,
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
    pub opportunity: OpportunityResponse,
    pub network: NetworkResponse,
    pub path_tokens: Vec<TokenResponse>,
    pub path_pools: Vec<PoolResponse>,
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
