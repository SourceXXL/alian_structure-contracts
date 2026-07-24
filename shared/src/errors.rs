use soroban_sdk::contracterror;

/// Shared error codes used across all contracts.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// Caller is not authorised to perform this action.
    Unauthorized = 1,
    /// The caller is not authorised for the requested action.
    NotAuthorized = 9,
    /// The requested item was not found in storage.
    NotFound = 2,
    /// The supplied argument is invalid.
    InvalidArgument = 3,
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
    /// The operation requires the contract to be paused.
    NotPaused = 9,
}
