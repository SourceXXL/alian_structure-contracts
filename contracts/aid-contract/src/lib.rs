#![no_std]

use soroban_sdk::{
    contract, contractimpl, panic_with_error, token, Address, Env, Symbol,
};
use shared::{emit, AID_CREATED, Error};

/// Storage keys
const KEY_TOKEN: Symbol = Symbol::new("token");
const KEY_AID_COUNTER: Symbol = Symbol::new("aid_cnt");

/// Aid status enum
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
#[repr(u32)]
pub enum AidStatus {
    Created = 0,
    Claimed = 1,
    Settled = 2,
    Refunded = 3,
}

impl From<u32> for AidStatus {
    fn from(value: u32) -> Self {
        match value {
            0 => AidStatus::Created,
            1 => AidStatus::Claimed,
            2 => AidStatus::Settled,
            3 => AidStatus::Refunded,
            _ => panic!("invalid aid status"),
        }
    }
}

/// AidRecord structure that stores all aid information
#[soroban_sdk::contracttype]
#[derive(Debug, Clone)]
pub struct AidRecord {
    pub id: u64,
    pub donor: Address,
    pub recipient: Address,
    pub amount: i128,
    pub status: u32,
    pub timestamp: u64,
    pub expiry: u64,
}

#[contract]
pub struct AidContract;

#[contractimpl]
impl AidContract {
    /// Initialise the contract, setting the admin address and token address.
    pub fn initialize(env: Env, admin: Address, token: Address) {
        shared::auth::set_admin(&env, &admin);
        env.storage().instance().set(&KEY_TOKEN, &token);
        // Initialize aid counter to 0
        env.storage().instance().set(&KEY_AID_COUNTER, &0u64);
    }

    /// Create a new aid record, escrowing funds from the donor.
    pub fn create_aid(
        env: Env,
        donor: Address,
        recipient: Address,
        amount: i128,
        expiry: u64,
    ) -> u64 {
        // Verify the donor is the caller and has authorized this action
        donor.require_auth();

        // Validate amount > 0
        if amount <= 0 {
            panic_with_error!(env, Error::InvalidArgument);
        }

        // Validate expiry is in the future
        let current_time = env.ledger().timestamp();
        if expiry <= current_time {
            panic_with_error!(env, Error::InvalidArgument);
        }

        // Get the token address
        let token = env.storage()
            .instance()
            .get::<Symbol, Address>(&KEY_TOKEN)
            .expect("token not initialized");

        // Transfer the amount from donor to this contract (escrow)
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&donor, &env.current_contract_address(), &amount);

        // Get the current counter and increment it to generate unique aid_id
        let mut current_counter = env.storage()
            .instance()
            .get::<Symbol, u64>(&KEY_AID_COUNTER)
            .unwrap_or(0);
        current_counter += 1;
        let aid_id = current_counter;
        env.storage().instance().set(&KEY_AID_COUNTER, &current_counter);

        // Create and store the AidRecord
        let aid_record = AidRecord {
            id: aid_id,
            donor: donor.clone(),
            recipient: recipient.clone(),
            amount,
            status: AidStatus::Created as u32,
            timestamp: current_time,
            expiry,
        };

        // Store the aid record using a unique key
        let aid_key = Symbol::new_from_str(&format!("aid_{}", aid_id));
        env.storage().persistent().set(&aid_key, &aid_record);

        // Emit the AidCreated event
        emit(&env, AID_CREATED, (aid_id, donor, recipient, amount, current_time, expiry));

        aid_id
    }

    /// Helper function to get an aid record by ID (useful for testing and other functions)
    pub fn get_aid(env: Env, aid_id: u64) -> AidRecord {
        let aid_key = Symbol::new_from_str(&format!("aid_{}", aid_id));
        env.storage()
            .persistent()
            .get(&aid_key)
            .expect("aid record not found")
    }

    /// Get the token address used by the contract
    pub fn get_token(env: Env) -> Address {
        env.storage()
            .instance()
            .get::<Symbol, Address>(&KEY_TOKEN)
            .expect("token not initialized")
    }
}