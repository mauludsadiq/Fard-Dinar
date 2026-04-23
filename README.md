# Fard Dinar

A deterministic monetary engine. Every execution produces an AHD-1024-256 receipt committing to inputs, state, and outputs. Same events, same genesis, same machine or different — identical final state hash, every time.

---

## Stack

| Layer | What it is |
|---|---|
| `fard/` | Engine, wallet, vendor, network, analytics — pure FARD v1.7.0 |
| AHD-1024 | Hash function — a 1600-bit sponge, called as a subprocess |

The only Rust is the AHD-1024 binary. Everything else is FARD.

---

## Setup

**1. Install fardrun**

    curl -sf https://raw.githubusercontent.com/mauludsadiq/FARD/main/install.sh | sh

**2. Build AHD**

    git clone https://github.com/mauludsadiq/AHD_1024
    cd AHD_1024 && cargo build --release

**3. Set the path in `fard/lib/hashes.fard`**

    let AHD_BIN = "/path/to/AHD_1024/target/release/ahd_1024"

---

## How money moves

**Deposit** — an authorized oracle signs an attestation. FD is minted 1:1 against `usd_cents`. The deposit ID is marked consumed so it cannot be replayed.

**Transfer** — sender signs a canonical payload. The engine checks nonce, balance, and signature. Rewards flow from the treasury:

- sender gets `floor(amount × user_bps / 10000)`
- merchant recipient gets `floor(amount × vendor_bps / 10000)`

**Conflict resolution** — when two events compete for the same slot `(from, nonce)` or the same `deposit_id`, the one with the lexicographically smallest `event_hash` wins. Canonical replay always produces deposits first, then transfers, both sorted deterministically.

---

## Payment flow

    Vendor generates fdpay: URI
        ↓
    Wallet scans URI, resolves nonce, signs transfer
        ↓
    Event submitted to registry
        ↓
    Registry resolves conflicts, stores canonical winner
        ↓
    Node applies canonical events in deterministic order
        ↓
    Receipt committed — ahd1024: hash over canonical trace

---

## Quickstart

    bash examples/network/fard/smoke_test.sh

Starts a registry and node, deposits to alice, generates a QR payment request, wallet pays it with auto-nonce, polls to apply, verifies the receipt. Full end-to-end in one command.

---

## Running

Every batch program:

    fardrun run --program fard/bin/<name>.fard --out ./out -- <flags>

Result always in `./out/result.json`.

---

## Engine

**Replay a full ledger**

    fardrun run --program fard/bin/fd_replay.fard --out ./out -- \
      --genesis examples/genesis_rewards.json \
      --events  examples/events.json \
      --objects examples/objects

    # { final_state_hash, supply, event_count, ok }

**Apply one event**

    fardrun run --program fard/bin/fd_apply.fard --out ./out -- \
      --event     examples/deposit_bob.json \
      --pre-state examples/genesis_rewards.json \
      --objects   examples/objects \
      --out       state.json

Writes post-state to `--out`, prints the receipt. Add `--receipt-out receipt.json` to persist.

**Consistency, supply, diff**

    fardrun run --program fard/bin/fd_consistency.fard --out ./out -- --events examples/events.json
    fardrun run --program fard/bin/fd_supply.fard --out ./out -- --state state.json
    fardrun run --program fard/bin/fd_diff.fard --out ./out -- --old s1.json --new s2.json

---

## Network

**Registry** — accepts events, resolves conflicts, serves canonical registry.

    fardrun run --program fard/bin/fd_registry.fard --out ./out -- \
      --watch        ./events \
      --registry-out registry.json \
      --bind         127.0.0.1:7371

    POST /v1/events    — ingest an event
    POST /v1/poll      — process watch dir, resolve conflicts
    GET  /v1/registry  — canonical registry state
    GET  /v1/info

**Node** — applies canonical events in deterministic order.

    fardrun run --program fard/bin/fd_node.fard --out ./out -- \
      --watch     ./events \
      --genesis   examples/genesis_rewards.json \
      --objects   examples/objects \
      --state-out state.json \
      --receipts  ./receipts \
      --registry  registry.json \
      --bind      127.0.0.1:7370

    POST /v1/poll            — apply pending canonical events in order
    GET  /v1/state           — current ledger state
    GET  /v1/receipts/<hex>  — receipt by run_id hex
    GET  /v1/info

**Launchers**

    bash examples/network/fard/single_node.sh     # registry :7371 + node :7370
    bash examples/network/fard/two_registry.sh    # registry A :7371, B :7373, node B :7372
    bash examples/network/fard/smoke_test.sh      # full end-to-end test

---

## Wallet

Wallet files: `{ "secret_key_hex": "..." }` for signing, `{ "public_key_hex": "..." }` for read-only.

**Sign a transfer**

    fardrun run --program fard/bin/wallet_sign_transfer.fard --out ./out -- \
      --secret wallet.json --to <pubkey> --amount 2000 --nonce 0 --out transfer.json

**Sign a deposit**

    fardrun run --program fard/bin/wallet_sign_deposit.fard --out ./out -- \
      --secret oracle.json --beneficiary <pubkey> \
      --usd-cents 10000 --external-ref bank-wire-0001 --timestamp 1710000000 \
      --out deposit.json

**Pay a payment request**

    fardrun run --program fard/bin/wallet_pay_request.fard --out ./out -- \
      --secret   wallet.json \
      --file     request.json \
      --node-url http://127.0.0.1:7371 \
      --out      payment.json

Decodes the `fdpay:` URI, resolves nonce from the node automatically, signs and submits. Pass `--nonce N` to override.

**History and rewards**

    fardrun run --program fard/bin/wallet_history.fard --out ./out -- \
      --wallet wallet_pub.json --receipts-dir ./receipts --events-dir ./events

    fardrun run --program fard/bin/wallet_rewards.fard --out ./out -- \
      --wallet wallet_pub.json --receipts-dir ./receipts --events-dir ./events --state state.json

---

## Vendor

Vendor files: `{ "public_key_hex": "..." }`.

**Generate QR payment request**

    fardrun run --program fard/bin/vendor_qr.fard --out ./out -- \
      --vendor vendor.json --amount 500 --memo "espresso" --out request.json

Produces a `fdpay:` URI — base64url of the canonical request JSON. Display as a QR code or share as a link.

**Inbox, summary, export**

    fardrun run --program fard/bin/vendor_inbox.fard --out ./out -- \
      --vendor vendor.json --receipts-dir ./receipts --events-dir ./events

    fardrun run --program fard/bin/vendor_summary.fard --out ./out -- \
      --vendor vendor.json --receipts-dir ./receipts --events-dir ./events --state state.json

    fardrun run --program fard/bin/vendor_export.fard --out ./out -- \
      --vendor vendor.json --receipts-dir ./receipts --events-dir ./events \
      --state state.json --out payments.csv

**Verify a receipt**

    fardrun run --program fard/bin/vendor_verify_receipt.fard --out ./out -- \
      --run-id ahd1024:<hex> --receipts-dir ./receipts

---

## Treasury analytics

    fardrun run --program fard/bin/treasury_analytics.fard --out ./out -- \
      --state        state.json \
      --genesis      examples/genesis_rewards.json \
      --events-dir   ./events \
      --receipts-dir ./receipts

Output includes: current balance, total burned, total rewards paid split by user vs vendor, per-transfer breakdown with run_ids, avg reward per transfer, and projected runway in remaining transfers.

---

## fdpay: URI format

    fdpay:<base64url(canonical_json(request))>

Where `request` is:

    {
      "amount":     <int>,
      "kind":       "fd_payment_request_v1",
      "memo":       "<text>",
      "nonce_mode": "auto",
      "to":         "<vendor_pubkey_hex>"
    }

The wallet decodes this, resolves the current nonce from the node, signs, and submits. No out-of-band communication required beyond the URI.

---

## Tests

    fardrun test --program fard/tests/test_foundation.fard
    fardrun test --program fard/tests/test_engine.fard

14 tests. The engine suite verifies the final replay state hash against `ahd1024:b350cffb...`, byte-for-byte identical to the reference implementation.

---

## Examples

`examples/genesis.json` — zero reward rates, no treasury required.

`examples/genesis_rewards.json` — 200 bps (2%) reward rates with a funded treasury.

`examples/events.json` — 5 events, 4 canonical after conflict resolution.

`examples/receipts/` — receipts for all 4 canonical events.

`examples/objects/` — content-addressed merchant registry and oracle set snapshots. File names are AHD-1024-256 hex. The store verifies hash, canonical re-serialization, and schema before use.

`examples/dev-fixtures.json` — deterministic key seeds for local testing.

`examples/network/fard/` — network launcher and smoke test scripts.

---

## Receipts and the join key

Every `fd_apply` run produces:

    {
      "run_id":          "ahd1024:...",
      "program_hash":    "ahd1024:...",
      "input_hash":      "ahd1024:...",
      "pre_state_hash":  "ahd1024:...",
      "post_state_hash": "ahd1024:...",
      "trace_hash":      "ahd1024:..."
    }

`input_hash` is `AHD("FD_EVENT_V1" + canonical_json(event))`. It is the join key between a receipt and its source event — how `wallet_history`, `vendor_inbox`, `treasury_analytics`, and all downstream views connect receipts to the events that produced them.

---

## Hashing

All hashes use AHD-1024-256 with the prefix `ahd1024:`. AHD-1024 is a 1600-bit sponge (rate 1024, capacity 576) with 24 rounds. Three independent implementations — Rust, Python, C — produce bit-identical outputs for all test vectors. See [AHD-1024](https://github.com/mauludsadiq/AHD_1024) for the full specification and cryptanalytic results.

---

## Why FARD

FARD executions are themselves content-addressed. Every run produces a `fard_run_digest` committing to source, imports, inputs, and result. The engine's determinism guarantee and FARD's execution receipt model are the same invariant expressed at two levels.

The engine, wallet, vendor surface, network orchestration, QR payment flow, and treasury analytics are all pure FARD. The only Rust is the AHD-1024 hash binary.

---

## License

MUI
