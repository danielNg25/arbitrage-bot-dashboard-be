# Opportunities API Route - Technical Specification

## Overview

The `/api/v1/opportunities` endpoint provides comprehensive access to arbitrage opportunity data with advanced filtering, pagination, and data limiting capabilities. This API is designed for high-performance dashboard applications and real-time monitoring systems.

## Endpoint Details

**URL:** `GET /api/v1/opportunities`  
**Base URL:** `http://localhost:8081` (development)  
**Content-Type:** `application/json`  
**Response Format:** JSON with pagination metadata

## Query Parameters

### Core Filtering Parameters

| Parameter    | Type     | Required | Default | Description                                    |
| ------------ | -------- | -------- | ------- | ---------------------------------------------- |
| `network_id` | `u64`    | No       | -       | Filter by specific blockchain network chain ID |
| `status`     | `string` | No       | -       | Filter by single opportunity execution status  |

### Profit Range Filtering

| Parameter        | Type  | Required | Default | Description                     |
| ---------------- | ----- | -------- | ------- | ------------------------------- |
| `min_profit_usd` | `f64` | No       | -       | Minimum profit threshold in USD |
| `max_profit_usd` | `f64` | No       | -       | Maximum profit threshold in USD |

**Usage Examples:**

-   `min_profit_usd=10.0` - Only opportunities with profit ≥ $10.00
-   `max_profit_usd=100.0` - Only opportunities with profit ≤ $100.00
-   `min_profit_usd=50.0&max_profit_usd=200.0` - Profit range $50.00 - $200.00

### Gas Cost Filtering

| Parameter              | Type     | Required | Default | Description                                           |
| ---------------------- | -------- | -------- | ------- | ----------------------------------------------------- |
| `min_gas_usd`          | `f64`    | No       | -       | Minimum gas cost threshold in USD                     |
| `max_gas_usd`          | `f64`    | No       | -       | Maximum gas cost threshold in USD                     |
| `min_source_timestamp` | `string` | No       | -       | Minimum source timestamp (ISO 8601 or Unix timestamp) |
| `max_source_timestamp` | `string` | No       | -       | Maximum source timestamp (ISO 8601 or Unix timestamp) |

**Usage Examples:**

-   `min_gas_usd=0.001` - Only opportunities with gas cost ≥ $0.001
-   `max_gas_usd=5.0` - Only opportunities with gas cost ≤ $5.00
-   `min_gas_usd=0.1&max_gas_usd=2.0` - Gas cost range $0.10 - $2.00

### Created At Timestamp Filtering

| Parameter        | Type     | Required | Default | Description                                            |
| ---------------- | -------- | -------- | ------- | ------------------------------------------------------ |
| `min_created_at` | `string` | No       | -       | Minimum created timestamp (ISO 8601 or Unix timestamp) |
| `max_created_at` | `string` | No       | -       | Maximum created timestamp (ISO 8601 or Unix timestamp) |

**Usage Examples:**

**ISO 8601 Format:**

-   `min_created_at=2024-01-01T00:00:00Z` - Only opportunities created after January 1, 2024
-   `max_created_at=2024-01-31T23:59:59Z` - Only opportunities created before January 31, 2024
-   `min_created_at=2024-01-01T00:00:00Z&max_created_at=2024-01-31T23:59:59Z` - Creation time range January 1-31, 2024

**Unix Timestamp Format:**

-   `min_created_at=1704067200` - Only opportunities created after January 1, 2024 (Unix: 1704067200)
-   `max_created_at=1706745599` - Only opportunities created before January 31, 2024 (Unix: 1706745599)
-   `min_created_at=1704067200&max_created_at=1706745599` - Creation time range January 1-31, 2024

**Mixed Format (also supported):**

-   `min_created_at=1704067200&max_created_at=2024-01-31T23:59:59Z` - Mix of Unix and ISO formats

### Pagination Parameters

| Parameter | Type  | Required | Default | Max Value | Description                    |
| --------- | ----- | -------- | ------- | --------- | ------------------------------ |
| `page`    | `u32` | No       | 1       | -         | Page number (1-based indexing) |
| `limit`   | `u32` | No       | 100     | 1000      | Number of items per page       |

**Pagination Rules:**

-   Page numbers start from 1
-   Default page size is 100 items
-   Maximum page size is enforced at 1000 items
-   If limit exceeds 1000, it's automatically capped

## Response Structure

### Main Response Object

```typescript
interface PaginatedOpportunitiesResponse {
    opportunities: OpportunityResponse[];
    pagination: PaginationInfo;
}
```

### Opportunity Data Structure

```typescript
interface OpportunityResponse {
    network_id: number; // Blockchain network chain ID
    status: string; // Execution status (e.g., "succeeded", "reverted")
    profit_usd: number | null; // Profit amount in USD (null if not available)
    profit_amount: string | null; // Profit amount in token's native units (null if not available)
    gas_usd: number | null; // Gas cost in USD (null if not available)
    created_at: string; // ISO 8601 timestamp (YYYY-MM-DDTHH:MM:SSZ)
    source_tx: string | null; // Source transaction hash (null if not available)
    source_block_number: number | null; // Source block number (null if not available)
    execute_block_number: number | null; // Block number when opportunity was executed (null if not available)
    profit_token: string; // Profit token contract address
    profit_token_name: string | null; // Profit token name (null if not available)
    profit_token_symbol: string | null; // Profit token symbol (null if not available)
    profit_token_decimals: number | null; // Profit token decimals (null if not available)
}
```

### Pagination Metadata

```typescript
interface PaginationInfo {
    page: number; // Current page number
    limit: number; // Items per page (actual limit applied)
    total: number; // Total number of opportunities matching filters
    total_pages: number; // Total number of pages available
    has_next: boolean; // Whether next page exists
    has_prev: boolean; // Whether previous page exists
}
```

## Status Values

The `status` field can contain the following values (based on actual database data):

-   `"Error"` - Transaction failed with an error
-   `"PartiallySucceeded"` - Transaction partially successful
-   `"Reverted"` - Transaction was reverted
-   `"Skipped"` - Transaction was skipped

**Note:**

-   The actual status values in the database use Title Case formatting
-   **Case-Insensitive Support**: The API supports both uppercase and lowercase forms for single status filtering
-   Single status: `"error"`, `"Error"`, `"ERROR"` all work
-   **Important**: Status "succeeded" does not exist in the current database - use "PartiallySucceeded" instead

## Complete API Examples

### 1. Basic Request (Default Pagination)

```bash
GET /api/v1/opportunities
```

**Response:** First 100 opportunities with default sorting

### 2. Network-Specific Filtering

```bash
GET /api/v1/opportunities?network_id=14&status=PartiallySucceeded
```

**Response:** All partially succeeded opportunities on network 14 (Polygon)

### 3. Multiple Status Filtering

```bash
GET /api/v1/opportunities?statuses=PartiallySucceeded&statuses=Error
```

**Response:** All opportunities with status "PartiallySucceeded" OR "Error"

```bash
GET /api/v1/opportunities?statuses=Error&statuses=Reverted&statuses=Skipped
```

**Response:** All opportunities with status "Error" OR "Reverted" OR "Skipped"

**Note:** The `statuses[]` format is also supported but may have parsing issues in some environments.

### 4. Profit Range Filtering

```bash
GET /api/v1/opportunities?min_profit_usd=10.0&max_profit_usd=100.0
```

**Response:** Opportunities with profit between $10.00 and $100.00

### 5. Gas Cost Analysis

```bash
GET /api/v1/opportunities?min_gas_usd=0.001&max_gas_usd=1.0
```

**Response:** Opportunities with gas costs between $0.001 and $1.00

### 6. Pagination Control

```bash
GET /api/v1/opportunities?page=2&limit=50
```

**Response:** Second page with 50 opportunities per page

### 7. Combined Filtering with Pagination

```bash
GET /api/v1/opportunities?network_id=14&min_profit_usd=50.0&page=1&limit=200
```

**Response:** First page of 200 opportunities on network 14 with profit ≥ $50.00

### 8. Advanced Status Combinations

```bash
GET /api/v1/opportunities?statuses=PartiallySucceeded&statuses=Error&min_profit_usd=0.01&network_id=252
```

**Response:** Partially succeeded and error opportunities on network 252 with profit ≥ $0.01

## Curl Examples

### Single Status Filtering

```bash
# Filter by single status (Error) - Case-insensitive
curl "http://localhost:8081/api/v1/opportunities?status=Error&limit=5"
curl "http://localhost:8081/api/v1/opportunities?status=error&limit=5"
curl "http://localhost:8081/api/v1/opportunities?status=ERROR&limit=5"

# Filter by single status (PartiallySucceeded) - Case-insensitive
curl "http://localhost:8081/api/v1/opportunities?status=PartiallySucceeded&limit=10"
curl "http://localhost:8081/api/v1/opportunities?status=partiallysucceeded&limit=10"

# Filter by single status with network - Case-insensitive
curl "http://localhost:8081/api/v1/opportunities?status=Reverted&network_id=252&limit=20"
curl "http://localhost:8081/api/v1/opportunities?status=reverted&network_id=252&limit=20"
```

### Combined Filtering Examples

```bash
# Single status + profit range + pagination - Case-insensitive
curl "http://localhost:8081/api/v1/opportunities?status=PartiallySucceeded&min_profit_usd=0.01&page=1&limit=50"
curl "http://localhost:8081/api/v1/opportunities?status=partiallysucceeded&min_profit_usd=0.01&page=1&limit=50"

# Single status + gas cost range + network - Case-insensitive
curl "http://localhost:8081/api/v1/opportunities?status=Error&min_gas_usd=0.001&max_gas_usd=0.01&network_id=252&limit=100"
curl "http://localhost:8081/api/v1/opportunities?status=error&min_gas_usd=0.001&max_gas_usd=0.01&network_id=252&limit=100"

# Single status + created timestamp range (ISO format) - Case-insensitive
curl "http://localhost:8081/api/v1/opportunities?status=PartiallySucceeded&min_created_at=2024-01-01T00:00:00Z&max_created_at=2024-01-31T23:59:59Z&limit=100"
curl "http://localhost:8081/api/v1/opportunities?status=partiallysucceeded&min_created_at=2024-01-01T00:00:00Z&max_created_at=2024-01-31T23:59:59Z&limit=100"

# Single status + created timestamp range (Unix format) - Case-insensitive
curl "http://localhost:8081/api/v1/opportunities?status=PartiallySucceeded&min_created_at=1704067200&max_created_at=1706745599&limit=100"
curl "http://localhost:8081/api/v1/opportunities?status=partiallysucceeded&min_created_at=1704067200&max_created_at=1706745599&limit=100"

# Complex combination - Case-insensitive
curl "http://localhost:8081/api/v1/opportunities?status=PartiallySucceeded&min_profit_usd=0.01&max_profit_usd=1.0&min_gas_usd=0.001&min_created_at=2024-01-01T00:00:00Z&network_id=252&page=1&limit=200"
curl "http://localhost:8081/api/v1/opportunities?status=partiallysucceeded&min_profit_usd=0.01&max_profit_usd=1.0&min_gas_usd=0.001&min_created_at=2024-01-01T00:00:00Z&network_id=252&page=1&limit=200"
```

## Response Examples

### Sample Response with Token Information

```json
{
    "opportunities": [
        {
            "network_id": 1,
            "status": "succeeded",
            "profit_usd": 25.5,
            "gas_usd": 3.2,
            "created_at": "2024-01-15T10:30:00Z",
            "source_tx": "0x1234567890abcdef1234567890abcdef12345678",
            "source_block_number": 18456789,
            "profit_token": "0xa0b86a33e6441b8c4c8c8c8c8c8c8c8c8c8c8c8c",
            "profit_token_name": "USD Coin",
            "profit_token_symbol": "USDC",
            "profit_token_decimals": 6
        },
        {
            "network_id": 14,
            "status": "partially_succeeded",
            "profit_usd": 12.75,
            "gas_usd": 1.8,
            "created_at": "2024-01-15T09:45:00Z",
            "source_tx": "0xabcdef1234567890abcdef1234567890abcdef12",
            "source_block_number": 45678901,
            "profit_token": "0x1f9840a85d5af5bf1d1762f925bdaddc4201f984",
            "profit_token_name": "Uniswap",
            "profit_token_symbol": "UNI",
            "profit_token_decimals": 18
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

## URL Parameter Format

### Case-Insensitive Status Filtering

The API supports case-insensitive status filtering for single status queries:

```bash
# All of these work the same way:
curl "http://localhost:8081/api/v1/opportunities?status=Error&limit=5"
curl "http://localhost:8081/api/v1/opportunities?status=error&limit=5"
curl "http://localhost:8081/api/v1/opportunities?status=ERROR&limit=5"
curl "http://localhost:8081/api/v1/opportunities?status=ErRoR&limit=5"
```

## Technical Implementation Details

### MongoDB Aggregation Pipeline (Join Query)

The API uses MongoDB's aggregation pipeline to efficiently join opportunities with token information in a single database query, eliminating the need for multiple API calls.

#### Pipeline Stages:

1. **`$match`** - Apply filters (network_id, status, profit_usd, gas_usd)
2. **`$lookup`** - Join with tokens collection based on network_id and profit_token address
3. **`$addFields`** - Extract token data from joined results
4. **`$sort`** - Sort by created_at (descending)
5. **`$skip`** - Apply pagination offset
6. **`$limit`** - Apply pagination limit

#### Join Logic:

```javascript
{
  "$lookup": {
    "from": "tokens",
    "let": {
      "opp_network_id": "$network_id",
      "opp_profit_token": "$profit_token"
    },
    "pipeline": [
      {
        "$match": {
          "$expr": {
            "$and": [
              { "$eq": ["$network_id", "$$opp_network_id"] },
              { "$eq": ["$address", "$$opp_profit_token"] }
            ]
          }
        }
      }
    ],
    "as": "token_info"
  }
}
```

#### Benefits:

-   **Single Query**: No N+1 query problem
-   **Efficient**: MongoDB handles the join optimization
-   **Scalable**: Works with large datasets
-   **Real-time**: Always returns current token information

### Data Enrichment

The API automatically enriches opportunity data with:

-   **Token Metadata**: Name, symbol, decimals from the tokens collection
-   **Transaction Details**: Source transaction hash and block number
-   **Network Context**: Full network information for each opportunity

This specification provides everything needed to build a comprehensive frontend interface for the opportunities API, including filtering, pagination, data visualization components, and enriched token information through efficient database joins.
