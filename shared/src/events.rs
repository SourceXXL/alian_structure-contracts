use soroban_sdk::{symbol_short, Env, Symbol};

/// Event topic constants shared across contracts.
pub const AID_CREATED: Symbol = symbol_short!("aid_crt");
pub const AID_CLAIMED: Symbol = symbol_short!("aid_clm");
pub const AID_SETTLED: Symbol = symbol_short!("aid_stl");
pub const AID_REFUNDED: Symbol = symbol_short!("aid_ref");
pub const COMMISSION_PAID: Symbol = symbol_short!("com_paid");
pub const REFERRAL_ACCRUED: Symbol = symbol_short!("ref_acc");
pub const REFERRER_SET: Symbol = symbol_short!("ref_set");
pub const TIER_CONFIG_SET: Symbol = symbol_short!("tier_cfg");
pub const TREASURY_SET: Symbol = symbol_short!("trs_set");
pub const TREASURY_DEPOSIT: Symbol = symbol_short!("t_dep");
pub const TREASURY_WITHDRAW: Symbol = symbol_short!("t_wdw");
pub const TREASURY_EMERGENCY_WITHDRAW: Symbol = symbol_short!("t_emrg");
pub const PARAMETER_CHANGED: Symbol = symbol_short!("param_chg");
pub const CONTRACT_PAUSED: Symbol = symbol_short!("paused");
pub const CONTRACT_RESUMED: Symbol = symbol_short!("resumed");
pub const CONTRACT_UPGRADED: Symbol = symbol_short!("upgraded");

/// Emit an event with a single data value.
pub fn emit<T: soroban_sdk::IntoVal<Env, soroban_sdk::Val>>(env: &Env, topic: Symbol, data: T) {
    env.events().publish((topic,), data);
}
