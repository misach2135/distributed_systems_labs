"""Task 2, Part I — Hazelcast Distributed Map functionality test.

Start the cluster first (see infra/task2.docker-compose.override.yaml), then:

    uv run utils/hazelcast.py fill    # write keys 0..9_999 into the map
    uv run utils/hazelcast.py check   # count entries, report missing keys

Key distribution across members is observed in Management Center
(http://localhost:8090) -> Storage -> Maps -> distributed-map.
"""

import argparse
import logging
import os
import sys

# This file is named hazelcast.py: drop its own directory from sys.path so
# `import hazelcast` resolves to the client library, not to this script.
sys.path = [p for p in sys.path if os.path.abspath(p) != os.path.dirname(os.path.abspath(__file__))]

import hazelcast

MAP_NAME = "distributed-map"
KEYS = range(10_000)

# The client logs a traceback on WARNING for every member it can't reach
# (e.g. one that was just killed); that is expected during the experiments.
logging.getLogger("hazelcast").setLevel(logging.ERROR)


def connect() -> hazelcast.HazelcastClient:
    # Docker Desktop doesn't route container IPs to the host, so the client
    # talks to the published ports only (smart routing off).
    return hazelcast.HazelcastClient(
        cluster_name="lab",
        cluster_members=["localhost:5701", "localhost:5702", "localhost:5703"],
        smart_routing=False,
    )


def fill(dmap) -> None:
    dmap.put_all({key: f"value-{key}" for key in KEYS})
    print(f"wrote {len(KEYS)} entries, map size = {dmap.size()}")


def check(dmap) -> None:
    missing = sorted(set(KEYS) - set(dmap.key_set()))
    print(f"map size = {dmap.size()}, expected = {len(KEYS)}, missing = {len(missing)}")
    if missing:
        print(f"first missing keys: {missing[:20]}")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=["fill", "check"])
    args = parser.parse_args()

    client = connect()
    try:
        dmap = client.get_map(MAP_NAME).blocking()
        fill(dmap) if args.command == "fill" else check(dmap)
    finally:
        client.shutdown()


if __name__ == "__main__":
    main()
