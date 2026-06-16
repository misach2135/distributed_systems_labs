// Package cplock implements a fault-tolerant, mutually-exclusive distributed
// lock on top of the Hazelcast CP Subsystem.
//
// Why this exists
// ---------------
// The lab asks for a Hazelcast FencedLock (client.CPSubsystem().GetLock(...)).
// The Hazelcast Java member fully supports FencedLock, but the official
// Hazelcast Go client (v1.6.0) does NOT expose a FencedLock proxy: its CP
// Subsystem only provides AtomicLong, AtomicRef and CPMap. See the README for
// details.
//
// To get the exact same guarantees the lab needs — single owner at a time, and
// automatic release when the owner crashes — we build a *lease-based lock* on
// CP AtomicRef. Because AtomicRef lives in the CP Subsystem, its
// CompareAndSet is linearizable (Raft consensus), which is precisely the
// primitive required to implement a correct mutual-exclusion lock.
//
// How it works
// ------------
//   - The lock state is a single string stored in a CP AtomicRef:
//     "<ownerID>|<expiryUnixMillis>", or nil when free.
//   - Acquire: an instance takes the lock with CompareAndSet only if the slot
//     is free OR the current lease has expired (the previous owner died).
//   - Lease renewal: while an instance holds the lock it runs a background
//     goroutine that keeps extending the expiry via CompareAndSet. This is the
//     analogue of a FencedLock CP session heartbeat.
//   - Crash handling: if the owner is killed (Ctrl+C / kill -9), renewal stops.
//     Once the lease expires, any other waiting instance's CompareAndSet
//     succeeds and it takes over — automatically, with no manual cleanup.
package cplock

import (
	"context"
	"errors"
	"fmt"
	"strconv"
	"strings"
	"sync"
	"time"

	"github.com/hazelcast/hazelcast-go-client"
)

// ErrLockLost is returned by Renew (and reported via the Lost channel) when the
// lock was stolen by another instance, i.e. this instance can no longer assume
// it holds exclusive access.
var ErrLockLost = errors.New("cplock: lock lease lost")

// LeaseLock is a distributed lease lock backed by a CP AtomicRef.
type LeaseLock struct {
	ref     *hazelcast.AtomicRef
	owner   string        // unique identity of this instance
	lease   time.Duration // how long a single lease is valid
	renewEv time.Duration // how often we renew while holding

	mu      sync.Mutex
	current string        // the exact value we last wrote (needed for CAS)
	lost    chan struct{} // closed when the lease is lost
	stop    chan struct{} // closed to stop the renewer on Unlock
	wg      sync.WaitGroup
}

// New creates a lease lock. owner must uniquely identify this instance.
// lease is the validity window of one lease; it must be comfortably larger than
// the worst-case pause between renewals so that a healthy owner never loses the
// lock by accident. renew is the renewal period (e.g. lease/3).
func New(ref *hazelcast.AtomicRef, owner string, lease, renew time.Duration) *LeaseLock {
	return &LeaseLock{
		ref:     ref,
		owner:   owner,
		lease:   lease,
		renewEv: renew,
	}
}

func encode(owner string, expiry int64) string {
	return owner + "|" + strconv.FormatInt(expiry, 10)
}

func decode(v string) (owner string, expiry int64, ok bool) {
	i := strings.LastIndexByte(v, '|')
	if i < 0 {
		return "", 0, false
	}
	exp, err := strconv.ParseInt(v[i+1:], 10, 64)
	if err != nil {
		return "", 0, false
	}
	return v[:i], exp, true
}

// Lock blocks until the lock is acquired or ctx is cancelled. On success it
// starts the background lease renewer; the caller must eventually call Unlock.
func (l *LeaseLock) Lock(ctx context.Context, retry time.Duration) error {
	for {
		acquired, err := l.tryAcquire(ctx)
		if err != nil {
			return err
		}
		if acquired {
			l.startRenewer()
			return nil
		}
		// Wait before retrying, but stay responsive to cancellation.
		select {
		case <-ctx.Done():
			return ctx.Err()
		case <-time.After(retry):
		}
	}
}

// tryAcquire attempts a single CompareAndSet acquisition.
func (l *LeaseLock) tryAcquire(ctx context.Context) (bool, error) {
	raw, err := l.ref.Get(ctx)
	if err != nil {
		return false, fmt.Errorf("read lock state: %w", err)
	}

	now := time.Now().UnixMilli()
	newExpiry := now + l.lease.Milliseconds()
	newVal := encode(l.owner, newExpiry)

	if raw == nil {
		// Lock is free: take it (expected value is nil).
		ok, err := l.ref.CompareAndSet(ctx, nil, newVal)
		if err != nil {
			return false, fmt.Errorf("cas (free): %w", err)
		}
		if ok {
			l.setCurrent(newVal)
		}
		return ok, nil
	}

	cur, _ := raw.(string)
	_, expiry, valid := decode(cur)
	if valid && now <= expiry {
		// A live lease is held by someone (possibly us already, but in this
		// design only one instance ever calls Lock at a time per process).
		return false, nil
	}

	// Lease expired (previous owner died) -> steal it via CAS on the exact
	// stale value so only one waiter wins.
	ok, err := l.ref.CompareAndSet(ctx, cur, newVal)
	if err != nil {
		return false, fmt.Errorf("cas (steal): %w", err)
	}
	if ok {
		l.setCurrent(newVal)
	}
	return ok, nil
}

// startRenewer launches the background goroutine that keeps the lease alive.
func (l *LeaseLock) startRenewer() {
	l.mu.Lock()
	l.lost = make(chan struct{})
	l.stop = make(chan struct{})
	l.mu.Unlock()

	l.wg.Add(1)
	go func() {
		defer l.wg.Done()
		ticker := time.NewTicker(l.renewEv)
		defer ticker.Stop()
		for {
			select {
			case <-l.stop:
				return
			case <-ticker.C:
				ctx, cancel := context.WithTimeout(context.Background(), l.lease)
				err := l.renew(ctx)
				cancel()
				if err != nil {
					// We lost the lease: stop renewing and notify the worker.
					l.mu.Lock()
					if l.lost != nil {
						close(l.lost)
						l.lost = nil
					}
					l.mu.Unlock()
					return
				}
			}
		}
	}()
}

// renew extends the lease by another window using CAS on our current value.
func (l *LeaseLock) renew(ctx context.Context) error {
	l.mu.Lock()
	expected := l.current
	l.mu.Unlock()

	newExpiry := time.Now().UnixMilli() + l.lease.Milliseconds()
	newVal := encode(l.owner, newExpiry)

	ok, err := l.ref.CompareAndSet(ctx, expected, newVal)
	if err != nil {
		// Treat a transient error as a renewal failure; the caller decides.
		return fmt.Errorf("renew cas: %w", err)
	}
	if !ok {
		return ErrLockLost
	}
	l.setCurrent(newVal)
	return nil
}

// Lost returns a channel that is closed if the lease renewer detects the lock
// was lost. The worker loop should select on this so it stops mutating shared
// state the moment it no longer holds exclusive access.
func (l *LeaseLock) Lost() <-chan struct{} {
	l.mu.Lock()
	defer l.mu.Unlock()
	if l.lost == nil {
		// Already lost (or not holding); return a closed channel.
		ch := make(chan struct{})
		close(ch)
		return ch
	}
	return l.lost
}

// Unlock stops the renewer and releases the lock (only if we still own it).
func (l *LeaseLock) Unlock(ctx context.Context) error {
	l.mu.Lock()
	if l.stop != nil {
		close(l.stop)
		l.stop = nil
	}
	expected := l.current
	l.current = ""
	l.mu.Unlock()

	l.wg.Wait()

	if expected == "" {
		return nil
	}
	// Release by CAS-ing our value back to nil. If we no longer own it, the CAS
	// simply fails and there is nothing to release.
	if _, err := l.ref.CompareAndSet(ctx, expected, nil); err != nil {
		return fmt.Errorf("release cas: %w", err)
	}
	return nil
}

func (l *LeaseLock) setCurrent(v string) {
	l.mu.Lock()
	l.current = v
	l.mu.Unlock()
}
