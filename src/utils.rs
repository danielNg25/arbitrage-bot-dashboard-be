use alloy::eips::BlockNumberOrTag;
use alloy::primitives::{Address, FixedBytes, U256};
use alloy::providers::Provider;
use alloy::rpc::types::{Filter, Log};
use anyhow::Result;

pub async fn fetch_events<P: Provider + Send + Sync>(
    provider: &P,
    addresses: Vec<Address>,
    topics: Vec<FixedBytes<32>>,
    from_block: BlockNumberOrTag,
    to_block: BlockNumberOrTag,
) -> Result<Vec<Log>> {
    let filter = Filter::new()
        .from_block(from_block)
        .to_block(to_block)
        .address(addresses)
        .event_signature(topics);

    let events = provider.get_logs(&filter).await?;
    Ok(events)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpportunityStatus {
    Succeeded,          // Transaction was successful
    PartiallySucceeded, // Transaction was partially successful
    Reverted,           // Transaction reverted
    Error,              // Transaction failed with an error
    Skipped,            // Transaction was skipped (e.g. no profit)
    None,
}

impl std::fmt::Display for OpportunityStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpportunityStatus::Succeeded => write!(f, "succeeded"),
            OpportunityStatus::PartiallySucceeded => write!(f, "partially_succeeded"),
            OpportunityStatus::Reverted => write!(f, "reverted"),
            OpportunityStatus::Error => write!(f, "error"),
            OpportunityStatus::Skipped => write!(f, "skipped"),
            OpportunityStatus::None => write!(f, "none"),
        }
    }
}

impl std::str::FromStr for OpportunityStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "succeeded" => Ok(OpportunityStatus::Succeeded),
            "partially_succeeded" => Ok(OpportunityStatus::PartiallySucceeded),
            "reverted" => Ok(OpportunityStatus::Reverted),
            "error" => Ok(OpportunityStatus::Error),
            "skipped" => Ok(OpportunityStatus::Succeeded),
            "none" => Ok(OpportunityStatus::None),
            _ => Err(format!("Unknown opportunity status: {}", s)),
        }
    }
}

/// Helper function to convert an Address to a string for MongoDB
pub fn address_to_string(address: &Address) -> String {
    format!("{:?}", address)
}

/// Helper function to convert a U256 to a string for MongoDB
pub fn u256_to_string(value: &U256) -> String {
    value.to_string()
}
