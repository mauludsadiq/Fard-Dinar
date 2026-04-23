# Fard Dinar

Fard Dinar is a deterministic monetary engine. Every execution produces an AHD-1024-256 receipt committing to inputs, state transitions, and outputs. Two replays of the same events on the same genesis produce the same final state hash — on any machine, at any time.

## Implementation

The engine is implemented in two layers:

| Layer | Language | Purpose |
|---|---|---|
| `fard/` | FARD v1.7.0 | Full engine, CLI programs, wallet |
| AHD-1024 | Rust | Cryptographic hash primitive only |

760 lines of FARD replace 1,547 lines of Rust — a 51% reduction. The only Rust remaining is the AHD-1024 binary, called as a subprocess for hashing.

## Dependencies

- [fardrun](https://github.com/mauludsadiq/FARD) v1.7.0
- [AHD-1024](https://github.com/mauludsadiq/AHD_1024) compiled binary

Install fardrun:

    curl -sf https://raw.githubusercontent.com/mauludsadiq/FARD/main/install.sh | sh

Build AHD binary:

    git clone https://github.com/mauludsadiq/AHD_1024
    cd AHD_1024 && cargo build --release

Set the AHD binary path in `fard/lib/hashes.fard`:

    let AHD_BIN = "/path/to/AHD_1024/target/release/ahd_1024"

## Repository layout

    fard/
    ├── lib/
    │   ├── hashes.fard    — AHD-1024-256 tagged hashing
    │   ├── canon.fard     — canonical JSON (sorted keys, no whitespace)
    │   ├── crypto.fard    — Ed25519 sign and verify
    │   ├── store.fard     — content-addressed object store
    │   ├── engine.fard    — deposit, transfer, replay, conflict resolution
    │   ├── receipt.fard   — transition receipt construction
    │   └── args.fard      — CLI flag parsing
    ├── bin/
    │   ├── fd_replay.fard
    │   ├── fd_apply.fard
    │   ├── fd_consistency.fard
    │   ├── fd_supply.fard
    │   ├── fd_diff.fard
    │   ├── wallet_gen.fard
    │   ├── wallet_sign_transfer.fard
    │   └── wallet_sign_deposit.fard
    └── tests/
        ├── test_foundation.fard   — 7 tests: hashes, canon, crypto
        └── test_engine.fard       — 7 tests: store, engine, replay
    examples/
    ├── genesis.json               — zero-reward genesis
    ├── genesis_rewards.json       — treasury-backed 200bps rewards
    ├── events.json                — example event set (5 events, 4 canonical)
    └── objects/                   — content-addressed registry snapshots

## Implemented monetary rules

### Deposit issuance

A deposit attestation signed by an authorized oracle mints FD 1:1 against `usd_cents` and marks the deposit ID as consumed.

### Transfer semantics

For a transfer of amount `A` with reward config `user_bps` and `vendor_bps`:

- sender balance decreases by `A`
- sender receives rebate `floor(A * user_bps / 10000)` from treasury
- recipient receives `A`
- recipient receives additional `floor(A * vendor_bps / 10000)` from treasury when registered as a merchant
- sender nonce increments by exactly one

### Conflict resolution

Conflicts are resolved on canonical `event_hash`:

- transfers conflict on `(from, nonce)`
- deposits conflict on `deposit_id`
- the winning event is the lexicographically smallest `event_hash`

### Replay order

Canonical replay order is:

- deposits before transfers
- deposits sorted by `(beneficiary, external_ref, event_hash)`
- transfers sorted by `(from, nonce, event_hash)`

### Consumed deposits

Consumed deposit IDs are stored in lexicographic order (equivalent to Rust `BTreeSet`), ensuring deterministic state across implementations.

### Determinism

The engine uses canonical JSON everywhere consensus depends on bytes: event hashes, signing payloads, registry loading verification, receipt input hashing, and final replay state hashing. All hashes use AHD-1024-256 with the `ahd1024:` tag prefix.

## Tests

    fardrun test --program fard/tests/test_foundation.fard
    fardrun test --program fard/tests/test_engine.fard

14 tests total. The engine test suite verifies the final replay state hash against the known value `ahd1024:b350cffb...`.

## CLI programs

### Replay the full example ledger

    fardrun run --program fard/bin/fd_replay.fard --out ./out -- \
      --genesis examples/genesis_rewards.json \
      --events  examples/events.json \
      --objects examples/objects

Output: `{ final_state_hash, supply, event_count, ok }`

### Apply a single event

    fardrun run --program fard/bin/fd_apply.fard --out ./out -- \
      --event     examples/deposit_bob.json \
      --pre-state examples/genesis_rewards.json \
      --objects   examples/objects \
      --out       state.json

Writes post-state to `--out` and prints the receipt.

### Check canonical consistency

    fardrun run --program fard/bin/fd_consistency.fard --out ./out -- \
      --events examples/events.json

### Show total supply

    fardrun run --program fard/bin/fd_supply.fard --out ./out -- \
      --state state.json

### Diff two states

    fardrun run --program fard/bin/fd_diff.fard --out ./out -- \
      --old state_before.json \
      --new state_after.json

### Sign a transfer

    fardrun run --program fard/bin/wallet_sign_transfer.fard --out ./out -- \
      --secret wallet.json \
      --to     <recipient_pubkey> \
      --amount 2000 \
      --nonce  0 \
      --out    transfer.json

### Sign a deposit

    fardrun run --program fard/bin/wallet_sign_deposit.fard --out ./out -- \
      --secret       oracle_wallet.json \
      --beneficiary  <pubkey> \
      --usd-cents    10000 \
      --external-ref bank-wire-0001 \
      --timestamp    1710000000 \
      --out          deposit.json

## Object store

`examples/objects/` is a content-addressed directory. Each file name is the hex portion of a tagged AHD-1024-256 hash. The file bytes are canonical JSON and re-hash to the exact tagged hash referenced by ledger state. The store loader verifies hash, UTF-8, canonical re-serialization, schema, and key uniqueness before any transition can use an object.

## Genesis configurations

Two reference genesis configurations are provided:

`examples/genesis.json` — zero reward rates, no treasury required, backward compatible.

`examples/genesis_rewards.json` — 200 bps (2%) reward rates, funded treasury account, full incentive model.

## Default account semantics

Absent accounts are interpreted as `{ balance: 0, next_nonce: 0 }`. Any write materializes the account in state.

## Receipt model

Each accepted transition produces a receipt with: `run_id`, `program_hash`, `input_hash`, `pre_state_hash`, `post_state_hash`, `trace_hash`. All fields are AHD-1024-256 tagged hashes over canonical JSON.

## Hashing

This repository uses AHD-1024-256 tagged hashes throughout. The tag prefix is `ahd1024:`. The AHD-1024 hash function is a 1600-bit sponge with 24 rounds — see the [AHD-1024 repository](https://github.com/mauludsadiq/AHD_1024) for the full specification, test vectors, and cryptanalytic results.

## Design choices

### Why canonical JSON everywhere?

The spec is only meaningful if every node signs, hashes, verifies, and replays the exact same bytes. Canonical JSON supplies that invariant for transfer signing payloads, deposit signing payloads, event hashes, registry snapshots, oracle sets, and receipts.

### Why conflict resolution on `event_hash`?

A receipt hash includes execution details. The economic identity of an event must be independent of who executed it or how they traced it. Canonical selection is therefore based on event content only.

### Why an object store instead of network fetch?

Consensus logic must not depend on live network IO. The engine resolves content-addressed objects from a local store, verifies them, and then uses them. Distribution is a separate concern from monetary validity.

### Why FARD?

FARD is a deterministic, content-addressed scripting language. Every FARD execution produces a receipt committing to source, imports, inputs, and outputs — the same guarantees Fard Dinar requires from its engine. 760 lines of FARD replace 1,547 lines of Rust with no loss of determinism or verifiability.

## System model

    Signed Intent -> Canonical Event Set -> Engine -> State + Receipts

1. Wallet signs intent using canonical JSON payload
2. Conflict resolution selects canonical winner per `(effect_kind, conflict_key)`
3. Engine executes deterministically against verified object store
4. Receipt commits the transition with AHD-1024-256 hashes

## Packaging

    cd ..
    zip -r "Fard Dinar.zip" "Fard Dinar"
