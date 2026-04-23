# FD-NETWORK v1.0 — Canonical Network Specification

## Overview

FD-NETWORK is the distributed orchestration layer for Fard Dinar. It defines how events propagate from wallets through registries to execution nodes, how conflicts are resolved, and what invariants the network must uphold at every point.

The network is deterministic. Given the same event set, any number of registries and nodes running independently will converge to the same canonical registry state and the same final ledger state hash.

---

## Components

### Registry

A registry accepts events, resolves conflicts, and serves a canonical event set.

**Responsibilities:**
- Accept raw events via `POST /v1/events`
- Write each event to the watch directory named by its `event_hash` hex
- On `POST /v1/poll`, scan the watch directory and resolve conflicts
- Merge peer registries during poll if peer URLs are configured
- Persist the canonical registry state to disk after each poll
- Serve the canonical registry via `GET /v1/registry`

**Endpoints:**

    POST /v1/events    — ingest a raw event
    POST /v1/poll      — process watch dir and resolve conflicts
    GET  /v1/registry  — canonical registry state
    GET  /v1/info      — liveness check

### Node

A node executes canonical events and persists state and receipts.

**Responsibilities:**
- Watch an event directory for new event files
- On `POST /v1/poll`, load all unprocessed events
- Filter events against the registry: only canonical winners are applied
- Sort filtered events using `canonical_event_set` (deposits first, then transfers, both deterministically ordered)
- Apply events in canonical order against the current state
- Write a receipt for each accepted event
- Persist state after each poll
- Serve current state via `GET /v1/state`
- Serve receipts via `GET /v1/receipts/<run_id_hex>`

**Endpoints:**

    POST /v1/poll            — apply pending canonical events
    GET  /v1/state           — current ledger state
    GET  /v1/receipts/<hex>  — receipt by run_id hex
    GET  /v1/info            — liveness check

---

## Conflict Resolution

Every event has a `conflict_key` that identifies its economic slot:

- transfer: `signing_payload_hash(from, nonce)` — one winner per sender nonce
- deposit:  `deposit_id(oracle_id, beneficiary, external_ref, timestamp)` — one winner per deposit

When two events compete for the same slot, the winner is the event with the lexicographically smallest `event_hash`. This is deterministic, requires no coordination, and produces the same result on every registry independently.

The losing event is marked `ignored` in the registry log. It is never applied by any node.

---

## Canonical Event Ordering

Before applying events, a node sorts its pending canonical winners using `canonical_event_set`:

1. Deposits first, sorted by `(beneficiary, external_ref, event_hash)`
2. Transfers second, sorted by `(from, nonce, event_hash)`

This ordering is deterministic and identical on every node. Two nodes receiving the same canonical event set in any order will apply them in the same sequence and produce the same final state hash.

---

## Peer Sync Protocol

There are two supported modes for propagating events across registries. Both produce the same canonical result.

### Mode A — Duplicate submission

Each registry receives the same raw events independently via `POST /v1/events`. Each registry runs conflict resolution on its own event set. Because conflict resolution is deterministic (min event_hash wins), both registries converge to the same canonical winners without communicating.

This is the mode exercised by `mesh_convergence_test.sh`. It is the simplest deployment model and requires no registry-to-registry connectivity.

### Mode B — Registry merge (peer sync)

A registry fetches the canonical registry state of a peer during `POST /v1/poll` and merges it locally.

**Merge semantics:**
- For each conflict slot in the peer registry, compare the peer's `event_hash` against the local winner
- If the peer's hash is lexicographically smaller, adopt the peer's entry
- Otherwise keep the local entry
- The merge is commutative and idempotent: order of merge does not affect the result

After merging, the operator copies the peer's canonical event files into the local watch directory and polls again to apply any newly adopted winners.

**Choosing a mode:**
- Use Mode A when registries are independent ingestion points receiving the same event stream
- Use Mode B when registries are federated and need to share event sets they did not originally receive

Both modes satisfy the convergence invariant: same effective event set → same canonical registry → same final state hash.

**Polling:**
- Poll is operator-triggered via `POST /v1/poll`
- There is no background polling interval — polling is explicit
- Automated polling can be layered on top using any scheduler

---

## Network Invariants

### Safety

At any point in time, every node applies only events that are canonical winners in its registry. A node never applies a losing event. A node never applies the same event twice.

### Liveness

After a finite number of poll operations, every node that has received the same canonical event set will have applied all events and reached a stable state.

### Eventual Convergence

If two registries receive the same set of raw events (in any order, through any path), they will converge to the same canonical registry after a finite number of poll-and-merge cycles.

If two nodes are each connected to a converged registry and poll to completion, they will reach the same final state hash.

This invariant is executable and verified by `mesh_convergence_test.sh`.

### Replay Equivalence

The final state produced by a live node is identical to the final state produced by `fd_replay` given the same genesis and canonical event set. There is no distinction between online and offline execution — both produce the same hash.

### Receipt Integrity

Every accepted event produces a receipt. The receipt's `input_hash` is `AHD("FD_EVENT_V1" + canonical_json(event))`. This is the join key between a receipt and its source event and is identical on every node that applies the same event.

---

## Topology Formats

Topology files are JSON documents describing a deployment:

    {
      "name":        "<topology name>",
      "nodes":       ["<node_url>", ...],
      "registries":  ["<registry_url>", ...]
    }

Included topologies:

| File | Description |
|---|---|
| `topology_single.json` | 1 registry (:7371) + 1 node (:7370) |
| `topology_two_registry.json` | 2 registries (:7371, :7373) + 1 node (:7372) |
| `topology_mesh.json` | 2 registries (:7371, :7373) + 2 nodes (:7370, :7372) |

Topology files are consumed by `fd_net_status` to check convergence across the mesh.

---

## Failure Modes

### Registry unreachable

`fd_health` and `fd_net_status` report `ok: false` with `err: "unreachable"` for any component that does not respond with HTTP 200. No automatic failover occurs. The operator is responsible for restarting components.

### Node falls behind

If a node has not been polled, its state will be stale relative to the registry. `fd_net_status` will report `nodes_converged: false` if two nodes have different state hashes. The fix is to poll the lagging node.

### Conflicting events arrive at different registries

If registry A receives event X and registry B receives event Y for the same conflict slot, each registry initially holds a different winner. After a merge cycle, both registries adopt the lexicographically smaller event hash and discard the other. The merge is deterministic regardless of which registry initiates it.

### Duplicate event submission

If the same event is submitted multiple times to the same registry, the watch directory will contain one file (named by event hash). Duplicate submissions are idempotent.

### Non-canonical event in node watch dir

If an event file is present in the node's watch directory but is not the canonical winner in the registry, the node skips it and logs `skipped (non-canonical)`. The event is never applied.

---

## Health Model

`fd_health` reports per-component health. `fd_net_status` reports mesh-level convergence.

**Per-node fields:**

    state_hash        — AHD-1024-256 of canonical(state)
    total_supply      — sum of all account balances
    account_count     — number of materialized accounts
    deposit_count     — number of consumed deposit IDs
    treasury_balance  — current treasury account balance
    reward_config     — active reward rates

**Per-registry fields:**

    registry_hash     — AHD-1024-256 of canonical(registry)
    entry_count       — number of canonical event slots

**Mesh-level fields:**

    fully_converged        — nodes_converged && registries_converged
    nodes_converged        — all reachable nodes share the same state_hash
    registries_converged   — all reachable registries share the same registry_hash
    reachable_nodes        — count of nodes that responded
    reachable_registries   — count of registries that responded

---

## Convergence Proof

`examples/network/fard/mesh_convergence_test.sh` is the executable proof of the convergence invariant.

It runs a two-registry two-node mesh, submits 5 events (including one conflicting pair) to each registry independently, polls all components to completion, and verifies:

- `nodes_converged: true`
- `registries_converged: true`
- `fully_converged: true`
- Both nodes report `state_hash: ahd1024:b350cffb...`
- Both registries report the same `registry_hash`
- The final state hash matches the offline `fd_replay` output

The network is correct if and only if this test passes.

---

## Operational Checklist

**Starting a single-node network:**

    bash examples/network/fard/single_node.sh

**Starting a two-registry mesh:**

    bash examples/network/fard/mesh.sh

**Checking health:**

    fardrun run --program fard/bin/fd_health.fard --out ./out -- \\
      --node http://127.0.0.1:7370 --registry http://127.0.0.1:7371

**Checking convergence:**

    fardrun run --program fard/bin/fd_net_status.fard --out ./out -- \\
      --topology examples/network/fard/topology_mesh.json

**Running the convergence proof:**

    bash examples/network/fard/mesh_convergence_test.sh

**Submitting an event:**

    curl -X POST http://127.0.0.1:7371/v1/events \\
      -H 'content-type: application/json' -d @event.json

**Polling:**

    curl -X POST http://127.0.0.1:7371/v1/poll
    curl -X POST http://127.0.0.1:7370/v1/poll

---

## Reference

| Component | Default port | Program |
|---|---|---|
| Registry | 7371 | `fard/bin/fd_registry.fard` |
| Node | 7370 | `fard/bin/fd_node.fard` |
| Health check | — | `fard/bin/fd_health.fard` |
| Net status | — | `fard/bin/fd_net_status.fard` |

| Topology | File |
|---|---|
| Single node | `examples/network/fard/topology_single.json` |
| Two registry | `examples/network/fard/topology_two_registry.json` |
| Mesh | `examples/network/fard/topology_mesh.json` |

---

## Stable Output Schemas

These output shapes are frozen as of FD-NETWORK v1.0.0. Consumers may depend on these fields being present.

### fd_health output

    {
      "ok":         bool,
      "converged":  bool,
      "state_hash": "ahd1024:..." | null,
      "nodes": [
        {
          "ok":               bool,
          "url":              string,
          "role":             "node",
          "state_hash":       "ahd1024:...",
          "total_supply":     int,
          "account_count":    int,
          "deposit_count":    int,
          "treasury_account": string,
          "treasury_balance": int,
          "reward_config":    { user_p2p_bps, user_spend_bps, vendor_spend_bps, treasury_account }
        }
      ],
      "registries": [
        {
          "ok":            bool,
          "url":           string,
          "role":          "registry",
          "registry_hash": "ahd1024:...",
          "entry_count":   int
        }
      ]
    }

On error, a component entry contains: `{ "ok": false, "url": string, "role": string, "err": string }`

### fd_net_status output

    {
      "ok":                    bool,
      "fully_converged":       bool,
      "nodes_converged":       bool,
      "registries_converged":  bool,
      "node_count":            int,
      "registry_count":        int,
      "reachable_nodes":       int,
      "reachable_registries":  int,
      "state_hash":            "ahd1024:..." | null,
      "registry_hash":         "ahd1024:..." | null,
      "nodes":                 [ <fd_health node shape> ],
      "registries":            [ <fd_health registry shape> ]
    }

### Convergence theorem

    same effective event set
      → same canonical registry (registry_hash identical across all registries)
      → same canonical event ordering
      → same execution sequence
      → same final state hash (state_hash identical across all nodes)

This is the network invariant. `mesh_convergence_test.sh` is its executable proof.
