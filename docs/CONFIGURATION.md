# Configuration Guide

The Arbitrage Bot API uses TOML configuration files for easy customization. This guide explains how to configure the API for different environments.

## Configuration File Locations

The API will look for configuration files in the following order:

1. `config.toml` in the project root
2. `config/config.toml` in the config directory
3. Environment variables (as fallback)

## Configuration Structure

### Server Configuration

```toml
[server]
host = "127.0.0.1"        # Server host (use "0.0.0.0" for production)
port = 8080               # Server port
log_level = "info"        # Logging level (error, warn, info, debug, trace)
```

### Database Configuration

```toml
[database]
uri = "mongodb://localhost:27017"           # MongoDB connection string
database_name = "arbitrage_bot"             # Database name
connection_timeout_ms = 5000                # Connection timeout in milliseconds
max_pool_size = 10                         # Maximum connection pool size
```

### CORS Configuration

```toml
[cors]
allowed_origins = [                         # Allowed frontend origins
    "http://localhost:3000",
    "http://127.0.0.1:3000"
]
allowed_methods = [                         # Allowed HTTP methods
    "GET",
    "POST",
    "PUT",
    "DELETE"
]
allowed_headers = [                         # Allowed HTTP headers
    "Authorization",
    "Accept",
    "Content-Type"
]
supports_credentials = true                 # Allow credentials (cookies, auth headers)
```

## MongoDB Configuration Examples

### Local Development

```toml
[database]
uri = "mongodb://localhost:27017"
database_name = "arbitrage_bot_dev"
connection_timeout_ms = 5000
max_pool_size = 5
```

### Remote MongoDB (Atlas)

```toml
[database]
uri = "mongodb+srv://username:password@cluster.mongodb.net/?retryWrites=true&w=majority"
database_name = "arbitrage_bot_prod"
connection_timeout_ms = 10000
max_pool_size = 20
```

### MongoDB with Authentication

```toml
[database]
uri = "mongodb://username:password@host:port/database?authSource=admin"
database_name = "arbitrage_bot"
connection_timeout_ms = 10000
max_pool_size = 15
```

### MongoDB with SSL/TLS

```toml
[database]
uri = "mongodb://username:password@host:port/database?ssl=true&sslVerifyCertificate=false"
database_name = "arbitrage_bot"
connection_timeout_ms = 10000
max_pool_size = 20
```

## Environment Variables

You can override configuration values using environment variables:

```bash
# Server configuration
export SERVER_HOST="0.0.0.0"
export SERVER_PORT="9090"
export RUST_LOG="debug"

# Database configuration
export MONGODB_URI="mongodb://localhost:27017"
export MONGODB_DATABASE="arbitrage_bot_prod"

# CORS configuration
export CORS_ORIGINS="https://yourdomain.com,https://www.yourdomain.com"
```

## Configuration File Examples

### Development Configuration (`config.toml`)

```toml
[server]
host = "127.0.0.1"
port = 8080
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
```

### Production Configuration (`config/production.toml`)

```toml
[server]
host = "0.0.0.0"
port = 8080
log_level = "warn"

[database]
uri = "mongodb://username:password@host:port/database?authSource=admin&ssl=true"
database_name = "arbitrage_bot_prod"
connection_timeout_ms = 15000
max_pool_size = 25

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
```

## MongoDB Connection String Format

### Basic Format

```
mongodb://[username:password@]host[:port][/database][?options]
```

### Common Options

-   `authSource=admin` - Authentication database
-   `ssl=true` - Enable SSL/TLS
-   `sslVerifyCertificate=false` - Skip SSL certificate verification
-   `retryWrites=true` - Enable retryable writes
-   `w=majority` - Write concern
-   `maxPoolSize=20` - Maximum connection pool size
-   `connectTimeoutMS=10000` - Connection timeout
-   `socketTimeoutMS=45000` - Socket timeout

### Examples

```bash
# Local MongoDB
mongodb://localhost:27017

# Local MongoDB with database
mongodb://localhost:27017/arbitrage_bot

# Remote MongoDB with authentication
mongodb://username:password@host:27017/arbitrage_bot?authSource=admin

# MongoDB Atlas
mongodb+srv://username:password@cluster.mongodb.net/arbitrage_bot?retryWrites=true&w=majority

# MongoDB with SSL
mongodb://username:password@host:27017/arbitrage_bot?ssl=true&sslVerifyCertificate=false
```

## Testing Configuration

You can test your configuration by running:

```bash
# Test configuration loading
cargo test config::tests

# Check if configuration is valid
cargo check

# Run with specific configuration
RUST_LOG=debug cargo run
```

## Troubleshooting

### Common Issues

1. **Configuration file not found**: Ensure the file exists and has the correct name
2. **Invalid TOML syntax**: Check for syntax errors in your TOML file
3. **MongoDB connection failed**: Verify the connection string and network access
4. **Permission denied**: Check file permissions for configuration files

### Debug Configuration

Enable debug logging to see configuration details:

```bash
RUST_LOG=debug cargo run
```

This will show:

-   Configuration file loading attempts
-   Final configuration values
-   MongoDB connection details

### Validation

The configuration system validates:

-   TOML syntax
-   Required fields
-   Data types
-   MongoDB connection string format

## Security Considerations

1. **Never commit sensitive data**: Use environment variables for passwords
2. **Restrict file permissions**: Set appropriate file permissions for config files
3. **Use SSL/TLS**: Enable SSL for production MongoDB connections
4. **Limit CORS origins**: Only allow necessary frontend origins
5. **Secure MongoDB**: Use authentication and network restrictions
