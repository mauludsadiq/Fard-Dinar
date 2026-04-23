#!/usr/bin/env bash
set -e

mkdir -p node_a/{state,receipts,events,objects}

cargo run --bin fardverify -- fd-node \
  --watch examples \
  --genesis examples/genesis.json \
  --objects examples/objects \
  --state-out node_a/state/state.json \
  --receipts node_a/receipts \
  --registry examples/dev-fixtures.json \
  --peer-watch node_b/events \
  --peer-registry examples/dev-fixtures.json
