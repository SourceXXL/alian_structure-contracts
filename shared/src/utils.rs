use soroban_sdk::Env;

/// Returns the current ledger timestamp.
pub fn now(env: &Env) -> u64 {
    env.ledger().timestamp()
}

/// Returns `true` if the given timestamp is in the past.
pub fn is_expired(env: &Env, expires_at: u64) -> bool {
    now(env) > expires_at
}
