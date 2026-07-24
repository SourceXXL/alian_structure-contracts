use soroban_sdk::{contracttype, symbol_short, Address, Env, Symbol};

use crate::errors::Error;

pub const KEY_ADMIN: Symbol = symbol_short!("admin");

// ---------------------------------------------------------------------------
// Role enum — single authoritative definition
// ---------------------------------------------------------------------------

/// Roles that can be granted to addresses in the system.
///
/// Every role is stored as a persistent `DataKey::Role(address, role)` entry.
/// Roles are independent; holding one does not imply another.
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
    /// Authorised to write referral configuration.
    ReferralManager,
    /// Authorised to post oracle signatures / verification proofs.
    OracleSigner,
}

// ---------------------------------------------------------------------------
// Storage key for role entries — stored in persistent storage
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Presence of this key means `address` holds `role`.
    Role(Address, Role),
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

/// Grants `role` to `user`. Admin-gated — `admin_caller` must be the
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

/// Revokes `role` from `user`. Admin-gated — `admin_caller` must be the
/// current admin with a valid signature.
///
/// Returns `Err(Error::Unauthorized)` when the caller is not the admin.
///
/// # Idempotency
/// If `user` does not currently hold `role` this is a no-op and returns
/// `Ok(())`.
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
