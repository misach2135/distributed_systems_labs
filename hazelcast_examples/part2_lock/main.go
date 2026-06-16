// Part 2: Distributed Lock + Atomic Counter (CP Subsystem).
//
// Goal: a highly-available, fault-tolerant counter that reaches 100,000.
// Run THREE copies of this program at the same time. Only one of them works at
// any moment (the one holding the lock). If you kill the active one (Ctrl+C or
// `kill -9`), another instance automatically takes over and resumes counting
// from the exact iteration where the previous one died.
//
// Structures used (all in the CP Subsystem, so they are strongly consistent and
// survive node failures via Raft):
//   - a lease lock backed by AtomicRef "lab-lock"  (mutual exclusion + failover)
//   - AtomicLong "iteration-counter"               (durable iteration number)
//   - Map "counter-map", key "total_likes"         (the actual counter value)
//
// NOTE on the lock: the official Hazelcast Go client v1.6.0 does not provide
// FencedLock. We therefore implement an equivalent lease lock on CP AtomicRef
// (see internal/cplock). It provides the same two properties the lab requires:
// only one owner at a time, and automatic release on crash (lease expiry ==
// FencedLock CP session expiry).
//
// Build & run (open 3 terminals):
//
//	cd hazelcast_examples
//	go run ./part2_lock     # terminal 1
//	go run ./part2_lock     # terminal 2
//	go run ./part2_lock     # terminal 3
package main

import (
	"context"
	"errors"
	"fmt"
	"log"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/google/uuid"
	"github.com/hazelcast/hazelcast-go-client"

	"hazelcast_examples/internal/cplock"
	"hazelcast_examples/internal/hzconfig"
)

const (
	lockRefName    = "lab-lock"
	iterCounterCP  = "iteration-counter"
	counterMapName = "counter-map"
	counterKey     = "total_likes"

	target = 100_000

	// Lock tuning. The lease must be safely larger than (renew period + a few
	// network round-trips) so a healthy owner never loses the lock by accident,
	// yet small enough that failover after a `kill -9` happens reasonably fast.
	leaseDuration = 10 * time.Second
	renewEvery    = 3 * time.Second
	retryEvery    = 1 * time.Second

	// Mandatory per the lab: gives a human time to kill the active process.
	workDelay = 100 * time.Millisecond
)

func main() {
	// A unique, human-readable identity for this instance.
	instanceID := fmt.Sprintf("PID-%d-%s", os.Getpid(), uuid.NewString()[:8])
	logf := func(format string, a ...any) {
		log.Printf("[%s] "+format, append([]any{instanceID}, a...)...)
	}
	log.SetFlags(log.Ltime)

	// Root context cancelled on Ctrl+C / SIGTERM for graceful shutdown.
	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()

	// Connect to the cluster.
	cfg := hzconfig.New(instanceID)
	client, err := hazelcast.StartNewClientWithConfig(ctx, cfg)
	if err != nil {
		log.Fatalf("failed to connect to Hazelcast cluster: %v", err)
	}
	defer func() {
		sctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
		defer cancel()
		_ = client.Shutdown(sctx)
	}()

	// Resolve the CP structures.
	lockRef, err := client.CPSubsystem().GetAtomicRef(ctx, lockRefName)
	if err != nil {
		log.Fatalf("get AtomicRef %q: %v", lockRefName, err)
	}
	iterCounter, err := client.CPSubsystem().GetAtomicLong(ctx, iterCounterCP)
	if err != nil {
		log.Fatalf("get AtomicLong %q: %v", iterCounterCP, err)
	}
	counterMap, err := client.GetMap(ctx, counterMapName)
	if err != nil {
		log.Fatalf("get Map %q: %v", counterMapName, err)
	}

	lock := cplock.New(lockRef, instanceID, leaseDuration, renewEvery)

	logf("Started. Target=%d. Waiting for the lock...", target)

	if err := run(ctx, logf, lock, iterCounter, counterMap); err != nil {
		if errors.Is(err, context.Canceled) {
			logf("Interrupted. Releasing lock and exiting.")
			return
		}
		log.Fatalf("[%s] fatal: %v", instanceID, err)
	}
}

// run drives the acquire -> work -> release cycle until the target is reached.
func run(
	ctx context.Context,
	logf func(string, ...any),
	lock *cplock.LeaseLock,
	iterCounter *hazelcast.AtomicLong,
	counterMap *hazelcast.Map,
) error {
	for {
		// Already finished by some instance? Then stop.
		done, err := isDone(ctx, iterCounter)
		if err != nil {
			return err
		}
		if done {
			val, _ := readCounter(ctx, counterMap)
			logf("Target %d already reached (counter=%d). Nothing to do.", target, val)
			return nil
		}

		// Block until we own the lock (or ctx is cancelled).
		if err := lock.Lock(ctx, retryEvery); err != nil {
			return err
		}

		finished, err := work(ctx, logf, lock, iterCounter, counterMap)

		// Always release the lock so another instance can proceed immediately
		// on graceful exit (crash failover is handled by lease expiry).
		uctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
		_ = lock.Unlock(uctx)
		cancel()

		if err != nil {
			if errors.Is(err, context.Canceled) {
				return err
			}
			if errors.Is(err, cplock.ErrLockLost) {
				logf("Lost the lock (lease expired). Will try to re-acquire...")
				continue
			}
			return err
		}
		if finished {
			return nil
		}
		// Lock released for another reason; loop and try again.
	}
}

// work runs the inner counting loop while the lock is held. It returns
// finished=true when the target is reached.
func work(
	ctx context.Context,
	logf func(string, ...any),
	lock *cplock.LeaseLock,
	iterCounter *hazelcast.AtomicLong,
	counterMap *hazelcast.Map,
) (bool, error) {
	// Reconcile the Map counter to the durable iteration value so the two can
	// never drift (e.g. if a previous owner crashed between the two writes).
	start, err := iterCounter.Get(ctx)
	if err != nil {
		return false, err
	}
	if err := counterMap.Set(ctx, counterKey, start); err != nil {
		return false, err
	}
	logf("Acquired lock. Starting/Resuming work from iteration %d...", start)

	lost := lock.Lost()
	for {
		// Stop immediately if interrupted or if we no longer hold the lock.
		select {
		case <-ctx.Done():
			return false, ctx.Err()
		case <-lost:
			return false, cplock.ErrLockLost
		default:
		}

		// Have we reached the target?
		cur, err := iterCounter.Get(ctx)
		if err != nil {
			return false, err
		}
		if cur >= target {
			val, _ := readCounter(ctx, counterMap)
			logf("DONE. Target reached: iteration=%d, counter=%d.", cur, val)
			return true, nil
		}

		// 1) Advance the durable iteration counter (AtomicLong).
		iter, err := iterCounter.IncrementAndGet(ctx)
		if err != nil {
			return false, err
		}

		// 2) Read the counter from the Map, increment it, store it back.
		val, err := readCounter(ctx, counterMap)
		if err != nil {
			return false, err
		}
		val++
		if err := counterMap.Set(ctx, counterKey, val); err != nil {
			return false, err
		}

		logf("Iteration: %d, Counter: %d", iter, val)

		if iter >= target {
			logf("DONE. Target reached: iteration=%d, counter=%d.", iter, val)
			return true, nil
		}

		// Mandatory delay so the failover can be tested by killing the process.
		select {
		case <-ctx.Done():
			return false, ctx.Err()
		case <-time.After(workDelay):
		}
	}
}

func isDone(ctx context.Context, iterCounter *hazelcast.AtomicLong) (bool, error) {
	v, err := iterCounter.Get(ctx)
	if err != nil {
		return false, err
	}
	return v >= target, nil
}

// readCounter reads the Map counter as an int64, treating a missing key as 0.
func readCounter(ctx context.Context, m *hazelcast.Map) (int64, error) {
	raw, err := m.Get(ctx, counterKey)
	if err != nil {
		return 0, err
	}
	switch v := raw.(type) {
	case nil:
		return 0, nil
	case int64:
		return v, nil
	case int32:
		return int64(v), nil
	case int:
		return int64(v), nil
	default:
		return 0, fmt.Errorf("unexpected counter type %T (%v)", raw, raw)
	}
}
