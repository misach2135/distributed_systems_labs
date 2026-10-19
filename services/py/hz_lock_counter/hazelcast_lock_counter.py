"""Task 2, Part II — counter service guarded by a Hazelcast FencedLock.

Several copies run at once; only the one holding the lock increments the
counter. The current iteration lives in a CP IAtomicLong, so when the lock
holder crashes, the next one continues from where it stopped.

    python hazelcast_lock_counter.py reset    # iteration = 0, counter = 0
    python hazelcast_lock_counter.py run      # take the lock, count to TARGET
    python hazelcast_lock_counter.py status   # print iteration and counter
"""

import argparse
import logging
import socket

import hazelcast

TARGET = 100_000

log = logging.getLogger("lock-counter")


def run(client: hazelcast.HazelcastClient, name: str) -> None:
    lock = client.cp_subsystem.get_lock("counter-lock").blocking()
    iteration = client.cp_subsystem.get_atomic_long("iteration").blocking()
    counter = client.get_map("counter").blocking()

    log.info("%s: waiting for the lock", name)
    fence = lock.lock()
    try:
        i = iteration.get()
        log.info("%s: got the lock (fence=%d), starting from iteration %d", name, fence, i)
        while i < TARGET:
            counter.put("value", i + 1)
            iteration.set(i + 1)
            i += 1
            if i % 1_000 == 0:
                log.info("%s: iteration %d", name, i)
        log.info("%s: done, counter = %d", name, counter.get("value"))
    finally:
        lock.unlock()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=["reset", "run", "status"])
    args = parser.parse_args()

    logging.basicConfig(level=logging.INFO, format="%(asctime)s %(message)s")
    logging.getLogger("hazelcast").setLevel(logging.WARNING)

    client = hazelcast.HazelcastClient(
        cluster_name="lab",
        cluster_members=["hazelcast-1", "hazelcast-2", "hazelcast-3"],
    )
    try:
        iteration = client.cp_subsystem.get_atomic_long("iteration").blocking()
        counter = client.get_map("counter").blocking()
        if args.command == "reset":
            iteration.set(0)
            counter.put("value", 0)
        elif args.command == "run":
            run(client, socket.gethostname())
        print(f"iteration = {iteration.get()}, counter = {counter.get('value')}")
    finally:
        client.shutdown()


if __name__ == "__main__":
    main()
