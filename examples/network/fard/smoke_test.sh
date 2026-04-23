#!/usr/bin/env bash
# Full smoke test: single-node topology end-to-end
# Deposits alice, generates QR, wallet pays, verifies final state.
set -e
cd "$(dirname "$0")/../../.."

rm -rf network_smoke
mkdir -p network_smoke/events network_smoke/receipts

echo "[smoke] starting registry on :7375"
fardrun run --program fard/bin/fd_registry.fard --out network_smoke/reg_out -- \
  --watch        network_smoke/events \
  --registry-out network_smoke/registry.json \
  --bind         127.0.0.1:7375 &
REG_PID=$!

sleep 1

echo "[smoke] starting node on :7374"
fardrun run --program fard/bin/fd_node.fard --out network_smoke/node_out -- \
  --watch     network_smoke/events \
  --genesis   examples/genesis_rewards.json \
  --objects   examples/objects \
  --state-out network_smoke/state.json \
  --receipts  network_smoke/receipts \
  --registry  network_smoke/registry.json \
  --bind      127.0.0.1:7374 &
NODE_PID=$!

sleep 2

echo ""
echo "[smoke] 1. deposit alice"
curl -s -X POST http://127.0.0.1:7375/v1/events \
  -H 'content-type: application/json' \
  -d @examples/deposit_alice.json
curl -s -X POST http://127.0.0.1:7375/v1/poll
curl -s -X POST http://127.0.0.1:7374/v1/poll
echo " done"

echo "[smoke] 2. vendor generates QR"
python3 -c "
import json
print(json.dumps({'public_key_hex':'ed4928c628d1c2c6eae90338905995612959273a5c63f93636c14614ac8737d1'}))
" > /tmp/smoke_vendor.json
fardrun run --program fard/bin/vendor_qr.fard --out network_smoke/qr_out -- \
  --vendor /tmp/smoke_vendor.json --amount 500 --memo "smoke-test" \
  --out network_smoke/request.json 2>/dev/null
URI=$(python3 -c "import json; print(json.load(open('network_smoke/request.json'))['uri'])")
echo " URI: ${URI:0:60}..."

echo "[smoke] 3. wallet pays (auto-nonce from node)"
python3 -c "
import json
print(json.dumps({'secret_key_hex':'0101010101010101010101010101010101010101010101010101010101010101'}))
" > /tmp/smoke_wallet.json
fardrun run --program fard/bin/wallet_pay_request.fard --out network_smoke/pay_out -- \
  --secret   /tmp/smoke_wallet.json \
  --file     network_smoke/request.json \
  --node-url http://127.0.0.1:7375 \
  --out      network_smoke/payment.json 2>/dev/null
HASH=$(python3 -c "import json; print(json.load(open('network_smoke/pay_out/result.json'))['result']['event_hash'])")
echo " event_hash: $HASH"

echo "[smoke] 4. poll to apply"
curl -s -X POST http://127.0.0.1:7375/v1/poll
curl -s -X POST http://127.0.0.1:7374/v1/poll
echo " done"

echo ""
echo "[smoke] 5. final state"
curl -s http://127.0.0.1:7374/v1/state | python3 -c "
import json,sys
s=json.load(sys.stdin)
for k,v in s['accounts'].items():
    label = k[:16]+'...'
    print(f'  {label}  balance={v[\"balance\"]}  nonce={v[\"next_nonce\"]}')
print(f'  consumed_deposits: {len(s[\"consumed_deposits\"])}')
"

echo ""
echo "[smoke] 6. verify receipt"
FIRST=$(ls network_smoke/receipts/ | head -1)
HEX="${FIRST%.json}"
python3 -c "import json; r=json.load(open('network_smoke/receipts/'+'${FIRST}')); print(' receipt run_id:', r.get('run_id','not found'))"
echo " receipts on disk: $(ls network_smoke/receipts/ | wc -l | tr -d ' ')"

echo ""
echo "[smoke] PASSED"

kill $REG_PID $NODE_PID 2>/dev/null
