// Part 1: Distributed Map.
//
// Demonstrates a Hazelcast Distributed (IMap) structure. It writes 10,000
// key-value pairs into the map "lab-map". The data is automatically partitioned
// and replicated across the 3 cluster nodes, so it can be inspected (and nodes
// killed to observe re-distribution) from the Hazelcast Management Center.
//
// Build & run:
//
//	cd hazelcast_examples
//	go run ./part1_map
//
// Override the cluster connection if needed:
//
//	HZ_CLUSTER_ADDRS="127.0.0.1:5701,127.0.0.1:5702,127.0.0.1:5703" \
//	HZ_CLUSTER_NAME="dev" go run ./part1_map
package main

import (
	"context"
	"fmt"
	"log"
	"strconv"
	"time"

	"github.com/hazelcast/hazelcast-go-client"

	"hazelcast_examples/internal/hzconfig"
)

const (
	mapName    = "lab-map"
	entryCount = 10_000 // keys 0..9999 -> 10_000 entries
)

func main() {
	ctx := context.Background()

	// Connect to the cluster.
	cfg := hzconfig.New("part1-map-writer")
	client, err := hazelcast.StartNewClientWithConfig(ctx, cfg)
	if err != nil {
		log.Fatalf("failed to connect to Hazelcast cluster: %v", err)
	}
	defer func() {
		// Best-effort graceful shutdown.
		shutdownCtx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
		defer cancel()
		_ = client.Shutdown(shutdownCtx)
	}()

	fmt.Printf("Connected to cluster %q. Populating distributed map %q...\n", cfg.Cluster.Name, mapName)

	// Access the distributed map.
	m, err := client.GetMap(ctx, mapName)
	if err != nil {
		log.Fatalf("failed to get map %q: %v", mapName, err)
	}

	// Start from a clean slate so the entry count is deterministic on reruns.
	if err := m.Clear(ctx); err != nil {
		log.Fatalf("failed to clear map: %v", err)
	}

	start := time.Now()
	for i := 0; i < entryCount; i++ {
		key := strconv.Itoa(i)
		value := fmt.Sprintf("value_%d", i)
		// Set is the cheapest write: it does not return the previous value.
		if err := m.Set(ctx, key, value); err != nil {
			log.Fatalf("failed to set key %q: %v", key, err)
		}
		if (i+1)%1000 == 0 {
			fmt.Printf("  inserted %d/%d entries...\n", i+1, entryCount)
		}
	}

	// Verify the final size as reported by the cluster.
	size, err := m.Size(ctx)
	if err != nil {
		log.Fatalf("failed to read map size: %v", err)
	}

	fmt.Printf("Done. Inserted %d entries into %q in %s (cluster reports size=%d).\n",
		entryCount, mapName, time.Since(start).Round(time.Millisecond), size)
	fmt.Println("Open Hazelcast Management Center to inspect partition distribution,")
	fmt.Println("then kill 1-2 nodes and watch the entries re-balance across survivors.")
}
