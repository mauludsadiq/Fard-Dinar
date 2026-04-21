use crate::{canonical_event_set, canonical_json_bytes, replay_events, sha256_tagged, Event, FdError, LedgerState, ObjectStore, ProgramManifest, Receipt};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayVerification {
    pub final_state_hash: String,
    pub supply: u64,
    pub event_count: usize,
    pub ok: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionVerification {
    pub ok: bool,
    pub post_state_hash: String,
    pub run_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsistencyVerification {
    pub ok: bool,
    pub canonical_event_count: usize,
}

pub fn verify_transition(store: &ObjectStore, program_manifest: &ProgramManifest, pre_state: &LedgerState, event: &Event) -> Result<TransitionVerification, FdError> {
    let result = crate::apply_event(store, program_manifest, pre_state, event)?;
    Ok(TransitionVerification {
        ok: true,
        post_state_hash: result.receipt.post_state_hash,
        run_id: result.receipt.run_id,
    })
}

pub fn verify_replay(store: &ObjectStore, program_manifest: &ProgramManifest, genesis: &LedgerState, events: &[Event]) -> Result<ReplayVerification, FdError> {
    let (final_state, receipts): (LedgerState, Vec<Receipt>) = replay_events(store, program_manifest, genesis, events)?;
    let final_state_hash = sha256_tagged(&canonical_json_bytes(&final_state).map_err(|e| FdError::InvalidJson(e.to_string()))?).0;
    let supply = final_state.accounts.values().map(|a| a.balance).sum();
    Ok(ReplayVerification {
        final_state_hash,
        supply,
        event_count: receipts.len(),
        ok: true,
    })
}

pub fn verify_consistency(events: &[Event]) -> Result<ConsistencyVerification, FdError> {
    let canonical = canonical_event_set(events);
    Ok(ConsistencyVerification {
        ok: true,
        canonical_event_count: canonical.len(),
    })
}
