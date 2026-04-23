# FD-NETWORK v0.1 — Deployment Topologies

## 1. Single-Node Topology

Components:
- 1 registry
- 1 node
- 1 HTTP surface

Use when:
- local development
- deterministic replay testing
- wallet/vendor integration tests

Flow:

    wallet -> registry -> node -> state/receipts -> http

---

## 2. Two-Registry / One-Node Topology

Components:
- Registry A
- Registry B
- Node B
- HTTP B

Use when:
- testing registry convergence
- validating peer-registry merge behavior
- validating remote event ingestion

Flow:

    wallet -> registry A -> registry B -> node B -> receipts/state -> http B

Properties:
- canonical winner selected by registry merge
- eventual consistency through peer-registry sync
- node executes only canonical winners

---

## 3. Two-Registry / Two-Node Topology

Components:
- Registry A
- Registry B
- Node A
- Node B
- HTTP A
- HTTP B

Use when:
- validating cross-node convergence
- validating transport-agnostic sync
- simulating distributed deployment

Flow:

    wallet -> registry A
           -> registry B
           -> node A / node B
           -> receipts + state on both sides

Expected invariant:
- same canonical registry winners
- same final deterministic state given same effective event set

---

## Health / Monitoring Surface

Use these endpoints for checks:

- GET /v1/info
- GET /v1/registry
- GET /v1/state
- GET /v1/receipts/<run_id>
- GET /v1/objects/<hash>

Recommended checks:
1. HTTP surface responds on /v1/info
2. Registry file exists and is readable over /v1/registry
3. State file exists and is readable over /v1/state
4. Receipt store is reachable via /v1/receipts/<run_id>
5. Object store is reachable via /v1/objects/<hash>

---

## Example Launchers

See:

- examples/network/registry_a.sh
- examples/network/registry_b.sh
- examples/network/node_a.sh
- examples/network/node_b.sh
- examples/network/http_a.sh
- examples/network/http_b.sh

