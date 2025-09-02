## WebSocket Reference (wss/ws)

Base (HTTP): `http://localhost:8081`
Base (HTTPS): `https://your-domain`

Endpoint:

-   Path: `/api/v1/ws/opportunities`
-   Protocol: `ws://` (local/dev) or `wss://` (prod over TLS)

Query parameters (same as Opportunities list filters):

-   `network_id` (u64, optional)
-   `status` (string, optional; case-insensitive exact match)
-   `min_profit_usd` (f64, optional)
-   `max_profit_usd` (f64, optional)
-   `min_estimate_profit_usd` (f64, optional)
-   `max_estimate_profit_usd` (f64, optional)
-   `min_gas_usd` (f64, optional)
-   `max_gas_usd` (f64, optional)

Notes:

-   This is a live stream of newly inserted opportunities only (no historical replay).
-   Time-window filters are not applied for the stream.
-   **Data structure is identical to the Opportunities REST API** - same fields, same formatting, same token enrichment.

Message format (JSON per message) - **Identical to `/api/v1/opportunities` response**:

```json
{
    "id": "68b45589fa99519f09784f84",
    "network_id": 1,
    "status": "Succeeded",
    "profit_usd": 2.34,
    "estimate_profit_usd": 2.5,
    "estimate_profit": "1000000000000000000",
    "profit_amount": "1000000000000000000",
    "gas_usd": 0.12,
    "created_at": "2024-01-01T00:00:00Z",
    "source_tx": "0xabc...123",
    "source_block_number": 12345678,
    "execute_block_number": 12345679,
    "profit_token": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
    "profit_token_name": "USD Coin",
    "profit_token_symbol": "USDC",
    "profit_token_decimals": 6,
    "simulation_time": 150,
    "error": null
}
```

**Field Details:**

-   `id`: MongoDB ObjectId as string
-   `network_id`: Chain ID (1, 137, 56, etc.)
-   `status`: Opportunity status (Succeeded, Failed, Pending, etc.)
-   `profit_usd`: Actual profit in USD (if available)
-   `estimate_profit_usd`: Estimated profit from debug data
-   `estimate_profit`: Raw profit amount from debug data
-   `profit_amount`: Profit amount in token's native units
-   `gas_usd`: Gas cost in USD
-   `created_at`: ISO 8601 formatted timestamp
-   `source_tx`: Source transaction hash
-   `source_block_number`: Source block number
-   `execute_block_number`: Execution block number
-   `profit_token`: Token contract address
-   `profit_token_name`: Full token name (enriched from tokens collection)
-   `profit_token_symbol`: Token symbol (ETH, USDC, etc.)
-   `profit_token_decimals`: Token decimal places
-   `simulation_time`: Simulation time in milliseconds
-   `error`: Error message if any

Example connections

-   JavaScript (browser/node):

```javascript
const params = new URLSearchParams({
    network_id: '1',
    status: 'Succeeded',
    min_profit_usd: '0.1',
});

const ws = new WebSocket(`wss://your-domain/api/v1/ws/opportunities?${params}`);

ws.onopen = () => console.log('WS connected');
ws.onmessage = (ev) => {
    const data = JSON.parse(ev.data);
    console.log('New opportunity', data);

    // Data structure is identical to REST API - use same components!
    if (data.profit_token_symbol === 'USDC') {
        console.log(`New USDC opportunity: $${data.profit_usd}`);
    }
};
ws.onclose = () => console.log('WS closed');
ws.onerror = (e) => console.error('WS error', e);
```

-   websocat (CLI):

```bash
websocat "ws://localhost:8081/api/v1/ws/opportunities?network_id=1&status=Succeeded&min_profit_usd=0.1"
```

**Integration Benefits:**

-   🎯 **Same data structure** as REST API - no need for different handling
-   🔄 **Seamless switching** between polling and real-time updates
-   📊 **Consistent UI components** - reuse existing opportunity display logic
-   🗄️ **Same database queries** - identical filtering and enrichment

**CORS Troubleshooting:**

If you're getting "Origin not allowed" errors from your local frontend:

1. **Check your CORS configuration** in `config/config.toml`:

    ```toml
    [cors]
    allowed_origins = [
        "http://localhost:3000",    # Your frontend port
        "http://127.0.0.1:3000",
        "http://localhost:3001",
    ]
    ```

2. **Ensure WebSocket headers are included**:

    ```toml
    allowed_headers = [
        "Upgrade",
        "Connection",
        "Sec-WebSocket-Key",
        "Sec-WebSocket-Version",
        "Sec-WebSocket-Protocol",
        # ... other headers
    ]
    ```

3. **Restart the server** after changing CORS config:

    ```bash
    cargo build && RUST_LOG=debug cargo run
    ```

4. **Check server logs** for CORS-related messages when connecting

5. **Verify frontend origin** matches exactly (including protocol, host, and port)

Common issues:

-   Frontend running on different port than configured
-   Missing WebSocket-specific headers in CORS config
-   Server not restarted after config changes

Enabling wss (TLS)

-   The server speaks plain HTTP; terminate TLS at a reverse proxy (e.g., Nginx/Caddy) and forward upgrade requests.

Nginx example:

```nginx
server {
    listen 443 ssl http2;
    server_name your-domain;

    ssl_certificate /etc/letsencrypt/live/your-domain/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/your-domain/privkey.pem;

    location /api/v1/ws/ {
        proxy_pass http://127.0.0.1:8081;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
    }

    location / {
        proxy_pass http://127.0.0.1:8081;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```
