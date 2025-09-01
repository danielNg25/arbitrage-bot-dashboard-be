## Arbitrage Bot API Reference

Base URL: `http://localhost:8081`

Media type: `application/json`

### Endpoints

-   GET `/api/v1/health`
-   GET `/api/v1/networks`
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

### GET /api/v1/opportunities

-   Description: Paginated opportunities with filtering and token enrichment

Query parameters:

-   `network_id` (u64, optional)
-   `status` (string, optional, case-insensitive)
-   `min_profit_usd` (f64, optional)
-   `max_profit_usd` (f64, optional)
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
