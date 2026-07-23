#![no_std]

use soroban_sdk::{contract, contractimpl, Address, Env};

#[contract]
pub struct RegistryContract;

#[contractimpl]
impl RegistryContract {
    /// Initialise the contract, setting the admin address.
    pub fn initialize(env: Env, admin: Address) {
        shared::auth::set_admin(&env, &admin);
    }
}
