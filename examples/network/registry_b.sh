#!/usr/bin/env bash
set -e

mkdir -p registry_b/events

cargo run --bin fardverify -- fd-registry \
  --watch registry_b/events \
  --registry-out registry_b/registry.json \
  --peer-registry registry_a/registry.json
