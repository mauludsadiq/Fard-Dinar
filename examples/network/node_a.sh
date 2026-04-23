#!/usr/bin/env bash
set -e

mkdir -p node_a/{events,receipts,state}

cargo run --bin fardverify -- fd-node \
  --watch node_a/events \
  --genesis examples/genesis_rewards.json \
  --objects examples/objects \
  --state-out node_a/state/state.json \
  --receipts node_a/receipts \
  --peer-watch registry_a/events \
  --peer-registry registry_a/registry.json
