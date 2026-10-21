"""logging-service: stores transactions in a Hazelcast Distributed Map.

    POST /log                      store a transaction (JSON body)
    GET  /transactions/<user_id>   all transactions of a user
    GET  /health
"""

import json
import logging
import os
import socket
import time
import urllib.request

import hazelcast
from flask import Flask, jsonify, request
from hazelcast.core import HazelcastJsonValue
from hazelcast.predicate import equal

NAME = socket.gethostname()

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(message)s")
logging.getLogger("hazelcast").setLevel(logging.WARNING)
log = logging.getLogger("logging-service")

# HZ_MEMBERS lists this instance's own member first: the client starts there
# and then works with the whole cluster, so losing that member is not fatal.
client = hazelcast.HazelcastClient(
    cluster_name="lab",
    cluster_members=os.environ["HZ_MEMBERS"].split(","),
)
transactions = client.get_map("transactions").blocking()


MAX_ATTEMPTS = 10


def register() -> None:
    """Register this instance in the config-server."""
    url = f"{os.environ['CONFIG_SERVER_URL']}/register"
    body = json.dumps({"service": "logger", "address": os.environ["SERVICE_URL"]}).encode()
    for attempt in range(1, MAX_ATTEMPTS + 1):
        try:
            urllib.request.urlopen(urllib.request.Request(url, body, {"Content-Type": "application/json"}), timeout=5)
            log.info("%s registered in config-server as %s", NAME, os.environ["SERVICE_URL"])
            return
        except OSError as e:
            if attempt == MAX_ATTEMPTS:
                raise
            log.warning("config-server unavailable (%s), retry %d", e, attempt)
            time.sleep(1)


register()

app = Flask(__name__)


@app.post("/log")
def log_transaction():
    tx = request.get_json()
    # Stored as JSON so the map can be queried by user_id. Same key on a
    # retry from the facade simply overwrites the entry.
    transactions.put(tx["transaction_id"], HazelcastJsonValue(json.dumps(tx)))
    log.info("%s stored tx=%s user=%s amount=%s", NAME, tx["transaction_id"], tx["user_id"], tx["amount"])
    return jsonify(status="ok")


@app.get("/transactions/<user_id>")
def transactions_by_user(user_id: str):
    txs = sorted((v.loads() for v in transactions.values(equal("user_id", user_id))), key=lambda tx: tx["timestamp"])
    log.info("%s returning %d transactions for user=%s", NAME, len(txs), user_id)
    return jsonify(txs)


@app.get("/health")
def health():
    return "ok"
