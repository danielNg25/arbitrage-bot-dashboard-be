use alloy::primitives::utils::format_units;
use alloy::primitives::U256;
use log::info;
use mongodb::Database;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Arc;
use teloxide::{prelude::*, Bot};

use crate::database;
use crate::models::{Network, Opportunity, Token};

/// Escape special characters for MarkdownV2
fn escape_markdownv2(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '_' | '[' | ']' | '(' | ')' | '~' | '`' | '>' | '#' | '+' | '-' | '=' | '|' | '{'
            | '}' | '.' | '!' => {
                format!("\\{}", c)
            }
            _ => c.to_string(),
        })
        .collect()
}

pub struct NotificationHandler {
    bot: Bot,
    chat_id: String,
    big_opp_thread_id: u64,
    failed_opp_thread_id: u64,
    new_pool_thread_id: u64,
    db: Option<Arc<Database>>,
}

pub enum NotificationType {
    HighOpportunity(Opportunity),
    NewPool {
        network_name: String,
        network_id: u64,
        pool_addresses: Vec<String>,
    },
}

impl NotificationHandler {
    pub fn _new(
        token: String,
        chat_id: String,
        big_opp_thread_id: u64,
        failed_opp_thread_id: u64,
        new_pool_thread_id: u64,
    ) -> Self {
        let bot = Bot::new(token);
        Self {
            bot,
            chat_id,
            big_opp_thread_id,
            failed_opp_thread_id,
            new_pool_thread_id,
            db: None,
        }
    }

    pub fn with_database(
        token: String,
        chat_id: String,
        big_opp_thread_id: u64,
        failed_opp_thread_id: u64,
        new_pool_thread_id: u64,
        db: Arc<Database>,
    ) -> Self {
        let bot = Bot::new(token);
        Self {
            bot,
            chat_id,
            big_opp_thread_id,
            failed_opp_thread_id,
            new_pool_thread_id,
            db: Some(db),
        }
    }

    pub fn from_config(config: &crate::config::Config, db: Arc<Database>) -> Option<Self> {
        if let (
            Some(token),
            Some(chat_id),
            Some(big_opp_thread_id),
            Some(failed_opp_thread_id),
            Some(new_pool_thread_id),
        ) = (
            &config.telegram.token,
            &config.telegram.chat_id,
            &config.telegram.big_opp_thread_id,
            &config.telegram.failed_opp_thread_id,
            &config.telegram.new_pool_thread_id,
        ) {
            if !token.is_empty() && !chat_id.is_empty() {
                return Some(Self::with_database(
                    token.clone(),
                    chat_id.clone(),
                    big_opp_thread_id.clone(),
                    failed_opp_thread_id.clone(),
                    new_pool_thread_id.clone(),
                    db,
                ));
            }
        }
        None
    }

    // Check if the notification handler is properly configured
    pub fn is_configured(&self) -> bool {
        self.db.is_some()
    }

    async fn get_network(&self, network_id: u64) -> Option<Network> {
        if let Some(db) = &self.db {
            match database::get_network(db, network_id).await {
                Ok(network) => return Some(network),
                Err(err) => {
                    eprintln!("Error fetching network: {}", err);
                    return None;
                }
            }
        }
        None
    }

    async fn get_token(&self, network_id: u64, token_address: &str) -> Option<Token> {
        if let Some(db) = &self.db {
            match database::get_token(db, network_id, token_address).await {
                Ok(token) => return Some(token),
                Err(err) => {
                    eprintln!("Error fetching token: {}", err);
                    return None;
                }
            }
        }
        None
    }

    pub async fn send_notification(&self, notification_type: NotificationType) {
        match notification_type {
            NotificationType::NewPool {
                network_name,
                network_id,
                pool_addresses,
            } => {
                // Create message for new pool
                let message = format!(
                    "🆕 *New {} Pool Detected*\n\nNetwork: *{}* ({})\nPools: \n`{}`",
                    pool_addresses.len(),
                    network_name,
                    network_id,
                    pool_addresses.join("`\n`")
                );

                // Escape the message content for MarkdownV2
                let escaped_message = escape_markdownv2(&message);

                // Send the message with Markdown parsing for clickable links
                if let Err(e) = self
                    .bot
                    .send_message(self.chat_id.clone(), escaped_message)
                    .message_thread_id(self.new_pool_thread_id as i32)
                    .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                    .disable_web_page_preview(true)
                    .send()
                    .await
                {
                    let error_msg = format!("Failed to send pool notification: {}", e);
                    eprintln!("{}", &error_msg);

                    // Log error to file
                    if let Err(log_err) = log_error_to_file(&error_msg) {
                        eprintln!("Failed to write to log file: {}", log_err);
                    }

                    if let Err(e) = self
                        .bot
                        .send_message(self.chat_id.clone(), "Error sending new pool notification")
                        .message_thread_id(self.failed_opp_thread_id as i32)
                        .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                        .disable_web_page_preview(true)
                        .send()
                        .await
                    {
                        let error_msg =
                            format!("Failed to send failed new pool notification: {}", e);
                        eprintln!("{}", &error_msg);

                        // Log error to file
                        if let Err(log_err) = log_error_to_file(&error_msg) {
                            eprintln!("Failed to write to log file: {}", log_err);
                        }
                    }
                }

                info!("New pool notification sent successfully");
            }
            NotificationType::HighOpportunity(opportunity) => {
                // Get network and token information
                let chain = self.get_network(opportunity.network_id).await;

                // Get block explorer URL from network if available
                let block_explorer = chain.as_ref().and_then(|n| n.block_explorer.clone());

                let chain_name = chain
                    .map(|n| n.name)
                    .unwrap_or_else(|| format!("Chain #{}", opportunity.network_id));

                let token_object = self
                    .get_token(opportunity.network_id, &opportunity.profit_token)
                    .await;
                let token = token_object
                    .clone()
                    .and_then(|t| t.symbol)
                    .unwrap_or_else(|| "Unknown".to_string());
                let token_decimals = token_object.and_then(|t| t.decimals).unwrap_or_else(|| 18);

                // Get opportunity ID
                let id = opportunity.id.map(|id| id.to_hex()).unwrap_or_default();
                let error_msg = opportunity.error.as_deref().unwrap_or("Unknown error");

                // Create dashboard and explorer links
                let dashboard_url = format!("http://188.245.98.132:8080/opportunities/{}", id);
                let explorer_url = if let Some(explorer) = block_explorer {
                    if let Some(tx) = &opportunity.execute_tx {
                        format!("{}/tx/{}", explorer, tx)
                    } else if let Some(tx) = &opportunity.source_tx {
                        format!("{}/tx/{}", explorer, tx)
                    } else {
                        "".to_string()
                    }
                } else {
                    "".to_string()
                };

                let profit = format_units(
                    U256::from_str_radix(
                        opportunity.profit.unwrap_or(String::from("0")).as_ref(),
                        10,
                    )
                    .unwrap_or(U256::ZERO),
                    token_decimals,
                )
                .unwrap_or("0.0".to_string())
                .parse::<f64>()
                .unwrap_or(0.0);

                let estimate_profit = opportunity.estimate_profit.unwrap_or("Unknown".to_string());
                let estimate_profit = format_units(
                    U256::from_str_radix(&estimate_profit, 10).unwrap_or(U256::ZERO),
                    token_decimals,
                )
                .unwrap_or("0.0".to_string())
                .parse::<f64>()
                .unwrap_or(0.0);
                let mut thread_id = self.big_opp_thread_id;

                let message = match opportunity.status.as_str().to_lowercase().as_str() {
                    "succeeded" => {
                        format!(
                            "🟢 *{:.4}* {} ~ $*{:.2}*\nStatus: *SUCCESS*\nNetwork: *{}*\nEstimated: *{:.4}* {} ~ $*{:.2}*\nGas: $*{:.2}*",
                            profit,
                            token,
                            opportunity.profit_usd.unwrap_or(0.0),
                            chain_name,
                            estimate_profit,
                            token,
                            opportunity.estimate_profit_usd.unwrap_or(0.0),
                            opportunity.gas_usd.unwrap_or(0.0)
                        )
                    }
                    "partially_succeeded" | "partiallysucceeded" => {
                        format!(
                            "🟡 *{:.4}* {} ~ $*{:.2}*\nStatus: *PARTIAL*\nNetwork: *{}*\nEstimated: *{:.4}* {} ~ $*{:.2}*\nGas: $*{:.2}*",
                            profit,
                            token,
                            opportunity.profit_usd.unwrap_or(0.0),
                            chain_name,
                            estimate_profit,
                            token,
                            opportunity.estimate_profit_usd.unwrap_or(0.0),
                            opportunity.gas_usd.unwrap_or(0.0)
                        )
                    }
                    "reverted" => {
                        thread_id = self.failed_opp_thread_id;
                        format!(
                            "🔴*{:.4}* {} ~ $*{:.2}*\nStatus: *REVERTED*\nNetwork: *{}*\nGas: $*{:.2}*\nBlock delay: *{}* blocks",
                            estimate_profit,
                            token,
                            opportunity.estimate_profit_usd.unwrap_or(0.0),
                            chain_name,
                            opportunity.gas_usd.unwrap_or(0.0),
                            opportunity.execute_block_number.unwrap_or(0) - opportunity.source_block_number.unwrap_or(0)
                        )
                    }
                    "error" => {
                        thread_id = self.failed_opp_thread_id;
                        format!(
                            "🔴*{:.4}* {} ~ $*{:.2}*\nStatus: *ERROR*\nNetwork: *{}*",
                            estimate_profit,
                            token,
                            opportunity.estimate_profit_usd.unwrap_or(0.0),
                            chain_name,
                        )
                    }
                    _ => {
                        return;
                    }
                };

                // Escape the message content for MarkdownV2
                let escaped_message = escape_markdownv2(&message);

                // Add error message if present
                let mut final_message = if !error_msg.contains("Unknown error") {
                    let escaped_error = escape_markdownv2(error_msg);
                    escaped_message + "\n" + &escaped_error
                } else {
                    escaped_message
                };

                // Escape URLs for MarkdownV2 format
                let dashboard_url_escaped = escape_markdownv2(&dashboard_url);
                let explorer_url_escaped = if !explorer_url.is_empty() {
                    escape_markdownv2(&explorer_url)
                } else {
                    String::new()
                };

                // Add links to dashboard and explorer using Telegram's MarkdownV2 link format
                final_message += "\n[View on Dashboard](";
                final_message += &dashboard_url_escaped;
                final_message += ")";

                if !explorer_url.is_empty() {
                    final_message += "\n[View on Explorer](";
                    final_message += &explorer_url_escaped;
                    final_message += ")";
                }

                // Send the message with Markdown parsing for clickable links
                if let Err(e) = self
                    .bot
                    .send_message(self.chat_id.clone(), final_message)
                    .message_thread_id(thread_id as i32)
                    .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                    .disable_web_page_preview(true)
                    .send()
                    .await
                {
                    let error_msg = format!("Failed to send notification: {}", e);
                    eprintln!("{}", &error_msg);

                    // Log error to file
                    if let Err(log_err) = log_error_to_file(&error_msg) {
                        eprintln!("Failed to write to log file: {}", log_err);
                    }

                    if let Err(e) = self
                        .bot
                        .send_message(self.chat_id.clone(), "Error sending notification")
                        .message_thread_id(self.failed_opp_thread_id as i32)
                        .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                        .disable_web_page_preview(true)
                        .send()
                        .await
                    {
                        let error_msg = format!("Failed to send failed notification: {}", e);
                        eprintln!("{}", &error_msg);

                        // Log error to file
                        if let Err(log_err) = log_error_to_file(&error_msg) {
                            eprintln!("Failed to write to log file: {}", log_err);
                        }
                    }
                }
            }
        }
        info!("Notification sent successfully");
    }
}

/// Log error message to a log file
fn log_error_to_file(error_msg: &str) -> std::io::Result<()> {
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let log_entry = format!("[{}] {}\n", timestamp, error_msg);

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("log.txt")?;

    file.write_all(log_entry.as_bytes())
}
