#![cfg(test)]

extern crate std;

use soroban_sdk::{testutils::Address as _, Address, Env};

use crate::{
    auth::{
        get_admin, grant_role, has_role, require_admin, require_not_paused, require_role,
        revoke_role, set_admin, Role,
    },
    errors::Error,
    storage::set_paused,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Creates a fresh environment and registers a random admin address.
fn setup() -> (Env, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    set_admin(&env, &admin);
    (env, admin)
}

// ---------------------------------------------------------------------------
// require_admin
// ---------------------------------------------------------------------------

#[test]
fn test_require_admin_succeeds_for_admin() {
    let (env, admin) = setup();
    assert_eq!(require_admin(&env, &admin), Ok(()));
}

#[test]
fn test_require_admin_fails_for_non_admin() {
    let (env, _admin) = setup();
    let other = Address::generate(&env);
    assert_eq!(require_admin(&env, &other), Err(Error::Unauthorized));
}

// ---------------------------------------------------------------------------
// grant_role / revoke_role — admin gating
// ---------------------------------------------------------------------------

#[test]
fn test_grant_role_by_admin_succeeds() {
    let (env, admin) = setup();
    let user = Address::generate(&env);
    let result = grant_role(&env, &admin, &user, Role::Upgrader);
    assert_eq!(result, Ok(()));
    assert!(has_role(&env, &user, Role::Upgrader));
}

#[test]
fn test_grant_role_by_non_admin_fails() {
    let (env, _admin) = setup();
    let attacker = Address::generate(&env);
    let user = Address::generate(&env);
    let result = grant_role(&env, &attacker, &user, Role::Upgrader);
    assert_eq!(result, Err(Error::Unauthorized));
    assert!(!has_role(&env, &user, Role::Upgrader));
}

#[test]
fn test_revoke_role_by_admin_succeeds() {
    let (env, admin) = setup();
    let user = Address::generate(&env);
    // Grant first, then revoke.
    grant_role(&env, &admin, &user, Role::TreasuryManager).unwrap();
    assert!(has_role(&env, &user, Role::TreasuryManager));

    let result = revoke_role(&env, &admin, &user, Role::TreasuryManager);
    assert_eq!(result, Ok(()));
    assert!(!has_role(&env, &user, Role::TreasuryManager));
}

#[test]
fn test_revoke_role_by_non_admin_fails() {
    let (env, admin) = setup();
    let attacker = Address::generate(&env);
    let user = Address::generate(&env);
    grant_role(&env, &admin, &user, Role::Pauser).unwrap();

    let result = revoke_role(&env, &attacker, &user, Role::Pauser);
    assert_eq!(result, Err(Error::Unauthorized));
    // Role should still be present after the failed revocation.
    assert!(has_role(&env, &user, Role::Pauser));
}

// ---------------------------------------------------------------------------
// require_role
// ---------------------------------------------------------------------------

#[test]
fn test_require_role_passes_when_role_held() {
    let (env, admin) = setup();
    let user = Address::generate(&env);
    grant_role(&env, &admin, &user, Role::Upgrader).unwrap();
    assert_eq!(require_role(&env, &user, Role::Upgrader), Ok(()));
}

#[test]
fn test_require_role_fails_when_role_not_held() {
    let (env, _admin) = setup();
    let user = Address::generate(&env);
    assert_eq!(
        require_role(&env, &user, Role::TreasuryManager),
        Err(Error::Unauthorized)
    );
}

#[test]
fn test_require_role_fails_after_role_revoked() {
    let (env, admin) = setup();
    let user = Address::generate(&env);
    grant_role(&env, &admin, &user, Role::Pauser).unwrap();
    revoke_role(&env, &admin, &user, Role::Pauser).unwrap();
    assert_eq!(
        require_role(&env, &user, Role::Pauser),
        Err(Error::Unauthorized)
    );
}

#[test]
fn test_roles_are_independent_per_role_variant() {
    let (env, admin) = setup();
    let user = Address::generate(&env);
    grant_role(&env, &admin, &user, Role::Upgrader).unwrap();
    // Holding Upgrader does not grant TreasuryManager.
    assert_eq!(
        require_role(&env, &user, Role::TreasuryManager),
        Err(Error::Unauthorized)
    );
}

// ---------------------------------------------------------------------------
// require_not_paused
// ---------------------------------------------------------------------------

#[test]
fn test_require_not_paused_passes_when_active() {
    let (env, _admin) = setup();
    // Default state: not paused.
    assert_eq!(require_not_paused(&env), Ok(()));
}

#[test]
fn test_require_not_paused_blocks_when_paused() {
    let (env, _admin) = setup();
    set_paused(&env, true);
    assert_eq!(require_not_paused(&env), Err(Error::ContractPaused));
}

#[test]
fn test_require_not_paused_passes_after_resume() {
    let (env, _admin) = setup();
    set_paused(&env, true);
    // Resume the contract.
    set_paused(&env, false);
    assert_eq!(require_not_paused(&env), Ok(()));
}

// ---------------------------------------------------------------------------
// get_admin
// ---------------------------------------------------------------------------

#[test]
fn test_get_admin_returns_set_admin() {
    let (env, admin) = setup();
    assert_eq!(get_admin(&env), admin);
}
