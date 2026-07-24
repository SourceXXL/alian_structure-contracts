use soroban_sdk::{contracttype, Address};

/// Lifecycle state of an aid record.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AidStatus {
    /// Escrowed, waiting for a valid claim.
    Pending,
    /// Claimed by the recipient; funds transferred out.
    Settled,
    /// Expired and refunded back to the donor (never claimed).
    Refunded,
}

/// A single aid disbursement record, escrowed inside this contract until
/// claimed by the recipient (or refunded to the donor after expiry).
#[contracttype]
#[derive(Clone, Debug)]
pub struct AidRecord {
    pub id: u64,
    pub donor: Address,
    pub recipient: Address,
    pub token: Address,
    pub amount: i128,
    /// Ledger sequence after which the aid can no longer be claimed.
    pub expiry_ledger: u32,
    pub status: AidStatus,
}