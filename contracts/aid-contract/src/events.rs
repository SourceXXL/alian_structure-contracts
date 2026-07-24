use soroban_sdk::{symbol_short, Address, Env};

/// Emitted when the recipient's claim is validated (before/alongside settlement).
pub fn emit_aid_claimed(env: &Env, aid_id: u64, claimant: &Address) {
    env.events()
        .publish((symbol_short!("aidclaim"), aid_id), claimant.clone());
}

/// Emitted once the record is marked Settled and funds have moved.
pub fn emit_aid_settled(env: &Env, aid_id: u64, recipient: &Address, amount: i128) {
    env.events().publish(
        (symbol_short!("aidsettle"), aid_id),
        (recipient.clone(), amount),
    );
}

/// Emitted when an expired, unclaimed aid record is refunded to the donor.
pub fn emit_aid_refunded(env: &Env, aid_id: u64, donor: &Address, amount: i128) {
    env.events().publish(
        (symbol_short!("aidrefund"), aid_id),
        (donor.clone(), amount),
    );
}