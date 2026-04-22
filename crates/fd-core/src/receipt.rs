use crate::{canonical_json_bytes, sha256_tagged, Event, LedgerState, Receipt, TransferEffects};
use serde::Serialize;

pub fn build_receipt<P: Serialize>(
    program_manifest: &P,
    event: &Event,
    pre_state: &LedgerState,
    post_state: &LedgerState,
    trace_bytes: &[u8],
) -> Receipt {
    let program_hash = sha256_tagged(
        &canonical_json_bytes(program_manifest).expect("program manifest must serialize"),
    );
    let input_hash = crate::engine::event_hash(event).0;
    let pre_state_hash = sha256_tagged(
        &canonical_json_bytes(pre_state).expect("pre state must serialize"),
    ).0;
    let post_state_hash = sha256_tagged(
        &canonical_json_bytes(post_state).expect("post state must serialize"),
    ).0;

    let trace_hash = sha256_tagged(trace_bytes).0;
    let run_id = trace_hash.clone();

    let transfer_effects = match event {
        Event::Transfer(tx) => {
            let pre_from = pre_state.account_or_default(&tx.from_key);
            let post_from = post_state.account_or_default(&tx.from_key);
            let pre_to = pre_state.account_or_default(&tx.to_key);
            let post_to = post_state.account_or_default(&tx.to_key);

            let user_reward = post_from
                .balance
                .saturating_add(tx.amount)
                .saturating_sub(pre_from.balance);
            let recipient_delta = post_to.balance.saturating_sub(pre_to.balance);
            let vendor_reward = recipient_delta.saturating_sub(tx.amount);
            let is_merchant = vendor_reward > 0;

            Some(TransferEffects {
                amount: tx.amount,
                user_reward,
                vendor_reward,
                is_merchant,
            })
        }
        Event::Deposit(_) => None,
    };

    Receipt {
        run_id,
        program_hash: program_hash.0,
        input_hash,
        pre_state_hash,
        post_state_hash,
        trace_hash,
        transfer_effects,
    }
}
