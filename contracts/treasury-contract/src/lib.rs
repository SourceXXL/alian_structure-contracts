#![no_std]

use shared::errors::Error;
use shared::events;
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, Symbol};

/// Storage key for the treasury's emergency reserve balance.
///
/// Tracked as its own instance-storage entry, separate from any other
/// treasury balance, so `emergency_withdraw` can only ever draw down funds
/// that were earmarked for the reserve.
const KEY_RESERVE_BALANCE: Symbol = symbol_short!("reserve");

#[contract]
pub struct TreasuryContract;

#[contractimpl]
impl TreasuryContract {
    /// Initialise the contract, setting the admin address.
    pub fn initialize(env: Env, admin: Address) {
        shared::auth::set_admin(&env, &admin);
    }

    /// Withdraw funds from the emergency reserve.
    ///
    /// Only callable by the admin, and only while the contract is paused —
    /// this path exists for admins to move reserve funds to a safe address
    /// when something has gone wrong, not for routine operations.
    ///
    /// # Panics
    /// - `Error::NotPaused` if the contract is not currently paused.
    /// - if `caller` is not the admin (see `shared::auth::require_admin`).
    /// - `Error::InvalidArgument` if `amount` is not strictly positive.
    /// - `Error::InsufficientBalance` if the reserve holds less than `amount`.
    pub fn emergency_withdraw(env: Env, caller: Address, to: Address, amount: i128) {
        if !shared::storage::is_paused(&env) {
            env.panic_with_error(Error::NotPaused);
        }
        shared::auth::require_admin(&env, &caller);

        if amount <= 0 {
            env.panic_with_error(Error::InvalidArgument);
        }

        let balance = reserve_balance(&env);
        let new_balance = match balance.checked_sub(amount) {
            Some(b) if b >= 0 => b,
            _ => env.panic_with_error(Error::InsufficientBalance),
        };
        set_reserve_balance(&env, new_balance);

        events::emit(&env, events::TREASURY_EMERGENCY_WITHDRAW, (caller, to, amount));
    }
}

fn reserve_balance(env: &Env) -> i128 {
    env.storage()
        .instance()
        .get::<Symbol, i128>(&KEY_RESERVE_BALANCE)
        .unwrap_or(0)
}

fn set_reserve_balance(env: &Env, balance: i128) {
    env.storage()
        .instance()
        .set::<Symbol, i128>(&KEY_RESERVE_BALANCE, &balance);
}

#[cfg(test)]
mod test;
