#![no_std]

pub mod auth;
pub mod batch;
pub mod errors;
pub mod event;
pub mod events;
pub mod math;
pub mod payments;
pub mod storage;
pub mod utils;

// Re-export the most commonly-needed items at crate root for ergonomic use.
pub use auth::{get_admin, require_admin, require_not_paused, set_admin};
pub use batch::{
    batch_invoke_no_args, execute_multi_invoke, execute_multi_transfer, multi_transfer_all,
    BatchConfig, BatchError, BatchMode, BatchResult, BatchTransfer, OperationResult,
    ABSOLUTE_MAX_BATCH_SIZE, DEFAULT_MAX_BATCH_SIZE,
};
pub use errors::Error;
pub use events::{
    emit, emit_collection_registered, emit_nft_auction, emit_nft_bid, emit_nft_listed,
    emit_nft_offer, emit_nft_settle, emit_nft_sold, emit_oracle_rejected, emit_oracle_submitted,
    emit_oracle_verified, emit_royalty_paid, AID_CLAIMED, AID_CREATED, AID_REFUNDED, AID_SETTLED,
    COMMISSION_PAID, CONTRACT_PAUSED, CONTRACT_RESUMED, CONTRACT_UPGRADED, PARAMETER_CHANGED,
    PAYMENT_ESCROW_CREATED, PAYMENT_ESCROW_REFUNDED, PAYMENT_ESCROW_RELEASED, PAYMENT_FEE,
    PAYMENT_TRANSFER, REFERRAL_ACCRUED, REFERRAL_REGISTERED, REFERRER_SET, TIER_CONFIG_SET,
    TREASURY_DEPOSIT, TREASURY_EMERGENCY_WITHDRAW, TREASURY_SET, TREASURY_WITHDRAW,
};
pub use payments::{
    calculate_fee, calculate_fee_split, create_escrow, deduct_fee, get_escrow, refund_escrow,
    release_escrow, safe_transfer, safe_transfer_from_contract, EscrowRecord, EscrowState,
    FeeConfig,
};
pub use storage::{
    instance_get, instance_has, instance_remove, instance_set, is_paused, persistent_extend_ttl,
    persistent_get, persistent_has, persistent_remove, persistent_set, set_paused, temporary_get,
    temporary_has, temporary_remove, temporary_set, PERSISTENT_BUMP_AMOUNT,
    PERSISTENT_TTL_THRESHOLD, TEMPORARY_BUMP_AMOUNT, TEMPORARY_TTL_THRESHOLD,
};
pub use utils::{is_expired, now};

#[cfg(test)]
mod test_auth;
#[cfg(test)]
mod test_storage;
