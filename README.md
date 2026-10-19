# Task 2 — Hazelcast: Distributed Map & Distributed Lock

Report: `report/task2/`.

## Setup

```bash
uv venv
uv pip install -r requirements.txt

C="docker compose -f infra/docker-compose.yaml -f infra/task2.docker-compose.override.yaml"
```

## Cluster

3 members with CP Subsystem + Management Center (http://localhost:8090):

```bash
$C up -d hazelcast-1 hazelcast-2 hazelcast-3 management-center
HZ_BACKUP_COUNT=2 $C up -d --force-recreate hazelcast-1 hazelcast-2 hazelcast-3   # change map backup-count (default 1)
```

## Part I — Distributed Map

```bash
uv run utils/hazelcast.py fill    # write keys 0..9_999
uv run utils/hazelcast.py check   # map size and missing keys
```

## Part II — Distributed Lock

3 counter services, FencedLock + IAtomicLong, count to 100 000:

```bash
$C run --rm lock-counter-1 reset                                        # iteration = 0, counter = 0
$C up --force-recreate lock-counter-1 lock-counter-2 lock-counter-3      # start the counters
docker kill -s KILL ds-lock-counter-<N>-1                               # crash the lock owner
$C run --rm lock-counter-1 status                                       # iteration and counter
```

Stop everything: `$C down`.
