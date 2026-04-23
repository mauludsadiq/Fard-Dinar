#!/usr/bin/env bash
set -e

mkdir -p node_b/{events,receipts}

cargo run --bin fardverify -- fd-node \
  --watch node_b/events \
  --genesis examples/genesis_rewards.json \
  --objects examples/objects \
  --state-out node_b/state.json \
  --receipts node_b/receipts \
  --peer-watch registry_b/events \
  --peer-registry registry_b/registry.json
