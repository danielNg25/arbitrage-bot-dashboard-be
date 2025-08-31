# Arbitrage Bot Backend API

A high-performance Rust backend API built with Actix Web framework for serving data to an arbitrage bot dashboard. The API integrates with MongoDB and provides real-time data for networks, opportunities, and profit analytics.

## Features

-   **High Performance**: Built with Actix Web and async MongoDB queries
-   **Real-time Data**: Serves live arbitrage opportunity data
-   **MongoDB Integration**: Uses the official MongoDB Rust driver
-   **CORS Support**: Configured for React frontend integration
-   **Comprehensive Logging**: Built-in logging with env_logger
-   **Error Handling**: Custom error responses with proper HTTP status codes
-   **Dummy Data**: Auto-initializes with sample data for testing

## API Endpoints

### 1. GET /api/v1/networks

Returns all networks with comprehensive metrics and statistics data.

**Response:**

```json
[
    {
        "chain_id": 1,
        "name": "Ethereum Mainnet",
        "rpc": "https://eth-mainnet.alchemyapi.io/v2/your-api-key",
        "block_explorer": "https://etherscan.io",
        "executed": 150,
        "success": 120,
        "failed": 30,
        "total_profit_usd": 1250.5,
        "total_gas_usd": 45.2,
        "last_proccesed_created_at": 1704067200,
        "created_at": 1704067200,
        "executed_opportunities": 150,
        "success_rate": 80.0
    }
]
```

### 2. GET /api/v1/opportunities

Returns opportunities with optional filtering and pagination. Maximum 1000 opportunities per request.

**Query Parameters:**

-   `network_id` (optional): Filter by specific network chain ID
-   `status` (optional): Filter by opportunity status
-   `min_profit_usd` (optional): Minimum profit in USD
-   `max_profit_usd` (optional): Maximum profit in USD
-   `min_gas_usd` (optional): Minimum gas cost in USD
-   `max_gas_usd` (optional): Maximum gas cost in USD
-   `page` (optional): Page number (default: 1)
-   `limit` (optional): Items per page (default: 100, max: 1000)

**Response:**

```json
{
    "opportunities": [
        {
            "network_id": 1,
            "status": "succeeded",
            "profit_usd": 25.5,
            "gas_usd": 3.2,
            "created_at": "2024-01-15T10:30:00Z"
        }
    ],
    "pagination": {
        "page": 1,
        "limit": 100,
        "total": 1250,
        "total_pages": 13,
        "has_next": true,
        "has_prev": false
    }
}
```

### 3. GET /api/v1/opportunities/profit-over-time

Returns aggregated profit data for the last 30 days, grouped by day.

**Response:**

```json
[
    {
        "date": "2024-01-15",
        "profit_usd": 1250.75
    }
]
```

### 4. GET /api/v1/health

Health check endpoint for monitoring.

**Response:**

```json
{
    "status": "healthy",
    "service": "arbitrage-bot-api",
    "timestamp": "2024-01-15T10:30:00Z"
}
```

## Prerequisites

-   Rust 1.70+ (stable)
-   MongoDB 5.0+ (optional - dummy data will be used if not available)
-   Cargo package manager

## Installation

1. **Clone the repository:**

```bash
git clone <repository-url>
cd arbitrage-bot-api
```

2. **Install dependencies:**

```bash
cargo build
```

3. **Generate configuration files:**

```bash
make config
```

4. **Configure MongoDB (optional):**

```bash
make mongodb-setup
```

5. **Run the application:**

```bash
cargo run
```

The API will be available at `http://127.0.0.1:8081`

## Quick Start with Makefile

The project includes a Makefile with common development tasks:

```bash
# Show all available commands
make help

# Generate configuration files
make config

# Setup MongoDB configuration
make mongodb-setup

# Build the project
make build

# Run the API server
make run

# Run tests
make test

# Setup development environment
make setup
```

## Configuration

1. `config.toml` in the project root
2. `config/config.toml` in the config directory
3. Environment variables (as fallback)

### Configuration File Structure

```toml
[server]
host = "127.0.0.1"
port = 8081
log_level = "info"

[database]
uri = "mongodb://localhost:27017"
database_name = "arbitrage_bot"
connection_timeout_ms = 5000
max_pool_size = 10

[cors]
allowed_origins = ["http://localhost:3000"]
allowed_methods = ["GET", "POST", "PUT", "DELETE"]
allowed_headers = ["Authorization", "Accept", "Content-Type"]
supports_credentials = true
```

### Environment Variables (Override)

-   `SERVER_HOST`: Server host (default: `127.0.0.1`)
-   `SERVER_PORT`: Server port (default: `8081`)
-   `RUST_LOG`: Logging level (default: `info`)
-   `MONGODB_URI`: MongoDB connection string
-   `MONGODB_DATABASE`: MongoDB database name
-   `CORS_ORIGINS`: Comma-separated list of allowed origins

### MongoDB Setup

If you have MongoDB running locally:

1. **Start MongoDB:**

```bash
# macOS with Homebrew
brew services start mongodb-community

# Ubuntu/Debian
sudo systemctl start mongod

# Docker
docker run -d -p 27017:27017 --name mongodb mongo:latest
```

2. **Create database:**

```bash
mongosh
use arbitrage_bot
```

## Development

### Project Structure

```
src/
├── main.rs          # Application entry point and server setup
├── models.rs        # MongoDB models and response types
├── utils.rs         # Utility functions and enums
├── database.rs      # Database connection and query logic
├── handlers.rs      # HTTP request handlers
├── routes.rs        # Route configuration
└── errors.rs        # Custom error types and handling
```

### Adding New Endpoints

1. **Add handler in `handlers.rs`:**

```rust
pub async fn new_endpoint(db: web::Data<Database>) -> Result<HttpResponse, ApiError> {
    // Implementation
    Ok(HttpResponse::Ok().json(data))
}
```

2. **Add route in `routes.rs`:**

```rust
.route("/new-endpoint", web::get().to(new_endpoint))
```

3. **Update models if needed in `models.rs`**

### Database Queries

The API uses MongoDB aggregation pipelines for complex queries. Example:

```rust
let pipeline = vec![
    doc! {
        "$match": { "status": "succeeded" }
    },
    doc! {
        "$group": {
            "_id": "$network_id",
            "total_profit": { "$sum": "$profit_usd" }
        }
    }
];
```

## Testing

### Manual Testing

Test the API endpoints using curl:

```bash
# Get networks
curl http://localhost:8081/api/v1/networks

# Get opportunities with basic filtering
curl "http://localhost:8081/api/v1/opportunities?network_id=1&status=succeeded"

# Get opportunities with multiple statuses
curl "http://localhost:8081/api/v1/opportunities?statuses=succeeded&statuses=partially_succeeded"

# Get opportunities with profit range filtering
curl "http://localhost:8081/api/v1/opportunities?min_profit_usd=10.0&max_profit_usd=100.0"

# Get opportunities with gas cost filtering
curl "http://localhost:8081/api/v1/opportunities?min_gas_usd=1.0&max_gas_usd=10.0"

# Get opportunities with pagination
curl "http://localhost:8081/api/v1/opportunities?page=2&limit=50"

# Get opportunities with combined filters and pagination
curl "http://localhost:8081/api/v1/opportunities?network_id=1&min_profit_usd=50.0&page=1&limit=200"

# Get profit over time
curl http://localhost:8081/api/v1/opportunities/profit-over-time

# Health check
curl http://localhost:8081/api/v1/health
```

### Unit Tests

Run the test suite:

```bash
cargo test
```

## Performance Considerations

-   **Async Operations**: All database operations are async for high concurrency
-   **Connection Pooling**: MongoDB client uses connection pooling
-   **Streaming**: Large result sets are streamed to avoid memory issues
-   **Indexing**: Ensure proper MongoDB indexes for production use

## Production Deployment

### Docker

```dockerfile
FROM rust:1.70 as builder
WORKDIR /usr/src/app
COPY . .
RUN cargo build --release

FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/app/target/release/arbitrage-bot-api /usr/local/bin/
EXPOSE 8081
CMD ["arbitrage-bot-api"]
```

### Environment Variables for Production

```bash
export MONGODB_URI="mongodb://username:password@host:port/database"
export RUST_LOG="warn"
export PORT="8081"
```

## Monitoring and Logging

The API includes comprehensive logging:

-   **Request Logging**: All HTTP requests are logged
-   **Error Logging**: Detailed error information with stack traces
-   **Performance Logging**: Database query timing and response times

### Log Levels

-   `error`: Errors and failures
-   `warn`: Warnings and potential issues
-   `info`: General information and requests
-   `debug`: Detailed debugging information
-   `trace`: Very detailed tracing

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Support

For questions and support:

-   Create an issue in the repository
-   Check the API documentation
-   Review the logs for debugging information

## Changelog

### v0.1.0

-   Initial release
-   Basic CRUD operations for networks and opportunities
-   Profit over time analytics
-   MongoDB integration with dummy data
-   CORS support for React frontend
