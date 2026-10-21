# Task 5 — Microservices in Kubernetes

Report: `report/`.

## Setup

```bash
uv venv
uv pip install -r requirements.txt

minikube start --driver=docker --container-runtime=docker --cpus=6 --memory=8g
# optional: copy infra images from Docker Desktop instead of pulling them
for i in postgres:17-alpine apache/kafka:4.3.2 hazelcast/hazelcast:5.4.0 hazelcast/management-center:5.4.1; do minikube image load $i; done
```

## Build

Images are built inside minikube, so Kubernetes uses them without a registry:

```bash
eval $(minikube docker-env)                                          # docker CLI -> minikube
docker build -t ds-facade -f services/rust/facade/Dockerfile .
docker build -t ds-counter -f services/rust/counter/Dockerfile .
docker build -t ds-logger services/py/logger
eval $(minikube docker-env -u)                                       # docker CLI -> Docker Desktop
```

## Run

Namespace `ds`: facade × 2, logger × 3 (Hazelcast Map), counter × 2 (PostgreSQL, reads from Kafka), Kafka, Hazelcast × 3, Management Center:

```bash
kubectl apply -f infra/k8s/namespace.yaml
kubectl apply -f infra/k8s/
kubectl -n ds get pods                                               # wait until all are Running 1/1
minikube tunnel                                                      # separate terminal: facade on localhost:8080
kubectl -n ds port-forward svc/management-center 8090:8080           # separate terminal: http://localhost:8090
```

After rebuilding an image: `kubectl -n ds rollout restart deploy/<facade|counter|logger>`.

## Discovery

```bash
kubectl -n ds get endpointslices                                     # ready pod IPs behind every Service
kubectl -n ds logs deploy/facade | grep 'endpoints updated'          # what the facade sees
```

## Requests

```bash
for i in $(seq 1 10); do curl -X POST localhost:8080/transaction --data "{user1, +$i}"; echo; done   # 10 transactions
curl localhost:8080/user/user1                                       # balance + transactions of a user
curl localhost:8080/accounts                                         # all balances
kubectl -n ds logs -l app=logger --prefix --tail=-1 | grep stored              # which logger got which transaction
kubectl -n ds logs -l app=counter --prefix --tail=-1 | grep applied            # transactions applied from Kafka
```

## Fault tolerance

```bash
P=$(kubectl -n ds get pods -l app=logger -o name | head -1 | cut -d/ -f2)     # one logger pod
eval $(minikube docker-env)
C=$(docker ps -q -f label=io.kubernetes.pod.name=$P -f label=io.kubernetes.container.name=logger)
docker pause $C                                                      # instance stops answering
kubectl -n ds get pods -l app=logger                                 # after ~9 s: 0/1 (readiness probe failed)
kubectl -n ds get endpointslice -l kubernetes.io/service-name=logger \
  -o jsonpath='{range .items[*].endpoints[*]}{.targetRef.name} ready={.conditions.ready}{"\n"}{end}'
for i in $(seq 1 10); do curl -X POST localhost:8080/transaction --data "{user2, +1}"; echo; done   # still served by the others
docker unpause $C                                                    # back to 1/1 (without it, liveness restarts the pod after ~30 s)
eval $(minikube docker-env -u)

kubectl -n ds delete pod $P                                          # Deployment creates a replacement
kubectl -n ds scale deploy/counter --replicas=1                      # fewer instances
kubectl -n ds scale deploy/counter --replicas=2
kubectl -n ds logs deploy/facade | grep 'endpoints updated'          # facade sees every change
```

## Load test

`/metrics` lives in each facade pod, so the test runs against a single replica:

```bash
kubectl -n ds scale deploy/facade --replicas=1
kubectl -n ds exec postgres-0 -- psql -U bank -d bank -c 'truncate accounts, processed_txs;'   # reset balances
uv run tests/load_test.py                                            # 10 clients × 10 000 tx, both scenarios
kubectl -n ds scale deploy/facade --replicas=2
```

Stop: `minikube stop` (`minikube delete` also wipes PostgreSQL and Kafka data).
