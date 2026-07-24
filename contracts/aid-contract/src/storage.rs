//! Aid-contract storage layer.
//!
//! All persistent reads and writes are routed through the shared storage
//! helpers so that TTL bumps are applied consistently on every access.

use soroban_sdk::{contracttype, Env};

use shared::storage::{
    instance_get, instance_set, persistent_get, persistent_has, persistent_remove, persistent_set,
};

use crate::types::AidRecord;

// ---------------------------------------------------------------------------
// Storage keys
// ---------------------------------------------------------------------------

/// Keys used by the aid contract.
///
/// | Key              | Storage type | Rationale                                       |
/// |------------------|--------------|------------------------------------------------|
/// | `Aid(id)`        | persistent   | Long-lived financial record; must outlive many  |
/// |                  |              | ledger closures until claimed/refunded.         |
/// | `AidCounter`     | instance     | Config-level counter; always needed when live.  |
/// | `Paused`         | instance     | Core config; must be readable while contract    |
/// |                  |              | instance exists (use shared `is_paused`).       |
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Persistent record for a single aid disbursement.
    Aid(u64),
    /// Monotonically-increasing aid identifier counter (instance).
    AidCounter,
}

// ---------------------------------------------------------------------------
// Aid record helpers — persistent storage with automatic TTL extension
// ---------------------------------------------------------------------------

/// Read an aid record from persistent storage.
///
/// Returns `None` when the ID does not exist.  Extends the entry TTL on every
/// hit so frequently-accessed records are never evicted while active.
pub fn get_aid(env: &Env, aid_id: u64) -> Option<AidRecord> {
    persistent_get(env, &DataKey::Aid(aid_id))
}

/// Write (or overwrite) an aid record to persistent storage.
///
/// TTL is extended immediately so the entry survives upcoming ledger closures.
pub fn set_aid(env: &Env, aid_id: u64, record: &AidRecord) {
    persistent_set(env, &DataKey::Aid(aid_id), record);
}

/// Returns `true` when an aid record with the given ID exists.
///
/// Does **not** extend TTL — call [`get_aid`] when you need the value.
pub fn has_aid(env: &Env, aid_id: u64) -> bool {
    persistent_has(env, &DataKey::Aid(aid_id))
}

/// Remove an aid record from persistent storage (e.g. after full settlement).
pub fn remove_aid(env: &Env, aid_id: u64) {
    persistent_remove(env, &DataKey::Aid(aid_id));
}

// ---------------------------------------------------------------------------
// Aid counter — instance storage
// ---------------------------------------------------------------------------

/// Read the current aid counter, defaulting to 0 if never set.
pub fn get_aid_counter(env: &Env) -> u64 {
    instance_get(env, &DataKey::AidCounter).unwrap_or(0)
}

/// Write the aid counter to instance storage.
pub fn set_aid_counter(env: &Env, counter: u64) {
    instance_set(env, &DataKey::AidCounter, &counter);
}
