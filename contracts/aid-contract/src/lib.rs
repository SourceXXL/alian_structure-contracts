#![no_std]

mod errors;
mod events;
mod storage;
mod types;

#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, token, Address, Env};

pub use errors::AidError;
pub use types::{AidRecord, AidStatus};

#[contract]
pub struct AidContract;

#[contractimpl]
impl AidContract {
    /// Initialise the contract, setting the admin address.
    pub fn initialize(env: Env, admin: Address) {
        shared::auth::set_admin(&env, &admin);
    }

    /// Admin-only: pause/unpause claims. Existing records are unaffected;
    /// only `claim_aid` is gated on this flag.
    pub fn set_paused(env: Env, admin: Address, paused: bool) -> Result<(), AidError> {
        admin.require_auth();
        shared::auth::require_admin(&env, &admin).map_err(|_| AidError::Unauthorized)?;
        storage::set_paused(&env, paused);
        Ok(())
    }

    /// Create an escrowed aid record. Pulls `amount` of `token` from `donor`
    /// into this contract's balance. Funds sit here until claimed or,
    /// once expired, refunded back to the donor.
    pub fn create_aid(
        env: Env,
        aid_id: u64,
        donor: Address,
        recipient: Address,
        token: Address,
        amount: i128,
        expiry_ledger: u32,
    ) -> Result<(), AidError> {
        donor.require_auth();

        if storage::has_aid(&env, aid_id) {
            // Reuse of an id is treated as already-settled to avoid clobbering
            // an existing record's history.
            return Err(AidError::AlreadyClaimed);
        }

        let token_client = token::Client::new(&env, &token);
        let contract_address = env.current_contract_address();
        token_client.transfer(&donor, &contract_address, &amount);

        let record = AidRecord {
            id: aid_id,
            donor,
            recipient,
            token,
            amount,
            expiry_ledger,
            status: AidStatus::Pending,
        };
        storage::set_aid(&env, aid_id, &record);

        Ok(())
    }

    /// Claim an aid record: verifies the caller is the designated recipient,
    /// that the record hasn't expired, already been claimed, or that claims
    /// aren't paused — then settles it.
    ///
    /// Ordering follows checks-effects-interactions: the record is marked
    /// `Settled` in storage *before* the external token transfer is made,
    /// so a reentrant call (or a failed/duplicate transfer) can never see a
    /// still-`Pending` record to claim twice.
    pub fn claim_aid(env: Env, aid_id: u64, claimant: Address) -> Result<(), AidError> {
        claimant.require_auth();

        // --- Checks ---
        if storage::is_paused(&env) {
            return Err(AidError::Paused);
        }

        let mut record = storage::get_aid(&env, aid_id).ok_or(AidError::NotFound)?;

        if record.status != AidStatus::Pending {
            return Err(AidError::AlreadyClaimed);
        }

        if env.ledger().sequence() > record.expiry_ledger {
            return Err(AidError::Expired);
        }

        if record.recipient != claimant {
            return Err(AidError::Unauthorized);
        }

        // --- Effects (before any external call) ---
        record.status = AidStatus::Settled;
        storage::set_aid(&env, aid_id, &record);

        // --- Interactions ---
        let token_client = token::Client::new(&env, &record.token);
        let contract_address = env.current_contract_address();
        token_client.transfer(&contract_address, &claimant, &record.amount);

        events::emit_aid_claimed(&env, aid_id, &claimant);
        events::emit_aid_settled(&env, aid_id, &claimant, record.amount);

        Ok(())
    }

    /// Alias kept for callers/tests that expect a `settle_aid` entry point;
    /// claiming *is* settlement in this design (single atomic step), so this
    /// simply forwards to `claim_aid`.
    pub fn settle_aid(env: Env, aid_id: u64, claimant: Address) -> Result<(), AidError> {
        Self::claim_aid(env, aid_id, claimant)
    }

    /// After expiry, anyone may trigger a refund of an unclaimed record back
    /// to the donor. Same checks-effects-interactions ordering applies.
    pub fn refund_expired(env: Env, aid_id: u64) -> Result<(), AidError> {
        let mut record = storage::get_aid(&env, aid_id).ok_or(AidError::NotFound)?;

        match record.status {
            AidStatus::Settled => return Err(AidError::AlreadyClaimed),
            AidStatus::Refunded => return Err(AidError::AlreadyRefunded),
            AidStatus::Pending => {}
        }

        if env.ledger().sequence() <= record.expiry_ledger {
            return Err(AidError::NotExpiredYet);
        }

        record.status = AidStatus::Refunded;
        storage::set_aid(&env, aid_id, &record);

        let token_client = token::Client::new(&env, &record.token);
        let contract_address = env.current_contract_address();
        token_client.transfer(&contract_address, &record.donor, &record.amount);

        events::emit_aid_refunded(&env, aid_id, &record.donor, record.amount);

        Ok(())
    }

    /// Read-only lookup, useful for indexers/UIs and tests.
    pub fn get_aid(env: Env, aid_id: u64) -> Option<AidRecord> {
        storage::get_aid(&env, aid_id)
    }
}