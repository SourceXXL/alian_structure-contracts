//! Shared storage helpers for Alian Structure Soroban contracts.
//!
//! ## Storage-type guidance
//!
//! | Data category                     | Storage type  | Rationale                                                                 |
//! |-----------------------------------|---------------|---------------------------------------------------------------------------|
//! | Contract config (admin, token…)   | `instance`    | Tied to contract instance lifetime; evicted only when the contract itself is evicted. |
//! | Pause flag                        | `instance`    | Config-level; must always be readable if the contract exists.             |
//! | Aid records                       | `persistent`  | Long-lived financial records; must survive across many ledger closures.   |
//! | Referral edges & accrued balances | `persistent`  | Per-address data that outlives a single transaction.                      |
//! | Role assignments                  | `persistent`  | Role grants must persist until explicitly revoked.                        |
//! | One-time claim nonces             | `temporary`   | Only needed until the claim window closes; can expire naturally.          |
//! | Oracle verification proofs        | `temporary`   | Short-lived attestation; contract does not need to keep them forever.     |
//!
//! ## TTL constants
//!
//! All `extend_ttl` calls use the two constants below unless overridden.
//! Tune them to match your expected ledger cadence.
//!
//! | Constant                   | Value (ledgers) | ~Wall-clock (5 s/ledger) |
//! |----------------------------|-----------------|--------------------------|
//! | `PERSISTENT_BUMP_AMOUNT`   | 120 960         | 7 days                   |
//! | `PERSISTENT_TTL_THRESHOLD` | 103 680         | 6 days (bump when < 1 d left) |
//! | `TEMPORARY_BUMP_AMOUNT`    | 17 280          | 1 day                    |
//! | `TEMPORARY_TTL_THRESHOLD`  | 8 640           | 12 hours                 |

use soroban_sdk::{symbol_short, Env, IntoVal, Symbol, TryFromVal, Val};

// ---------------------------------------------------------------------------
// TTL constants
// ---------------------------------------------------------------------------

/// Persistent entries are bumped to this lifetime on every read / write.
pub const PERSISTENT_BUMP_AMOUNT: u32 = 120_960; // ~7 days at 5 s/ledger
/// Only bump a persistent entry when its remaining TTL drops below this.
pub const PERSISTENT_TTL_THRESHOLD: u32 = PERSISTENT_BUMP_AMOUNT - 17_280; // bump when < 1 day left

/// Temporary entries are bumped to this lifetime on every read / write.
pub const TEMPORARY_BUMP_AMOUNT: u32 = 17_280; // ~1 day at 5 s/ledger
/// Only bump a temporary entry when its remaining TTL drops below this.
pub const TEMPORARY_TTL_THRESHOLD: u32 = TEMPORARY_BUMP_AMOUNT / 2; // bump when < 12 hours left

// ---------------------------------------------------------------------------
// Instance storage — contract-scoped configuration
// ---------------------------------------------------------------------------

/// Read a value from **instance** storage.
///
/// Returns `None` when the key is absent.  Instance storage is evicted only
/// when the contract instance itself is evicted, so no TTL bumping is needed.
pub fn instance_get<K, V>(env: &Env, key: &K) -> Option<V>
where
    K: IntoVal<Env, Val>,
    V: TryFromVal<Env, Val>,
{
    env.storage().instance().get(key)
}

/// Write a value to **instance** storage.
pub fn instance_set<K, V>(env: &Env, key: &K, value: &V)
where
    K: IntoVal<Env, Val>,
    V: IntoVal<Env, Val>,
{
    env.storage().instance().set(key, value);
}

/// Returns `true` when the key exists in **instance** storage.
pub fn instance_has<K>(env: &Env, key: &K) -> bool
where
    K: IntoVal<Env, Val>,
{
    env.storage().instance().has(key)
}

/// Remove a key from **instance** storage.
pub fn instance_remove<K>(env: &Env, key: &K)
where
    K: IntoVal<Env, Val>,
{
    env.storage().instance().remove(key);
}

// ---------------------------------------------------------------------------
// Persistent storage — long-lived per-record data
// ---------------------------------------------------------------------------

/// Read a value from **persistent** storage and extend its TTL.
///
/// The TTL is bumped on every read so that frequently-accessed records stay
/// alive.  Returns `None` when the key is absent.
pub fn persistent_get<K, V>(env: &Env, key: &K) -> Option<V>
where
    K: IntoVal<Env, Val>,
    V: TryFromVal<Env, Val>,
{
    let value = env.storage().persistent().get(key);
    if value.is_some() {
        env.storage()
            .persistent()
            .extend_ttl(key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
    }
    value
}

/// Write a value to **persistent** storage and extend its TTL immediately.
pub fn persistent_set<K, V>(env: &Env, key: &K, value: &V)
where
    K: IntoVal<Env, Val>,
    V: IntoVal<Env, Val>,
{
    env.storage().persistent().set(key, value);
    env.storage()
        .persistent()
        .extend_ttl(key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
}

/// Returns `true` when the key exists in **persistent** storage.
///
/// Does **not** extend the TTL — use [`persistent_get`] when you intend to
/// read the value immediately after.
pub fn persistent_has<K>(env: &Env, key: &K) -> bool
where
    K: IntoVal<Env, Val>,
{
    env.storage().persistent().has(key)
}

/// Remove a key from **persistent** storage.
pub fn persistent_remove<K>(env: &Env, key: &K)
where
    K: IntoVal<Env, Val>,
{
    env.storage().persistent().remove(key);
}

/// Explicitly extend the TTL of an existing **persistent** entry.
///
/// Useful when a record is updated in-place by a separate function and you
/// need to separate the TTL bump from the write.
pub fn persistent_extend_ttl<K>(env: &Env, key: &K)
where
    K: IntoVal<Env, Val>,
{
    env.storage()
        .persistent()
        .extend_ttl(key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
}

// ---------------------------------------------------------------------------
// Temporary storage — short-lived data (nonces, proofs)
// ---------------------------------------------------------------------------

/// Read a value from **temporary** storage and bump its TTL.
///
/// Returns `None` when the key is absent or has already expired.
pub fn temporary_get<K, V>(env: &Env, key: &K) -> Option<V>
where
    K: IntoVal<Env, Val>,
    V: TryFromVal<Env, Val>,
{
    let value = env.storage().temporary().get(key);
    if value.is_some() {
        env.storage()
            .temporary()
            .extend_ttl(key, TEMPORARY_TTL_THRESHOLD, TEMPORARY_BUMP_AMOUNT);
    }
    value
}

/// Write a value to **temporary** storage and set its initial TTL.
pub fn temporary_set<K, V>(env: &Env, key: &K, value: &V)
where
    K: IntoVal<Env, Val>,
    V: IntoVal<Env, Val>,
{
    env.storage().temporary().set(key, value);
    env.storage()
        .temporary()
        .extend_ttl(key, TEMPORARY_TTL_THRESHOLD, TEMPORARY_BUMP_AMOUNT);
}

/// Returns `true` when the key exists (and has not expired) in **temporary**
/// storage.
pub fn temporary_has<K>(env: &Env, key: &K) -> bool
where
    K: IntoVal<Env, Val>,
{
    env.storage().temporary().has(key)
}

/// Remove a key from **temporary** storage.
pub fn temporary_remove<K>(env: &Env, key: &K)
where
    K: IntoVal<Env, Val>,
{
    env.storage().temporary().remove(key);
}

// ---------------------------------------------------------------------------
// Well-known instance keys
// ---------------------------------------------------------------------------

/// Key used to store the paused flag.
pub const KEY_PAUSED: Symbol = symbol_short!("paused");

/// Returns `true` when the contract is paused.
///
/// Reads from instance storage — the pause flag is part of core contract
/// configuration and must always be accessible.
pub fn is_paused(env: &Env) -> bool {
    instance_get::<Symbol, bool>(env, &KEY_PAUSED).unwrap_or(false)
}

/// Set (or clear) the paused flag in instance storage.
pub fn set_paused(env: &Env, paused: bool) {
    instance_set(env, &KEY_PAUSED, &paused);
}
