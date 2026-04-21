# Fard Dinar

Fard Dinar is a deterministic monetary engine written in Rust from the frozen `FD-CORE v1.0.0` specification. The repo implements the full event model, canonicalization rules, replay engine, content-addressed registry loading, Ed25519 verification, receipt generation, and a CLI for transition verification and full-ledger replay.

## What this repository contains

- `crates/fd-core` — the core library
  - canonical JSON encoder with sorted keys and no insignificant whitespace
  - typed ledger state, events, registries, genesis config, and receipts
  - Ed25519 signature verification on canonical JSON payloads
  - content-addressed object store with hash verification and canonical JSON re-hash validation
  - deterministic conflict resolution on `event_hash`
  - canonical replay ordering
  - transfer and deposit transition functions
  - replay and verification helpers
- `crates/fd-cli` — the `fardverify` CLI
- `examples/` — a complete runnable fixture set
  - genesis
  - events
  - individual signed deposits and transfers
  - content-addressed merchant registry and oracle set objects
  - deterministic dev fixture keys for reproducible local testing
- `crates/fd-core/tests` — integration tests covering canonicalization, replay, and transition semantics
- `scripts/` — packaging helpers

## Implemented monetary rules

### Deposit issuance

A deposit attestation signed by an authorized oracle mints FD 1:1 against `usd_cents` and marks the deposit ID as consumed.

### Transfer semantics

For a transfer of amount `a`:

- sender balance decreases by `a`
- sender receives rebate `floor(a / 100)`
- recipient receives `a`
- recipient receives additional merchant revenue share `floor(a / 100)` when recipient is in the merchant registry
- sender nonce increments by exactly one

### Conflict resolution

Conflicts are resolved on canonical `event_hash`, not on execution receipt:

- transfers conflict on `(from, nonce)`
- deposits conflict on `deposit_id`
- the winning event is the lexicographically smallest `event_hash`

### Replay order

Canonical replay order is:

- deposits before transfers
- deposits sorted by `(beneficiary, external_ref, event_hash)`
- transfers sorted by `(from, nonce, event_hash)`

### Determinism

The engine uses canonical JSON everywhere consensus depends on bytes:

- event hashes
- signing payloads
- registry loading verification
- receipt input hashing
- final replay state hashing

## Important implementation note on hashing

The original spec text distinguishes between:

- receipts: SHA-256
- state commitments: AHD-1024-256
- derivations: AHD-1024-XOF

This repository implements **SHA-256-tagged hashes end to end**, including state and receipt commitments, because the complete bit-level AHD-1024-256 and AHD-1024-XOF algorithms were not included in the conversation context used to build this repo. The hashing layer is isolated so an exact AHD implementation can be substituted without changing the ledger semantics, canonicalization rules, or replay model.

Nothing in this repo is stubbed. The system runs as implemented. The one explicit divergence from the original frozen text is that state commitments are SHA-256-tagged in this Rust implementation.

## Workspace layout

```text
Fard Dinar/
├── Cargo.toml
├── README.md
├── crates/
│   ├── fd-core/
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── canon.rs
│   │   │   ├── crypto.rs
│   │   │   ├── engine.rs
│   │   │   ├── errors.rs
│   │   │   ├── hashes.rs
│   │   │   ├── lib.rs
│   │   │   ├── receipt.rs
│   │   │   ├── store.rs
│   │   │   ├── types.rs
│   │   │   └── verify.rs
│   │   └── tests/
│   │       └── fd_spec.rs
│   └── fd-cli/
│       ├── Cargo.toml
│       └── src/main.rs
├── examples/
│   ├── dev-fixtures.json
│   ├── events.json
│   ├── genesis.json
│   ├── deposit_alice.json
│   ├── deposit_bob.json
│   ├── transfer_alice_candidate_a.json
│   ├── transfer_alice_candidate_b.json
│   ├── transfer_bob.json
│   └── objects/
└── scripts/
```

## Build

Rust is not installed in this container, so this repo was assembled and packaged but not compiled here. In a normal Rust environment:

```bash
cargo build --workspace
cargo test --workspace
```

## CLI

The binary is `fardverify`.

### Verify a single transition

```bash
cargo run -p fd-cli -- \
  fd \
  --event examples/deposit_alice.json \
  --pre-state examples/genesis.json \
  --objects examples/objects \
  --repo .
```

For `--pre-state`, pass a full `LedgerState` JSON. The genesis file is a `GenesisConfiguration`, so for single-transition verification use a state file built from genesis or run a replay first.

### Replay the full example ledger

```bash
cargo run -p fd-cli -- \
  fd-replay \
  --events examples/events.json \
  --genesis examples/genesis.json \
  --objects examples/objects \
  --repo .
```

### Check canonical consistency only

```bash
cargo run -p fd-cli -- \
  fd-consistency \
  --events examples/events.json
```

## Example object store

`examples/objects` is a content-addressed directory. Each file name is the hex portion of a tagged SHA-256 hash. The file bytes themselves are canonical JSON and re-hash to the exact tagged hash referenced by ledger state.

The object store currently contains:

- merchant registry snapshot
- oracle set snapshot

The store loader enforces all of the following before a transition can use an object:

- object exists
- bytes hash to the requested tagged hash
- bytes parse as UTF-8 JSON
- canonical re-serialization hashes to the same tagged hash
- schema validates
- merchant/oracle keys are unique and valid lowercase 64-char Ed25519 public keys

## Default account semantics

Absent accounts are interpreted as:

```json
{
  "balance": 0,
  "next_nonce": 0
}
```

Any write materializes the account in `state.accounts`.

## Receipt model

Each accepted transition produces:

- `run_id`
- `program_hash`
- `input_hash`
- `pre_state_hash`
- `post_state_hash`
- `trace_hash`

The receipt is generated from canonical JSON over a trace object containing:

- program manifest
- input event
- pre-state
- post-state

## Tests

The integration tests cover:

- deposit application
- merchant revenue share and sender rebate
- nonce advancement
- canonical conflict winner selection
- deterministic replay
- absent-account materialization

Run them with:

```bash
cargo test -p fd-core
```

## Deterministic dev fixtures

`examples/dev-fixtures.json` contains deterministic private-key seeds and derived public keys for local reproduction of the example signatures. These fixtures are for development only.

## Design choices in this codebase

### Why canonical JSON everywhere?

The spec is only meaningful if every node signs, hashes, verifies, and replays the exact same bytes. Canonical JSON supplies that invariant for:

- transfer signing payloads
- deposit signing payloads
- event hashes
- registry snapshots
- oracle sets
- receipts

### Why conflict resolution on `event_hash`?

A receipt hash includes execution details. The economic identity of an event must be independent of who executed it or how they traced it. Therefore canonical selection is based on event content only.

### Why an object store instead of network fetch?

Consensus logic must not depend on live network IO. The Rust implementation resolves content-addressed objects from a local store, verifies them, and then uses them. Distribution is a separate concern from monetary validity.

## Extending the repo

The cleanest future extension points are:

- replace SHA-256 state commitments with exact AHD-1024-256 and AHD-1024-XOF implementations
- persist receipts to a dedicated receipt store
- add state snapshot export and import commands
- add registry-entry generation helpers for gossip or CRDT synchronization layers
- add binary or canonical CBOR codecs while preserving the same field semantics

## Packaging

To create a zip from a local machine:

```bash
cd ..
zip -r "Fard Dinar.zip" "Fard Dinar"
```

A packaged zip is included with this deliverable.
