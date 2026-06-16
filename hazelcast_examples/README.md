# Hazelcast Distributed Structures — Go examples

Go client applications for **Task 2 — Hazelcast Distributed Structures** (Hazelcast 5.4.x,
CP Subsystem enabled). They talk to a **pre-existing, manually started 3-node cluster**.

- **Part 1** — demonstrates a Distributed Map (`IMap`): writes 10,000 entries so you can
  watch partition distribution and node-failure re-balancing in Management Center.
- **Part 2** — a highly-available, fault-tolerant counter that reaches **100,000** using the
  **CP Subsystem**: a distributed lock (mutual exclusion + automatic failover), an
  `AtomicLong` for the durable iteration number, and a `Map` for the counter value.

```text
hazelcast_examples/
├── go.mod
├── go.sum
├── README.md
├── internal/
│   ├── hzconfig/hzconfig.go   # builds client config from env vars
│   └── cplock/cplock.go       # lease-based distributed lock (CP AtomicRef)
├── part1_map/main.go          # Part 1 — Distributed Map
└── part2_lock/main.go         # Part 2 — Distributed Lock + Atomic Counter
```

---

## Important: FencedLock is not available in the Go client

The lab text suggests using a Hazelcast **FencedLock** via
`client.CPSubsystem().GetLock(...)`. The Hazelcast **Java member** supports FencedLock, but
the **official Hazelcast Go client (v1.6.0) does not expose a FencedLock proxy**. Its CP
Subsystem only provides:

```
client.CPSubsystem().GetAtomicLong(...)
client.CPSubsystem().GetAtomicRef(...)
client.CPSubsystem().GetMap(...)        // CP map
```

There is **no** `GetLock` / `FencedLock` in the Go client. To deliver the exact behaviour the
lab requires, this project implements an **equivalent lease-based lock on top of CP
`AtomicRef`** (see [`internal/cplock/cplock.go`](internal/cplock/cplock.go)).

Because `AtomicRef` lives in the CP Subsystem, its `CompareAndSet` is **linearizable**
(backed by Raft consensus) — exactly the primitive needed to build a correct
mutual-exclusion lock. The lease lock provides the two guarantees the lab cares about:

| FencedLock property                       | How the lease lock provides it                                  |
| ----------------------------------------- | --------------------------------------------------------------- |
| Only one owner at a time                  | Acquisition uses linearizable `CompareAndSet` on a CP AtomicRef |
| Auto-release when the owner crashes       | The owner holds a **time-bounded lease** and renews it in the background; if it dies, the lease expires and another instance steals it — the analogue of FencedLock CP **session expiry** |

> If you specifically need real FencedLock semantics from Go, the alternatives are to use a
> Java/Python/Node client (which do implement FencedLock), or to drive it over the
> Management Center / REST. For this lab, the CP `AtomicRef` lease lock is functionally
> equivalent and uses the same CP Subsystem.

---

## Prerequisites

- Go 1.21+ (developed with Go 1.25).
- A running 3-node Hazelcast 5.4.x cluster with the **CP Subsystem enabled**
  (`cp-subsystem.cp-member-count: 3`), reachable on `localhost:5701` (and `5702`, `5703`).
- (Optional) Hazelcast Management Center to inspect data distribution.

Dependencies are already declared in `go.mod`:

- `github.com/hazelcast/hazelcast-go-client`
- `github.com/google/uuid` (unique per-instance identity)

```bash
cd hazelcast_examples
go mod download   # or: go mod tidy
go build ./...    # sanity check — should succeed
```

## Configuration (environment variables)

Both programs connect to `localhost:5701` / cluster name `dev` by default. Override with:

| Variable           | Default            | Meaning                                  |
| ------------------ | ------------------ | ---------------------------------------- |
| `HZ_CLUSTER_ADDRS` | `localhost:5701`   | Comma-separated member addresses         |
| `HZ_CLUSTER_NAME`  | `dev`              | Cluster name configured on the members   |

Listing all three members is recommended so the smart client keeps working when one node
is killed:

```bash
export HZ_CLUSTER_ADDRS="127.0.0.1:5701,127.0.0.1:5702,127.0.0.1:5703"
export HZ_CLUSTER_NAME="dev"
```

---

## Part 1 — Distributed Map

[`part1_map/main.go`](part1_map/main.go)

What it does:

1. Connects to the cluster.
2. Opens the distributed map **`lab-map`** and clears it (so reruns are deterministic).
3. Inserts **10,000** entries: keys `"0".."9999"`, values `"value_0".."value_9999"`.
4. Prints progress and the final map size reported by the cluster.

Run:

```bash
cd hazelcast_examples
go run ./part1_map
```

Expected tail of the output:

```
  inserted 10000/10000 entries...
Done. Inserted 10000 entries into "lab-map" in 1.2s (cluster reports size=10000).
```

**Observing fault tolerance (manual, lab requirement):** open Management Center → *Map →
lab-map* and look at how the 10,000 entries (271 partitions by default) are spread across
the 3 nodes. Then `kill` one or two members and refresh — Hazelcast re-balances the
partitions onto the surviving nodes from the backup replicas, and the total entry count
stays 10,000.

---

## Part 2 — Distributed Lock + Atomic Counter (CP Subsystem)

[`part2_lock/main.go`](part2_lock/main.go)

### Structures used (all strongly consistent, survive node failures via Raft)

| Structure              | Name                 | Role                                                |
| ---------------------- | -------------------- | --------------------------------------------------- |
| CP `AtomicRef`         | `lab-lock`           | Backs the lease lock (mutual exclusion + failover)  |
| CP `AtomicLong`        | `iteration-counter`  | Durable iteration number → exact resume point       |
| `Map` key `total_likes`| `counter-map`        | The actual counter value the lab asks for           |

### Logic flow

1. Connect; resolve the three structures above; build a `LeaseLock` over `lab-lock`.
2. **Acquire the lock** (`lock.Lock`). Blocks until this instance wins it.
3. Print `Acquired lock. Starting/Resuming work from iteration N...`.
4. Reconcile the Map counter to the durable `AtomicLong` value (so they can never drift if a
   previous owner crashed mid-step).
5. **Inner loop while holding the lock**, until `AtomicLong` reaches 100,000:
   - `IncrementAndGet` the `AtomicLong` (durable iteration).
   - Read `total_likes` from the Map, `+1`, write it back.
   - Print `Iteration: X, Counter: Y`.
   - **Sleep 100 ms** — mandatory, so you have time to kill the process and test failover.
   - A background goroutine renews the lock lease every 3 s; if the lease is lost the worker
     stops mutating shared state immediately and tries to re-acquire.
6. On graceful exit (Ctrl+C) the lock is released immediately so failover is instant.

### Run three instances (three terminals)

```bash
cd hazelcast_examples
go run ./part2_lock     # terminal 1
go run ./part2_lock     # terminal 2
go run ./part2_lock     # terminal 3
```

You'll see **only one** instance counting at a time; the others sit waiting on the lock.

### Test the failover

- **Graceful kill (Ctrl+C):** the active instance releases the lock and another instance
  prints `Acquired lock. Starting/Resuming work from iteration N...` within ~1 second and
  continues from the next number.
- **Hard kill (`kill -9 <pid>`):** the dead instance cannot release anything. After the
  **lease expires (~10 s)** another waiting instance's `CompareAndSet` succeeds, it steals
  the lock and resumes from the exact iteration stored in the `AtomicLong`. This is the
  CP-session-expiry equivalent that the lab is asking you to demonstrate.

When the target is reached you'll see:

```
[PID-12345-ab12cd34] DONE. Target reached: iteration=100000, counter=100000.
```

Any instance started afterwards detects the target is already met and exits cleanly.

### Resetting between runs

`AtomicLong` and the Map persist in the cluster, so a second run finds the target already
reached. To start over, restart the cluster, or clear the structures (e.g. set
`iteration-counter` to 0 and remove `total_likes` from `counter-map`) — for example from a
Java/SQL console or by adding a tiny reset helper.

---

## Lock tuning (in `part2_lock/main.go`)

| Constant        | Value  | Meaning                                                                 |
| --------------- | ------ | ----------------------------------------------------------------------- |
| `leaseDuration` | 10 s   | How long one lease is valid → also the worst-case failover time on `kill -9` |
| `renewEvery`    | 3 s    | How often a healthy owner extends its lease                             |
| `retryEvery`    | 1 s    | How often a waiting instance retries acquisition                        |
| `workDelay`     | 100 ms | Mandatory per-iteration delay so you can kill the process mid-run       |

The lease must be comfortably larger than the renewal period plus a few network round-trips
so a healthy owner never loses the lock by accident; smaller leases give faster failover
after a hard kill. Lower `leaseDuration` if you want failover to happen faster during the
demo.
