use crate::{canonical_json_bytes, sha256_tagged, Event, LedgerState, Receipt};
use serde::Serialize;
use sha2::{Digest, Sha256};

pub fn build_receipt<P: Serialize>(program_manifest: &P, event: &Event, pre_state: &LedgerState, post_state: &LedgerState, trace_bytes: &[u8]) -> Receipt {
    let program_hash = sha256_tagged(&canonical_json_bytes(program_manifest).expect("program manifest must serialize"));
    let input_hash = crate::engine::event_hash(event).0;
    let pre_state_hash = sha256_tagged(&canonical_json_bytes(pre_state).expect("pre state must serialize")).0;
    let post_state_hash = sha256_tagged(&canonical_json_bytes(post_state).expect("post state must serialize")).0;

    let mut trace_hasher = Sha256::new();
    trace_hasher.update(trace_bytes);
    let trace_hash = format!("sha256:{}", hex::encode(trace_hasher.finalize()));
    let run_id = trace_hash.clone();

    Receipt {
        run_id,
        program_hash: program_hash.0,
        input_hash,
        pre_state_hash,
        post_state_hash,
        trace_hash,
    }
}
