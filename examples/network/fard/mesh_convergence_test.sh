#!/usr/bin/env bash
# Mesh convergence test — proves node A and node B converge to the same state
# after events flow through two independent registries.
set -e
cd "$(dirname "$0")/../../.."

rm -rf network_mesh
mkdir -p network_mesh/reg_a/events \
         network_mesh/reg_b/events \
         network_mesh/node_a/events \
         network_mesh/node_a/receipts \
         network_mesh/node_b/events \
         network_mesh/node_b/receipts

echo "[mesh] starting registry A on :7371"
fardrun run --program fard/bin/fd_registry.fard --out network_mesh/reg_a/out -- \
  --watch network_mesh/reg_a/events \
  --registry-out network_mesh/reg_a/registry.json --bind 127.0.0.1:7371 &
REG_A=$!

sleep 1

echo "[mesh] starting registry B on :7373"
fardrun run --program fard/bin/fd_registry.fard --out network_mesh/reg_b/out -- \
  --watch network_mesh/reg_b/events \
  --registry-out network_mesh/reg_b/registry.json --bind 127.0.0.1:7373 &
REG_B=$!

sleep 1

echo "[mesh] starting node A on :7370"
fardrun run --program fard/bin/fd_node.fard --out network_mesh/node_a/out -- \
  --watch network_mesh/node_a/events \
  --genesis examples/genesis_rewards.json --objects examples/objects \
  --state-out network_mesh/node_a/state.json \
  --receipts network_mesh/node_a/receipts \
  --registry network_mesh/reg_a/registry.json --bind 127.0.0.1:7370 &
NODE_A=$!

sleep 1

echo "[mesh] starting node B on :7372"
fardrun run --program fard/bin/fd_node.fard --out network_mesh/node_b/out -- \
  --watch network_mesh/node_b/events \
  --genesis examples/genesis_rewards.json --objects examples/objects \
  --state-out network_mesh/node_b/state.json \
  --receipts network_mesh/node_b/receipts \
  --registry network_mesh/reg_b/registry.json --bind 127.0.0.1:7372 &
NODE_B=$!

sleep 2

echo ""
echo "[mesh] step 1 — submit all events to registry A"
curl -s -X POST http://127.0.0.1:7371/v1/events -H 'content-type: application/json' -d @examples/deposit_alice.json
curl -s -X POST http://127.0.0.1:7371/v1/events -H 'content-type: application/json' -d @examples/deposit_bob.json
curl -s -X POST http://127.0.0.1:7371/v1/events -H 'content-type: application/json' -d @examples/transfer_alice_candidate_a.json
curl -s -X POST http://127.0.0.1:7371/v1/events -H 'content-type: application/json' -d @examples/transfer_alice_candidate_b.json
curl -s -X POST http://127.0.0.1:7371/v1/events -H 'content-type: application/json' -d @examples/transfer_bob.json
echo " done"

echo "[mesh] step 2 — poll registry A"
curl -s -X POST http://127.0.0.1:7371/v1/poll
echo ""

echo "[mesh] step 3 — feed canonical events to node A and poll"
cp network_mesh/reg_a/events/*.json network_mesh/node_a/events/
curl -s -X POST http://127.0.0.1:7370/v1/poll
echo ""

echo "[mesh] step 4 — submit same events to registry B and poll (simulates peer sync)"
curl -s -X POST http://127.0.0.1:7373/v1/events -H 'content-type: application/json' -d @examples/deposit_alice.json
curl -s -X POST http://127.0.0.1:7373/v1/events -H 'content-type: application/json' -d @examples/deposit_bob.json
curl -s -X POST http://127.0.0.1:7373/v1/events -H 'content-type: application/json' -d @examples/transfer_alice_candidate_a.json
curl -s -X POST http://127.0.0.1:7373/v1/events -H 'content-type: application/json' -d @examples/transfer_alice_candidate_b.json
curl -s -X POST http://127.0.0.1:7373/v1/events -H 'content-type: application/json' -d @examples/transfer_bob.json
curl -s -X POST http://127.0.0.1:7373/v1/poll
echo ""

echo "[mesh] step 5 — feed canonical events to node B and poll"
cp network_mesh/reg_b/events/*.json network_mesh/node_b/events/
curl -s -X POST http://127.0.0.1:7372/v1/poll
echo ""

echo ""
echo "[mesh] step 6 — check convergence"
fardrun run --program fard/bin/fd_net_status.fard --out network_mesh/status_out -- \
  --topology examples/network/fard/topology_mesh.json 2>/dev/null

python3 -c "
import json
r = json.load(open('network_mesh/status_out/result.json'))['result']
print('  fully_converged:    ', r['fully_converged'])
print('  nodes_converged:    ', r['nodes_converged'])
print('  registries_converged:', r['registries_converged'])
print('  state_hash:         ', r['state_hash'])
print('  registry_hash:      ', r['registry_hash'])
for n in r['nodes']:
    print('  node', n['url'], 'state_hash:', n.get('state_hash','ERR'), 'supply:', n.get('total_supply','?'))
for reg in r['registries']:
    print('  registry', reg['url'], 'registry_hash:', reg.get('registry_hash','ERR'), 'entries:', reg.get('entry_count','?'))
if r['fully_converged']:
    print()
    print('[mesh] CONVERGED')
else:
    print()
    print('[mesh] DIVERGED — invariant violation')
    exit(1)
"

kill $REG_A $REG_B $NODE_A $NODE_B 2>/dev/null
