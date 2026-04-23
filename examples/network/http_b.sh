#!/usr/bin/env bash
set -e

cargo run --bin fardverify -- fd-http \
  --bind 127.0.0.1:8082 \
  --registry registry_b/registry.json \
  --state node_b/state.json \
  --receipts-dir node_b/receipts \
  --objects-dir examples/objects \
  --ingest-dir registry_b/events
