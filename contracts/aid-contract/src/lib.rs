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

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct AidClaimMetadata {
    pub claim_hash: Option<BytesN<32>>,
    pub max_claims: u32,
    pub claims_used: u32,
    pub expires_at: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
enum DataKey {
    Aid(u64),
    ClaimNonce(u64, BytesN<32>),
}

#[contractimpl]
impl AidContract {
    /// Initialise the contract, setting the admin address.
    ///
    /// Returns `Error::AlreadyInitialized` if an administrator has already
    /// been stored for this contract instance.
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if env.storage().instance().has(&shared::auth::KEY_ADMIN) {
            return Err(Error::AlreadyInitialized);
        }

        shared::auth::set_admin(&env, &admin);

        Ok(())
    }

    /// Create a new aid with optional claim-link metadata.
    pub fn create_aid(
        env: Env,
        admin: Address,
        aid_id: u64,
        claim_hash: Option<BytesN<32>>,
        max_claims: u32,
        expires_at: u64,
    ) -> Result<(), shared::errors::Error> {
        shared::auth::require_admin(&env, &admin);

        let metadata = AidClaimMetadata {
            claim_hash: claim_hash.clone(),
            max_claims,
            claims_used: 0,
            expires_at,
        };

        env.storage().persistent().set(&DataKey::Aid(aid_id), &metadata);
        Ok(())
    }

    /// Claim an aid using a secret whose hash matches the stored claim hash.
    pub fn claim_with_secret(
        env: Env,
        aid_id: u64,
        secret: Bytes,
        claimant: Address,
    ) -> Result<(), shared::errors::Error> {
        claimant.require_auth();

        let mut metadata = env
            .storage()
            .persistent()
            .get::<DataKey, AidClaimMetadata>(&DataKey::Aid(aid_id))
            .ok_or(shared::errors::Error::NotFound)?;

        let secret_hash = env.crypto().sha256(&secret);

        match &metadata.claim_hash {
            Some(expected_hash) if expected_hash != &secret_hash => {
                return Err(shared::errors::Error::NotAuthorized)
            }
            Some(_) => {}
            None => return Err(shared::errors::Error::NotFound),
        }

        if env.ledger().timestamp() >= metadata.expires_at {
            return Err(shared::errors::Error::Expired);
        }

        let nonce_key = DataKey::ClaimNonce(aid_id, secret_hash.clone());
        if env.storage().temporary().has(&nonce_key) {
            return Err(shared::errors::Error::AlreadyClaimed);
        }

        if metadata.claims_used >= metadata.max_claims {
            return Err(shared::errors::Error::AlreadyClaimed);
        }

        metadata.claims_used += 1;
        env.storage().persistent().set(&DataKey::Aid(aid_id), &metadata);
        env.storage().temporary().set(&nonce_key, &true);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn valid_secret_allows_claim_and_replay_is_blocked() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, AidContract);
        let client = AidContractClient::new(&env, &contract_id);

        let admin = Address::random(&env);
        client.initialize(&admin);

        let secret = Bytes::from_array(&env, &[1, 2, 3, 4]);
        let claim_hash = env.crypto().sha256(&secret);
        let aid_id = 7u64;
        let claimant = Address::random(&env);

        client
            .create_aid(&admin, &aid_id, &Some(claim_hash.clone()), &1u32, &(env.ledger().timestamp() + 100))
            .unwrap();

        assert!(client.claim_with_secret(&aid_id, &secret, &claimant).is_ok());
        assert_eq!(
            client.claim_with_secret(&aid_id, &secret, &claimant),
            Err(shared::errors::Error::AlreadyClaimed)
        );
    }

    #[test]
    fn invalid_secret_is_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, AidContract);
        let client = AidContractClient::new(&env, &contract_id);

        let admin = Address::random(&env);
        client.initialize(&admin);

        let secret = Bytes::from_array(&env, &[1, 2, 3, 4]);
        let wrong_secret = Bytes::from_array(&env, &[9, 9, 9, 9]);
        let claim_hash = env.crypto().sha256(&secret);
        let aid_id = 8u64;
        let claimant = Address::random(&env);

        client
            .create_aid(&admin, &aid_id, &Some(claim_hash), &1u32, &(env.ledger().timestamp() + 100))
            .unwrap();

        assert_eq!(
            client.claim_with_secret(&aid_id, &wrong_secret, &claimant),
            Err(shared::errors::Error::NotAuthorized)
        );
    }

    #[test]
    fn exceeding_max_claims_is_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, AidContract);
        let client = AidContractClient::new(&env, &contract_id);

        let admin = Address::random(&env);
        client.initialize(&admin);

        let secret = Bytes::from_array(&env, &[5, 6, 7, 8]);
        let other_secret = Bytes::from_array(&env, &[9, 10, 11, 12]);
        let claim_hash = env.crypto().sha256(&secret);
        let aid_id = 9u64;
        let claimant_one = Address::random(&env);
        let claimant_two = Address::random(&env);

        client
            .create_aid(&admin, &aid_id, &Some(claim_hash), &1u32, &(env.ledger().timestamp() + 100))
            .unwrap();
        assert!(client.claim_with_secret(&aid_id, &secret, &claimant_one).is_ok());
        assert_eq!(
            client.claim_with_secret(&aid_id, &other_secret, &claimant_two),
            Err(shared::errors::Error::AlreadyClaimed)
        );
    }

    #[test]
    fn expired_claims_are_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, AidContract);
        let client = AidContractClient::new(&env, &contract_id);

        let admin = Address::random(&env);
        client.initialize(&admin);

        let secret = Bytes::from_array(&env, &[13, 14, 15, 16]);
        let claim_hash = env.crypto().sha256(&secret);
        let aid_id = 10u64;
        let claimant = Address::random(&env);

        client
            .create_aid(&admin, &aid_id, &Some(claim_hash), &1u32, &1u64)
            .unwrap();

        assert_eq!(
            client.claim_with_secret(&aid_id, &secret, &claimant),
            Err(shared::errors::Error::Expired)
        );
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