// Package hzconfig builds a Hazelcast client configuration from environment
// variables so that both example programs (part1_map and part2_lock) connect to
// the cluster in exactly the same, overridable way.
//
// Supported environment variables:
//
//	HZ_CLUSTER_ADDRS  comma-separated member addresses (default "localhost:5701")
//	HZ_CLUSTER_NAME   cluster name as configured on the members (default "dev")
//
// Example:
//
//	HZ_CLUSTER_ADDRS="127.0.0.1:5701,127.0.0.1:5702,127.0.0.1:5703" go run ./part1_map
package hzconfig

import (
	"os"
	"strings"

	"github.com/hazelcast/hazelcast-go-client"
)

// New returns a hazelcast.Config populated from the environment. The clientName
// is used purely for identification in the Management Center / member logs.
func New(clientName string) hazelcast.Config {
	cfg := hazelcast.NewConfig()
	cfg.ClientName = clientName

	// Cluster name must match the name configured on the members.
	cfg.Cluster.Name = getenv("HZ_CLUSTER_NAME", "dev")

	// Member addresses. We list all three nodes so the smart client can keep
	// working (and route partition-aware ops) even when one of them is killed.
	addrs := splitAddrs(getenv("HZ_CLUSTER_ADDRS", "localhost:5701"))
	cfg.Cluster.Network.SetAddresses(addrs...)

	return cfg
}

func getenv(key, fallback string) string {
	if v := strings.TrimSpace(os.Getenv(key)); v != "" {
		return v
	}
	return fallback
}

func splitAddrs(raw string) []string {
	parts := strings.Split(raw, ",")
	out := make([]string, 0, len(parts))
	for _, p := range parts {
		if p = strings.TrimSpace(p); p != "" {
			out = append(out, p)
		}
	}
	return out
}
