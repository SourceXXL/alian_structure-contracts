use soroban_sdk::{contracttype, Env};

use crate::types::AidRecord;

/// TTL constants (in ledgers). Tune to your network's expected usage pattern.
const AID_BUMP_AMOUNT: u32 = 120_960; // ~7 days at 5s ledgers
const AID_LIFETIME_THRESHOLD: u32 = AID_BUMP_AMOUNT - 17_280; // bump when <1 day left

#[contracttype]
pub enum DataKey {
    Aid(u64),
    Paused,
}

pub fn get_aid(env: &Env, aid_id: u64) -> Option<AidRecord> {
    let key = DataKey::Aid(aid_id);
    let record = env.storage().persistent().get::<_, AidRecord>(&key);
    if record.is_some() {
        env.storage().persistent().extend_ttl(
            &key,
            AID_LIFETIME_THRESHOLD,
            AID_BUMP_AMOUNT,
        );
    }
    record
}

pub fn set_aid(env: &Env, aid_id: u64, record: &AidRecord) {
    let key = DataKey::Aid(aid_id);
    env.storage().persistent().set(&key, record);
    env.storage()
        .persistent()
        .extend_ttl(&key, AID_LIFETIME_THRESHOLD, AID_BUMP_AMOUNT);
}

pub fn has_aid(env: &Env, aid_id: u64) -> bool {
    env.storage().persistent().has(&DataKey::Aid(aid_id))
}

pub fn is_paused(env: &Env) -> bool {
    env.storage()
        .instance()
        .get(&DataKey::Paused)
        .unwrap_or(false)
}

pub fn set_paused(env: &Env, paused: bool) {
    env.storage().instance().set(&DataKey::Paused, &paused);
}