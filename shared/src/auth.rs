use soroban_sdk::{contracttype, symbol_short, Address, Env, Symbol};

use crate::errors::Error;

pub const KEY_ADMIN: Symbol = symbol_short!("admin");

/// Stores the admin address during contract initialisation.
pub fn set_admin(env: &Env, admin: &Address) {
    env.storage()
        .instance()
        .set::<Symbol, Address>(&KEY_ADMIN, admin);
}

/// Returns the current admin address.
///
/// # Panics
/// Panics if no admin has been set.
pub fn get_admin(env: &Env) -> Address {
    env.storage()
        .instance()
        .get::<Symbol, Address>(&KEY_ADMIN)
        .expect("admin not initialised")
}

/// Verifies that `caller` is the admin.
///
/// # Panics
/// Panics if `caller` is not the admin.
pub fn require_admin(env: &Env, caller: &Address) {
    let admin = get_admin(env);
    assert_eq!(*caller, admin, "caller is not the admin");
    caller.require_auth();
}

// ---------------------------------------------------------------------------
// NEW: lightweight, contract-scoped roles (additive — does not touch the
// single-admin functions above). Introduced for the treasury withdrawal
// feature so contracts can gate specific entry points to a role narrower
// than "the admin", e.g. TreasuryManager, without a full governance/RBAC
// module. Uses `Error::Unauthorized` to stay consistent with the existing
// error convention in this crate.
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Role {
    TreasuryManager,
    ReferralManager,
    OracleSigner,
}

#[contracttype]
pub enum RoleKey {
    Role(Address, Role),
}

/// Grants `role` to `who`. Callers are expected to gate access to this
/// (e.g. via `require_admin`) themselves before calling it.
pub fn grant_role(env: &Env, who: &Address, role: Role) {
    env.storage()
        .persistent()
        .set(&RoleKey::Role(who.clone(), role), &true);
}

/// Revokes `role` from `who`.
pub fn revoke_role(env: &Env, who: &Address, role: Role) {
    env.storage()
        .persistent()
        .remove(&RoleKey::Role(who.clone(), role));
}

/// Returns whether `who` currently holds `role`.
pub fn has_role(env: &Env, who: &Address, role: Role) -> bool {
    env.storage()
        .persistent()
        .get::<RoleKey, bool>(&RoleKey::Role(who.clone(), role))
        .unwrap_or(false)
}

/// Requires that `who` has authorized the current invocation AND holds
/// `role`. Returns `Error::Unauthorized` otherwise.
pub fn require_role(env: &Env, who: &Address, role: Role) -> Result<(), Error> {
    who.require_auth();
    if !has_role(env, who, role) {
        return Err(Error::Unauthorized);
    }
    Ok(())
}