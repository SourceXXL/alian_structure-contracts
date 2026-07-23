use soroban_sdk::{symbol_short, Address, Env, Symbol};

/// Storage key for the admin address.
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
