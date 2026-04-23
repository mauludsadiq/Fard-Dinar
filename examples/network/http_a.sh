#!/usr/bin/env bash
set -e

cargo run --bin fardverify -- fd-http \
  --bind 127.0.0.1:8081 \
  --registry registry_a/registry.json
