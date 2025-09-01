# Arbitrage Bot Backend API

A high-performance, async Rust backend API using Actix Web and MongoDB for an arbitrage bot dashboard.

## Configuration

The application uses a `config.toml` file for configuration. Create this file in the `config/` directory:

### config.toml

```toml
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
allowed_origins = ["http://localhost:3000"]
allowed_methods = ["GET", "POST", "PUT", "DELETE"]
allowed_headers = ["Authorization", "Accept", "Content-Type"]
supports_credentials = true

[indexer]
interval_minutes = 5
```

### Indexing Configuration

The `[indexer]` section controls the background indexing behavior:

-   **`interval_minutes`**: How often the background indexer runs (in minutes)
    -   **Recommended values**: 5-30 minutes
    -   **Too frequent** (1-2 min): May impact database performance
    -   **Too infrequent** (60+ min): Dashboard data may become stale

### Examples

**Every 10 minutes:**

```toml
[indexer]
interval_minutes = 10
```

**Every 30 minutes:**

```toml
[indexer]
interval_minutes = 30
```

**Every hour:**

```toml
[indexer]
interval_minutes = 60
```

## Running the Application

1. **Build the project:**

    ```bash
    cargo build
    ```

2. **Start the server:**

    ```bash
    cargo run
    ```

3. **Manual indexing trigger:**
    ```bash
    curl -X POST http://localhost:8080/api/v1/admin/index
    ```

## API Endpoints

-   `GET /api/v1/networks` - Get all networks with aggregated metrics
-   `GET /api/v1/opportunities` - Get opportunities with filtering and pagination
-   `GET /api/v1/opportunities/{id}` - Get detailed opportunity information
-   `GET /api/v1/opportunities/tx/{tx_hash}` - Get opportunity by transaction hash
-   `GET /api/v1/opportunities/profit-over-time` - Get profit data over time
-   `POST /api/v1/admin/index` - Trigger manual indexing

## Background Indexing

The application automatically runs a background indexing process that:

1. Aggregates profit and gas metrics for networks
2. Counts executed, successful, and failed opportunities
3. Updates token profit totals
4. Runs at the configured interval (default: every 5 minutes)

## Environment Variables

If no `config.toml` file is found, the application falls back to environment variables:

-   `SERVER_HOST` - Server host (default: 127.0.0.1)
-   `SERVER_PORT` - Server port (default: 8081)
-   `MONGODB_URI` - MongoDB connection string
-   `MONGODB_DATABASE` - Database name
