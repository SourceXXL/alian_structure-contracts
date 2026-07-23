#![no_std]

use soroban_sdk::{contract, contractimpl, Address, Env};

#[contract]
pub struct AidContract;

#[contractimpl]
impl AidContract {
    /// Initialise the contract, setting the admin address.
    pub fn initialize(env: Env, admin: Address) {
        shared::auth::set_admin(&env, &admin);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Bytes, BytesN, Env};

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

        client.create_aid(&admin, &aid_id, &Some(claim_hash.clone()), &1u32, &(env.ledger().timestamp() + 100));

        assert!(client.claim_with_secret(&aid_id, &secret, &claimant).is_ok());
        assert_eq!(client.claim_with_secret(&aid_id, &secret, &claimant), Err(shared::errors::Error::AlreadyClaimed));
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

        client.create_aid(&admin, &aid_id, &Some(claim_hash), &1u32, &(env.ledger().timestamp() + 100));

        assert_eq!(client.claim_with_secret(&aid_id, &wrong_secret, &claimant), Err(shared::errors::Error::NotAuthorized));
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
        let other_claim_hash = env.crypto().sha256(&other_secret);
        let aid_id = 9u64;
        let claimant_one = Address::random(&env);
        let claimant_two = Address::random(&env);

        client.create_aid(&admin, &aid_id, &Some(claim_hash), &1u32, &(env.ledger().timestamp() + 100));
        assert!(client.claim_with_secret(&aid_id, &secret, &claimant_one).is_ok());
        assert_eq!(client.claim_with_secret(&aid_id, &other_secret, &claimant_two), Err(shared::errors::Error::AlreadyClaimed));
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

        client.create_aid(&admin, &aid_id, &Some(claim_hash), &1u32, &1u64);

        assert_eq!(client.claim_with_secret(&aid_id, &secret, &claimant), Err(shared::errors::Error::Expired));
    }
}
