#!/usr/bin/env bash
# Two-registry two-node mesh
# Events submitted to registry A propagate to registry B via poll.
# Node A and Node B both execute canonical winners independently.
# Invariant: node_a.state_hash == node_b.state_hash after convergence.
set -e
cd "$(dirname "$0")/../../.."

rm -rf network_mesh
mkdir -p network_mesh/reg_a/events \
         network_mesh/reg_b/events \
         network_mesh/node_a/events \
         network_mesh/node_a/receipts \
         network_mesh/node_b/events \
         network_mesh/node_b/receipts

echo "[mesh] registry A  :7371"
fardrun run --program fard/bin/fd_registry.fard --out network_mesh/reg_a/out -- \
  --watch        network_mesh/reg_a/events \
  --registry-out network_mesh/reg_a/registry.json \
  --bind         127.0.0.1:7371 &
REG_A=$!

sleep 1

echo "[mesh] registry B  :7373  (will peer-sync from A)"
fardrun run --program fard/bin/fd_registry.fard --out network_mesh/reg_b/out -- \
  --watch        network_mesh/reg_b/events \
  --registry-out network_mesh/reg_b/registry.json \
  --bind         127.0.0.1:7373 &
REG_B=$!

sleep 1

echo "[mesh] node A  :7370  (registry A)"
fardrun run --program fard/bin/fd_node.fard --out network_mesh/node_a/out -- \
  --watch     network_mesh/node_a/events \
  --genesis   examples/genesis_rewards.json \
  --objects   examples/objects \
  --state-out network_mesh/node_a/state.json \
  --receipts  network_mesh/node_a/receipts \
  --registry  network_mesh/reg_a/registry.json \
  --bind      127.0.0.1:7370 &
NODE_A=$!

sleep 1

echo "[mesh] node B  :7372  (registry B)"
fardrun run --program fard/bin/fd_node.fard --out network_mesh/node_b/out -- \
  --watch     network_mesh/node_b/events \
  --genesis   examples/genesis_rewards.json \
  --objects   examples/objects \
  --state-out network_mesh/node_b/state.json \
  --receipts  network_mesh/node_b/receipts \
  --registry  network_mesh/reg_b/registry.json \
  --bind      127.0.0.1:7372 &
NODE_B=$!

echo ""
echo "[mesh] registry A  http://127.0.0.1:7371"
echo "[mesh] registry B  http://127.0.0.1:7373"
echo "[mesh] node A      http://127.0.0.1:7370"
echo "[mesh] node B      http://127.0.0.1:7372"
echo ""
echo "[mesh] sync flow:"
echo "  1. submit events to registry A"
echo "  2. poll registry A  ->  canonical winners in reg_a/events"
echo "  3. copy reg_a/events to node_a/events, poll node A"
echo "  4. poll registry B with peer A  ->  merges reg_a winners"
echo "  5. copy reg_b/events to node_b/events, poll node B"
echo "  6. check convergence: fd_net_status --topology topology_mesh.json"
echo ""
echo "[mesh] press ctrl-c to stop"

trap "kill $REG_A $REG_B $NODE_A $NODE_B 2>/dev/null; echo stopped" EXIT
wait
