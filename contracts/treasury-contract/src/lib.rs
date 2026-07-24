#![no_std]

use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, Symbol};

use shared::auth::{self, Role};
use shared::errors::Error;
use shared::events::{self, TREASURY_WITHDRAW};

/// Storage key prefix for per-category balances; the full key is
/// `(BALANCE, category)`.
const BALANCE: Symbol = symbol_short!("cat_bal");
/// Storage key for the configurable max per-transaction withdrawal limit.
const MAX_WD: Symbol = symbol_short!("max_wd");

#[contract]
pub struct TreasuryContract;

const KEY_TOKEN: Symbol = symbol_short!("token");
const KEY_BALANCES: Symbol = symbol_short!("balances");
const CATEGORY_RESERVE: Symbol = symbol_short!("Reserve");
const CATEGORY_REWARDS: Symbol = symbol_short!("Rewards");
const CATEGORY_FEES: Symbol = symbol_short!("Fees");

#[contractimpl]
impl TreasuryContract {
    /// Initialise the contract: sets the admin address, grants the admin
    /// the `TreasuryManager` role, and sets the initial max
    /// per-transaction withdrawal limit.
    ///
    /// CHANGED: signature extended from `initialize(env, admin)` to also
    /// take `max_withdrawal_limit` — update deploy/init scripts (e.g.
    /// `scripts/initialize.sh`) accordingly.
    pub fn initialize(env: Env, admin: Address, max_withdrawal_limit: i128) -> Result<(), Error> {
        if max_withdrawal_limit <= 0 {
            return Err(Error::InvalidArgument);
        }
        auth::set_admin(&env, &admin);
        // Grant the admin the TreasuryManager role so they can operate the
        // treasury immediately after initialisation without a separate call.
        // We write the persistent role entry directly here because require_admin
        // has just been established in the line above.
        env.storage()
            .persistent()
            .set::<shared::auth::DataKey, bool>(
                &shared::auth::DataKey::Role(admin.clone(), Role::TreasuryManager),
                &true,
            );
        env.storage().instance().set(&MAX_WD, &max_withdrawal_limit);
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
        auth::require_admin(&env, &caller);
        if new_limit <= 0 {
            return Err(Error::InvalidArgument);
        }
        env.storage().instance().set(&MAX_WD, &new_limit);
        Ok(())
    }

    /// Credits `amount` into `category`'s balance. `TreasuryManager` only.
    /// Used by other protocol contracts / setup flows to fund the
    /// treasury's internal accounting.
    pub fn deposit(env: Env, caller: Address, category: Symbol, amount: i128) -> Result<(), Error> {
        auth::require_role(&env, &caller, Role::TreasuryManager)?;
        if amount <= 0 {
            return Err(Error::InvalidArgument);
        }
        let key = (BALANCE, category);
        let balance: i128 = env.storage().instance().get(&key).unwrap_or(0);
        let new_balance = balance.checked_add(amount).ok_or(Error::Overflow)?;
        env.storage().instance().set(&key, &new_balance);
        Ok(())
    }

    /// Returns the current balance for `category` (0 if never funded).
    pub fn category_balance(env: Env, category: Symbol) -> i128 {
        env.storage().instance().get(&(BALANCE, category)).unwrap_or(0)
    }

    /// Returns the currently configured max per-transaction withdrawal limit.
    pub fn withdrawal_limit(env: Env) -> i128 {
        env.storage().instance().get(&MAX_WD).unwrap_or(0)
    }

    /// Withdraws `amount` from `category` to `to`.
    ///
    /// Guards, in order:
    /// 1. `caller` must authorize the call AND hold `TreasuryManager`
    ///    -> `Error::Unauthorized`
    /// 2. `amount` must be > 0                     -> `Error::InvalidArgument`
    /// 3. `amount` must not exceed the configured max withdrawal limit
    ///    -> `Error::WithdrawalLimitExceeded`
    /// 4. `amount` must not exceed the category balance
    ///    -> `Error::InsufficientBalance`
    ///
    /// On success, decrements the category balance and emits the shared
    /// `TREASURY_WITHDRAW` event with `(category, to, amount, remaining)`.
    pub fn withdraw(
        env: Env,
        caller: Address,
        to: Address,
        amount: i128,
        category: Symbol,
    ) -> Result<(), Error> {
        // 1. Auth + role gate.
        auth::require_role(&env, &caller, Role::TreasuryManager)?;

        // 2. Basic input validation.
        if amount <= 0 {
            return Err(Error::InvalidArgument);
        }

        // 3. Configurable per-transaction limit.
        let limit: i128 = env.storage().instance().get(&MAX_WD).unwrap_or(0);
        if amount > limit {
            return Err(Error::WithdrawalLimitExceeded);
        }

        // 4. Sufficient category balance.
        let key = (BALANCE, category.clone());
        let balance: i128 = env.storage().instance().get(&key).unwrap_or(0);
        if amount > balance {
            return Err(Error::InsufficientBalance);
        }

        // Effects before interactions/events.
        let remaining = balance - amount;
        env.storage().instance().set(&key, &remaining);

        // NOTE: as with the rest of this contract, balances here are
        // internal accounting only. If this treasury custodies a live
        // SAC/token, wire a `token::Client::transfer(&to, &amount)` call
        // here (before the event emit) using a stored token address.
        events::emit(&env, TREASURY_WITHDRAW, (category, to, amount, remaining));

        Ok(())
    }

    #[test]
    fn deposit_updates_balances_and_emits_event() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let alice = Address::generate(&env);
        let treasury_id = env.register_contract(None, TreasuryContract);
        let token_id = env.register_contract(None, MockTokenContract);
        let treasury = TreasuryClient::new(&env, &treasury_id);
        let token = MockTokenClient::new(&env, &token_id);

        treasury.initialize(&admin, &token_id);
        token.initialize(&admin);
        token.mint(&alice, &100);

        assert!(treasury.deposit(&alice, &50, &symbol_short!("Reserve")).is_ok());
        assert!(treasury.deposit(&alice, &25, &symbol_short!("Rewards")).is_ok());

        assert_eq!(treasury.balance(&symbol_short!("Reserve")), 50);
        assert_eq!(treasury.balance(&symbol_short!("Rewards")), 25);
        assert_eq!(treasury.balance(&symbol_short!("Fees")), 0);
        assert_eq!(treasury.total_balance(), 75);
        assert_eq!(token.balance(&treasury_id), 75);

        let events = env.events().all();
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn deposit_returns_overflow_for_category_overflow() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let alice = Address::generate(&env);
        let treasury_id = env.register_contract(None, TreasuryContract);
        let token_id = env.register_contract(None, MockTokenContract);
        let treasury = TreasuryClient::new(&env, &treasury_id);
        let token = MockTokenClient::new(&env, &token_id);

        treasury.initialize(&admin, &token_id);
        token.initialize(&admin);
        token.mint(&alice, &100);

        assert!(treasury.deposit(&alice, &i128::MAX, &symbol_short!("Reserve")).is_ok());
        assert_eq!(
            treasury.deposit(&alice, &1, &symbol_short!("Reserve")).unwrap_err(),
            Error::Overflow
        );
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

#[cfg(test)]
mod test;
