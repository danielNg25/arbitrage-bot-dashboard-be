# Opportunity Details API Specification

## Endpoint: `GET /api/v1/opportunities/{id}`

Returns comprehensive details for a specific opportunity, including the opportunity data, network information, and the complete trading path with tokens and pools.

### URL Parameters

| Parameter | Type     | Required | Description                         |
| --------- | -------- | -------- | ----------------------------------- |
| `id`      | `string` | Yes      | MongoDB ObjectId of the opportunity |

### Response Format

```json
{
    "opportunity": {
        "network_id": 1,
        "status": "succeeded",
        "profit_usd": 25.5,
        "profit_amount": "1000000000000000000",
        "gas_usd": 2.3,
        "created_at": "2024-01-15T10:30:00Z",
        "source_tx": "0x1234...",
        "source_block_number": 12345678,
        "execute_block_number": 12345679,
        "profit_token": "0x6B175474E89094C44Da98b954EedeAC495271d0F",
        "profit_token_name": "Dai Stablecoin",
        "profit_token_symbol": "DAI",
        "profit_token_decimals": 18
    },
    "network": {
        "chain_id": 1,
        "name": "Ethereum Mainnet",
        "rpc": "https://eth-mainnet.alchemyapi.io/v2/your-api-key",
        "block_explorer": "https://etherscan.io",
        "executed": 1500,
        "success": 1200,
        "failed": 300,
        "total_profit_usd": 50000.0,
        "total_gas_usd": 5000.0,
        "last_proccesed_created_at": 1705312200,
        "created_at": 1704067200,
        "success_rate": 0.8
    },
    "path_tokens": [
        {
            "address": "0xA0b86a33E6441b8c4C8D7d3c2b8b7c1d5e2f3a4b",
            "name": "Wrapped Ether",
            "symbol": "WETH",
            "decimals": 18,
            "price": 2500.0
        },
        {
            "address": "0x6B175474E89094C44Da98b954EedeAC495271d0F",
            "name": "Dai Stablecoin",
            "symbol": "DAI",
            "decimals": 18,
            "price": 1.0
        }
    ],
    "path_pools": [
        {
            "address": "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc",
            "pool_type": "UniswapV2",
            "tokens": [
                "0xA0b86a33E6441b8c4C8D7d3c2b8b7c1d5e2f3a4b",
                "0x6B175474E89094C44Da98b954EedeAC495271d0F"
            ]
        }
    ]
}
```

### Response Fields

#### `opportunity` - Opportunity Information

-   `id`: MongoDB ObjectId as string for further API operations
-   `network_id`: Network chain ID where the opportunity was executed
-   `status`: Current status of the opportunity (e.g., "succeeded", "reverted", "error")
-   `profit_usd`: Profit in USD (if available)
-   `profit_amount`: Profit amount in the token's native units
-   `gas_usd`: Gas cost in USD (if available)
-   `created_at`: ISO 8601 timestamp when the opportunity was created
-   `source_tx`: Transaction hash that discovered the opportunity
-   `source_block_number`: Block number where the opportunity was discovered
-   `execute_block_number`: Block number where the opportunity was executed
-   `profit_token`: Address of the token received as profit
-   `profit_token_name`: Human-readable name of the profit token
-   `profit_token_symbol`: Symbol of the profit token
-   `profit_token_decimals`: Decimal places for the profit token

#### `network` - Network Information

-   `id`: MongoDB ObjectId as string for further API operations
-   `chain_id`: Unique identifier for the blockchain network
-   `name`: Human-readable network name
-   `rpc`: RPC endpoint URL for the network
-   `block_explorer`: Block explorer URL for the network
-   `executed`: Total number of executed opportunities on this network
-   `success`: Number of successful opportunities
-   `failed`: Number of failed opportunities
-   `total_profit_usd`: Total profit generated on this network
-   `total_gas_usd`: Total gas costs on this network
-   `last_proccesed_created_at`: Timestamp of last processed opportunity
-   `created_at`: When the network was added to the system
-   `success_rate`: Calculated success rate (success/executed)

#### `path_tokens` - Trading Path Tokens

Array of tokens involved in the arbitrage path:

-   `id`: MongoDB ObjectId as string for further API operations
-   `address`: Token contract address
-   `name`: Human-readable token name
-   `symbol`: Token symbol
-   `decimals`: Number of decimal places
-   `price`: Current token price in USD (if available)

#### `path_pools` - Trading Path Pools

Array of pools involved in the arbitrage path:

-   `id`: MongoDB ObjectId as string for further API operations
-   `address`: Pool contract address
-   `pool_type`: Type of pool (e.g., "UniswapV2", "UniswapV3", "ERC4626")
-   `tokens`: Array of token addresses in the pool

### Path Structure

The trading path follows a specific pattern:

-   **Even indices (0, 2, 4, ...)**: Token addresses
-   **Odd indices (1, 3, 5, ...)**: Pool addresses

This represents the flow: Token → Pool → Token → Pool → Token, etc.

### Usage Examples

#### Basic Request

```bash
curl "http://localhost:8081/api/v1/opportunities/507f1f77bcf86cd799439011"
```

#### Response with jq

```bash
curl "http://localhost:8081/api/v1/opportunities/507f1f77bcf86cd799439011" | jq '.opportunity.status'
curl "http://localhost:8081/api/v1/opportunities/507f1f77bcf86cd799439011" | jq '.network.name'
curl "http://localhost:8081/api/v1/opportunities/507f1f77bcf86cd799439011" | jq '.path_tokens[0].symbol'
```

### Error Responses

#### 404 Not Found

```json
{
    "error": "Opportunity not found"
}
```

#### 500 Internal Server Error

```json
{
    "error": "Failed to fetch opportunity details"
}
```

### Notes

-   The endpoint performs multiple MongoDB queries to gather all related data
-   If a token or pool in the path is not found in the database, basic information is returned with the address
-   The profit token details are automatically populated if the token exists in the path
-   All timestamps are returned in ISO 8601 format
-   The path arrays maintain the order from the original opportunity discovery
