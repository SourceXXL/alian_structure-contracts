use soroban_sdk::{contracttype, symbol_short, Address, Env, Symbol};

use crate::errors::Error;

pub const KEY_ADMIN: Symbol = symbol_short!("admin");

/// Persistent-storage key type used to index per-address role assignments.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Presence of this key means `address` holds `role`.
    Role(Address, Role),
}

// ---------------------------------------------------------------------------
// Role enum
// ---------------------------------------------------------------------------

/// Roles that can be granted to addresses in the system.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Role {
    /// Full administrative control.
    Admin,
    /// Authorised to trigger contract upgrades.
    Upgrader,
    /// Authorised to move treasury funds.
    TreasuryManager,
    /// Authorised to pause / unpause the contract.
    Pauser,
}

// ---------------------------------------------------------------------------
// Admin helpers
// ---------------------------------------------------------------------------

/// Stores the admin address during contract initialisation.
pub fn set_admin(env: &Env, admin: &Address) {
    env.storage()
        .instance()
        .set::<Symbol, Address>(&KEY_ADMIN, admin);
}

/// Returns the current admin address.
///
/// # Panics
/// Panics if no admin has been set (misconfigured contract).
pub fn get_admin(env: &Env) -> Address {
    env.storage()
        .instance()
        .get::<Symbol, Address>(&KEY_ADMIN)
        .expect("admin not initialised")
}

/// Verifies that `caller` is the admin **and** has provided a valid
/// on-chain signature.
///
/// Returns `Err(Error::Unauthorized)` if either check fails.
pub fn require_admin(env: &Env, caller: &Address) -> Result<(), Error> {
    let admin = get_admin(env);
    if *caller != admin {
        return Err(Error::Unauthorized);
    }
    caller.require_auth();
    Ok(())
}

// ---------------------------------------------------------------------------
// Role storage helpers
// ---------------------------------------------------------------------------

/// Returns `true` when `user` holds `role`.
pub fn has_role(env: &Env, user: &Address, role: Role) -> bool {
    env.storage()
        .persistent()
        .has::<DataKey>(&DataKey::Role(user.clone(), role))
}

/// Grants `role` to `user`.  Admin-gated — `admin_caller` must be the
/// current admin with a valid signature.
///
/// Returns `Err(Error::Unauthorized)` when the caller is not the admin.
pub fn grant_role(
    env: &Env,
    admin_caller: &Address,
    user: &Address,
    role: Role,
) -> Result<(), Error> {
    require_admin(env, admin_caller)?;
    env.storage()
        .persistent()
        .set::<DataKey, bool>(&DataKey::Role(user.clone(), role), &true);
    Ok(())
}

/// Revokes `role` from `user`.  Admin-gated — `admin_caller` must be the
/// current admin with a valid signature.
///
/// Returns `Err(Error::Unauthorized)` when the caller is not the admin.
///
/// # Idempotency
/// If `user` does not currently hold `role` this is a no-op and returns
/// `Ok(())`.  Callers should not rely on this function to detect whether
/// the role was actually present; use [`has_role`] for that.
pub fn revoke_role(
    env: &Env,
    admin_caller: &Address,
    user: &Address,
    role: Role,
) -> Result<(), Error> {
    require_admin(env, admin_caller)?;
    env.storage()
        .persistent()
        .remove::<DataKey>(&DataKey::Role(user.clone(), role));
    Ok(())
}

// ---------------------------------------------------------------------------
// Role guard
// ---------------------------------------------------------------------------

/// Verifies that `caller` holds `role` **and** has provided a valid
/// on-chain signature.
///
/// Returns `Err(Error::Unauthorized)` if either check fails.
pub fn require_role(env: &Env, caller: &Address, role: Role) -> Result<(), Error> {
    if !has_role(env, caller, role) {
        return Err(Error::Unauthorized);
    }
    caller.require_auth();
    Ok(())
}

// ---------------------------------------------------------------------------
// Pause guard
// ---------------------------------------------------------------------------

/// Returns `Err(Error::ContractPaused)` when the contract is paused,
/// and `Ok(())` when it is active.
///
/// Call this at the top of every state-changing entry point.
pub fn require_not_paused(env: &Env) -> Result<(), Error> {
    if crate::storage::is_paused(env) {
        return Err(Error::ContractPaused);
    }
    Ok(())
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