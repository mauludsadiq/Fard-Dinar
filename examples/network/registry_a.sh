#!/usr/bin/env bash
set -e

mkdir -p registry_a/events

cargo run --bin fardverify -- fd-registry \
  --watch registry_a/events \
  --registry-out registry_a/registry.json
