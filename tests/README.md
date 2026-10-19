# Integration tests

## Load tests

Async load & correctness tests for the facade-service, written with
`asyncio` + `aiohttp`.

### Setup

```bash
uv venv
uv pip install -r requirements.txt
```

### Run

Make sure the stack is up first (`docker compose up --build` from the repo
root), then:

```bash
# both assignment scenarios, 10 clients x 10_000 tx
uv run tests/load_test.py

# only scenario A (separate accounts) or B (one shared account)
uv run tests/load_test.py --scenario a
uv run tests/load_test.py --scenario b

# quicker smoke run
uv run tests/load_test.py --clients 10 --tx 1000

# point at a different facade
uv run tests/load_test.py --url http://localhost:8080
```

### What it reports

- **total requests** and **total time**
- **throughput** (requests/second = count / time)
- **latency breakdown** from the facade `/metrics` endpoint: how much
  accumulated time went to `logging-service` vs `counter-service`
- **correctness**: scenario A expects 10_000 on each of 10 accounts,
  scenario B expects 100_000 on the single shared account
