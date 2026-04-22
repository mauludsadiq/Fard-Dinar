use ed25519_dalek::{Signer, SigningKey};
use fd_core::{apply_event, canonical_event_set, deposit_signing_payload, event_hash, replay_events, transfer_signing_payload, DepositAttestation, Event, LedgerState, MerchantRegistrySnapshot, ObjectStore, OracleSetSnapshot, ProgramManifest, TransferIntent};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_store() -> (ObjectStore, PathBuf) {
    let mut path = std::env::temp_dir();
    path.push(format!("fd-core-test-{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
    fs::create_dir_all(&path).unwrap();
    (ObjectStore::new(&path), path)
}

fn signing_key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

fn public_hex(sk: &SigningKey) -> String {
    hex::encode(sk.verifying_key().to_bytes())
}

fn write_snapshot<T: serde::Serialize>(store: &ObjectStore, value: &T) -> String {
    let bytes = fd_core::canonical_json_bytes(value).unwrap();
    store.write_bytes(&bytes).unwrap()
}

fn fixture() -> (ObjectStore, LedgerState, SigningKey, SigningKey, SigningKey, SigningKey) {
    let (store, _path) = temp_store();
    let alice = signing_key(1);
    let bob = signing_key(2);
    let merchant = signing_key(3);
    let oracle = signing_key(7);

    let merchant_snapshot = MerchantRegistrySnapshot {
        version: "1.0.0".into(),
        merchants: vec![public_hex(&merchant)],
    };
    let oracle_snapshot = OracleSetSnapshot {
        version: "1.0.0".into(),
        oracles: vec![public_hex(&oracle)],
    };
    let state = LedgerState {
        accounts: BTreeMap::new(),
        consumed_deposits: BTreeSet::new(),
        merchant_registry_hash: write_snapshot(&store, &merchant_snapshot),
        oracle_set_hash: write_snapshot(&store, &oracle_snapshot),
    };
    (store, state, alice, bob, merchant, oracle)
}

#[test]
fn deposit_then_transfer_updates_balances_and_nonces() {
    let (store, mut state, alice, _bob, merchant, oracle) = fixture();
    let manifest = ProgramManifest::default();

    let mut dep = DepositAttestation {
        kind: "deposit".into(),
        oracle_id: public_hex(&oracle),
        usd_cents: 10_000,
        beneficiary: public_hex(&alice),
        external_ref: "ref-1".into(),
        timestamp: 1,
        signature: String::new(),
    };
    dep.signature = hex::encode(oracle.sign(&deposit_signing_payload(&dep)).to_bytes());

    let result = apply_event(&store, &manifest, &state, &Event::Deposit(dep)).unwrap();
    state = result.state;
    assert_eq!(state.account_or_default(&public_hex(&alice)).balance, 10_000);

    let mut tx = TransferIntent {
        kind: "transfer".into(),
        from_key: public_hex(&alice),
        to_key: public_hex(&merchant),
        amount: 2_000,
        nonce: 0,
        signature: String::new(),
    };
    tx.signature = hex::encode(alice.sign(&transfer_signing_payload(&tx)).to_bytes());
    let result = apply_event(&store, &manifest, &state, &Event::Transfer(tx)).unwrap();
    let state = result.state;

    assert_eq!(state.account_or_default(&public_hex(&alice)).balance, 8_020);
    assert_eq!(state.account_or_default(&public_hex(&alice)).next_nonce, 1);
    assert_eq!(state.account_or_default(&public_hex(&merchant)).balance, 2_020);
}

#[test]
fn canonical_event_set_picks_lexicographically_smallest_conflict() {
    let (_store, _state, alice, _bob, merchant, _oracle) = fixture();
    let carol = signing_key(4);

    let mut a = TransferIntent {
        kind: "transfer".into(),
        from_key: public_hex(&alice),
        to_key: public_hex(&merchant),
        amount: 100,
        nonce: 0,
        signature: String::new(),
    };
    a.signature = hex::encode(alice.sign(&transfer_signing_payload(&a)).to_bytes());

    let mut b = TransferIntent {
        kind: "transfer".into(),
        from_key: public_hex(&alice),
        to_key: public_hex(&carol),
        amount: 100,
        nonce: 0,
        signature: String::new(),
    };
    b.signature = hex::encode(alice.sign(&transfer_signing_payload(&b)).to_bytes());

    let canonical = canonical_event_set(&[Event::Transfer(a.clone()), Event::Transfer(b.clone())]);
    assert_eq!(canonical.len(), 1);
    let winner = match &canonical[0] { Event::Transfer(tx) => tx, _ => unreachable!() };
    let expected = if event_hash(&Event::Transfer(a.clone())).0 <= event_hash(&Event::Transfer(b.clone())).0 { a } else { b };
    assert_eq!(winner.to_key, expected.to_key);
}

#[test]
fn replay_is_deterministic_and_materializes_absent_accounts() {
    let (store, genesis, alice, bob, merchant, oracle) = fixture();
    let manifest = ProgramManifest::default();

    let mut dep = DepositAttestation {
        kind: "deposit".into(),
        oracle_id: public_hex(&oracle),
        usd_cents: 5_000,
        beneficiary: public_hex(&alice),
        external_ref: "ref-2".into(),
        timestamp: 2,
        signature: String::new(),
    };
    dep.signature = hex::encode(oracle.sign(&deposit_signing_payload(&dep)).to_bytes());

    let mut tx1 = TransferIntent {
        kind: "transfer".into(),
        from_key: public_hex(&alice),
        to_key: public_hex(&merchant),
        amount: 1_000,
        nonce: 0,
        signature: String::new(),
    };
    tx1.signature = hex::encode(alice.sign(&transfer_signing_payload(&tx1)).to_bytes());

    let mut dep_bob = DepositAttestation {
        kind: "deposit".into(),
        oracle_id: public_hex(&oracle),
        usd_cents: 250,
        beneficiary: public_hex(&bob),
        external_ref: "ref-3".into(),
        timestamp: 3,
        signature: String::new(),
    };
    dep_bob.signature = hex::encode(oracle.sign(&deposit_signing_payload(&dep_bob)).to_bytes());

    let events = vec![Event::Deposit(dep_bob), Event::Deposit(dep), Event::Transfer(tx1)];
    let (state_a, receipts_a) = replay_events(&store, &manifest, &genesis, &events).unwrap();
    let (state_b, receipts_b) = replay_events(&store, &manifest, &genesis, &events).unwrap();

    assert_eq!(state_a, state_b);
    assert_eq!(receipts_a.last().unwrap().post_state_hash, receipts_b.last().unwrap().post_state_hash);
    assert!(state_a.accounts.contains_key(&public_hex(&bob)));
}

#[test]
fn print_object_hashes() {
    use std::fs;
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap().parent().unwrap()
        .join("examples/objects");
    for entry in fs::read_dir(&dir).unwrap() {
        let entry = entry.unwrap();
        let bytes = fs::read(entry.path()).unwrap();
        let hash = fd_core::sha256_tagged(&bytes);
        println!("FILE: {}  NEW_HASH: {}", entry.file_name().to_string_lossy(), hash.as_str());
    }
}

#[test]
fn print_event_hashes() {
    use std::fs;
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap().parent().unwrap()
        .join("examples/events.json");
    let bytes = fs::read(&path).unwrap();
    let events: Vec<fd_core::Event> = serde_json::from_slice(&bytes).unwrap();
    for (i, event) in events.iter().enumerate() {
        let hash = fd_core::event_hash(event);
        println!("event_{}: {}", i, hash.as_str());
    }
}

#[test]
fn hash_candidate_file() {
    let bytes = std::fs::read("/tmp/merchant_registry_candidate.json").unwrap();
    let hash = fd_core::sha256_tagged(&bytes);
    println!("HASH: {}", hash.as_str());
    println!("BYTES: {}", String::from_utf8_lossy(&bytes));
}
