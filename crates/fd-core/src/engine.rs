use crate::{
    canonical_json_bytes, verify_ed25519, DepositAttestation, Event, FdError, LedgerState, MerchantRegistrySnapshot,
    ObjectStore, OracleSetSnapshot, Receipt, TransferIntent, TransitionResult,
};
use serde::Serialize;
use std::cmp::Ordering;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize)]
pub struct ProgramManifest {
    pub name: String,
    pub version: String,
    pub modules: BTreeMap<String, String>,
}

impl Default for ProgramManifest {
    fn default() -> Self {
        let mut modules = BTreeMap::new();
        modules.insert("fd-core".to_string(), env!("CARGO_PKG_VERSION").to_string());
        Self {
            name: "fd-core".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            modules,
        }
    }
}

pub fn txid(tx: &TransferIntent) -> String {
    crate::sha256_tagged(
        &canonical_json_bytes(&serde_json::json!({
            "domain": "FD_TX_V1",
            "from": tx.from_key,
            "to": tx.to_key,
            "amount": tx.amount,
            "nonce": tx.nonce
        }))
        .expect("transfer txid payload must serialize"),
    )
    .0
}

pub fn deposit_id(dep: &DepositAttestation) -> String {
    crate::sha256_tagged(
        &canonical_json_bytes(&serde_json::json!({
            "domain": "FD_DEP_V1",
            "oracle_id": dep.oracle_id,
            "external_ref": dep.external_ref,
        }))
        .expect("deposit id payload must serialize"),
    )
    .0
}

pub fn conflict_key(event: &Event) -> String {
    match event {
        Event::Transfer(tx) => crate::sha256_tagged(
            &canonical_json_bytes(&serde_json::json!({
                "domain": "FD_CONFLICT_V1",
                "from": tx.from_key,
                "nonce": tx.nonce,
            }))
            .expect("conflict payload must serialize"),
        )
        .0,
        Event::Deposit(dep) => deposit_id(dep),
    }
}

pub fn event_hash(event: &Event) -> crate::TaggedHash {
    let mut prefixed = b"FD_EVENT_V1".to_vec();
    prefixed.extend(canonical_json_bytes(event).expect("event must serialize"));
    crate::sha256_event_hash(&prefixed)
}

pub fn transfer_signing_payload(tx: &TransferIntent) -> Vec<u8> {
    canonical_json_bytes(&serde_json::json!({
        "domain": "FD_TRANSFER_V1",
        "from": tx.from_key,
        "to": tx.to_key,
        "amount": tx.amount,
        "nonce": tx.nonce,
    }))
    .expect("transfer signing payload must serialize")
}

pub fn deposit_signing_payload(dep: &DepositAttestation) -> Vec<u8> {
    canonical_json_bytes(&serde_json::json!({
        "domain": "FD_DEPOSIT_V1",
        "oracle_id": dep.oracle_id,
        "usd_cents": dep.usd_cents,
        "beneficiary": dep.beneficiary,
        "external_ref": dep.external_ref,
        "timestamp": dep.timestamp,
    }))
    .expect("deposit signing payload must serialize")
}

pub fn is_merchant(store: &ObjectStore, state: &LedgerState, pubkey: &str) -> Result<bool, FdError> {
    let snapshot: MerchantRegistrySnapshot = store.load_registry(&state.merchant_registry_hash)?;
    Ok(snapshot.merchants.binary_search_by(|entry| entry.as_bytes().cmp(pubkey.as_bytes())).is_ok())
}

pub fn load_oracle_set(store: &ObjectStore, state: &LedgerState) -> Result<OracleSetSnapshot, FdError> {
    store.load_oracle_set(&state.oracle_set_hash)
}

pub fn apply_event(store: &ObjectStore, program_manifest: &ProgramManifest, state: &LedgerState, event: &Event) -> Result<TransitionResult, FdError> {
    let pre_state = state.clone();
    let mut post_state = state.clone();

    match event {
        Event::Deposit(dep) => apply_deposit(store, &mut post_state, dep)?,
        Event::Transfer(tx) => apply_transfer(store, &mut post_state, tx)?,
    }

    let trace_bytes = build_trace_bytes(program_manifest, event, &pre_state, &post_state)?;
    let receipt = crate::build_receipt(program_manifest, event, &pre_state, &post_state, &trace_bytes);
    Ok(TransitionResult { state: post_state, receipt })
}

fn build_trace_bytes(program_manifest: &ProgramManifest, event: &Event, pre_state: &LedgerState, post_state: &LedgerState) -> Result<Vec<u8>, FdError> {
    let trace = serde_json::json!({
        "program_manifest": program_manifest,
        "event": event,
        "pre_state": pre_state,
        "post_state": post_state,
    });
    canonical_json_bytes(&trace).map_err(|e| FdError::InvalidJson(e.to_string()))
}

fn apply_deposit(store: &ObjectStore, state: &mut LedgerState, dep: &DepositAttestation) -> Result<(), FdError> {
    if dep.kind != "deposit" {
        return Err(FdError::UnsupportedEventKind(dep.kind.clone()));
    }
    let dep_id = deposit_id(dep);
    if state.consumed_deposits.contains(&dep_id) {
        return Err(FdError::DepositAlreadyConsumed);
    }

    let oracle_set = load_oracle_set(store, state)?;
    if !oracle_set.oracles.iter().any(|oracle| oracle == &dep.oracle_id) {
        return Err(FdError::UnauthorizedOracle);
    }
    verify_ed25519(&dep.oracle_id, &deposit_signing_payload(dep), &dep.signature)?;

    let account = state.materialize_account_mut(&dep.beneficiary);
    account.balance = account.balance.checked_add(dep.usd_cents).expect("balance overflow");
    state.consumed_deposits.insert(dep_id);
    Ok(())
}

fn apply_transfer(store: &ObjectStore, state: &mut LedgerState, tx: &TransferIntent) -> Result<(), FdError> {
    if tx.kind != "transfer" {
        return Err(FdError::UnsupportedEventKind(tx.kind.clone()));
    }
    if tx.from_key == tx.to_key {
        return Err(FdError::SelfTransfer);
    }
    verify_ed25519(&tx.from_key, &transfer_signing_payload(tx), &tx.signature)?;

    let from_account = state.account_or_default(&tx.from_key);
    if from_account.next_nonce != tx.nonce {
        return Err(FdError::InvalidNonce {
            expected: from_account.next_nonce,
            actual: tx.nonce,
        });
    }
    if from_account.balance < tx.amount {
        return Err(FdError::InsufficientBalance);
    }

    let treasury_account = state.reward_config.treasury_account.clone();
    let user_p2p_bps = state.reward_config.user_p2p_bps;
    let user_spend_bps = state.reward_config.user_spend_bps;
    let vendor_spend_bps = state.reward_config.vendor_spend_bps;
    let is_vendor = is_merchant(store, state, &tx.to_key)?;
    let user_bps = if is_vendor { user_spend_bps } else { user_p2p_bps };
    let vendor_bps = if is_vendor { vendor_spend_bps } else { 0 };

    let sender_rebate = (tx.amount as u128 * user_bps as u128 / 10_000) as u64;
    let merchant_rebate = (tx.amount as u128 * vendor_bps as u128 / 10_000) as u64;
    let total_reward = sender_rebate + merchant_rebate;

    if !state.accounts.contains_key(&treasury_account) {
        return Err(FdError::TreasuryNotFound);
    }
    if state.account_or_default(&treasury_account).balance < total_reward {
        return Err(FdError::InsufficientTreasury);
    }

    {
        let from_mut = state.materialize_account_mut(&tx.from_key);
        from_mut.balance -= tx.amount;
        from_mut.next_nonce += 1;
    }
    {
        let to_mut = state.materialize_account_mut(&tx.to_key);
        to_mut.balance += tx.amount;
        if merchant_rebate > 0 {
            to_mut.balance += merchant_rebate;
        }
    }
    if sender_rebate > 0 || merchant_rebate > 0 {
        let treasury_mut = state.materialize_account_mut(&treasury_account);
        treasury_mut.balance -= total_reward;
    }
    if sender_rebate > 0 {
        let from_mut = state.materialize_account_mut(&tx.from_key);
        from_mut.balance += sender_rebate;
    }
    Ok(())
}

pub fn canonical_event_set(events: &[Event]) -> Vec<Event> {
    let mut winners: BTreeMap<(u8, String), (String, Event)> = BTreeMap::new();
    for event in events {
        let effect_kind = match event {
            Event::Deposit(_) => 0_u8,
            Event::Transfer(_) => 1_u8,
        };
        let key = (effect_kind, conflict_key(event));
        let eh = event_hash(event).0;
        match winners.get(&key) {
            Some((current, _)) if current <= &eh => {}
            _ => {
                winners.insert(key, (eh, event.clone()));
            }
        }
    }
    let mut canonical = winners.into_values().map(|(_, e)| e).collect::<Vec<_>>();
    canonical.sort_by(event_sort_key_cmp);
    canonical
}

fn event_sort_key_cmp(a: &Event, b: &Event) -> Ordering {
    match (a, b) {
        (Event::Deposit(da), Event::Deposit(db)) => Ordering::Equal
            .then_with(|| da.beneficiary.as_bytes().cmp(db.beneficiary.as_bytes()))
            .then_with(|| da.external_ref.as_bytes().cmp(db.external_ref.as_bytes()))
            .then_with(|| event_hash(a).0.as_bytes().cmp(event_hash(b).0.as_bytes())),
        (Event::Transfer(ta), Event::Transfer(tb)) => Ordering::Equal
            .then_with(|| ta.from_key.as_bytes().cmp(tb.from_key.as_bytes()))
            .then_with(|| ta.nonce.cmp(&tb.nonce))
            .then_with(|| event_hash(a).0.as_bytes().cmp(event_hash(b).0.as_bytes())),
        (Event::Deposit(_), Event::Transfer(_)) => Ordering::Less,
        (Event::Transfer(_), Event::Deposit(_)) => Ordering::Greater,
    }
}

pub fn replay_events(store: &ObjectStore, program_manifest: &ProgramManifest, genesis: &LedgerState, events: &[Event]) -> Result<(LedgerState, Vec<Receipt>), FdError> {
    let canonical = canonical_event_set(events);
    let mut state = genesis.clone();
    let mut receipts = Vec::with_capacity(canonical.len());
    for event in canonical {
        let result = apply_event(store, program_manifest, &state, &event)?;
        state = result.state;
        receipts.push(result.receipt);
    }
    Ok((state, receipts))
}
