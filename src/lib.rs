pub mod config;
pub mod contract;
pub mod database;
pub mod errors;
pub mod handlers;
pub mod indexer;
pub mod models;
pub mod notification_handler;
pub mod pool_indexer;
pub mod routes;
pub mod utils;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opportunity_status_display() {
        let status = crate::utils::OpportunityStatus::Succeeded;
        assert_eq!(status.to_string(), "succeeded");
    }

    #[test]
    fn test_network_creation() {
        let network = crate::models::Network::new(
            1,
            "Ethereum".to_string(),
            Some("https://eth-mainnet.alchemyapi.io/v2/test".to_string()),
            Some("0x3fC91A3afd70395Cd496C647d5a6CC9D4B2b7FAD".to_string()),
            Some(vec![
                "0x3fC91A3afd70395Cd496C647d5a6CC9D4B2b7FAD".to_string()
            ]),
        );
        assert_eq!(network.chain_id, 1);
        assert_eq!(network.name, "Ethereum");
        assert_eq!(network.total_profit_usd, 0.0);
    }
}
