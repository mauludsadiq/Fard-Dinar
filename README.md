# Fard Dinar

A deterministic monetary engine. Every execution produces an AHD-1024-256 receipt committing to inputs, state, and outputs. Same events, same genesis, same machine or different — identical final state hash, every time.

---

## Stack

| Layer | What it is |
|---|---|
| `fard/` | The engine, wallet, and vendor tooling — pure FARD v1.7.0 |
| AHD-1024 | The hash function — a 1600-bit sponge, called as a subprocess |

760 lines of FARD. The only Rust is the AHD-1024 binary.

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

## Running

Every program follows the same pattern:

    fardrun run --program fard/bin/<name>.fard --out ./out -- <flags>

Result is always in `./out/result.json`.

### Engine

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

**Check canonical consistency**

    fardrun run --program fard/bin/fd_consistency.fard --out ./out -- \
      --events examples/events.json

**Supply**

    fardrun run --program fard/bin/fd_supply.fard --out ./out -- \
      --state state.json

**Diff two states**

    fardrun run --program fard/bin/fd_diff.fard --out ./out -- \
      --old state_before.json --new state_after.json

---

### Wallet

Wallet files: `{ "secret_key_hex": "..." }` for signing, `{ "public_key_hex": "..." }` for read-only views.

**Sign a transfer**

    fardrun run --program fard/bin/wallet_sign_transfer.fard --out ./out -- \
      --secret wallet.json \
      --to     <recipient_pubkey> \
      --amount 2000 \
      --nonce  0 \
      --out    transfer.json

**Sign a deposit**

    fardrun run --program fard/bin/wallet_sign_deposit.fard --out ./out -- \
      --secret       oracle_wallet.json \
      --beneficiary  <pubkey> \
      --usd-cents    10000 \
      --external-ref bank-wire-0001 \
      --timestamp    1710000000 \
      --out          deposit.json

**Transaction history**

    fardrun run --program fard/bin/wallet_history.fard --out ./out -- \
      --wallet       wallet_pub.json \
      --receipts-dir examples/receipts \
      --events-dir   examples

    # { public_key_hex, count, history: [{ run_id, kind, direction, counterparty, amount }] }

**Rewards earned**

    fardrun run --program fard/bin/wallet_rewards.fard --out ./out -- \
      --wallet       wallet_pub.json \
      --receipts-dir examples/receipts \
      --events-dir   examples \
      --state        state.json

    # { public_key_hex, total_rewards, by_counterparty }

---

### Vendor

Vendor files: `{ "public_key_hex": "..." }`.

**Generate a payment request**

    fardrun run --program fard/bin/vendor_request_payment.fard --out ./out -- \
      --vendor vendor.json \
      --amount 100 \
      --memo   coffee \
      --out    request.json

    # { kind: "fd_payment_request_v1", to, amount, memo, nonce_mode }

**Verify a receipt**

    fardrun run --program fard/bin/vendor_verify_receipt.fard --out ./out -- \
      --run-id       ahd1024:<hex> \
      --receipts-dir examples/receipts

**Inbox — all incoming payments**

    fardrun run --program fard/bin/vendor_inbox.fard --out ./out -- \
      --vendor       vendor.json \
      --receipts-dir examples/receipts \
      --events-dir   examples

    # { payment_count, total_received, payments }

**P&L summary**

    fardrun run --program fard/bin/vendor_summary.fard --out ./out -- \
      --vendor       vendor.json \
      --receipts-dir examples/receipts \
      --events-dir   examples \
      --state        state.json

    # { payment_count, total_received, total_rewards, gross_revenue, current_balance }

**Export to CSV**

    fardrun run --program fard/bin/vendor_export.fard --out ./out -- \
      --vendor       vendor.json \
      --receipts-dir examples/receipts \
      --events-dir   examples \
      --state        state.json \
      --out          payments.csv

    # columns: run_id, from, to, amount, vendor_reward

---

## Tests

    fardrun test --program fard/tests/test_foundation.fard
    fardrun test --program fard/tests/test_engine.fard

14 tests. The engine suite verifies the final replay state hash against the known value `ahd1024:b350cffb...`, byte-for-byte identical to the Rust implementation.

---

## Examples

**`examples/genesis.json`** — zero reward rates, no treasury required.

**`examples/genesis_rewards.json`** — 200 bps (2%) reward rates with a funded treasury. Use this for the full incentive model.

**`examples/events.json`** — 5 events, 4 canonical after conflict resolution.

**`examples/receipts/`** — receipts for all 4 canonical events, generated by `fd_apply`.

**`examples/objects/`** — content-addressed merchant registry and oracle set snapshots. File names are AHD-1024-256 hex. The store verifies hash, canonical re-serialization, and schema before use.

---

## Receipts and the join key

Every `fd_apply` run produces:

    {
      run_id:          "ahd1024:...",
      program_hash:    "ahd1024:...",
      input_hash:      "ahd1024:...",
      pre_state_hash:  "ahd1024:...",
      post_state_hash: "ahd1024:...",
      trace_hash:      "ahd1024:..."
    }

`input_hash` is `AHD("FD_EVENT_V1" + canonical_json(event))`. It is the join key between a receipt and its source event — how `wallet_history`, `vendor_inbox`, and all downstream views connect receipts to the events that produced them.

---

## Hashing

All hashes use AHD-1024-256 with the prefix `ahd1024:`. AHD-1024 is a 1600-bit sponge (rate 1024, capacity 576) with 24 rounds. Three independent implementations — Rust, Python, C — produce bit-identical outputs for all test vectors. See [AHD-1024](https://github.com/mauludsadiq/AHD_1024) for the full specification and cryptanalytic results.

---

## Why FARD

FARD executions are themselves content-addressed. Every run produces a `fard_run_digest` committing to source, imports, inputs, and result. The engine's determinism guarantee and FARD's execution receipt model are the same invariant expressed at two levels.

760 lines of FARD replace 1,547 lines of Rust. The reduction comes from removing boilerplate, not from cutting features — every monetary rule, conflict resolution path, and receipt field is preserved and verified.
