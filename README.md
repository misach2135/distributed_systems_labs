# Task 4 — Microservices with Message Queue (Kafka) and Config Server

Report: `report/`.

## Setup

```bash
uv venv
uv pip install -r requirements.txt

C="docker compose -f infra/docker-compose.yaml"
```

## Run

facade :8080, config-server :8003, 3 × logger (Hazelcast Map), counter (PostgreSQL, reads from Kafka), Kafka, Hazelcast × 3, Management Center (http://localhost:8090):

```bash
$C up -d --build
```

## Config server

```bash
curl localhost:8003/services/logger                             # addresses of registered loggers
curl localhost:8003/services/counter
curl localhost:8003/services/facade
```

## Requests

```bash
for i in $(seq 1 10); do curl -X POST localhost:8080/transaction --data "{user1, +$i}"; echo; done   # 10 transactions
curl localhost:8080/user/user1                                  # balance + transactions of a user
curl localhost:8080/accounts                                    # all balances
$C logs logger-1 logger-2 logger-3 | grep stored                # which logger got which transaction
$C logs counter-service | grep applied                          # transactions applied from Kafka
```

## Fault tolerance

```bash
docker pause ds-counter-service-1           # POSTs still succeed (queued in Kafka), GETs fail
docker unpause ds-counter-service-1         # counter applies the queued transactions
$C stop logger-1 logger-2                   # facade switches to the remaining logger
$C start logger-1 logger-2
```

## Load test

```bash
$C exec postgres psql -U bank -d bank -c 'truncate accounts, processed_txs;'   # reset balances
uv run tests/load_test.py                                                      # 10 clients × 10 000 tx, both scenarios
```

Stop: `$C down` (`$C down -v` also wipes PostgreSQL and Kafka data).
