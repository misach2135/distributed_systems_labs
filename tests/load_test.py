#!/usr/bin/env python3
"""Load & correctness test for the banking microservices.

Drives the facade-service with many concurrent POST /transaction calls and
verifies the resulting balances, then reports throughput and the latency
contribution of logging-service vs counter-service (via /metrics).

Two scenarios (from the assignment):

  A. 10 clients, 10k tx of +1 each to *their own* account
     -> every one of the 10 accounts must end at 10_000.

  B. 10 clients, 10k tx of +1 each to *one shared* account
     -> that single account must end at 100_000.

The counter-service applies transactions asynchronously (Kafka), so after
sending, the test waits until the balances reach the expected values and
also reports how long that took.

Usage:
    uv venv && uv pip install aiohttp
    uv run tests/load_test.py                 # run both scenarios
    uv run tests/load_test.py --scenario a
    uv run tests/load_test.py --clients 10 --tx 1000   # quicker smoke run
"""

import argparse
import asyncio
import time

import aiohttp

DEFAULT_URL = "http://localhost:8080"


async def client_worker(session, url, user_id, n_tx):
    """One simulated client: n_tx sequential +1 transactions."""
    body_template = "{{{user}, +1}}"  # -> "{user1, +1}"
    body = body_template.format(user=user_id)
    endpoint = f"{url}/transaction"
    for _ in range(n_tx):
        async with session.post(endpoint, data=body) as resp:
            if resp.status != 200:
                text = await resp.text()
                raise RuntimeError(f"tx failed ({resp.status}): {text}")
            await resp.read()


async def reset_metrics(session, url):
    async with session.post(f"{url}/metrics/reset") as resp:
        await resp.read()


async def get_metrics(session, url):
    async with session.get(f"{url}/metrics") as resp:
        return await resp.json()


async def wait_for_balances(session, url, expected, timeout=300):
    """Poll /accounts until every balance in `expected` is reached (or timeout)."""
    deadline = time.perf_counter() + timeout
    while True:
        async with session.get(f"{url}/accounts") as resp:
            accounts = await resp.json()
        done = all(accounts.get(uid, 0) == value for uid, value in expected.items())
        if done or time.perf_counter() > deadline:
            return accounts
        await asyncio.sleep(0.5)


def print_report(name, total_requests, elapsed, consistent_after, metrics, checks):
    rps = total_requests / elapsed if elapsed > 0 else float("inf")
    print(f"\n=== Scenario {name} ===")
    print(f"  total requests : {total_requests}")
    print(f"  total time     : {elapsed:.3f} s")
    print(f"  throughput     : {rps:,.1f} req/s")
    print(f"  balances consistent after: {consistent_after:.3f} s (from the first request)")
    print("  --- facade latency breakdown (accumulated) ---")
    print(
        f"  logging-service: {metrics['logging_total_ms']:.1f} ms total "
        f"over {metrics['logging_calls']} calls "
        f"(avg {metrics['logging_avg_ms']:.3f} ms)"
    )
    print(
        f"  counter-service: {metrics['counter_total_ms']:.1f} ms total "
        f"over {metrics['counter_calls']} calls "
        f"(avg {metrics['counter_avg_ms']:.3f} ms)"
    )
    print("  --- correctness ---")
    all_ok = True
    for user_id, actual, expected in checks:
        ok = abs(actual - expected) < 1e-9
        all_ok = all_ok and ok
        mark = "OK " if ok else "FAIL"
        print(f"  [{mark}] {user_id}: balance={actual:g} expected={expected:g}")
    print(f"  result: {'PASSED' if all_ok else 'FAILED'}")
    return all_ok


async def run_scenario(name, url, clients, n_tx, shared):
    connector = aiohttp.TCPConnector(limit=clients + 4)
    async with aiohttp.ClientSession(connector=connector) as session:
        await reset_metrics(session, url)

        if shared:
            user_ids = ["shared"] * clients
        else:
            user_ids = [f"user{i}" for i in range(clients)]

        start = time.perf_counter()
        await asyncio.gather(
            *(client_worker(session, url, uid, n_tx) for uid in user_ids)
        )
        elapsed = time.perf_counter() - start

        metrics = await get_metrics(session, url)

        # Build correctness checks.
        if shared:
            expected = {"shared": clients * n_tx}
        else:
            expected = {uid: n_tx for uid in user_ids}
        accounts = await wait_for_balances(session, url, expected)
        consistent_after = time.perf_counter() - start
        checks = [(uid, accounts.get(uid, 0), value) for uid, value in expected.items()]

        total_requests = clients * n_tx
        return print_report(name, total_requests, elapsed, consistent_after, metrics, checks)


async def main():
    parser = argparse.ArgumentParser(description="Microservices load test")
    parser.add_argument("--url", default=DEFAULT_URL, help="facade base URL")
    parser.add_argument("--clients", type=int, default=10)
    parser.add_argument("--tx", type=int, default=10_000, help="transactions per client")
    parser.add_argument(
        "--scenario",
        choices=["a", "b", "both"],
        default="both",
        help="a = separate accounts, b = one shared account",
    )
    args = parser.parse_args()

    results = []
    if args.scenario in ("a", "both"):
        results.append(
            await run_scenario(
                "A (separate accounts)", args.url, args.clients, args.tx, shared=False
            )
        )
    if args.scenario in ("b", "both"):
        results.append(
            await run_scenario(
                "B (one shared account)", args.url, args.clients, args.tx, shared=True
            )
        )

    print()
    if all(results):
        print("ALL SCENARIOS PASSED")
    else:
        print("SOME SCENARIOS FAILED")
        raise SystemExit(1)


if __name__ == "__main__":
    asyncio.run(main())
