#![no_std]

use soroban_sdk::{contract, contractimpl, Address, Env};

#[contract]
pub struct ReferralContract;

#[contractimpl]
impl ReferralContract {
    /// Initialise the contract, setting the admin address.
    pub fn initialize(env: Env, admin: Address) {
        shared::auth::set_admin(&env, &admin);
    }
}
