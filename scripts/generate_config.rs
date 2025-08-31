use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create config directory if it doesn't exist
    if !Path::new("config").exists() {
        fs::create_dir("config")?;
    }

    // Generate root config.toml
    let root_config = r#"# Arbitrage Bot API Configuration

[server]
host = "127.0.0.1"
port = 8080
log_level = "info"

[database]
uri = "mongodb://localhost:27017"
database_name = "arbitrage_bot"
connection_timeout_ms = 5000
max_pool_size = 10

[cors]
allowed_origins = [
    "http://localhost:3000",
    "http://127.0.0.1:3000"
]
allowed_methods = [
    "GET",
    "POST", 
    "PUT",
    "DELETE"
]
allowed_headers = [
    "Authorization",
    "Accept",
    "Content-Type"
]
supports_credentials = true
"#;

    fs::write("config.toml", root_config)?;
    println!("Generated config.toml");

    // Generate development config
    let dev_config = r#"# Arbitrage Bot API Configuration (Development)

[server]
host = "127.0.0.1"
port = 8081
log_level = "debug"

[database]
uri = "mongodb://localhost:27017"
database_name = "arbitrage_bot_dev"
connection_timeout_ms = 10000
max_pool_size = 5

[cors]
allowed_origins = [
    "http://localhost:3000",
    "http://127.0.0.1:3000",
    "http://localhost:3001"
]
allowed_methods = [
    "GET",
    "POST", 
    "PUT",
    "DELETE",
    "OPTIONS"
]
allowed_headers = [
    "Authorization",
    "Accept",
    "Content-Type",
    "X-Requested-With"
]
supports_credentials = true
"#;

    fs::write("config/config.toml", dev_config)?;
    println!("Generated config/config.toml");

    // Generate production config template
    let prod_config = r#"# Arbitrage Bot API Configuration (Production)
# Copy this file and modify as needed

[server]
host = "0.0.0.0"
port = 8080
log_level = "warn"

[database]
uri = "mongodb://username:password@host:port/database"
database_name = "arbitrage_bot_prod"
connection_timeout_ms = 10000
max_pool_size = 20

[cors]
allowed_origins = [
    "https://yourdomain.com",
    "https://www.yourdomain.com"
]
allowed_methods = [
    "GET",
    "POST", 
    "PUT",
    "DELETE"
]
allowed_headers = [
    "Authorization",
    "Accept",
    "Content-Type"
]
supports_credentials = true
"#;

    fs::write("config/production.toml", prod_config)?;
    println!("Generated config/production.toml");

    println!("\nConfiguration files generated successfully!");
    println!("Edit config.toml to customize your settings.");
    println!("For production, copy config/production.toml and modify as needed.");

    Ok(())
}
