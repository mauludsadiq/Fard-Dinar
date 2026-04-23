#!/usr/bin/env bash
# Single-node topology — registry + node on localhost
# Usage: bash examples/network/fard/single_node.sh
set -e
cd "$(dirname "$0")/../../.."

mkdir -p network_run/events network_run/receipts

echo "[fd] starting registry on :7371"
fardrun run --program fard/bin/fd_registry.fard --out network_run/registry_out -- \
  --watch        network_run/events \
  --registry-out network_run/registry.json \
  --bind         127.0.0.1:7371 &
REG_PID=$!

sleep 1

echo "[fd] starting node on :7370"
fardrun run --program fard/bin/fd_node.fard --out network_run/node_out -- \
  --watch     network_run/events \
  --genesis   examples/genesis_rewards.json \
  --objects   examples/objects \
  --state-out network_run/state.json \
  --receipts  network_run/receipts \
  --registry  network_run/registry.json \
  --bind      127.0.0.1:7370 &
NODE_PID=$!

echo "[fd] registry  http://127.0.0.1:7371"
echo "[fd] node      http://127.0.0.1:7370"
echo "[fd] press ctrl-c to stop"

trap "kill $REG_PID $NODE_PID 2>/dev/null; echo stopped" EXIT
wait
