# FD-NETWORK v0.1 — Example Multi-Node Topology

This directory contains example topology artifacts for running Fard Dinar
across multiple registries, nodes, and HTTP surfaces.

## Topology

- Registry A
- Registry B
- Node A
- Node B
- HTTP A
- HTTP B

Data flow:

    Wallet -> Registry A -> Registry B -> Node B -> Receipts/State
           \-> Node A (optional local execution path)

## Conventions

Each participant should have its own directories:

- registry_a_events/
- registry_b_events/
- node_a_events/
- node_b_events/
- node_a_receipts/
- node_b_receipts/
- node_a_state.json
- node_b_state.json

HTTP surfaces should expose:

- /v1/info
- /v1/registry
- /v1/state
- /v1/objects/<hash>
- /v1/receipts/<run_id>
- /v1/events

## Current Included Artifact

- node_a.sh

More launch scripts will be added for:
- registry_a.sh
- registry_b.sh
- node_b.sh
- http_a.sh
- http_b.sh

## Goal

Prove deterministic convergence under:
- filesystem peer sync
- HTTP registry sync
- canonical registry merge
- deterministic node execution gating

