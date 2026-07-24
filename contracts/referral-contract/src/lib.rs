#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, vec, Address, Env, IntoVal, Symbol};

use shared::errors::Error;

const MAX_SUPPORTED_TIERS: u32 = 10;
const MIN_REWARD_CAP: i128 = 0;
const MAX_REWARD_CAP: i128 = 1_000_000_000_000_000_000;
const MIN_TIER_BPS: i128 = 0;
const MAX_TIER_BPS: i128 = 10_000;

type ContractResult<T> = core::result::Result<T, Error>;

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Treasury,
    MaxTiers,
    RewardCap,
    TierBps(u32),
    Referrer(Address),
    Accrued(Address),
    LifetimeAccrued(Address),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TierConfig {
    pub tier_bps: soroban_sdk::Vec<i128>,
    pub max_tiers: u32,
    pub reward_cap: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferrerSetEvent {
    pub referred_wallet: Address,
    pub referrer: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccruedRewardEvent {
    pub referred_wallet: Address,
    pub referrer: Address,
    pub tier: u32,
    pub amount: i128,
    pub accrued_balance: i128,
    pub lifetime_accrued: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimRewardsEvent {
    pub referrer: Address,
    pub amount: i128,
}

#[contract]
pub struct ReferralContract;

#[contractimpl]
impl ReferralContract {
    /// Initialise the contract, setting the admin address.
    pub fn initialize(env: Env, admin: Address) {
        shared::auth::set_admin(&env, &admin);
        env.storage().instance().set(&DataKey::MaxTiers, &1_u32);
        env.storage().instance().set(&DataKey::RewardCap, &0_i128);
        env.storage().instance().set(&DataKey::TierBps(1), &0_i128);
    }

    /// Configure the treasury contract used for referral reward claims.
    pub fn set_treasury(env: Env, caller: Address, treasury: Address) -> Result<(), Error> {
        require_admin(&env, &caller)?;
        env.storage().instance().set(&DataKey::Treasury, &treasury);
        env.events()
            .publish((shared::events::TREASURY_SET,), treasury);
        Ok(())
    }

    /// Read the configured treasury contract address.
    pub fn get_treasury(env: Env) -> Result<Address, Error> {
        read_treasury(&env)
    }

    /// Configure tier percentages and the lifetime cap enforced per referrer.
    pub fn set_tier_config(
        env: Env,
        caller: Address,
        tier_bps: soroban_sdk::Vec<i128>,
        max_tiers: u32,
        reward_cap: i128,
    ) -> Result<(), Error> {
        require_admin(&env, &caller)?;
        validate_tier_config(&tier_bps, max_tiers, reward_cap)?;

        env.storage().instance().set(&DataKey::MaxTiers, &max_tiers);
        env.storage()
            .instance()
            .set(&DataKey::RewardCap, &reward_cap);

        let mut tier = 1_u32;
        while tier <= max_tiers {
            let bps = tier_bps.get(tier - 1).ok_or(Error::InvalidArgument)?;
            env.storage().instance().set(&DataKey::TierBps(tier), &bps);
            tier += 1;
        }

        env.events().publish(
            (shared::events::TIER_CONFIG_SET,),
            TierConfig {
                tier_bps,
                max_tiers,
                reward_cap,
            },
        );
        Ok(())
    }

    /// Return the active tier configuration.
    pub fn get_tier_config(env: Env) -> Result<TierConfig, Error> {
        let max_tiers = read_max_tiers(&env)?;
        let mut tier_bps = soroban_sdk::Vec::new(&env);
        let mut tier = 1_u32;
        while tier <= max_tiers {
            tier_bps.push_back(read_tier_bps(&env, tier)?);
            tier += 1;
        }

        Ok(TierConfig {
            tier_bps,
            max_tiers,
            reward_cap: read_reward_cap(&env)?,
        })
    }

    /// Store a referral edge in the graph.
    pub fn set_referrer(
        env: Env,
        caller: Address,
        referred_wallet: Address,
        referrer: Address,
    ) -> Result<(), Error> {
        require_admin(&env, &caller)?;
        if referred_wallet == referrer || would_create_cycle(&env, &referred_wallet, &referrer) {
            return Err(Error::InvalidArgument);
        }

        env.storage()
            .instance()
            .set(&DataKey::Referrer(referred_wallet.clone()), &referrer);
        env.events().publish(
            (shared::events::REFERRER_SET,),
            ReferrerSetEvent {
                referred_wallet,
                referrer,
            },
        );
        Ok(())
    }

    /// Read a wallet's direct referrer, if one has been registered.
    pub fn get_referrer(env: Env, wallet: Address) -> Option<Address> {
        read_referrer(&env, &wallet)
    }

    /// Accrue referral rewards for a referred wallet and base transaction amount.
    pub fn accrue(
        env: Env,
        caller: Address,
        referred_wallet: Address,
        base_amount: i128,
    ) -> Result<i128, Error> {
        require_admin(&env, &caller)?;
        if base_amount <= 0 {
            return Err(Error::InvalidArgument);
        }

        let max_tiers = read_max_tiers(&env)?;
        let reward_cap = read_reward_cap(&env)?;
        let mut tier = 1_u32;
        let mut current_wallet = referred_wallet.clone();
        let mut total_credited = 0_i128;

        while tier <= max_tiers {
            let referrer = match read_referrer(&env, &current_wallet) {
                Some(address) => address,
                None => break,
            };
            let tier_bps = read_tier_bps(&env, tier)?;
            let commission = shared::math::bps_of(base_amount, tier_bps).ok_or(Error::Overflow)?;

            if commission > 0 {
                let credited = credit_referrer(
                    &env,
                    &referred_wallet,
                    &referrer,
                    tier,
                    commission,
                    reward_cap,
                )?;
                total_credited =
                    shared::math::safe_add(total_credited, credited).ok_or(Error::Overflow)?;
            }

            current_wallet = referrer;
            tier += 1;
        }

        Ok(total_credited)
    }

    /// Read the currently claimable accrued balance for a referrer.
    pub fn accrued_balance(env: Env, referrer: Address) -> i128 {
        read_accrued(&env, &referrer)
    }

    /// Read total lifetime rewards accrued for cap enforcement.
    pub fn lifetime_accrued(env: Env, referrer: Address) -> i128 {
        read_lifetime_accrued(&env, &referrer)
    }

    /// Claim accrued referral rewards from treasury. A second claim after a
    /// successful payout returns zero and leaves treasury untouched.
    pub fn claim_rewards(env: Env, referrer: Address) -> Result<i128, Error> {
        referrer.require_auth();
        let amount = read_accrued(&env, &referrer);
        if amount == 0 {
            return Ok(0);
        }

        let treasury = read_treasury(&env)?;
        call_treasury_distribute_reward(&env, &treasury, &referrer, amount)?;
        env.storage()
            .instance()
            .set(&DataKey::Accrued(referrer.clone()), &0_i128);
        env.events().publish(
            (shared::events::COMMISSION_PAID,),
            ClaimRewardsEvent { referrer, amount },
        );
        Ok(amount)
    }
}

fn require_admin(env: &Env, caller: &Address) -> ContractResult<()> {
    if *caller != shared::auth::get_admin(env) {
        return Err(Error::Unauthorized);
    }
    caller.require_auth();
    Ok(())
}

fn validate_tier_config(
    tier_bps: &soroban_sdk::Vec<i128>,
    max_tiers: u32,
    reward_cap: i128,
) -> ContractResult<()> {
    if max_tiers == 0 || max_tiers > MAX_SUPPORTED_TIERS || tier_bps.len() != max_tiers {
        return Err(Error::InvalidArgument);
    }
    if !(MIN_REWARD_CAP..=MAX_REWARD_CAP).contains(&reward_cap) {
        return Err(Error::InvalidArgument);
    }

    let mut total_bps = 0_i128;
    let mut index = 0_u32;
    while index < tier_bps.len() {
        let bps = tier_bps.get(index).ok_or(Error::InvalidArgument)?;
        if !(MIN_TIER_BPS..=MAX_TIER_BPS).contains(&bps) {
            return Err(Error::InvalidArgument);
        }
        total_bps = shared::math::safe_add(total_bps, bps).ok_or(Error::Overflow)?;
        index += 1;
    }

    if total_bps > MAX_TIER_BPS {
        return Err(Error::InvalidArgument);
    }
    Ok(())
}

fn would_create_cycle(env: &Env, referred_wallet: &Address, referrer: &Address) -> bool {
    let mut current_wallet = referrer.clone();
    let mut depth = 0_u32;
    while depth < MAX_SUPPORTED_TIERS {
        if current_wallet == *referred_wallet {
            return true;
        }
        current_wallet = match read_referrer(env, &current_wallet) {
            Some(address) => address,
            None => return false,
        };
        depth += 1;
    }
    false
}

fn read_treasury(env: &Env) -> ContractResult<Address> {
    env.storage()
        .instance()
        .get::<DataKey, Address>(&DataKey::Treasury)
        .ok_or(Error::NotFound)
}

fn read_max_tiers(env: &Env) -> ContractResult<u32> {
    env.storage()
        .instance()
        .get::<DataKey, u32>(&DataKey::MaxTiers)
        .ok_or(Error::NotFound)
}

fn read_reward_cap(env: &Env) -> ContractResult<i128> {
    env.storage()
        .instance()
        .get::<DataKey, i128>(&DataKey::RewardCap)
        .ok_or(Error::NotFound)
}

fn read_tier_bps(env: &Env, tier: u32) -> ContractResult<i128> {
    env.storage()
        .instance()
        .get::<DataKey, i128>(&DataKey::TierBps(tier))
        .ok_or(Error::NotFound)
}

fn read_referrer(env: &Env, wallet: &Address) -> Option<Address> {
    env.storage()
        .instance()
        .get::<DataKey, Address>(&DataKey::Referrer(wallet.clone()))
}

fn read_accrued(env: &Env, referrer: &Address) -> i128 {
    env.storage()
        .instance()
        .get::<DataKey, i128>(&DataKey::Accrued(referrer.clone()))
        .unwrap_or(0)
}

fn read_lifetime_accrued(env: &Env, referrer: &Address) -> i128 {
    env.storage()
        .instance()
        .get::<DataKey, i128>(&DataKey::LifetimeAccrued(referrer.clone()))
        .unwrap_or(0)
}

fn credit_referrer(
    env: &Env,
    referred_wallet: &Address,
    referrer: &Address,
    tier: u32,
    commission: i128,
    reward_cap: i128,
) -> ContractResult<i128> {
    let lifetime_accrued = read_lifetime_accrued(env, referrer);
    if lifetime_accrued >= reward_cap {
        return Ok(0);
    }

    let remaining_cap =
        shared::math::safe_sub(reward_cap, lifetime_accrued).ok_or(Error::Overflow)?;
    let credited = if commission > remaining_cap {
        remaining_cap
    } else {
        commission
    };
    if credited == 0 {
        return Ok(0);
    }

    let accrued_balance = read_accrued(env, referrer);
    let new_accrued_balance =
        shared::math::safe_add(accrued_balance, credited).ok_or(Error::Overflow)?;
    let new_lifetime_accrued =
        shared::math::safe_add(lifetime_accrued, credited).ok_or(Error::Overflow)?;

    env.storage()
        .instance()
        .set(&DataKey::Accrued(referrer.clone()), &new_accrued_balance);
    env.storage().instance().set(
        &DataKey::LifetimeAccrued(referrer.clone()),
        &new_lifetime_accrued,
    );
    env.events().publish(
        (shared::events::REFERRAL_ACCRUED,),
        AccruedRewardEvent {
            referred_wallet: referred_wallet.clone(),
            referrer: referrer.clone(),
            tier,
            amount: credited,
            accrued_balance: new_accrued_balance,
            lifetime_accrued: new_lifetime_accrued,
        },
    );

    Ok(credited)
}

fn call_treasury_distribute_reward(
    env: &Env,
    treasury: &Address,
    referrer: &Address,
    amount: i128,
) -> ContractResult<()> {
    let args = vec![env, referrer.clone().into_val(env), amount.into_val(env)];
    match env.try_invoke_contract::<(), Error>(
        treasury,
        &Symbol::new(env, "distribute_reward"),
        args,
    ) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(_)) => Err(Error::InvalidArgument),
        Err(Ok(error)) => Err(error),
        Err(Err(_)) => Err(Error::InvalidArgument),
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[contracttype]
    #[derive(Clone)]
    enum MockTreasuryKey {
        Rewards,
        Paid(Address),
    }

    #[contract]
    struct MockTreasury;

    #[contractimpl]
    impl MockTreasury {
        pub fn deposit_rewards(env: Env, amount: i128) -> Result<(), Error> {
            if amount < 0 {
                return Err(Error::InvalidArgument);
            }
            let balance = Self::rewards_balance(env.clone());
            let new_balance = shared::math::safe_add(balance, amount).ok_or(Error::Overflow)?;
            env.storage()
                .instance()
                .set(&MockTreasuryKey::Rewards, &new_balance);
            Ok(())
        }

        pub fn rewards_balance(env: Env) -> i128 {
            env.storage()
                .instance()
                .get::<MockTreasuryKey, i128>(&MockTreasuryKey::Rewards)
                .unwrap_or(0)
        }

        pub fn paid_to(env: Env, recipient: Address) -> i128 {
            env.storage()
                .instance()
                .get::<MockTreasuryKey, i128>(&MockTreasuryKey::Paid(recipient))
                .unwrap_or(0)
        }

        pub fn distribute_reward(env: Env, recipient: Address, amount: i128) -> Result<(), Error> {
            if amount <= 0 {
                return Err(Error::InvalidArgument);
            }
            let balance = Self::rewards_balance(env.clone());
            if balance < amount {
                return Err(Error::InsufficientBalance);
            }

            let paid = Self::paid_to(env.clone(), recipient.clone());
            let new_paid = shared::math::safe_add(paid, amount).ok_or(Error::Overflow)?;
            let new_balance = shared::math::safe_sub(balance, amount).ok_or(Error::Overflow)?;

            env.storage()
                .instance()
                .set(&MockTreasuryKey::Paid(recipient.clone()), &new_paid);
            env.storage()
                .instance()
                .set(&MockTreasuryKey::Rewards, &new_balance);
            env.events()
                .publish((shared::events::COMMISSION_PAID,), (recipient, amount));
            Ok(())
        }
    }

    fn setup() -> (
        Env,
        Address,
        Address,
        Address,
        Address,
        Address,
        Address,
        Address,
    ) {
        let env = Env::default();
        env.mock_all_auths();
        let referral_id = env.register(ReferralContract, ());
        let treasury_id = env.register(MockTreasury, ());
        let admin = Address::generate(&env);
        let referred = Address::generate(&env);
        let tier_one = Address::generate(&env);
        let tier_two = Address::generate(&env);
        let tier_three = Address::generate(&env);
        let tier_four = Address::generate(&env);

        let referral = ReferralContractClient::new(&env, &referral_id);
        let treasury = MockTreasuryClient::new(&env, &treasury_id);
        referral.initialize(&admin);
        referral.set_treasury(&admin, &treasury_id);
        treasury.deposit_rewards(&1_000_000_000);

        (
            env,
            referral_id,
            admin,
            referred,
            tier_one,
            tier_two,
            tier_three,
            tier_four,
        )
    }

    #[test]
    fn accrues_multi_tier_rewards_until_max_depth() {
        let (env, referral_id, admin, referred, tier_one, tier_two, tier_three, tier_four) =
            setup();
        let referral = ReferralContractClient::new(&env, &referral_id);

        referral.set_tier_config(
            &admin,
            &soroban_sdk::vec![&env, 1_000_i128, 500_i128],
            &2,
            &10_000,
        );
        referral.set_referrer(&admin, &referred, &tier_one);
        referral.set_referrer(&admin, &tier_one, &tier_two);
        referral.set_referrer(&admin, &tier_two, &tier_three);
        referral.set_referrer(&admin, &tier_three, &tier_four);

        assert_eq!(referral.accrue(&admin, &referred, &10_000), 1_500);
        assert_eq!(referral.accrued_balance(&tier_one), 1_000);
        assert_eq!(referral.accrued_balance(&tier_two), 500);
        assert_eq!(referral.accrued_balance(&tier_three), 0);
        assert_eq!(referral.accrued_balance(&tier_four), 0);
    }

    #[test]
    fn enforces_lifetime_reward_cap_per_referrer() {
        let (env, referral_id, admin, referred, tier_one, _tier_two, _tier_three, _tier_four) =
            setup();
        let referral = ReferralContractClient::new(&env, &referral_id);

        referral.set_tier_config(&admin, &soroban_sdk::vec![&env, 1_000_i128], &1, &1_100);
        referral.set_referrer(&admin, &referred, &tier_one);

        assert_eq!(referral.accrue(&admin, &referred, &10_000), 1_000);
        assert_eq!(referral.accrue(&admin, &referred, &10_000), 100);
        assert_eq!(referral.accrue(&admin, &referred, &10_000), 0);
        assert_eq!(referral.accrued_balance(&tier_one), 1_100);
        assert_eq!(referral.lifetime_accrued(&tier_one), 1_100);
    }

    #[test]
    fn claim_rewards_distributes_from_treasury_and_double_claim_pays_nothing() {
        let (env, referral_id, admin, referred, tier_one, _tier_two, _tier_three, _tier_four) =
            setup();
        let referral = ReferralContractClient::new(&env, &referral_id);
        let treasury_id = referral.get_treasury();
        let treasury = MockTreasuryClient::new(&env, &treasury_id);

        referral.set_tier_config(&admin, &soroban_sdk::vec![&env, 1_000_i128], &1, &10_000);
        referral.set_referrer(&admin, &referred, &tier_one);
        assert_eq!(referral.accrue(&admin, &referred, &10_000), 1_000);

        assert_eq!(referral.claim_rewards(&tier_one), 1_000);
        assert_eq!(referral.accrued_balance(&tier_one), 0);
        assert_eq!(treasury.paid_to(&tier_one), 1_000);
        assert_eq!(treasury.rewards_balance(), 999_999_000);

        assert_eq!(referral.claim_rewards(&tier_one), 0);
        assert_eq!(treasury.paid_to(&tier_one), 1_000);
        assert_eq!(treasury.rewards_balance(), 999_999_000);
    }

    #[test]
    fn accrual_math_rejects_overflow() {
        let (env, referral_id, admin, referred, tier_one, _tier_two, _tier_three, _tier_four) =
            setup();
        let referral = ReferralContractClient::new(&env, &referral_id);

        referral.set_tier_config(
            &admin,
            &soroban_sdk::vec![&env, 10_000_i128],
            &1,
            &MAX_REWARD_CAP,
        );
        referral.set_referrer(&admin, &referred, &tier_one);

        assert!(matches!(
            referral.try_accrue(&admin, &referred, &i128::MAX),
            Err(Ok(Error::Overflow))
        ));
        assert_eq!(referral.accrued_balance(&tier_one), 0);
    }

    #[test]
    fn rejects_invalid_admin_config_and_cycles() {
        let (env, referral_id, admin, referred, tier_one, tier_two, _tier_three, _tier_four) =
            setup();
        let referral = ReferralContractClient::new(&env, &referral_id);
        let attacker = Address::generate(&env);

        assert!(matches!(
            referral.try_set_tier_config(
                &attacker,
                &soroban_sdk::vec![&env, 1_000_i128],
                &1,
                &10_000,
            ),
            Err(Ok(Error::Unauthorized))
        ));
        assert!(matches!(
            referral.try_set_tier_config(
                &admin,
                &soroban_sdk::vec![&env, 9_000_i128, 2_000_i128],
                &2,
                &10_000,
            ),
            Err(Ok(Error::InvalidArgument))
        ));
        assert!(matches!(
            referral.try_set_referrer(&admin, &referred, &referred),
            Err(Ok(Error::InvalidArgument))
        ));

        referral.set_referrer(&admin, &referred, &tier_one);
        referral.set_referrer(&admin, &tier_one, &tier_two);
        assert!(matches!(
            referral.try_set_referrer(&admin, &tier_two, &referred),
            Err(Ok(Error::InvalidArgument))
        ));
    }
}
