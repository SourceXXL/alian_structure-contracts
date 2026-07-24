#![no_std]

use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, Symbol};

use shared::auth::{self, Role};
use shared::errors::Error;
use shared::events::{self, TREASURY_WITHDRAW};
use shared::storage::{instance_get, instance_set, persistent_set};

/// Storage key prefix for per-category balances; the full key is
/// `(BALANCE, category)`.
const BALANCE: Symbol = symbol_short!("cat_bal");
/// Storage key for the configurable max per-transaction withdrawal limit.
const MAX_WD: Symbol = symbol_short!("max_wd");
/// Category symbol for the emergency reserve (used by `emergency_withdraw`).
const RESERVE_CATEGORY: Symbol = symbol_short!("reserve");

#[contract]
pub struct TreasuryContract;

#[contractimpl]
impl TreasuryContract {
    /// Initialise the contract: sets the admin address, grants the admin
    /// the `TreasuryManager` role, and sets the initial max
    /// per-transaction withdrawal limit.
    pub fn initialize(env: Env, admin: Address, max_withdrawal_limit: i128) -> Result<(), Error> {
        if max_withdrawal_limit <= 0 {
            return Err(Error::InvalidArgument);
        }
        auth::set_admin(&env, &admin);
        // Grant the admin the TreasuryManager role immediately after
        // initialisation so routine operations do not need a separate call.
        // Written via persistent_set so the entry gets a TTL bump.
        persistent_set(
            &env,
            &shared::auth::DataKey::Role(admin.clone(), Role::TreasuryManager),
            &true,
        );
        instance_set(&env, &MAX_WD, &max_withdrawal_limit);
        Ok(())
    }

    /// Grants the `TreasuryManager` role to `who`. Admin only.
    pub fn add_treasury_manager(env: Env, caller: Address, who: Address) -> Result<(), Error> {
        auth::grant_role(&env, &caller, &who, Role::TreasuryManager)
    }

    /// Revokes the `TreasuryManager` role from `who`. Admin only.
    pub fn remove_treasury_manager(env: Env, caller: Address, who: Address) -> Result<(), Error> {
        auth::revoke_role(&env, &caller, &who, Role::TreasuryManager)
    }

    /// Updates the max per-transaction withdrawal limit. Admin only.
    pub fn set_withdrawal_limit(env: Env, caller: Address, new_limit: i128) -> Result<(), Error> {
        auth::require_admin(&env, &caller)?;
        if new_limit <= 0 {
            return Err(Error::InvalidArgument);
        }
        instance_set(&env, &MAX_WD, &new_limit);
        Ok(())
    }

    /// Credits `amount` into `category`'s balance. `TreasuryManager` only.
    pub fn deposit(env: Env, caller: Address, category: Symbol, amount: i128) -> Result<(), Error> {
        auth::require_role(&env, &caller, Role::TreasuryManager)?;
        if amount <= 0 {
            return Err(Error::InvalidArgument);
        }
        let key = (BALANCE, category);
        let balance: i128 = instance_get(&env, &key).unwrap_or(0);
        let new_balance = balance.checked_add(amount).ok_or(Error::Overflow)?;
        instance_set(&env, &key, &new_balance);
        Ok(())
    }

    /// Returns the current balance for `category` (0 if never funded).
    pub fn category_balance(env: Env, category: Symbol) -> i128 {
        instance_get::<_, i128>(&env, &(BALANCE, category)).unwrap_or(0)
    }

    /// Returns the currently configured max per-transaction withdrawal limit.
    pub fn withdrawal_limit(env: Env) -> i128 {
        instance_get::<_, i128>(&env, &MAX_WD).unwrap_or(0)
    }

    /// Withdraws `amount` from `category` to `to`.
    ///
    /// Guards, in order:
    /// 1. `caller` must hold `TreasuryManager`          → `Error::Unauthorized`
    /// 2. `amount` must be > 0                           → `Error::InvalidArgument`
    /// 3. `amount` must not exceed the withdrawal limit  → `Error::WithdrawalLimitExceeded`
    /// 4. `amount` must not exceed the category balance  → `Error::InsufficientBalance`
    ///
    /// On success, decrements the category balance and emits `TREASURY_WITHDRAW`.
    pub fn withdraw(
        env: Env,
        caller: Address,
        to: Address,
        amount: i128,
        category: Symbol,
    ) -> Result<(), Error> {
        auth::require_role(&env, &caller, Role::TreasuryManager)?;

        if amount <= 0 {
            return Err(Error::InvalidArgument);
        }

        let limit: i128 = instance_get(&env, &MAX_WD).unwrap_or(0);
        if amount > limit {
            return Err(Error::WithdrawalLimitExceeded);
        }

        let key = (BALANCE, category.clone());
        let balance: i128 = instance_get(&env, &key).unwrap_or(0);
        if amount > balance {
            return Err(Error::InsufficientBalance);
        }

        let remaining = balance - amount;
        instance_set(&env, &key, &remaining);

        events::emit(&env, TREASURY_WITHDRAW, (category, to, amount, remaining));
        Ok(())
    }

    /// Emergency reserve withdrawal.
    ///
    /// Only callable by the admin, and only while the contract is paused.
    /// Intended to move reserve funds to safety when something has gone wrong.
    ///
    /// # Errors
    /// - `Error::NotPaused`            — contract is currently active.
    /// - `Error::Unauthorized`         — caller is not the admin.
    /// - `Error::InvalidArgument`      — `amount` is not strictly positive.
    /// - `Error::InsufficientBalance`  — reserve balance is insufficient.
    pub fn emergency_withdraw(
        env: Env,
        caller: Address,
        to: Address,
        amount: i128,
    ) -> Result<(), Error> {
        if !shared::storage::is_paused(&env) {
            return Err(Error::NotPaused);
        }
        auth::require_admin(&env, &caller)?;

        if amount <= 0 {
            return Err(Error::InvalidArgument);
        }

        let key = (BALANCE, RESERVE_CATEGORY);
        let balance: i128 = instance_get(&env, &key).unwrap_or(0);
        let new_balance = balance.checked_sub(amount).ok_or(Error::InsufficientBalance)?;
        if new_balance < 0 {
            return Err(Error::InsufficientBalance);
        }
        instance_set(&env, &key, &new_balance);

        events::emit(
            &env,
            events::TREASURY_EMERGENCY_WITHDRAW,
            (caller, to, amount),
        );
        Ok(())
    }
}

#[cfg(test)]
mod test;
