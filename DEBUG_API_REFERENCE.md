# Debug API Reference

## Debug Opportunity Endpoint

### POST `/api/v1/debug/opportunity`

Debug an opportunity by simulating the transaction using Foundry's `cast call` at a specific block.

#### Request Body

```json
{
    "opportunity": {
        "id": "ObjectId",
        "network_id": 1,
        "status": "pending",
        "profit_token": "0x...",
        "amount": "1000000000000000000",
        "profit": "100000000000000000",
        "profit_usd": 100.5,
        "gas_usd": 5.25,
        "path": ["0x...", "0x..."],
        "source_block_number": 12345678,
        "source_tx": "0x...",
        "created_at": 1704067200,
        "updated_at": 1704067200
    },
    "block_number": "12345678",
    "from_address": "0x...",
    "to": "0x3fC91A3afd70395Cd496C647d5a6CC9D4B2b7FAD"
}
```

#### Request Parameters

| Field          | Type          | Required | Description                                                                          |
| -------------- | ------------- | -------- | ------------------------------------------------------------------------------------ |
| `opportunity`  | `Opportunity` | Yes      | The opportunity object to debug                                                      |
| `block_number` | `string`      | Yes       | Block number to simulate at (defaults to latest)                                     |
| `from_address` | `string`      | Yes       | Address to simulate from (defaults to zero address)                                  |
| `to`           | `string`      | No\*     | Router contract address to call (defaults to network's router_address if configured) |

\*Either `to` parameter or network's router_address must be provided

#### Response

```json
{
    "success": true,
    "error": null,
    "trace": "Detailed trace output...",
    "gas_used": "21000",
    "block_number": "12345678",
    "transaction_data": "0x3593564c..."
}
```

#### Response Fields

| Field     | Type      | Description                          |
| --------- | --------- | ------------------------------------ |
| `success` | `boolean` | Whether the cast call was successful |
| `error`   | `string`  | Error message if the call failed     |
| `trace`   | `string`  | Detailed execution trace             |

| `gas_used` | `string` | Gas consumed during execution |
| `block_number` | `string` | Block number used for simulation |
| `transaction_data` | `string` | Hex-encoded transaction data that was sent |

#### Example Usage

```bash
curl -X POST http://localhost:8081/api/v1/debug/opportunity \
  -H "Content-Type: application/json" \
  -d '{
    "opportunity": {
      "id": "507f1f77bcf86cd799439011",
      "network_id": 1,
      "status": "pending",
      "profit_token": "0xA0b86a33E6441b8c4C8C0C8C0C8C0C8C0C8C0C8C",
      "amount": "1000000000000000000",
      "profit": "100000000000000000",
      "profit_usd": 100.50,
      "gas_usd": 5.25,
      "path": ["0xA0b86a33E6441b8c4C8C0C8C0C8C0C8C0C8C0C8C", "0xB1c97a44F7552c9d5D1D1D1D1D1D1D1D1D1D1D1D1D"],
      "source_block_number": 12345678,
      "source_tx": "0x1234567890abcdef...",
      "created_at": 1704067200,
      "updated_at": 1704067200
    },
    "block_number": "12345678",
    "from_address": "0x742d35Cc6634C0532925a3b8D4C9db96C4b4d8b6",
    "to": "0x3fC91A3afd70395Cd496C647d5a6CC9D4B2b7FAD"
  }'
```

#### Error Responses

```json
{
    "success": false,
    "error": "Cast call failed: execution reverted",
    "trace": null,
    "gas_used": null,
    "block_number": "12345678",
    "transaction_data": "0x3593564c..."
}
```

#### Prerequisites

1. **Foundry Installation**: The server must have Foundry installed with `cast` command available
2. **Network Configuration**: The network must have a valid RPC URL configured
3. **Valid Opportunity**: The opportunity must have valid token addresses and amounts

#### Notes

-   The API uses the provided `to` address or the network's router_address (an error is returned if neither is available)
-   Transaction data is built from the opportunity's path and amount fields
-   The simulation runs at the specified block number or the latest block if not provided
-   Gas usage is extracted from the trace output for analysis

#### Limitations

-   Currently uses a simplified transaction data construction
-   Either the `to` parameter or a configured router_address on the network is required
-   Path reconstruction is basic (may need enhancement for complex arbitrage paths)
-   Requires the opportunity to have valid path data for accurate simulation
