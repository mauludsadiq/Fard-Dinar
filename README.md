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

- receipts: AHD-1024-256
- state commitments: AHD-1024-256
- derivations: AHD-1024-XOF

This repository implements **AHD-1024-256-tagged hashes end to end**, including state and receipt commitments. The hashing layer is isolated from ledger semantics, canonicalization rules, and the replay model.


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

### Apply a single event and materialize state

```bash
cargo run -p fd-cli -- \
  fd-apply \
  --event examples/deposit_alice.json \
  --pre-state examples/pre_state_genesis.json \
  --objects examples/objects \
  --out state.json \
  --repo .
```

This command writes the full post-state to `--out` and prints the receipt JSON to stdout.

Optional:

```bash
--receipt-out receipt.json
```

will persist the receipt for later inspection.

You can inspect a receipt with:

```bash
cargo run -p fd-cli -- \
  fd-receipt \
  --receipt receipt.json
```

You can chain multiple events by feeding the emitted state file into the next `fd-apply` invocation.

### Show total supply for a materialized state

```bash
cargo run -p fd-cli -- \
  fd-supply \
  --state state.json
```

This prints the total FD supply in the state and the number of materialized accounts.

### Diff two materialized states

```bash
cargo run -p fd-cli -- \
  fd-diff state_1.json state_2.json
```

This prints account-level changes and total supply delta between two materialized states.

### Check canonical consistency only

```bash
cargo run -p fd-cli -- \
  fd-consistency \
  --events examples/events.json
```

## Example object store

`examples/objects` is a content-addressed directory. Each file name is the hex portion of a tagged AHD-1024-256 hash. The file bytes themselves are canonical JSON and re-hash to the exact tagged hash referenced by ledger state.

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


## Live Node (execution)

    cargo run -p fd-cli -- \
      fd-node \
      --watch ./events \
      --genesis examples/genesis.json \
      --objects ./objects \
      --state-out state.json \
      --receipts ./receipts

Watches a directory, applies valid events, persists:
- state
- receipts
- processed index
- rejection logs (_errors/)

---

## Registry Node (canonicalization)

    cargo run -p fd-cli -- \
      fd-registry \
      --watch ./registry_events \
      --registry-out registry_state.json

Maintains canonical event selection using:
- conflict key: (effect_kind, conflict_key)
- resolution rule: min(event_hash)

Logs:
- accepted = first insert
- replaced = better candidate
- ignored = worse candidate

---

## System Model

    Signed Intent -> Registry -> Node -> Receipts

Flow:
1. Wallet signs intent
2. Registry selects canonical winner
3. Node executes deterministically
4. Receipt commits the transition

---

## Runtime Artifacts (ignored)

    node_events/
    node_receipts/
    node_state.json
    node_state.processed.json

    registry_events/
    registry_state.json
    registry_state.processed.json


---

## Peer Sync (Distributed Mode)

### Registry → Registry

Run two registries and connect them:

    cargo run -p fd-cli -- \
      fd-registry \
      --watch ./registry_events_b \
      --registry-out registry_b.json \
      --peer-registry registry_a.json

Behavior:
- pulls entries from peer registries
- merges using min(event_hash)
- converges deterministically

---

### Node → Peer Events + Registry

    cargo run -p fd-cli -- \
      fd-node \
      --watch ./node_events \
      --genesis examples/genesis.json \
      --objects ./objects \
      --state-out state.json \
      --receipts ./receipts \
      --peer-watch ./peer_events \
      --peer-registry registry_a.json

Behavior:
- copies events from peer directories
- gates execution using canonical registry winners
- defers events with missing prerequisites (e.g. insufficient balance)
- retries automatically when dependencies arrive

---

### Distributed Flow

    Wallet → Registry A → Registry B → Node B → State + Receipts

Properties:
- deterministic convergence across nodes
- no coordination required
- eventual consistency via registry merge
- execution strictly gated by canonical winners

---



---

## HTTP Transport

FD supports network-based registry sync and state access.

### Start HTTP Server

    cargo run -p fd-cli -- \
      fd-http \
      --bind 127.0.0.1:8081 \
      --registry registry.json \
      --state state.json

Endpoints:

- GET /registry → returns RegistryState
- GET /state → returns LedgerState

Example:

    curl http://127.0.0.1:8081/registry

---

### HTTP Registry Sync

Registries should prefer versioned endpoints:

    cargo run -p fd-cli -- \
      fd-registry \
      --watch ./registry_events \
      --registry-out registry_b.json \
      --peer-registry http://127.0.0.1:8081/v1/registry

Behavior:
- fetches JSON over HTTP
- merges with local registry
- deterministic convergence via min(event_hash)

---

### Hybrid Topology

You can mix transports:

    peer-registry:
      - ./local_registry.json
      - http://peer1:8081/registry
      - http://peer2:8081/registry

System remains:
- deterministic
- eventually consistent
- transport-agnostic

---

### Architecture

    Filesystem ←→ HTTP ←→ Node/Registry

All transports feed the same canonical logic:
- conflict_key
- event_hash ordering
- deterministic merge

---



---

## HTTP Event Ingestion (Direct Routing)

HTTP ingestion can now write directly into a registry or node watch directory.

### Start HTTP with Ingest Target

    cargo run -p fd-cli -- \
      fd-http \
      --bind 127.0.0.1:8083 \
      --registry registry.json \
      --ingest-dir ./registry_events

### Send Event

    curl -X POST http://127.0.0.1:8083/ingest \
      -H 'content-type: application/json' \
      --data-binary @examples/transfer_alice_candidate_a.json

Behavior:
- validates JSON event
- computes event_hash
- writes to target directory as:

    <event_hash>.json

- immediately visible to registry/node

---

### Updated Flow

    Client → HTTP (/ingest) → Watch Dir → Registry → Node → State

No intermediate staging directory required.

---

### Properties

- transport-independent ingestion
- deterministic file naming (event_hash)
- immediate integration with existing pipeline
- no duplication or race conditions

---



---

## HTTP API Surface

Current HTTP endpoints:

- GET /v1/info
- GET /v1/registry
- GET /v1/state
- GET /v1/objects/<hash>
- GET /v1/receipts/<run_id>
- POST /v1/events

Legacy compatibility aliases remain available:
- GET /info
- GET /registry
- GET /state
- GET /objects/<hash>
- GET /receipts/<run_id>
- POST /ingest

Example:

    cargo run -p fd-cli -- \
      fd-http \
      --bind 127.0.0.1:8084 \
      --registry peer_registry_a.json \
      --state peer_node_state_b.json \
      --ingest-dir peer_registry_events_a \
      --receipts-dir peer_node_receipts_b \
      --objects-dir examples/objects

Example queries:

    curl http://127.0.0.1:8084/v1/info
    curl http://127.0.0.1:8084/v1/registry
    curl http://127.0.0.1:8084/v1/state
    curl http://127.0.0.1:8084/v1/objects/ahd1024:72456d65ef7adfa93a7295d48532c8d4b1e604d29371cc4a82ad04e1816232d7
    curl http://127.0.0.1:8084/v1/receipts/ahd1024:60359a4106309b79c4f82ea5a6cda100665da0e9527aef793787f26992773abe
    curl -X POST http://127.0.0.1:8084/v1/events \
      -H 'content-type: application/json' \
      --data-binary @examples/transfer_alice_candidate_a.json

Behavior:
- ingest writes directly into the configured watch directory
- objects and receipts are addressable over HTTP
- state and registry are readable over HTTP
- transport is now filesystem + HTTP hybrid

Next moves:
1. add versioned aliases under /v1/*
2. keep current routes as compatibility aliases
3. move node/registry peer sync to prefer /v1 endpoints
4. then freeze the wire contract in code



---

## HTTP Response Format (Deterministic)

All HTTP endpoints now return canonical JSON responses.

### Success

    {
      "ok": true
    }

or (for data endpoints):

    { ...canonical JSON payload... }

### Error

    {
      "ok": false,
      "code": "<machine_code>",
      "error": "<human_message>"
    }

Examples:

- not found:

    {
      "ok": false,
      "code": "not_found",
      "error": "not found"
    }

- configuration error:

    {
      "ok": false,
      "code": "objects_not_configured",
      "error": "objects not configured"
    }

- ingest error:

    {
      "ok": false,
      "code": "no_ingest_dir",
      "error": "no ingest dir configured"
    }

### Properties

- no plain-text responses
- stable machine-readable error codes
- deterministic JSON formatting
- uniform across all endpoints

---



---

## Treasury-Backed Rewards (FD-CORE v1.1.0)

Transfers now use a configurable, treasury-funded reward model.

### RewardConfig

Defined in genesis and carried in state:

    {
      "user_p2p_bps": 200,
      "user_spend_bps": 200,
      "vendor_spend_bps": 200,
      "treasury_account": "<public_key_hex>"
    }

- `bps` = basis points (1/100 of a percent)
- 200 bps = 2%

### Semantics

For a transfer `amount = A`:

- Determine if recipient is a registered merchant.
- Compute:
  - `user_reward = floor(A * user_bps / 10_000)`
  - `vendor_reward = floor(A * vendor_bps / 10_000)` (merchant only)
- Debit total rewards from `treasury_account`.
- Apply:
  - sender: `-A + user_reward`
  - recipient: `+A (+ vendor_reward if merchant)`

### Guards

- `TreasuryNotFound` if treasury account is missing
- `InsufficientTreasury` if treasury balance < total rewards

### Properties

- No implicit minting (supply-conserving except via treasury policy)
- Fully deterministic (included in state and receipts)
- Tunable via genesis/state without code changes

---



---

## Genesis Configurations

Two reference genesis configurations are provided:

### 1. Zero-Reward (Backward Compatible)

    examples/genesis.json

- All reward rates set to 0
- No treasury required
- Behavior identical to pre-v1.1.0

### 2. Treasury-Backed Rewards

    examples/genesis_rewards.json

- Reward rates set to 200 bps (2%)
- Includes a funded treasury account:

    "accounts": {
      "TREASURY": { "balance": 100000000, "next_nonce": 0 }
    }

- Transfers debit rewards from treasury
- Enables full incentive model

### Notes

- `treasury_account` must exist in `accounts`
- Rewards fail deterministically if treasury is missing or underfunded
- Both configurations are replay-compatible and deterministic

---



---

## FD-CLIENT v0.1 (Rust SDK)

A Rust client is provided for interacting with FD nodes over the canonical `/v1` API.

Location:

    crates/fd-client

### Client

Provides typed access to the wire protocol:

- `get_info()`
- `get_registry() -> RegistryState`
- `get_state() -> LedgerState`
- `get_receipt(run_id) -> Receipt`
- `get_object(hash) -> JSON`
- `submit_event(event)`

Example:

    let client = Client::new("http://127.0.0.1:8085");
    let state = client.get_state()?;

### Wallet (Signing Helpers)

The client includes a deterministic wallet layer for constructing and signing events.

Create wallet from secret:

    let wallet = Wallet::from_secret_hex("<32-byte hex>")?;

Get public key:

    let pubkey = wallet.public_key_hex();

Build signed transfer:

    let tx = wallet.build_signed_transfer(
        "<to_pubkey>",
        2000,
        0
    );

Build signed deposit:

    let dep = wallet.build_signed_deposit(
        "<beneficiary>",
        10000,
        "ref-1",
        1
    );

Event wrappers:

    let evt = wallet.build_signed_transfer_event(...);

### Properties

- Uses the same signing payloads as fd-core
- Produces identical signatures to CLI wallet commands
- Fully deterministic (no randomness in signing)
- Canonical JSON compatible with FD-WIRE

### Example

Run the demo:

    cargo run -p fd-client --example client_demo

---



### Convenience Submission

The client provides one-call submission helpers that combine signing + submission:

Submit transfer:

    let res = client.submit_signed_transfer(
        &wallet,
        "<to_pubkey>",
        2000,
        0
    )?;

Submit deposit:

    let res = client.submit_signed_deposit(
        &wallet,
        "<beneficiary>",
        10000,
        "ref-1",
        1
    )?;

These methods:
- build the canonical event
- sign using the correct payload
- submit to `/v1/events`

This removes all boilerplate from wallet implementations.



---

## FD-WALLET v0.1 (Reference Wallet CLI)

A minimal, deterministic reference wallet is provided.

Location:

    crates/fd-wallet

### Commands

Initialize wallet:

    fd-wallet init --secret-hex <hex> --out wallet.json

Get address:

    fd-wallet address --wallet wallet.json

Check balance:

    fd-wallet balance --wallet wallet.json --base-url http://127.0.0.1:8085

Send transfer:

    fd-wallet send       --wallet wallet.json       --base-url http://127.0.0.1:8085       --to <recipient_pubkey>       --amount 100       --nonce 1

### Properties

- Uses fd-client for signing + submission
- Fully deterministic (no randomness)
- Compatible with FD-WIRE `/v1` API
- Enforces nonce correctness via node validation
- Works directly with reward-enabled state

### Notes

- Nonce must match `next_nonce` in state
- Wallet file is a simple JSON:
  
      {
        "secret_key_hex": "...",
        "public_key_hex": "..."
      }

- Secret keys in fixtures are deterministic for testing only

---



### Auto Nonce (Wallet UX)

The wallet supports automatic nonce resolution:

    fd-wallet send \
      --wallet wallet.json \
      --base-url http://127.0.0.1:8085 \
      --to <recipient_pubkey> \
      --amount 100 \
      --auto-nonce

Behavior:
- Fetches `/v1/state`
- Reads `next_nonce` for the wallet address
- Uses it for signing

Manual override remains available:

    fd-wallet send ... --nonce 3

### Recommendation

Use `--auto-nonce` for normal operation.
Use explicit `--nonce` only for debugging or replay scenarios.

