//! Oracle-contract types.
//!
//! ## Privacy
//!
//! [`VerificationRecord`] carries **no personal data**: evidence is referenced
//! by SHA-256 `evidence_hash` and off-chain systems resolve `metadata_uri`
//! (IPFS/HTTPS pointer). Nothing in the record is used to derive or store PII.

use soroban_sdk::{contracttype, Address, Bytes, BytesN, Symbol};

/// Status of a verification record.
#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum VerificationStatus {
    /// Submitted, awaiting a verifier decision.
    Pending = 0,
    /// Approved by a verifier.
    Verified = 1,
    /// Rejected by a verifier.
    Rejected = 2,
    /// Was `Verified` and later revoked.
    Revoked = 3,
}

/// Maximum accepted length of `metadata_uri` in bytes.
///
/// Keeps the worst-case record well within Soroban's 128 KiB ledger-entry
/// budget; URIs longer than this must be moved off-chain behind the evidence
/// hash.
pub const MAX_METADATA_URI_LEN: u32 = 256;

/// A single verification record.
///
/// Off-chain systems resolve `metadata_uri` to the evidence bundle; the hash
/// pins its integrity. `expires_at` is advisory here — consumers enforce it
/// (see FEATURE_ISSUES #3 for on-chain expiry/revocation).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationRecord {
    /// Unique record identifier (1-based, sequential).
    pub id: u32,
    /// Verified wallet/recipient address.
    pub subject: Address,
    /// Verification kind, e.g. `"identity"`, `"need"`.
    pub kind: Symbol,
    /// Current lifecycle status.
    pub status: VerificationStatus,
    /// SHA-256 of the off-chain evidence bundle.
    pub evidence_hash: BytesN<32>,
    /// IPFS/HTTPS pointer to the evidence bundle (no PII).
    pub metadata_uri: Bytes,
    /// Verifier that decided the record; `None` while `Pending`.
    pub verifier: Option<Address>,
    /// Ledger timestamp of submission.
    pub submitted_at: u64,
    /// Ledger timestamp of the decision; `None` while `Pending`.
    pub decided_at: Option<u64>,
    /// Optional expiry timestamp; enforced by consumers (see issue #3).
    pub expires_at: Option<u64>,
}

/// One page of cursor-paginated verification results.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationPage {
    /// Records on this page (length ≤ `MAX_PAGE_SIZE`).
    pub records: soroban_sdk::Vec<VerificationRecord>,
    /// Total number of records in the subject's index.
    pub total: u32,
}
