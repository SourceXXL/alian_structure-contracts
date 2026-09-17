//! Oracle-contract storage layer.
//!
//! All persistent reads and writes are routed through the shared storage
//! helpers so that TTL bumps are applied consistently on every access.
//!
//! ## Privacy
//!
//! The contract never stores personal data. Evidence is referenced by hash
//! (`evidence_hash`) and off-chain systems resolve `metadata_uri`; neither
//! field is used to derive or store PII.
//!
//! ## Key layout
//!
//! | Key                  | Storage type | Rationale                                       |
//! |----------------------|--------------|-------------------------------------------------|
//! | `Verification(id)`   | persistent   | Long-lived verification record; must survive    |
//! |                      |              | many ledger closures until consumed/expired.    |
//! | `VerificationIndex(subject)` | persistent | Append-only record-ID list per subject.  |
//! | `VerificationCount`  | instance     | Config-level ID counter; always needed when live.|

use soroban_sdk::{contracttype, Address, Env, Vec};

use shared::storage::{
    instance_get, instance_set, persistent_get, persistent_read, persistent_set,
};

use crate::types::VerificationRecord;

// ---------------------------------------------------------------------------
// Storage keys
// ---------------------------------------------------------------------------

/// Keys used by the oracle contract.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Persistent record for a single verification.
    Verification(u32),
    /// Append-only list of verification IDs per subject (persistent).
    VerificationIndex(Address),
    /// Monotonically-increasing verification ID counter (instance).
    VerificationCount,
}

// ---------------------------------------------------------------------------
// Verification records — persistent storage with automatic TTL extension
// ---------------------------------------------------------------------------

/// Read a verification record from persistent storage, extending its TTL.
///
/// Returns `None` when the ID does not exist. Use this when the caller will
/// keep the record alive (hot reads); the write paths already bump TTL.
#[inline]
pub fn get_verification_with_bump(env: &Env, id: u32) -> Option<VerificationRecord> {
    persistent_get(env, &DataKey::Verification(id))
}

/// Read a verification record without extending its TTL.
///
/// Gas-cheapest read path — use for getters and pagination loops.
#[inline]
pub fn get_verification_no_bump(env: &Env, id: u32) -> Option<VerificationRecord> {
    persistent_read(env, &DataKey::Verification(id))
}

/// Write (or overwrite) a verification record to persistent storage.
///
/// TTL is extended immediately so the entry survives upcoming ledger closures.
#[inline]
pub fn set_verification(env: &Env, id: u32, record: &VerificationRecord) {
    persistent_set(env, &DataKey::Verification(id), record);
}

/// Returns `true` when a verification record with the given ID exists.
#[inline]
pub fn has_verification(env: &Env, id: u32) -> bool {
    get_verification_no_bump(env, id).is_some()
}

// ---------------------------------------------------------------------------
// Subject index — append-only ID lists for pagination
// ---------------------------------------------------------------------------

/// Read the full list of verification IDs submitted for `subject`.
///
/// The list is append-only: entries are added on `submit_verification` and
/// never removed, so decision/revocation transitions only mutate the
/// underlying records, keeping every index entry valid.
pub fn get_subject_index(env: &Env, subject: &Address) -> Vec<u32> {
    persistent_get(env, &DataKey::VerificationIndex(subject.clone()))
        .unwrap_or_else(|| Vec::new(env))
}

/// Append `id` to `subject`'s index.
pub fn append_subject_index(env: &Env, subject: &Address, id: u32) {
    let mut ids = get_subject_index(env, subject);
    ids.push_back(id);
    persistent_set(env, &DataKey::VerificationIndex(subject.clone()), &ids);
}

// ---------------------------------------------------------------------------
// Verification counter — instance storage
// ---------------------------------------------------------------------------

/// Read the current verification counter, defaulting to 0 if never set.
#[inline]
pub fn get_verification_counter(env: &Env) -> u32 {
    instance_get(env, &DataKey::VerificationCount).unwrap_or(0)
}

/// Write the verification counter to instance storage.
#[inline]
pub fn set_verification_counter(env: &Env, counter: u32) {
    instance_set(env, &DataKey::VerificationCount, &counter);
}

/// Allocate the next verification ID (1-based).
///
/// # Panics
/// Panics when the ID space is exhausted (u32::MAX allocations).
#[inline]
pub fn next_verification_id(env: &Env) -> u32 {
    let next = get_verification_counter(env)
        .checked_add(1)
        .expect("verification id overflow");
    set_verification_counter(env, next);
    next
}
