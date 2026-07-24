#![no_std]

use soroban_sdk::{contract, contractimpl, contracterror, token, Address, Env};

use shared::{emit, AID_CLAIMED, AID_CREATED, AID_REFUNDED, AID_SETTLED};
use shared::storage::{is_paused, set_paused as shared_set_paused};

pub mod storage;
pub mod types;

use storage::{get_aid, get_aid_counter, has_aid, set_aid, set_aid_counter};

// Re-export so test modules (and `use super::*`) have access.
pub use types::{AidRecord, AidStatus};

// ---------------------------------------------------------------------------
// Contract-specific error codes  (range 100-199 per shared/README.md)
// ---------------------------------------------------------------------------

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum AidError {
    /// Caller is not the authorised recipient.
    Unauthorized = 100,
    /// The requested aid record was not found.
    NotFound = 101,
    /// The aid has already been settled or refunded.
    AlreadyClaimed = 102,
    /// The claim window has expired (past `expiry_ledger`).
    Expired = 103,
    /// The contract is paused; no state-changing operations are allowed.
    Paused = 104,
}

// ---------------------------------------------------------------------------
// Contract
// ---------------------------------------------------------------------------

#[contract]
pub struct AidContract;

#[contractimpl]
impl AidContract {
    // -----------------------------------------------------------------------
    // Lifecycle
    // -----------------------------------------------------------------------

    /// Initialise the contract, storing the admin address.
    ///
    /// Must be called exactly once immediately after deployment.
    pub fn initialize(env: Env, admin: Address) {
        shared::auth::set_admin(&env, &admin);
    }

    // -----------------------------------------------------------------------
    // Aid creation
    // -----------------------------------------------------------------------

    /// Create a new aid disbursement and escrow funds from the donor.
    ///
    /// Transfers `amount` of `token` from `donor` to this contract for
    /// safekeeping until the recipient claims or the aid expires.
    ///
    /// Returns the newly allocated `aid_id`.
    pub fn create_aid(
        env: Env,
        aid_id: u64,
        donor: Address,
        recipient: Address,
        token: Address,
        amount: i128,
        expiry_ledger: u32,
    ) -> u64 {
        donor.require_auth();

        if amount <= 0 {
            env.panic_with_error(shared::Error::InvalidAmount);
        }
        if expiry_ledger <= env.ledger().sequence() {
            env.panic_with_error(shared::Error::InvalidArgument);
        }
        if has_aid(&env, aid_id) {
            env.panic_with_error(shared::Error::InvalidArgument);
        }

        // Escrow funds from donor into contract.
        token::Client::new(&env, &token).transfer(
            &donor,
            &env.current_contract_address(),
            &amount,
        );

        let record = AidRecord {
            id: aid_id,
            donor: donor.clone(),
            recipient: recipient.clone(),
            token: token.clone(),
            amount,
            expiry_ledger,
            status: AidStatus::Pending,
        };
        set_aid(&env, aid_id, &record);

        // Track the highest used id so auto-increment helpers work.
        let counter = get_aid_counter(&env);
        if aid_id > counter {
            set_aid_counter(&env, aid_id);
        }

        emit(&env, AID_CREATED, (aid_id, donor, recipient, amount, expiry_ledger));
        aid_id
    }

    // -----------------------------------------------------------------------
    // Aid claiming
    // -----------------------------------------------------------------------

    /// Claim a pending aid disbursement and transfer funds to the recipient.
    ///
    /// # Errors (via `env.panic_with_error`)
    /// - [`AidError::Paused`]         — contract is paused.
    /// - [`AidError::NotFound`]       — `aid_id` does not exist.
    /// - [`AidError::Expired`]        — `expiry_ledger` has passed.
    /// - [`AidError::AlreadyClaimed`] — status is not `Pending`.
    /// - [`AidError::Unauthorized`]   — `caller` is not the recipient.
    pub fn claim_aid(env: Env, aid_id: u64, caller: Address) -> Result<(), AidError> {
        if is_paused(&env) {
            return Err(AidError::Paused);
        }
        caller.require_auth();

        let mut record = get_aid(&env, aid_id).ok_or(AidError::NotFound)?;

        if env.ledger().sequence() > record.expiry_ledger {
            return Err(AidError::Expired);
        }
        if record.status != AidStatus::Pending {
            return Err(AidError::AlreadyClaimed);
        }
        if caller != record.recipient {
            return Err(AidError::Unauthorized);
        }

        // Checks-effects-interactions: write status first, then transfer.
        record.status = AidStatus::Settled;
        set_aid(&env, aid_id, &record);

        token::Client::new(&env, &record.token).transfer(
            &env.current_contract_address(),
            &record.recipient,
            &record.amount,
        );

        emit(&env, AID_CLAIMED, aid_id);
        emit(&env, AID_SETTLED, aid_id);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Refunds
    // -----------------------------------------------------------------------

    /// Refund an expired, unclaimed aid disbursement to the original donor.
    ///
    /// Anyone may call this after expiry to trigger a refund; it is not
    /// gated to the admin so expired funds cannot be held hostage.
    ///
    /// # Errors (via `env.panic_with_error`)
    /// - [`AidError::NotFound`]       — `aid_id` does not exist.
    /// - [`AidError::AlreadyClaimed`] — already settled or refunded.
    /// - [`shared::Error::InvalidArgument`] — expiry has not yet passed.
    pub fn refund_expired(env: Env, aid_id: u64) -> Result<(), AidError> {
        let mut record = get_aid(&env, aid_id).ok_or(AidError::NotFound)?;

        if record.status != AidStatus::Pending {
            return Err(AidError::AlreadyClaimed);
        }
        if env.ledger().sequence() <= record.expiry_ledger {
            env.panic_with_error(shared::Error::InvalidArgument);
        }

        // Checks-effects-interactions.
        record.status = AidStatus::Refunded;
        set_aid(&env, aid_id, &record);

        token::Client::new(&env, &record.token).transfer(
            &env.current_contract_address(),
            &record.donor,
            &record.amount,
        );

        emit(&env, AID_REFUNDED, aid_id);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Queries
    // -----------------------------------------------------------------------

    /// Return the aid record for `aid_id`, or `None` if it does not exist.
    pub fn get_aid(env: Env, aid_id: u64) -> Option<AidRecord> {
        storage::get_aid(&env, aid_id)
    }

    // -----------------------------------------------------------------------
    // Admin controls
    // -----------------------------------------------------------------------

    /// Pause or resume the contract.  Admin only.
    pub fn set_paused(env: Env, caller: Address, paused: bool) {
        shared::auth::require_admin(&env, &caller).expect("unauthorized");
        shared_set_paused(&env, paused);
    }
}

#[cfg(test)]
mod tests;
