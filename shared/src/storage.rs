use soroban_sdk::{symbol_short, Env, Symbol};

/// Key used to store the paused flag.
pub const KEY_PAUSED: Symbol = symbol_short!("paused");

/// Returns `true` when the contract is paused.
pub fn is_paused(env: &Env) -> bool {
    env.storage()
        .instance()
        .get::<Symbol, bool>(&KEY_PAUSED)
        .unwrap_or(false)
}

/// Pause the contract.
pub fn set_paused(env: &Env, paused: bool) {
    env.storage()
        .instance()
        .set::<Symbol, bool>(&KEY_PAUSED, &paused);
}
