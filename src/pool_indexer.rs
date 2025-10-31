use alloy::eips::BlockNumberOrTag;
use alloy::primitives::Address;
use alloy::providers::{Provider, ProviderBuilder};
use alloy::sol_types::SolEvent;
use alloy::transports::http::reqwest::Url;
use futures::StreamExt;
use log::{debug, error, info, warn};
use mongodb::bson::doc;
use mongodb::Database;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;

use crate::contract::{IAlgebraFactory, IUniswapV2Factory, IUniswapV3Factory};
use crate::models::Network;
use crate::notification_handler::{NotificationHandler, NotificationType};
use crate::utils::{address_to_string, fetch_events};

const DEFAULT_BLOCKS_PER_BATCH: u64 = 1000;
const DEFAULT_INDEXING_INTERVAL_SECONDS: u64 = 300; // 5 minutes

pub struct PoolIndexer {
    db: Arc<Database>,
    notification_handler: Arc<NotificationHandler>,
    indexing_interval_seconds: u64,
    blocks_per_batch: u64,
}

impl PoolIndexer {
    pub fn new(
        db: Arc<Database>,
        notification_handler: Arc<NotificationHandler>,
        indexing_interval_seconds: Option<u64>,
        blocks_per_batch: Option<u64>,
    ) -> Self {
        Self {
            db,
            notification_handler,
            indexing_interval_seconds: indexing_interval_seconds
                .unwrap_or(DEFAULT_INDEXING_INTERVAL_SECONDS),
            blocks_per_batch: blocks_per_batch.unwrap_or(DEFAULT_BLOCKS_PER_BATCH),
        }
    }

    pub fn from_config(
        db: Arc<Database>,
        notification_handler: Arc<NotificationHandler>,
        config: &crate::config::Config,
    ) -> Self {
        Self::new(
            db,
            notification_handler,
            Some(config.pool_indexer.interval_seconds),
            Some(config.pool_indexer.blocks_per_batch),
        )
    }

    pub async fn start(&self) {
        info!("Starting pool indexer service...");

        let db = self.db.clone();
        let notification_handler = self.notification_handler.clone();
        let indexing_interval = self.indexing_interval_seconds;
        let blocks_per_batch = self.blocks_per_batch;

        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(indexing_interval));

            loop {
                interval.tick().await;
                info!("Running pool indexing task...");

                // Get all networks
                let networks = match get_all_networks(&db).await {
                    Ok(networks) => networks,
                    Err(e) => {
                        error!("Failed to get networks: {}", e);
                        continue;
                    }
                };

                for mut network in networks {
                    if network.enable_pool_indexer.is_none()
                        || !network.enable_pool_indexer.unwrap()
                    {
                        info!(
                            "Pool indexer is not enabled for network: {} ({})",
                            network.name, network.chain_id
                        );
                        continue;
                    }
                    if let Some(rpc) = &network.rpc {
                        info!(
                            "Indexing pools for network: {} ({})",
                            network.name, network.chain_id
                        );
                        // Create provider
                        let provider =
                            ProviderBuilder::new().connect_http(Url::parse(rpc).unwrap());

                        // Get latest block number
                        let latest_block = match provider.get_block_number().await {
                            Ok(block) => block,
                            Err(e) => {
                                error!(
                                    "Failed to get latest block for network {}: {}",
                                    network.chain_id, e
                                );
                                continue;
                            }
                        };
                        if network.last_pool_index_block.is_none() {
                            network
                                .update_last_pool_index_block(&db, latest_block)
                                .await;
                        }
                        // Determine start block
                        let start_block = network.last_pool_index_block.unwrap_or(latest_block);

                        // If we're already at the latest block, skip
                        if start_block >= latest_block {
                            debug!(
                                "Network {} already at latest block ({}), skipping",
                                network.chain_id, latest_block
                            );
                            continue;
                        }

                        let mut pool_addresses: Vec<Address> = Vec::new();

                        // Process blocks in batches
                        let mut current_block = start_block;
                        while current_block < latest_block {
                            let end_block =
                                std::cmp::min(current_block + blocks_per_batch, latest_block);

                            info!(
                                "Indexing pools for network {} from block {} to {}",
                                network.chain_id, current_block, end_block
                            );

                            // Process all pool creation events
                            match index_all_pools(
                                &notification_handler,
                                &provider,
                                &network,
                                current_block,
                                end_block,
                            )
                            .await
                            {
                                Ok(pool_addresses_result) => {
                                    pool_addresses.extend(pool_addresses_result);
                                }
                                Err(e) => {
                                    error!(
                                        "Error indexing pools on network {}: {}",
                                        network.chain_id, e
                                    );
                                    // retry in 1 second
                                    tokio::time::sleep(Duration::from_secs(1)).await;
                                    continue;
                                }
                            }

                            // Update last indexed block
                            match update_last_pool_index_block(&db, network.chain_id, end_block)
                                .await
                            {
                                Ok(_) => {
                                    debug!(
                                        "Updated last_pool_index_block for network {} to {}",
                                        network.chain_id, end_block
                                    );
                                }
                                Err(e) => {
                                    error!(
                                        "Failed to update last_pool_index_block for network {}: {}",
                                        network.chain_id, e
                                    );
                                }
                            }

                            current_block = end_block;
                        }
                        if !pool_addresses.is_empty() {
                            info!(
                                "New {} pools detected on {} ({})",
                                pool_addresses.len(),
                                network.name,
                                network.chain_id,
                            );
                            send_new_pool_notification(
                                &notification_handler,
                                &network.name,
                                network.chain_id,
                                pool_addresses,
                            )
                            .await;
                        } else {
                            info!(
                                "No new pools detected on {} ({})",
                                network.name, network.chain_id
                            );
                        }
                        network
                            .update_last_pool_index_block(&db, latest_block)
                            .await;
                    } else {
                        warn!("Network {} has no RPC URL configured", network.chain_id);
                    }
                }

                info!("Pool indexing task completed");
            }
        });

        info!("Pool indexer service started");
    }
}

async fn get_all_networks(
    db: &Database,
) -> Result<Vec<Network>, Box<dyn std::error::Error + Send + Sync>> {
    let networks_collection = db.collection::<Network>("networks");
    let mut cursor = networks_collection.find(None, None).await?;

    let mut networks = Vec::new();
    while let Some(network_result) = cursor.next().await {
        if let Ok(network) = network_result {
            networks.push(network);
        }
    }

    Ok(networks)
}

async fn update_last_pool_index_block(
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

async fn index_all_pools(
    notification_handler: &NotificationHandler,
    provider: &impl Provider,
    network: &Network,
    from_block: u64,
    to_block: u64,
) -> Result<Vec<Address>, Box<dyn std::error::Error + Send + Sync>> {
    // Define all the known pool creation event signatures
    let event_signatures = vec![
        // UniswapV2 PairCreated(address indexed token0, address indexed token1, address pair, uint256)
        IUniswapV2Factory::PairCreated::SIGNATURE_HASH,
        // UniswapV3 PoolCreated(address indexed token0, address indexed token1, uint24 indexed fee, int24 tickSpacing, address pool)
        IUniswapV3Factory::PoolCreated::SIGNATURE_HASH,
        // Algebra Pool(address indexed token0, address indexed token1, address pool)
        IAlgebraFactory::Pool::SIGNATURE_HASH,
    ];

    let logs = fetch_events(
        provider,
        vec![],
        event_signatures,
        BlockNumberOrTag::Number(from_block as u64),
        BlockNumberOrTag::Number(to_block as u64),
    )
    .await?;

    let mut pool_addresses = Vec::new();
    for log in logs {
        let topic0 = log.topic0().unwrap();
        match topic0 {
            &IUniswapV2Factory::PairCreated::SIGNATURE_HASH => {
                let swap_data: IUniswapV2Factory::PairCreated = log.log_decode()?.inner.data;
                pool_addresses.push(swap_data.pair);
            }
            &IUniswapV3Factory::PoolCreated::SIGNATURE_HASH => {
                let swap_data: IUniswapV3Factory::PoolCreated = log.log_decode()?.inner.data;
                pool_addresses.push(swap_data.pool);
            }
            &IAlgebraFactory::Pool::SIGNATURE_HASH => {
                let swap_data: IAlgebraFactory::Pool = log.log_decode()?.inner.data;
                pool_addresses.push(swap_data.pool);
            }
            _ => continue,
        }
    }

    Ok(pool_addresses)
}

async fn send_new_pool_notification(
    notification_handler: &NotificationHandler,
    network_name: &str,
    network_id: u64,
    pool_addresses: Vec<Address>,
) {
    // Log the new pool
    info!(
        "New {} pools detected on {} ({}): {}",
        pool_addresses.len(),
        network_name,
        network_id,
        pool_addresses
            .iter()
            .map(address_to_string)
            .collect::<Vec<String>>()
            .join(", "),
    );

    // Send notification via Telegram
    if notification_handler.is_configured() {
        notification_handler
            .send_notification(NotificationType::NewPool {
                network_name: network_name.to_string(),
                network_id,
                pool_addresses: pool_addresses.iter().map(address_to_string).collect(),
            })
            .await;
    }
}
