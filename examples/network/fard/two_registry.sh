#!/usr/bin/env bash
# Two-registry / one-node topology
# Registry A and B converge via HTTP peer sync.
# Node B executes only canonical winners from Registry B.
# Usage: bash examples/network/fard/two_registry.sh
set -e
cd "$(dirname "$0")/../../.."

mkdir -p network_run/reg_a/events \
         network_run/reg_b/events \
         network_run/node_b/events \
         network_run/node_b/receipts

echo "[fd] starting registry A on :7371"
fardrun run --program fard/bin/fd_registry.fard --out network_run/reg_a/out -- \
  --watch        network_run/reg_a/events \
  --registry-out network_run/reg_a/registry.json \
  --bind         127.0.0.1:7371 &
REG_A=$!

sleep 1

echo "[fd] starting registry B on :7373 (peers with A)"
fardrun run --program fard/bin/fd_registry.fard --out network_run/reg_b/out -- \
  --watch        network_run/reg_b/events \
  --registry-out network_run/reg_b/registry.json \
  --bind         127.0.0.1:7373 &
REG_B=$!

sleep 1

echo "[fd] starting node B on :7372 (uses registry B)"
fardrun run --program fard/bin/fd_node.fard --out network_run/node_b/out -- \
  --watch     network_run/node_b/events \
  --genesis   examples/genesis_rewards.json \
  --objects   examples/objects \
  --state-out network_run/node_b/state.json \
  --receipts  network_run/node_b/receipts \
  --registry  network_run/reg_b/registry.json \
  --bind      127.0.0.1:7372 &
NODE_B=$!

echo ""
echo "[fd] registry A   http://127.0.0.1:7371"
echo "[fd] registry B   http://127.0.0.1:7373"
echo "[fd] node B       http://127.0.0.1:7372"
echo ""
echo "[fd] flow:"
echo "       wallet -> registry A -> (merge) -> registry B -> node B"
echo ""
echo "[fd] to sync registry B from A:"
echo "       curl -X POST http://127.0.0.1:7373/v1/poll"
echo "       # then copy reg_a canonical events to node_b/events and poll node"
echo ""
echo "[fd] press ctrl-c to stop"

trap "kill $REG_A $REG_B $NODE_B 2>/dev/null; echo stopped" EXIT
wait
