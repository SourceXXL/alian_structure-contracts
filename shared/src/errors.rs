use soroban_sdk::contracterror;

/// Stable error codes shared by every contract in the workspace.
///
/// Codes from 900 to 999 are reserved for errors whose meaning is shared
/// consistently across multiple contract modules.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// Caller is not authorised to perform this action.
    Unauthorized = 1,
    /// The requested item was not found in storage.
    NotFound = 2,
    /// The supplied monetary amount is invalid.
    InvalidAmount = 3,
    /// An arithmetic operation would overflow.
    Overflow = 4,
    /// The operation is not permitted while the contract is paused.
    ContractPaused = 5,
    /// The claim link has expired.
    Expired = 6,
    /// The claim has already been used.
    AlreadyClaimed = 7,
    /// Insufficient treasury balance.
    InsufficientBalance = 8,
    /// Requested withdrawal exceeds the configured per-transaction limit.
    /// NEW: added for the treasury withdrawal feature.
    WithdrawalLimitExceeded = 9,
}