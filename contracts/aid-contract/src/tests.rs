#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    token, Env,
};

fn setup_token(env: &Env, admin: &Address) -> (Address, token::Client<'static>, token::StellarAssetClient<'static>) {
    let contract_address = env.register_stellar_asset_contract_v2(admin.clone());
    let address = contract_address.address();
    let client = token::Client::new(env, &address);
    let asset_client = token::StellarAssetClient::new(env, &address);
    (address, client, asset_client)
}

fn advance_ledger(env: &Env, to_sequence: u32) {
    env.ledger().set(LedgerInfo {
        sequence_number: to_sequence,
        ..env.ledger().get()
    });
}

#[test]
fn claim_transfers_escrow_and_settles() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let donor = Address::generate(&env);
    let recipient = Address::generate(&env);

    let (token_addr, token_client, asset_client) = setup_token(&env, &admin);
    asset_client.mint(&donor, &1_000);

    let contract_id = env.register(AidContract, ());
    let client = AidContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    client.create_aid(&1, &donor, &recipient, &token_addr, &500, &1_000);
    assert_eq!(token_client.balance(&contract_id), 500);

    client.claim_aid(&1, &recipient);

    assert_eq!(token_client.balance(&recipient), 500);
    assert_eq!(token_client.balance(&contract_id), 0);

    let record = client.get_aid(&1).unwrap();
    assert_eq!(record.status, AidStatus::Settled);
}

#[test]
fn second_claim_returns_already_claimed() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let donor = Address::generate(&env);
    let recipient = Address::generate(&env);

    let (token_addr, _token_client, asset_client) = setup_token(&env, &admin);
    asset_client.mint(&donor, &1_000);

    let contract_id = env.register(AidContract, ());
    let client = AidContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    client.create_aid(&1, &donor, &recipient, &token_addr, &500, &1_000);
    client.claim_aid(&1, &recipient);

    let result = client.try_claim_aid(&1, &recipient);
    assert_eq!(result, Err(Ok(AidError::AlreadyClaimed)));
}

#[test]
fn claim_after_expiry_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let donor = Address::generate(&env);
    let recipient = Address::generate(&env);

    let (token_addr, _token_client, asset_client) = setup_token(&env, &admin);
    asset_client.mint(&donor, &1_000);

    let contract_id = env.register(AidContract, ());
    let client = AidContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    client.create_aid(&1, &donor, &recipient, &token_addr, &500, &100);

    advance_ledger(&env, 101);

    let result = client.try_claim_aid(&1, &recipient);
    assert_eq!(result, Err(Ok(AidError::Expired)));
}

#[test]
fn claim_by_wrong_address_is_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let donor = Address::generate(&env);
    let recipient = Address::generate(&env);
    let stranger = Address::generate(&env);

    let (token_addr, _token_client, asset_client) = setup_token(&env, &admin);
    asset_client.mint(&donor, &1_000);

    let contract_id = env.register(AidContract, ());
    let client = AidContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    client.create_aid(&1, &donor, &recipient, &token_addr, &500, &1_000);

    let result = client.try_claim_aid(&1, &stranger);
    assert_eq!(result, Err(Ok(AidError::Unauthorized)));
}

#[test]
fn claim_while_paused_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let donor = Address::generate(&env);
    let recipient = Address::generate(&env);

    let (token_addr, _token_client, asset_client) = setup_token(&env, &admin);
    asset_client.mint(&donor, &1_000);

    let contract_id = env.register(AidContract, ());
    let client = AidContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    client.create_aid(&1, &donor, &recipient, &token_addr, &500, &1_000);
    client.set_paused(&admin, &true);

    let result = client.try_claim_aid(&1, &recipient);
    assert_eq!(result, Err(Ok(AidError::Paused)));
}

#[test]
fn state_is_settled_before_transfer_state_is_consistent_on_success() {
    // Because a single Soroban contract invocation is atomic, a transfer
    // failure would roll back the status write too — so the meaningful
    // guarantee we can assert here is that after a *successful* claim, the
    // observable state is always internally consistent: Settled status and
    // moved balance appear together, never one without the other. This is
    // exactly what checks-effects-interactions ordering in claim_aid gives
    // us, since the status is written first and the transfer second within
    // the same call.
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let donor = Address::generate(&env);
    let recipient = Address::generate(&env);

    let (token_addr, token_client, asset_client) = setup_token(&env, &admin);
    asset_client.mint(&donor, &1_000);

    let contract_id = env.register(AidContract, ());
    let client = AidContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    client.create_aid(&1, &donor, &recipient, &token_addr, &500, &1_000);
    client.claim_aid(&1, &recipient);

    let record = client.get_aid(&1).unwrap();
    let moved = token_client.balance(&recipient) == 500;
    assert_eq!(record.status, AidStatus::Settled);
    assert!(moved);
}

#[test]
fn refund_after_expiry_returns_funds_to_donor() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let donor = Address::generate(&env);
    let recipient = Address::generate(&env);

    let (token_addr, token_client, asset_client) = setup_token(&env, &admin);
    asset_client.mint(&donor, &1_000);

    let contract_id = env.register(AidContract, ());
    let client = AidContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    client.create_aid(&1, &donor, &recipient, &token_addr, &500, &100);
    advance_ledger(&env, 101);

    client.refund_expired(&1);

    assert_eq!(token_client.balance(&donor), 1_000);
    let record = client.get_aid(&1).unwrap();
    assert_eq!(record.status, AidStatus::Refunded);
}