# Task 1 — Microservices banking system

Three microservices communicating over HTTP, orchestrated with Docker
Compose. Implements a small banking system: clients submit credit/debit
transactions, which are logged and accumulated into per-account balances.

## Architecture

```
                 ┌─────────────────┐
   client ─────▶ │  facade-service │  :8080 (host)
   {user, +/-amt}│  - POST /transaction
                 │  - GET  /user/{id}
                 │  - GET  /accounts
                 │  - GET  /metrics
                 └───┬─────────┬───┘
            POST /log│         │POST /process
            GET /tx  │         │GET /balance(s)
                 ┌───▼───┐ ┌───▼────┐
                 │logging│ │counter │   (internal :8000)
                 │service│ │service │
                 └───────┘ └────────┘
```

- **facade-service** — entry point. Parses the client message, assigns a
  `transaction_id` (UUID) + timestamp, fans out concurrently to logging and
  counter services, and measures the time spent in each downstream call.
- **logging-service** — stores every transaction in memory
  (`transaction_id -> Transaction`) and returns them per user.
- **counter-service** — keeps per-account balances in memory and applies
  credit/debit operations atomically (safe under high concurrency).

Shared types and the custom message parser live in the `common` crate.

### Client message format

Custom (non-JSON) format `{user_Id, +/- amount}`, parsed via `FromStr`/
`Display` + regex in `common`:

```
{user1, +100}
{ user_42 , -42.5 }
{acc, 10}
```

Inter-service communication uses JSON.

## Run

```bash
docker compose up --build
```

Ports on the host:

| service         | host port | internal |
|-----------------|-----------|----------|
| facade-service  | 8080      | 8000     |
| logging-service | 8001      | 8000     |
| counter-service | 8002      | 8000     |

## Example requests

```bash
# Submit transactions (custom text body)
curl -X POST localhost:8080/transaction --data '{user1, +100}'
curl -X POST localhost:8080/transaction --data '{user1, -30}'
curl -X POST localhost:8080/transaction --data '{user2, +50}'
# -> {"transaction_id":"...","balance":70.0}

# One user: balance + their transactions
curl localhost:8080/user/user1

# All balances
curl localhost:8080/accounts
# -> {"user1":70.0,"user2":50.0}

# Accumulated latency: logging-service vs counter-service
curl localhost:8080/metrics

# Reset latency counters
curl -X POST localhost:8080/metrics/reset
```

## Load testing

See [`tests/README.md`](tests/README.md):

```bash
uv venv && uv pip install aiohttp
uv run tests/load_test.py
```

## Local build (without Docker)

```bash
cargo build --workspace
cargo test -p common

# run each service in its own terminal
BIND_ADDR=0.0.0.0:8001 cargo run -p logging_service
BIND_ADDR=0.0.0.0:8002 cargo run -p counter_service
BIND_ADDR=0.0.0.0:8080 LOGGING_URL=http://localhost:8001 \
  COUNTER_URL=http://localhost:8002 cargo run -p facade_service
```

(The facade defaults already point at `:8001` / `:8002`.)
