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

const KEY_TOKEN: Symbol = symbol_short!("token");
const KEY_BALANCES: Symbol = symbol_short!("balances");
const CATEGORY_RESERVE: Symbol = symbol_short!("Reserve");
const CATEGORY_REWARDS: Symbol = symbol_short!("Rewards");
const CATEGORY_FEES: Symbol = symbol_short!("Fees");

#[contractimpl]
impl TreasuryContract {
    /// Initialize the contract, setting the admin address and token contract.
    pub fn initialize(env: Env, admin: Address, token: Address) {
        auth::set_admin(&env, &admin);
        env.storage().instance().set(&KEY_TOKEN, &token);
    }

    /// Deposit tokens into the treasury under the requested category.
    pub fn deposit(env: Env, from: Address, amount: i128, category: Symbol) -> Result<(), Error> {
        if amount <= 0 {
            return Err(Error::InvalidArgument);
        }

        if category != CATEGORY_RESERVE && category != CATEGORY_REWARDS && category != CATEGORY_FEES {
            return Err(Error::InvalidArgument);
        }

        let token: Address = env.storage().instance().get(&KEY_TOKEN).unwrap();
        let treasury = env.current_contract_address();

        from.require_auth();
        env.invoke_contract::<()>(
            &token,
            &symbol_short!("transfer"),
            (from.clone(), treasury, amount),
        );

        let mut balances: Map<Symbol, i128> = env
            .storage()
            .instance()
            .get(&KEY_BALANCES)
            .unwrap_or_else(|| Map::new(&env));

        let current = balances.get(&category).unwrap_or(0);
        let updated = math::safe_add(current, amount).ok_or(Error::Overflow)?;
        balances.set(&category, &updated);
        env.storage().instance().set(&KEY_BALANCES, &balances);

        events::emit(&env, events::TREASURY_DEPOSIT, (from, amount, category));

        Ok(())
    }

    /// Return the balance stored for the requested category.
    pub fn balance(env: Env, category: Symbol) -> i128 {
        let balances: Map<Symbol, i128> = env
            .storage()
            .instance()
            .get(&KEY_BALANCES)
            .unwrap_or_else(|| Map::new(&env));

        balances.get(&category).unwrap_or(0)
    }

    /// Return the total balance across all treasury categories.
    pub fn total_balance(env: Env) -> i128 {
        let balances: Map<Symbol, i128> = env
            .storage()
            .instance()
            .get(&KEY_BALANCES)
            .unwrap_or_else(|| Map::new(&env));

        let reserve = balances.get(&CATEGORY_RESERVE).unwrap_or(0);
        let rewards = balances.get(&CATEGORY_REWARDS).unwrap_or(0);
        let fees = balances.get(&CATEGORY_FEES).unwrap_or(0);

        math::safe_add(reserve, rewards)
            .and_then(|sum| math::safe_add(sum, fees))
            .expect("total balance overflow")
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Events;

    #[contract]
    struct MockTokenContract;

    #[contractimpl]
    impl MockTokenContract {
        pub fn initialize(env: Env, admin: Address) {
            env.storage().instance().set(&symbol_short!("admin"), &admin);
        }

        pub fn mint(env: Env, to: Address, amount: i128) {
            let mut balances: Map<Address, i128> = env
                .storage()
                .instance()
                .get(&symbol_short!("bal"))
                .unwrap_or_else(|| Map::new(&env));
            let balance = balances.get(&to).unwrap_or(0);
            balances.set(&to, &(balance + amount));
            env.storage().instance().set(&symbol_short!("bal"), &balances);
        }

        pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
            from.require_auth();
            let mut balances: Map<Address, i128> = env
                .storage()
                .instance()
                .get(&symbol_short!("bal"))
                .unwrap_or_else(|| Map::new(&env));
            let from_balance = balances.get(&from).unwrap_or(0);
            let to_balance = balances.get(&to).unwrap_or(0);
            balances.set(&from, &(from_balance - amount));
            balances.set(&to, &(to_balance + amount));
            env.storage().instance().set(&symbol_short!("bal"), &balances);
        }

        pub fn balance(env: Env, id: Address) -> i128 {
            let balances: Map<Address, i128> = env
                .storage()
                .instance()
                .get(&symbol_short!("bal"))
                .unwrap_or_else(|| Map::new(&env));
            balances.get(&id).unwrap_or(0)
        }
    }

    struct MockTokenClient<'a> {
        env: &'a Env,
        contract_id: &'a Address,
    }

    impl<'a> MockTokenClient<'a> {
        fn new(env: &'a Env, contract_id: &'a Address) -> Self {
            Self { env, contract_id }
        }

        fn initialize(&self, admin: &Address) {
            self.env
                .invoke_contract::<()>(&self.contract_id, &symbol_short!("initialize"), (admin,));
        }

        fn mint(&self, to: &Address, amount: &i128) {
            self.env
                .invoke_contract::<()>(&self.contract_id, &symbol_short!("mint"), (to, amount));
        }

        fn balance(&self, id: &Address) -> i128 {
            self.env
                .invoke_contract::<i128>(&self.contract_id, &symbol_short!("balance"), (id,))
        }
    }

    struct TreasuryClient<'a> {
        env: &'a Env,
        contract_id: &'a Address,
    }

    impl<'a> TreasuryClient<'a> {
        fn new(env: &'a Env, contract_id: &'a Address) -> Self {
            Self { env, contract_id }
        }

        fn initialize(&self, admin: &Address, token: &Address) {
            self.env
                .invoke_contract::<()>(&self.contract_id, &symbol_short!("initialize"), (admin, token));
        }

        fn deposit(&self, from: &Address, amount: &i128, category: &Symbol) -> Result<(), Error> {
            self.env.invoke_contract::<Result<(), Error>>(
                &self.contract_id,
                &symbol_short!("deposit"),
                (from, amount, category),
            )
        }

        fn balance(&self, category: &Symbol) -> i128 {
            self.env
                .invoke_contract::<i128>(&self.contract_id, &symbol_short!("balance"), (category,))
        }

        fn total_balance(&self) -> i128 {
            self.env
                .invoke_contract::<i128>(&self.contract_id, &symbol_short!("total_balance"), ())
        }
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

fn set_reserve_balance(env: &Env, balance: i128) {
    env.storage()
        .instance()
        .set::<Symbol, i128>(&KEY_RESERVE_BALANCE, &balance);
}

#[cfg(test)]
mod test;
