# FD-CORE v1.0.0

This repository implements the frozen Fard Dinar monetary protocol as a deterministic event-driven state machine with canonical conflict resolution, replay, registry verification, and Ed25519-authenticated deposits and transfers.

## Normative implementation notes in this repo

- canonical JSON is the byte-level source of truth for event hashes and signing payloads
- conflicts are resolved on canonical event hash, never on run receipt
- absent accounts are interpreted as `{ "balance": 0, "next_nonce": 0 }`
- content-addressed registry objects are local and hash-verified before use
- all serialized hash values are tagged SHA-256 strings in the form `sha256:<64-lowercase-hex>`
- this Rust code uses SHA-256 for state commitments in place of AHD-1024-256 because the full AHD bit-level spec was not available in the source material used to assemble the repo

See the top-level `README.md` for operational details and CLI examples.
