#![no_std]

pub mod storage;
pub mod types;

#[cfg(test)]
mod tests;

use shared::events::{
    emit_module_initialized, emit_oracle_rejected, emit_oracle_submitted, emit_oracle_verified,
};
use soroban_sdk::{
    contract, contracterror, contractimpl, symbol_short, Address, Bytes, BytesN, Env, Symbol, Vec,
};

use storage::{
    append_subject_index, get_subject_index, get_verification_counter, get_verification_no_bump,
    next_verification_id, set_verification,
};
use types::{VerificationPage, VerificationRecord, VerificationStatus, MAX_METADATA_URI_LEN};

/// Hard cap on records per listing page (mirrors aid's `MAX_QUERY_LIMIT`;
/// see FEATURE_ISSUES #7 for the gas-DoS rationale).
pub const MAX_PAGE_SIZE: u32 = 50;

// ---------------------------------------------------------------------------
// Contract-specific error codes (range 500-599 per shared conventions)
// ---------------------------------------------------------------------------

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum OracleError {
    /// The requested verification record does not exist.
    VerificationNotFound = 500,
    /// A decision (verify/reject) has already been recorded for this verification.
    AlreadyDecided = 501,
    /// The caller is not authorised to decide verifications.
    NotVerifier = 502,
    /// The submitted evidence is invalid (empty or oversized metadata URI),
    /// or a query cursor/limit argument is out of range.
    InvalidEvidence = 503,
    /// The operation is not permitted while the contract is paused.
    Paused = 507,
    /// The attestation has expired and must be re-verified.
    Expired = 504,
    /// The attestation was verified and later revoked.
    Revoked = 505,
    /// The contract has already been initialized.
    AlreadyInitialized = 506,
}

#[contract]
pub struct OracleContract;

#[contractimpl]
impl OracleContract {
    // -----------------------------------------------------------------------
    // Lifecycle
    // -----------------------------------------------------------------------

    /// Initialise the contract, setting the admin address.
    ///
    /// The admin acts as the verifier gate for this issue; the dedicated
    /// verifier allowlist (issue #2) takes over `decide_verification`
    /// authorization once it lands.
    ///
    /// # Errors
    /// * [`OracleError::AlreadyInitialized`] — called more than once.
    pub fn initialize(env: Env, admin: Address) -> Result<(), OracleError> {
        if env.storage().instance().has(&symbol_short!("admin")) {
            return Err(OracleError::AlreadyInitialized);
        }
        shared::auth::set_admin(&env, &admin);
        emit_module_initialized(
            &env,
            symbol_short!("oracle"),
            1,
            &admin,
            env.ledger().timestamp(),
        );
        Ok(())
    }

    /// Get the admin address.
    pub fn get_admin(env: Env) -> Address {
        shared::auth::get_admin(&env)
    }

    // -----------------------------------------------------------------------
    // Submissions (permissionless)
    // -----------------------------------------------------------------------

    /// Submit a verification for `subject`. Permissionless — anyone may file
    /// evidence; the record starts `Pending` until a verifier decides.
    ///
    /// Returns the allocated verification ID (1-based, sequential).
    ///
    /// # Errors
    /// * [`OracleError::InvalidEvidence`] — `metadata_uri` is empty or exceeds
    ///   [`types::MAX_METADATA_URI_LEN`] bytes.
    /// * [`shared::Error::ContractPaused`] — the contract is paused.
    pub fn submit_verification(
        env: Env,
        subject: Address,
        kind: Symbol,
        evidence_hash: BytesN<32>,
        metadata_uri: Bytes,
    ) -> Result<u32, OracleError> {
        if shared::storage::is_paused(&env) {
            return Err(OracleError::Paused);
        }

        if metadata_uri.is_empty() || metadata_uri.len() > MAX_METADATA_URI_LEN {
            return Err(OracleError::InvalidEvidence);
        }

        let id = next_verification_id(&env);
        let now = env.ledger().timestamp();

        let record = VerificationRecord {
            id,
            subject: subject.clone(),
            kind: kind.clone(),
            status: VerificationStatus::Pending,
            evidence_hash: evidence_hash.clone(),
            metadata_uri: metadata_uri.clone(),
            verifier: None,
            submitted_at: now,
            decided_at: None,
            expires_at: None,
        };
        set_verification(&env, id, &record);
        append_subject_index(&env, &subject, id);

        emit_oracle_submitted(&env, id, &subject, &kind, &evidence_hash, now);

        Ok(id)
    }

    // -----------------------------------------------------------------------
    // Decisions (admin-gated until the verifier allowlist lands in issue #2)
    // -----------------------------------------------------------------------

    /// Verify or reject a pending verification.
    ///
    /// Admin-gated for this issue: `caller` must be the current admin with a
    /// valid on-chain signature. Issue #2 switches this gate to the verifier
    /// allowlist with signed attestations.
    ///
    /// # Errors
    /// * [`OracleError::NotVerifier`] — `caller` is not the admin.
    /// * [`OracleError::VerificationNotFound`] — no record with `id`.
    /// * [`OracleError::AlreadyDecided`] — record is not `Pending`.
    /// * [`OracleError::Paused`] — the contract is paused.
    pub fn decide_verification(
        env: Env,
        caller: Address,
        id: u32,
        approved: bool,
        expires_at: Option<u64>,
    ) -> Result<(), OracleError> {
        if shared::storage::is_paused(&env) {
            return Err(OracleError::Paused);
        }

        let admin = shared::auth::get_admin(&env);
        if caller != admin {
            return Err(OracleError::NotVerifier);
        }
        caller.require_auth();

        // No TTL bump here: the write path below already extends the entry.
        let mut record =
            get_verification_no_bump(&env, id).ok_or(OracleError::VerificationNotFound)?;

        if record.status != VerificationStatus::Pending {
            return Err(OracleError::AlreadyDecided);
        }

        let decided_at = env.ledger().timestamp();
        record.status = if approved {
            VerificationStatus::Verified
        } else {
            VerificationStatus::Rejected
        };
        record.verifier = Some(caller.clone());
        record.decided_at = Some(decided_at);
        record.expires_at = expires_at;
        set_verification(&env, id, &record);

        if approved {
            emit_oracle_verified(
                &env,
                id,
                &record.subject,
                &record.kind,
                &record.evidence_hash,
                &caller,
                decided_at,
                record.expires_at,
            );
        } else {
            emit_oracle_rejected(
                &env,
                id,
                &record.subject,
                &record.kind,
                &record.evidence_hash,
                &caller,
                decided_at,
            );
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Queries (read-only)
    // -----------------------------------------------------------------------

    /// Get a verification record by ID, or `None` when unknown.
    ///
    /// Read-only: this path does not extend the record's TTL.
    pub fn get_verification(env: Env, id: u32) -> Option<VerificationRecord> {
        get_verification_no_bump(&env, id)
    }

    /// List verifications for `subject` with cursor pagination.
    ///
    /// `cursor` is the number of index entries already returned (0 to start).
    /// `limit` is hard-capped at [`MAX_PAGE_SIZE`]; pages are never empty when
    /// more entries remain. Only records that still exist in storage are
    /// returned (the subject index is append-only; purged records are skipped).
    ///
    /// # Errors
    /// * [`OracleError::InvalidEvidence`] — `cursor` is beyond the end of the
    ///   subject's index.
    pub fn list_verifications_by_subject(
        env: Env,
        subject: Address,
        cursor: u32,
        limit: u32,
    ) -> Result<VerificationPage, OracleError> {
        let ids = get_subject_index(&env, &subject);
        let total = ids.len();
        if cursor > total {
            return Err(OracleError::InvalidEvidence);
        }

        let capped = if limit > MAX_PAGE_SIZE || limit == 0 {
            MAX_PAGE_SIZE
        } else {
            limit
        };
        let end = cursor.saturating_add(capped).min(total);

        let mut records = Vec::new(&env);
        for i in cursor..end {
            if let Some(record) = get_verification_no_bump(&env, ids.get(i).unwrap_or(0)) {
                records.push_back(record);
            }
        }

        Ok(VerificationPage { records, total })
    }

    /// Total number of verifications ever submitted.
    pub fn verification_count(env: Env) -> u32 {
        get_verification_counter(&env)
    }
}
