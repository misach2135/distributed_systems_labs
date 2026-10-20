# Task 3 — Microservices with Hazelcast Distributed Map

Report: `report/`.

## Setup

```bash
uv venv
uv pip install -r requirements.txt

C="docker compose -f infra/docker-compose.yaml"
```

## Run

facade :8080, 3 × logger (Hazelcast Map), counter (PostgreSQL), Hazelcast × 3, Management Center (http://localhost:8090):

```bash
$C up -d --build
```

## Requests

```bash
for i in $(seq 1 10); do curl -X POST localhost:8080/transaction --data "{user1, +$i}"; echo; done   # 10 transactions
curl localhost:8080/user/user1                                  # balance + transactions of a user
curl localhost:8080/accounts                                    # all balances
$C logs logger-1 logger-2 logger-3 | grep stored                # which logger got which transaction
```

## Fault tolerance

```bash
$C stop logger-1 logger-2                   # stop logger instances
$C start logger-1 logger-2
docker kill -s KILL ds-hazelcast-1-1        # crash Hazelcast members (backup-count = 2)
$C up -d hazelcast-1
```

## Load test

```bash
$C exec postgres psql -U bank -d bank -c 'truncate accounts;'   # reset balances
uv run tests/load_test.py                                       # 10 clients × 10 000 tx, both scenarios
```

Stop: `$C down` (`$C down -v` also wipes the PostgreSQL data).
