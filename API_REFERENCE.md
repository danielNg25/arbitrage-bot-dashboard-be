## Arbitrage Bot API Reference

Base URL: `http://localhost:8081/api/v1/networks`

Media type: `application/json`

### Endpoints

-   GET `/api/v1/health`
-   GET `/api/v1/networks`
-   GET `/api/v1/networks/{network_id}/aggregations`
-   GET `/api/v1/tokens/performance`
-   GET `/api/v1/time-aggregations`
-   GET `/api/v1/summary-aggregations`
-   GET `/api/v1/opportunities`
-   GET `/api/v1/opportunities/{id}`
-   GET `/api/v1/opportunities/tx/{tx_hash}`
-   GET `/api/v1/opportunities/profit-over-time`
-   POST `/api/v1/admin/index`

---

### GET /api/v1/health

-   Description: Service health check
-   Request body: none

Response 200:

```json
{
    "status": "healthy",
    "service": "arbitrage-bot-api",
    "timestamp": "2024-01-01T00:00:00Z"
}
```

---

### GET /api/v1/networks

-   Description: Returns all networks with aggregated statistics
-   Request body: none

Response 200 (array of `NetworkResponse`):

```typescript
interface NetworkResponse {
    id: string;
    chain_id: number;
    name: string;
    rpc: string | null;
    block_explorer: string | null;
    router_address: string | null;
    executors: string[] | null; // Executor addresses for this network
    executed: number | null;
    success: number | null;
    failed: number | null;
    total_profit_usd: number;
    total_gas_usd: number;
    last_proccesed_created_at: number | null;
    created_at: number;
    success_rate: number | null; // success / executed when executed > 0
}
```

---

### GET /api/v1/networks/{network_id}/aggregations

-   Description: Returns time-based aggregations for a specific network
-   Request body: none

Path parameters:

-   `network_id` (number) - The chain ID of the network to query

Query parameters:

-   `period` (string, optional) - Time period: "hourly", "daily", "monthly"
-   `start_time` (string, optional) - Start time filter (ISO 8601 or Unix timestamp)
-   `end_time` (string, optional) - End time filter (ISO 8601 or Unix timestamp)
-   `limit` (number, optional) - Number of results to return (default 100, max 1000)
-   `offset` (number, optional) - Number of results to skip (default 0)

Response 200 (array of `TimeAggregationResponse`):

```typescript
interface TimeAggregationResponse {
    network_id: number;
    network_name: string;
    period: string; // "hourly", "daily", "monthly"
    timestamp: number; // Unix timestamp for the period start
    period_start: string; // ISO 8601 formatted period start
    period_end: string; // ISO 8601 formatted period end
    total_opportunities: number;
    executed_opportunities: number;
    successful_opportunities: number;
    failed_opportunities: number;
    total_profit_usd: number;
    total_gas_usd: number;
    avg_profit_usd: number; // Calculated on-the-fly: total_profit_usd / total_opportunities
    avg_gas_usd: number; // Calculated on-the-fly: total_gas_usd / total_opportunities
    success_rate: number; // Calculated on-the-fly: successful_opportunities / executed_opportunities
    top_profit_tokens: {
        address: string;
        name: string | null;
        symbol: string | null;
        total_profit_usd: number;
        total_profit: string; // U256 as string
        opportunity_count: number;
        avg_profit_usd: number;
    }[];
}
```

Example request:

```bash
curl "http://localhost:8081/api/v1/networks/1/aggregations?period=hourly&limit=24"
```

Example response:

```json
[
    {
        "network_id": 1,
        "network_name": "Ethereum",
        "period": "hourly",
        "timestamp": 1704067200,
        "period_start": "2024-01-01T00:00:00Z",
        "period_end": "2024-01-01T01:00:00Z",
        "total_opportunities": 150,
        "executed_opportunities": 120,
        "successful_opportunities": 95,
        "failed_opportunities": 25,
        "total_profit_usd": 1250.75,
        "total_gas_usd": 45.2,
        "avg_profit_usd": 8.34,
        "avg_gas_usd": 0.3,
        "success_rate": 0.79,
        "top_profit_tokens": [
            {
                "address": "0xA0b86a33E6441b8c4C8C0E4b8c4C8C0E4b8c4C8C0",
                "name": "USD Coin",
                "symbol": "USDC",
                "total_profit_usd": 450.25,
                "total_profit": "450250000",
                "opportunity_count": 25,
                "avg_profit_usd": 18.01
            }
        ]
    }
]
```

---

### GET /api/v1/tokens/performance

-   Description: Returns token performance data with aggregated profit metrics
-   Request body: none

Query parameters:

-   `network_id` (u64, optional) - Filter by specific network/chain
-   `limit` (u64, optional) - Number of results to return (default 50, max 1000)
-   `offset` (u64, optional) - Number of results to skip (default 0)

Response 200 (array of `TokenPerformanceResponse`):

```typescript
interface TokenPerformanceResponse {
    name: string | null; // Full token name
    symbol: string | null; // Token symbol (ETH, USDC, etc.)
    total_profit_usd: number; // Total profit from arbitrage
    total_profit: string; // Total profit in token units (U256 string)
    price: number | null; // Current USD price
    address: string; // Contract address
    network_id: number; // Chain ID (1, 137, 56, etc.)
    network_name: string; // Network name
}
```

Example request:

```bash
curl -X GET "http://localhost:8081/api/v1/tokens/performance?network_id=1&limit=10"
```

Example response:

```json
[
    {
        "name": "Ethereum",
        "symbol": "ETH",
        "total_profit_usd": 1250.75,
        "total_profit": "500000000000000000",
        "price": 2500.15,
        "address": "0x0000000000000000000000000000000000000000",
        "network_id": 1,
        "network_name": "Ethereum"
    },
    {
        "name": "USD Coin",
        "symbol": "USDC",
        "total_profit_usd": 850.25,
        "total_profit": "850250000",
        "price": 1.0,
        "address": "0xa0b86a33e6c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0",
        "network_id": 1,
        "network_name": "Ethereum"
    }
]
```

---

### GET /api/v1/time-aggregations

-   Description: Returns time-based aggregations (hourly, daily, monthly) of arbitrage performance
-   Request body: none

Query parameters:

-   `network_id` (u64, optional) - Filter by specific network/chain
-   `period` (string, optional) - Time period: "hourly", "daily", "monthly"
-   `start_time` (string, optional) - Start time filter (ISO 8601 or Unix timestamp)
-   `end_time` (string, optional) - End time filter (ISO 8601 or Unix timestamp)
-   `limit` (u64, optional) - Number of results to return (default 100, max 1000)
-   `offset` (u64, optional) - Number of results to skip (default 0)

Response 200 (array of `TimeAggregationResponse`):

```typescript
interface TimeAggregationResponse {
    network_id: number;
    network_name: string;
    period: string; // "hourly", "daily", "monthly"
    timestamp: number; // Unix timestamp for period start
    period_start: string; // ISO 8601 formatted period start
    period_end: string; // ISO 8601 formatted period end
    total_opportunities: number;
    executed_opportunities: number;
    successful_opportunities: number;
    failed_opportunities: number;
    total_profit_usd: number;
    total_gas_usd: number;
    avg_profit_usd: number; // Calculated on-the-fly: total_profit_usd / total_opportunities
    avg_gas_usd: number; // Calculated on-the-fly: total_gas_usd / total_opportunities
    success_rate: number; // Calculated on-the-fly: successful_opportunities / executed_opportunities
    top_profit_tokens: TokenAggregationResponse[];
}

interface TokenAggregationResponse {
    address: string;
    name: string | null;
    symbol: string | null;
    total_profit_usd: number;
    total_profit: string; // U256 string
    opportunity_count: number;
    avg_profit_usd: number;
}
```

Example request:

```bash
curl -X GET "http://localhost:8081/api/v1/time-aggregations?period=daily&network_id=1&limit=7"
```

Example response:

```json
[
    {
        "network_id": 1,
        "network_name": "Ethereum",
        "period": "daily",
        "timestamp": 1735689600,
        "period_start": "2025-01-01T00:00:00Z",
        "period_end": "2025-01-02T00:00:00Z",
        "total_opportunities": 1250,
        "executed_opportunities": 850,
        "successful_opportunities": 720,
        "failed_opportunities": 130,
        "total_profit_usd": 12500.75,
        "total_gas_usd": 850.25,
        "avg_profit_usd": 10.0,
        "avg_gas_usd": 0.68,
        "success_rate": 0.847,
        "top_profit_tokens": [
            {
                "address": "0x0000000000000000000000000000000000000000",
                "name": "Ethereum",
                "symbol": "ETH",
                "total_profit_usd": 5000.25,
                "total_profit": "2000000000000000000",
                "opportunity_count": 250,
                "avg_profit_usd": 20.0
            }
        ]
    }
]
```

---

### GET /api/v1/summary-aggregations

-   Description: Returns cross-network summary aggregations (hourly, daily, monthly) of arbitrage performance
-   Request body: none

Query parameters:

-   `period` (string, optional) - Time period: "hourly", "daily", "monthly"
-   `start_time` (string, optional) - Start time filter (ISO 8601 or Unix timestamp)
-   `end_time` (string, optional) - End time filter (ISO 8601 or Unix timestamp)
-   `limit` (u64, optional) - Number of results to return (default 100, max 1000)
-   `offset` (u64, optional) - Number of results to skip (default 0)

Response 200 (array of `SummaryAggregationResponse`):

```typescript
interface SummaryAggregationResponse {
    period: string; // "hourly", "daily", "monthly"
    timestamp: number; // Unix timestamp for the period start
    period_start: string; // ISO 8601 formatted period start
    period_end: string; // ISO 8601 formatted period end
    total_opportunities: number; // Total across all networks
    executed_opportunities: number; // Total executed across all networks
    successful_opportunities: number; // Total successful across all networks
    failed_opportunities: number; // Total failed across all networks
    total_profit_usd: number; // Total profit across all networks
    total_gas_usd: number; // Total gas costs across all networks
}
```

Example request:

```bash
curl -X GET "http://localhost:8081/api/v1/summary-aggregations?period=daily&limit=10"
```

Example response:

```json
[
    {
        "period": "daily",
        "timestamp": 1704067200,
        "period_start": "2024-01-01T00:00:00Z",
        "period_end": "2024-01-02T00:00:00Z",
        "total_opportunities": 1250,
        "executed_opportunities": 1050,
        "successful_opportunities": 890,
        "failed_opportunities": 160,
        "total_profit_usd": 12500.75,
        "total_gas_usd": 850.25
    }
]
```

---

### GET /api/v1/opportunities

-   Description: Paginated opportunities with filtering and token enrichment

Query parameters:

-   `network_id` (u64, optional)
-   `status` (string, optional, case-insensitive)
-   `min_profit_usd` (f64, optional)
-   `max_profit_usd` (f64, optional)
-   `min_estimate_profit_usd` (f64, optional)
-   `max_estimate_profit_usd` (f64, optional)
-   `min_gas_usd` (f64, optional)
-   `max_gas_usd` (f64, optional)
-   `min_created_at` (string, optional; ISO 8601 or Unix seconds)
-   `max_created_at` (string, optional; ISO 8601 or Unix seconds)
-   `page` (u32, optional; default 1)
-   `limit` (u32, optional; default 100, max 1000)

Response 200 (`PaginatedOpportunitiesResponse`):

```typescript
interface PaginationInfo {
    page: number;
    limit: number;
    total: number;
    total_pages: number;
    has_next: boolean;
    has_prev: boolean;
}

interface OpportunityResponse {
    id: string;
    network_id: number;
    status: string;
    profit_usd: number | null;
    estimate_profit_usd: number | null;
    estimate_profit: string | null;
    profit_amount: string | null;
    gas_usd: number | null;
    created_at: string; // ISO 8601
    source_tx: string | null;
    source_block_number: number | null;
    execute_block_number: number | null;
    profit_token: string;
    profit_token_name: string | null;
    profit_token_symbol: string | null;
    profit_token_decimals: number | null;
    simulation_time: number | null;
    error: string | null;
}

interface PaginatedOpportunitiesResponse {
    opportunities: OpportunityResponse[];
    pagination: PaginationInfo;
}
```

Example:

```bash
curl "http://localhost:8081/api/v1/opportunities?status=Error&min_profit_usd=0.01&page=1&limit=50"
```

Notes:

-   Status filter is case-insensitive (e.g., `Error`, `error`, `ERROR`)
-   `min_created_at`/`max_created_at` accept ISO 8601 (`YYYY-MM-DDTHH:MM:SSZ`) or Unix seconds
-   Results include token metadata joined from the `tokens` collection

---

### GET /api/v1/opportunities/{id}

-   Description: Detailed opportunity by MongoDB ObjectId
-   Path params:
    -   `id` (string; 24-char hex ObjectId)

Response 200 (`OpportunityDetailsResponse`):

```typescript
interface TokenResponse {
    id: string;
    address: string;
    name: string | null;
    symbol: string | null;
    decimals: number | null;
    price: number | null;
}

interface PoolResponse {
    id: string;
    address: string;
    pool_type: string;
    tokens: string[];
}

interface OpportunityDetailsData {
    id: string;
    network_id: number;
    status: string;
    profit_usd: number | null;
    profit_amount: string | null;
    gas_usd: number | null;
    created_at: string; // ISO 8601
    source_tx: string | null;
    source_block_number: number | null;
    source_log_index: number | null;
    source_pool: string | null;
    execute_block_number: number | null;
    execute_tx: string | null;
    profit_token: string;
    profit_token_name: string | null;
    profit_token_symbol: string | null;
    profit_token_decimals: number | null;
    amount: string | null;
    gas_token_amount: string | null;
    updated_at: string; // ISO 8601
    // Debug fields
    estimate_profit: string | null;
    estimate_profit_usd: number | null;
    path: string[] | null;
    received_at: string | null; // ISO 8601
    send_at: string | null; // ISO 8601
    simulation_time: number | null;
    error: string | null;
    gas_amount: number | null;
    gas_price: number | null;
}

interface NetworkResponse {
    /* same as above */
}

interface OpportunityDetailsResponse {
    opportunity: OpportunityDetailsData;
    network: NetworkResponse;
    path_tokens: TokenResponse[];
    path_pools: PoolResponse[];
}
```

Example:

```bash
curl "http://localhost:8081/api/v1/opportunities/68b45589fa99519f09784f84"
```

---

### GET /api/v1/opportunities/tx/{tx_hash}

-   Description: Detailed opportunity found by `execute_tx`
-   Path params:
    -   `tx_hash` (string; 0x-prefixed hex)

Response 200: Same schema as `/api/v1/opportunities/{id}`

Example:

```bash
curl "http://localhost:8081/api/v1/opportunities/tx/0xabc...123"
```

---

### GET /api/v1/opportunities/profit-over-time

-   Description: Total profit per day for the last 30 days
-   Request body: none

Response 200 (array):

```typescript
interface ProfitOverTimeResponse {
    date: string; // YYYY-MM-DD
    profit_usd: number;
}
```

Example:

```bash
curl "http://localhost:8081/api/v1/opportunities/profit-over-time"
```

---

### POST /api/v1/admin/index

-   Description: Trigger manual indexing of network and token metrics
-   Request body: none

Response 200:

```json
{
    "status": "success",
    "message": "Indexing completed successfully",
    "timestamp": "2024-01-01T00:00:00Z"
}
```

---

### Status values

Status field may include (examples):

-   `Error`
-   `PartiallySucceeded`
-   `Reverted`
-   `Skipped`

Notes:

-   Single-status filtering is case-insensitive in `/opportunities` list endpoint
-   Timestamps provided by the API are ISO 8601 formatted strings where indicated; filters accept ISO 8601 or Unix seconds
-   Derived fields (`avg_profit_usd`, `avg_gas_usd`, `success_rate`) are calculated on-the-fly during API response generation
-   Summary aggregations contain only the core metrics; derived fields are not stored in the database to ensure data consistency
