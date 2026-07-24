use crate::{TreasuryContract, TreasuryContractClient};
use shared::errors::Error;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env};

fn setup() -> (Env, TreasuryContractClient<'static>, Address, Address) {
    let env = Env::default();
    let contract_id = env.register(TreasuryContract, ());
    let client = TreasuryContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    (env, client, contract_id, admin)
}

#[test]
fn emergency_withdraw_rejects_when_not_paused() {
    let (env, client, _contract_id, admin) = setup();
    let to = Address::generate(&env);

    env.mock_all_auths();
    let result = client.try_emergency_withdraw(&admin, &to, &100);

    assert_eq!(result, Err(Ok(soroban_sdk::Error::from(Error::NotPaused))));
}

#[test]
#[should_panic(expected = "caller is not the admin")]
fn emergency_withdraw_rejects_non_admin() {
    let (env, client, contract_id, _admin) = setup();
    let non_admin = Address::generate(&env);
    let to = Address::generate(&env);

    env.as_contract(&contract_id, || {
        shared::storage::set_paused(&env, true);
    });

    env.mock_all_auths();
    client.emergency_withdraw(&non_admin, &to, &100);
}

#[test]
fn emergency_withdraw_succeeds_for_admin_while_paused() {
    let (env, client, contract_id, admin) = setup();
    let to = Address::generate(&env);

    env.as_contract(&contract_id, || {
        shared::storage::set_paused(&env, true);
        crate::set_reserve_balance(&env, 1_000);
    });

    env.mock_all_auths();
    client.emergency_withdraw(&admin, &to, &400);

    let remaining = env.as_contract(&contract_id, || crate::reserve_balance(&env));
    assert_eq!(remaining, 600);
}
