#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

use shared::errors::Error;

const MIN_AID_DEFAULT_EXPIRY: i128 = 60;
const MAX_AID_DEFAULT_EXPIRY: i128 = 31_536_000;
const DEFAULT_AID_DEFAULT_EXPIRY: i128 = 604_800;

const MIN_TREASURY_WITHDRAWAL_LIMIT: i128 = 0;
const MAX_TREASURY_WITHDRAWAL_LIMIT: i128 = 1_000_000_000_000_000_000;
const DEFAULT_TREASURY_WITHDRAWAL_LIMIT: i128 = 100_000_000_000;

const MIN_REFERRAL_TIER_BPS: i128 = 0;
const MAX_REFERRAL_TIER_BPS: i128 = 10_000;
const DEFAULT_REFERRAL_TIER_ONE_BPS: i128 = 500;
const DEFAULT_REFERRAL_TIER_TWO_BPS: i128 = 250;
const DEFAULT_REFERRAL_TIER_THREE_BPS: i128 = 100;

const MIN_REFERRAL_MAX_TIERS: i128 = 1;
const MAX_REFERRAL_MAX_TIERS: i128 = 10;
const DEFAULT_REFERRAL_MAX_TIERS: i128 = 3;

const MIN_REFERRAL_REWARD_CAP: i128 = 0;
const MAX_REFERRAL_REWARD_CAP: i128 = 1_000_000_000_000_000_000;
const DEFAULT_REFERRAL_REWARD_CAP: i128 = 10_000_000_000;

type ContractResult<T> = core::result::Result<T, Error>;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParameterKey {
    AidDefaultExpiry,
    TreasuryWithdrawalLimit,
    ReferralTierBps(u32),
    ReferralMaxTiers,
    ReferralRewardCap,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Parameter(ParameterKey),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParameterBounds {
    pub min: i128,
    pub max: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParameterChangedEvent {
    pub key: ParameterKey,
    pub value: i128,
}

#[contract]
pub struct GovernanceContract;

#[contractimpl]
impl GovernanceContract {
    /// Initialise the contract, setting the admin address.
    pub fn initialize(env: Env, admin: Address) {
        shared::auth::set_admin(&env, &admin);
        seed_defaults(&env);
    }

    /// Update a protocol parameter. This repository does not yet contain
    /// proposal execution, so governance authority is represented by admin auth.
    pub fn set_param(
        env: Env,
        caller: Address,
        key: ParameterKey,
        value: i128,
    ) -> Result<(), Error> {
        require_governance(&env, &caller)?;
        validate_param(&key, value)?;
        write_param(&env, &key, value);
        env.events().publish(
            (shared::events::PARAMETER_CHANGED,),
            ParameterChangedEvent { key, value },
        );
        Ok(())
    }

    /// Read a protocol parameter from the typed catalog.
    pub fn get_param(env: Env, key: ParameterKey) -> Result<i128, Error> {
        read_param(&env, &key)
    }

    /// Return documented bounds for a protocol parameter.
    pub fn get_bounds(_env: Env, key: ParameterKey) -> Result<ParameterBounds, Error> {
        bounds_for(&key).ok_or(Error::InvalidArgument)
    }

    pub fn aid_default_expiry(env: Env) -> Result<i128, Error> {
        read_param(&env, &ParameterKey::AidDefaultExpiry)
    }

    pub fn treasury_withdrawal_limit(env: Env) -> Result<i128, Error> {
        read_param(&env, &ParameterKey::TreasuryWithdrawalLimit)
    }

    pub fn referral_tier_bps(env: Env, tier: u32) -> Result<i128, Error> {
        read_param(&env, &ParameterKey::ReferralTierBps(tier))
    }

    pub fn referral_max_tiers(env: Env) -> Result<i128, Error> {
        read_param(&env, &ParameterKey::ReferralMaxTiers)
    }

    pub fn referral_reward_cap(env: Env) -> Result<i128, Error> {
        read_param(&env, &ParameterKey::ReferralRewardCap)
    }
}

fn seed_defaults(env: &Env) {
    write_param(
        env,
        &ParameterKey::AidDefaultExpiry,
        DEFAULT_AID_DEFAULT_EXPIRY,
    );
    write_param(
        env,
        &ParameterKey::TreasuryWithdrawalLimit,
        DEFAULT_TREASURY_WITHDRAWAL_LIMIT,
    );
    write_param(
        env,
        &ParameterKey::ReferralTierBps(1),
        DEFAULT_REFERRAL_TIER_ONE_BPS,
    );
    write_param(
        env,
        &ParameterKey::ReferralTierBps(2),
        DEFAULT_REFERRAL_TIER_TWO_BPS,
    );
    write_param(
        env,
        &ParameterKey::ReferralTierBps(3),
        DEFAULT_REFERRAL_TIER_THREE_BPS,
    );
    write_param(
        env,
        &ParameterKey::ReferralMaxTiers,
        DEFAULT_REFERRAL_MAX_TIERS,
    );
    write_param(
        env,
        &ParameterKey::ReferralRewardCap,
        DEFAULT_REFERRAL_REWARD_CAP,
    );
}

fn require_governance(env: &Env, caller: &Address) -> ContractResult<()> {
    if *caller != shared::auth::get_admin(env) {
        return Err(Error::Unauthorized);
    }
    caller.require_auth();
    Ok(())
}

fn validate_param(key: &ParameterKey, value: i128) -> ContractResult<()> {
    let bounds = bounds_for(key).ok_or(Error::InvalidArgument)?;
    if value < bounds.min || value > bounds.max {
        return Err(Error::InvalidArgument);
    }
    Ok(())
}

fn bounds_for(key: &ParameterKey) -> Option<ParameterBounds> {
    match key {
        ParameterKey::AidDefaultExpiry => Some(ParameterBounds {
            min: MIN_AID_DEFAULT_EXPIRY,
            max: MAX_AID_DEFAULT_EXPIRY,
        }),
        ParameterKey::TreasuryWithdrawalLimit => Some(ParameterBounds {
            min: MIN_TREASURY_WITHDRAWAL_LIMIT,
            max: MAX_TREASURY_WITHDRAWAL_LIMIT,
        }),
        ParameterKey::ReferralTierBps(tier) => {
            if *tier == 0 || i128::from(*tier) > MAX_REFERRAL_MAX_TIERS {
                None
            } else {
                Some(ParameterBounds {
                    min: MIN_REFERRAL_TIER_BPS,
                    max: MAX_REFERRAL_TIER_BPS,
                })
            }
        }
        ParameterKey::ReferralMaxTiers => Some(ParameterBounds {
            min: MIN_REFERRAL_MAX_TIERS,
            max: MAX_REFERRAL_MAX_TIERS,
        }),
        ParameterKey::ReferralRewardCap => Some(ParameterBounds {
            min: MIN_REFERRAL_REWARD_CAP,
            max: MAX_REFERRAL_REWARD_CAP,
        }),
    }
}

fn storage_key(key: &ParameterKey) -> DataKey {
    DataKey::Parameter(key.clone())
}

fn write_param(env: &Env, key: &ParameterKey, value: i128) {
    env.storage().instance().set(&storage_key(key), &value);
}

fn read_param(env: &Env, key: &ParameterKey) -> ContractResult<i128> {
    env.storage()
        .instance()
        .get::<DataKey, i128>(&storage_key(key))
        .ok_or(Error::NotFound)
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use soroban_sdk::testutils::{Address as _, Events};
    use soroban_sdk::{Env, IntoVal, TryFromVal};

    fn setup() -> (Env, Address, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(GovernanceContract, ());
        let client = GovernanceContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let other = Address::generate(&env);
        client.initialize(&admin);
        (env, contract_id, admin, other)
    }

    #[test]
    fn initializes_documented_catalog_defaults() {
        let (env, contract_id, _admin, _other) = setup();
        let client = GovernanceContractClient::new(&env, &contract_id);

        assert_eq!(client.aid_default_expiry(), DEFAULT_AID_DEFAULT_EXPIRY);
        assert_eq!(
            client.treasury_withdrawal_limit(),
            DEFAULT_TREASURY_WITHDRAWAL_LIMIT
        );
        assert_eq!(client.referral_tier_bps(&1), DEFAULT_REFERRAL_TIER_ONE_BPS);
        assert_eq!(client.referral_tier_bps(&2), DEFAULT_REFERRAL_TIER_TWO_BPS);
        assert_eq!(
            client.referral_tier_bps(&3),
            DEFAULT_REFERRAL_TIER_THREE_BPS
        );
        assert_eq!(client.referral_max_tiers(), DEFAULT_REFERRAL_MAX_TIERS);
        assert_eq!(client.referral_reward_cap(), DEFAULT_REFERRAL_REWARD_CAP);
    }

    #[test]
    fn authorized_update_changes_parameter_and_emits_event() {
        let (env, contract_id, admin, _other) = setup();
        let client = GovernanceContractClient::new(&env, &contract_id);
        let key = ParameterKey::ReferralTierBps(2);
        let value = 350;

        client.set_param(&admin, &key, &value);
        let events = env.events().all();
        let last = events.last().unwrap();
        assert_eq!(last.1, (shared::events::PARAMETER_CHANGED,).into_val(&env));
        assert_eq!(
            ParameterChangedEvent::try_from_val(&env, &last.2).unwrap(),
            ParameterChangedEvent { key, value }
        );

        assert_eq!(client.get_param(&ParameterKey::ReferralTierBps(2)), value);
        assert_eq!(client.referral_tier_bps(&2), value);
    }

    #[test]
    fn unauthorized_update_is_rejected() {
        let (env, contract_id, _admin, other) = setup();
        let client = GovernanceContractClient::new(&env, &contract_id);

        assert!(matches!(
            client.try_set_param(&other, &ParameterKey::AidDefaultExpiry, &120),
            Err(Ok(Error::Unauthorized))
        ));
        assert_eq!(client.aid_default_expiry(), DEFAULT_AID_DEFAULT_EXPIRY);
    }

    #[test]
    fn out_of_range_values_are_rejected() {
        let (env, contract_id, admin, _other) = setup();
        let client = GovernanceContractClient::new(&env, &contract_id);

        assert!(matches!(
            client.try_set_param(
                &admin,
                &ParameterKey::AidDefaultExpiry,
                &(MAX_AID_DEFAULT_EXPIRY + 1),
            ),
            Err(Ok(Error::InvalidArgument))
        ));
        assert!(matches!(
            client.try_set_param(&admin, &ParameterKey::ReferralTierBps(1), &10_001),
            Err(Ok(Error::InvalidArgument))
        ));
        assert!(matches!(
            client.try_set_param(&admin, &ParameterKey::ReferralTierBps(0), &100),
            Err(Ok(Error::InvalidArgument))
        ));
        assert!(matches!(
            client.try_set_param(&admin, &ParameterKey::ReferralMaxTiers, &0),
            Err(Ok(Error::InvalidArgument))
        ));
    }

    #[test]
    fn bounds_are_readable_for_dependent_contracts() {
        let (env, contract_id, _admin, _other) = setup();
        let client = GovernanceContractClient::new(&env, &contract_id);

        assert_eq!(
            client.get_bounds(&ParameterKey::ReferralTierBps(1)),
            ParameterBounds {
                min: MIN_REFERRAL_TIER_BPS,
                max: MAX_REFERRAL_TIER_BPS,
            }
        );
        assert!(matches!(
            client.try_get_bounds(&ParameterKey::ReferralTierBps(11)),
            Err(Ok(Error::InvalidArgument))
        ));
    }
}
