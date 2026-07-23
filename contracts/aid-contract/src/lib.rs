#![no_std]

use shared::Error;
use soroban_sdk::{contract, contractimpl, Address, Env};

#[contract]
pub struct AidContract;

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
}
