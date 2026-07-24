#![no_std]

pub mod auth;
pub mod errors;
pub mod events;
pub mod math;
pub mod storage;
pub mod utils;

// Re-export the most commonly-needed items at crate root for ergonomic use.
pub use auth::{get_admin, require_admin, require_not_paused, set_admin};
pub use errors::Error;
pub use events::{
    emit, AID_CLAIMED, AID_CREATED, AID_REFUNDED, AID_SETTLED, COMMISSION_PAID,
    CONTRACT_PAUSED, CONTRACT_RESUMED, CONTRACT_UPGRADED, PARAMETER_CHANGED,
    REFERRAL_ACCRUED, REFERRER_SET, TIER_CONFIG_SET, TREASURY_DEPOSIT,
    TREASURY_SET, TREASURY_WITHDRAW,
};
pub use storage::{
    instance_get, instance_has, instance_remove, instance_set,
    is_paused, persistent_extend_ttl, persistent_get, persistent_has,
    persistent_remove, persistent_set, set_paused, temporary_get,
    temporary_has, temporary_remove, temporary_set,
    PERSISTENT_BUMP_AMOUNT, PERSISTENT_TTL_THRESHOLD,
    TEMPORARY_BUMP_AMOUNT, TEMPORARY_TTL_THRESHOLD,
};
pub use utils::{is_expired, now};

#[cfg(test)]
mod test_auth;
#[cfg(test)]
mod test_storage;
