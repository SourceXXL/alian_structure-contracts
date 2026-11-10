#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, Symbol};

use shared::auth::{self, Role};
use shared::errors::Error;
use shared::events;
use shared::events::{emit_action_executed, emit_module_initialized};
use shared::storage::{instance_get, instance_set, persistent_set};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

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

const MIN_QUORUM_PERCENTAGE_BPS: i128 = 1;
const MAX_QUORUM_PERCENTAGE_BPS: i128 = 10_000;
const DEFAULT_QUORUM_PERCENTAGE_BPS: i128 = 5_000;

type ContractResult<T> = core::result::Result<T, Error>;

// ---------------------------------------------------------------------------
// Storage key symbols (all <= 9 chars for symbol_short!)
// ---------------------------------------------------------------------------

const KEY_THRESHOLD: Symbol = symbol_short!("thresh");
const KEY_ADMIN_SET: Symbol = symbol_short!("adm_set");
const KEY_PROP_CNT: Symbol = symbol_short!("prop_cnt");
const KEY_PROPOSAL: Symbol = symbol_short!("proposal");
const KEY_APPROVAL: Symbol = symbol_short!("approval");

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParameterKey {
    AidDefaultExpiry,
    TreasuryWithdrawalLimit,
    ReferralTierBps(u32),
    ReferralMaxTiers,
    ReferralRewardCap,
    QuorumPercentageBps,
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

/// The possible actions that a multi-sig proposal can execute.
///
/// Uses tuple-style variants because Soroban `contracttype` does not support
/// named fields in enums.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProposalAction {
    /// Grant `Role` to `Address`.
    GrantRole(Address, Role),
    /// Revoke `Role` from `Address`.
    RevokeRole(Address, Role),
    /// Update a protocol parameter: `(key, value)`.
    SetParameter(ParameterKey, i128),
    /// Pause the contract.
    Pause,
    /// Unpause the contract.
    Unpause,
}

/// Status of a proposal.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProposalStatus {
    Pending,
    Executed,
}

/// A multi-sig proposal.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Proposal {
    pub id: u64,
    pub proposer: Address,
    pub action: ProposalAction,
    pub approval_count: u32,
    pub status: ProposalStatus,
    pub created_at: u64,
}

// ---------------------------------------------------------------------------
// Contract
// ---------------------------------------------------------------------------

#[contract]
pub struct GovernanceContract;

#[contractimpl]
impl GovernanceContract {
    /// Initialise the contract: sets the admin address, seeds parameter
    /// defaults, and configures the initial multi-sig threshold.
    ///
    /// # Arguments
    /// * `admin` — The initial admin used as the "first admin" for legacy
    ///   `set_admin` / `require_admin` checks.
    /// * `threshold` — The minimum number of approvals (M) required to
    ///   execute a proposal. Must be >= 1 and <= admin_set length.
    /// * `admin_set` — The list of addresses that can approve proposals.
    ///   Must contain at least `threshold` addresses.
    ///   All addresses in this set are granted the `Admin` role.
    pub fn initialize(
        env: Env,
        admin: Address,
        threshold: u32,
        admin_set: soroban_sdk::Vec<Address>,
    ) -> Result<(), Error> {
        if threshold < 1 {
            return Err(Error::InvalidArgument);
        }
        if admin_set.len() < threshold {
            return Err(Error::InvalidArgument);
        }

        shared::auth::set_admin(&env, &admin);
        // Grant the Admin role to every address in the admin set so they
        // can propose, approve, and execute.
        let mut i: u32 = 0;
        while i < admin_set.len() {
            let addr = admin_set.get(i).unwrap();
            persistent_set(&env, &shared::auth::DataKey::Role(addr, Role::Admin), &true);
            i += 1;
        }

        // Store the threshold and admin set.
        instance_set(&env, &KEY_THRESHOLD, &threshold);
        instance_set(&env, &KEY_ADMIN_SET, &admin_set);

        // Seed parameter defaults.
        seed_defaults(&env);
        emit_module_initialized(
            &env,
            symbol_short!("gov"),
            1,
            &admin,
            env.ledger().timestamp(),
        );

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Role management
    // -----------------------------------------------------------------------

    /// Grants `role` to `user`. Only callable by an admin.
    pub fn grant_role(env: Env, caller: Address, user: Address, role: Role) -> Result<(), Error> {
        // Auth check only once — avoid double-auth from calling
        // `auth::grant_role` which would call `require_auth` again.
        require_admin_role(&env, &caller)?;
        persistent_set(
            &env,
            &shared::auth::DataKey::Role(user.clone(), role.clone()),
            &true,
        );
        events::emit_role_granted(
            &env,
            &caller,
            &user,
            role_name(&role),
            env.ledger().timestamp(),
        );
        Ok(())
    }

    /// Revokes `role` from `user`. Only callable by an admin.
    pub fn revoke_role(env: Env, caller: Address, user: Address, role: Role) -> Result<(), Error> {
        require_admin_role(&env, &caller)?;
        shared::storage::persistent_remove(
            &env,
            &shared::auth::DataKey::Role(user.clone(), role.clone()),
        );
        events::emit_role_revoked(
            &env,
            &caller,
            &user,
            role_name(&role),
            env.ledger().timestamp(),
        );
        Ok(())
    }

    /// Returns `true` if `user` holds `role`.
    pub fn has_role(env: Env, user: Address, role: Role) -> bool {
        auth::has_role(&env, &user, role)
    }

    // -----------------------------------------------------------------------
    // Admin set management
    // -----------------------------------------------------------------------

    /// Updates the set of addresses that can participate in multi-sig
    /// approvals. Also updates the threshold. Only callable by an admin.
    pub fn set_admin_set(
        env: Env,
        caller: Address,
        new_admin_set: soroban_sdk::Vec<Address>,
        new_threshold: u32,
    ) -> Result<(), Error> {
        require_admin_role(&env, &caller)?;
        if new_threshold < 1 || new_admin_set.len() < new_threshold {
            return Err(Error::InvalidArgument);
        }
        instance_set(&env, &KEY_THRESHOLD, &new_threshold);
        instance_set(&env, &KEY_ADMIN_SET, &new_admin_set);
        Ok(())
    }

    /// Returns the current multi-sig threshold (M).
    pub fn get_threshold(env: Env) -> u32 {
        instance_get(&env, &KEY_THRESHOLD).unwrap_or(0)
    }

    /// Returns the current admin set (N).
    pub fn get_admin_set(env: Env) -> soroban_sdk::Vec<Address> {
        instance_get(&env, &KEY_ADMIN_SET).unwrap_or(soroban_sdk::Vec::new(&env))
    }

    /// Returns `true` if the contract is currently paused.
    pub fn is_paused(env: Env) -> bool {
        shared::storage::is_paused(&env)
    }

    // -----------------------------------------------------------------------
    // Multi-sig proposal flow
    // -----------------------------------------------------------------------

    /// Creates a new proposal. Only callable by an admin.
    ///
    /// Returns the newly created proposal ID.
    pub fn propose(env: Env, caller: Address, action: ProposalAction) -> Result<u64, Error> {
        require_admin_role(&env, &caller)?;

        let proposal_id: u64 = instance_get(&env, &KEY_PROP_CNT).unwrap_or(0);
        let new_id = proposal_id.checked_add(1).ok_or(Error::Overflow)?;
        instance_set(&env, &KEY_PROP_CNT, &new_id);

        let proposal = Proposal {
            id: new_id,
            proposer: caller.clone(),
            action: action.clone(),
            approval_count: 0,
            status: ProposalStatus::Pending,
            created_at: env.ledger().timestamp(),
        };

        let proposal_key = (KEY_PROPOSAL, new_id);
        instance_set(&env, &proposal_key, &proposal);

        events::emit_proposal_created(
            &env,
            new_id,
            &caller,
            action_symbol(&action),
            env.ledger().timestamp(),
        );

        Ok(new_id)
    }

    /// Approves a pending proposal. Only callable by an admin.
    ///
    /// Each admin may only approve a proposal once.
    pub fn approve(env: Env, caller: Address, proposal_id: u64) -> Result<(), Error> {
        require_admin_role(&env, &caller)?;

        let proposal_key = (KEY_PROPOSAL, proposal_id);
        let mut proposal: Proposal =
            instance_get(&env, &proposal_key).ok_or(Error::ProposalNotFound)?;

        if proposal.status != ProposalStatus::Pending {
            return Err(Error::AlreadyExecuted);
        }

        // Check for duplicate approval.
        let approval_key = (KEY_APPROVAL, proposal_id, caller.clone());
        if instance_get::<_, bool>(&env, &approval_key).unwrap_or(false) {
            return Err(Error::AlreadyApproved);
        }

        instance_set(&env, &approval_key, &true);
        proposal.approval_count = proposal
            .approval_count
            .checked_add(1)
            .ok_or(Error::Overflow)?;
        instance_set(&env, &proposal_key, &proposal);

        events::emit_proposal_approved(
            &env,
            proposal_id,
            &caller,
            proposal.approval_count,
            env.ledger().timestamp(),
        );

        Ok(())
    }

    /// Executes a proposal once the approval threshold is met.
    /// Only callable by an admin.
    ///
    /// # Errors
    /// * `Error::ProposalNotFound` — proposal does not exist.
    /// * `Error::AlreadyExecuted` — proposal already executed.
    /// * `Error::BelowThreshold` — approval count < threshold.
    pub fn execute(env: Env, caller: Address, proposal_id: u64) -> Result<(), Error> {
        require_admin_role(&env, &caller)?;

        let proposal_key = (KEY_PROPOSAL, proposal_id);
        let mut proposal: Proposal =
            instance_get(&env, &proposal_key).ok_or(Error::ProposalNotFound)?;

        if proposal.status == ProposalStatus::Executed {
            return Err(Error::AlreadyExecuted);
        }

        let threshold: u32 = instance_get(&env, &KEY_THRESHOLD).unwrap_or(0);
        if proposal.approval_count < threshold {
            return Err(Error::BelowThreshold);
        }

        execute_action(&env, &proposal.action)?;

        proposal.status = ProposalStatus::Executed;
        instance_set(&env, &proposal_key, &proposal);

        events::emit_proposal_executed(
            &env,
            proposal_id,
            &caller,
            proposal.approval_count,
            env.ledger().timestamp(),
        );

        Ok(())
    }

    /// Returns a proposal by ID.
    pub fn get_proposal(env: Env, proposal_id: u64) -> Result<Proposal, Error> {
        let proposal_key = (KEY_PROPOSAL, proposal_id);
        instance_get(&env, &proposal_key).ok_or(Error::ProposalNotFound)
    }

    // -----------------------------------------------------------------------
    // Parameter management
    // -----------------------------------------------------------------------

    /// Update a protocol parameter. Admin only.
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
        emit_action_executed(
            &env,
            symbol_short!("gov"),
            symbol_short!("set_param"),
            &caller,
            true,
            env.ledger().timestamp(),
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

    pub fn quorum_percentage_bps(env: Env) -> Result<i128, Error> {
        read_param(&env, &ParameterKey::QuorumPercentageBps)
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Requires the caller to hold the `Admin` role.
fn require_admin_role(env: &Env, caller: &Address) -> ContractResult<()> {
    auth::require_role(env, caller, Role::Admin)
}

/// Requires the caller to be the admin (for backward-compatible parameter
/// management).
fn require_governance(env: &Env, caller: &Address) -> ContractResult<()> {
    if *caller != shared::auth::get_admin(env) {
        return Err(Error::Unauthorized);
    }
    caller.require_auth();
    Ok(())
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
    write_param(
        env,
        &ParameterKey::QuorumPercentageBps,
        DEFAULT_QUORUM_PERCENTAGE_BPS,
    );
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
        ParameterKey::QuorumPercentageBps => Some(ParameterBounds {
            min: MIN_QUORUM_PERCENTAGE_BPS,
            max: MAX_QUORUM_PERCENTAGE_BPS,
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

fn action_symbol(action: &ProposalAction) -> Symbol {
    match action {
        ProposalAction::GrantRole(..) => symbol_short!("grt_role"),
        ProposalAction::RevokeRole(..) => symbol_short!("rvk_role"),
        ProposalAction::SetParameter(..) => symbol_short!("set_param"),
        ProposalAction::Pause => symbol_short!("pause"),
        ProposalAction::Unpause => symbol_short!("unpause"),
    }
}

fn role_name(role: &Role) -> Symbol {
    match role {
        Role::Admin => symbol_short!("admin"),
        Role::Upgrader => symbol_short!("upgrader"),
        Role::TreasuryManager => symbol_short!("treas_m"),
        Role::Pauser => symbol_short!("pauser"),
        Role::ReferralManager => symbol_short!("refrl_m"),
        Role::OracleSigner => symbol_short!("oracle_s"),
    }
}

fn execute_action(env: &Env, action: &ProposalAction) -> ContractResult<()> {
    match action {
        ProposalAction::GrantRole(user, role) => {
            persistent_set(
                env,
                &shared::auth::DataKey::Role(user.clone(), role.clone()),
                &true,
            );
        }
        ProposalAction::RevokeRole(user, role) => {
            shared::storage::persistent_remove(
                env,
                &shared::auth::DataKey::Role(user.clone(), role.clone()),
            );
        }
        ProposalAction::SetParameter(key, value) => {
            validate_param(key, *value)?;
            write_param(env, key, *value);
        }
        ProposalAction::Pause => {
            shared::storage::set_paused(env, true);
        }
        ProposalAction::Unpause => {
            shared::storage::set_paused(env, false);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use soroban_sdk::testutils::{Address as _, Events};
    use soroban_sdk::{Env, IntoVal, TryFromVal};

    /// Creates a 2-of-2 governance setup. Returns (env, client, admin).
    fn setup() -> (Env, GovernanceContractClient<'static>, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, GovernanceContract);
        let client = GovernanceContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let other = Address::generate(&env);
        let mut admin_set: soroban_sdk::Vec<Address> = soroban_sdk::Vec::new(&env);
        admin_set.push_back(admin.clone());
        admin_set.push_back(other.clone());
        client.initialize(&admin, &2, &admin_set);
        (env, client, admin)
    }

    fn make_admin_set(env: &Env, n: usize) -> soroban_sdk::Vec<Address> {
        let mut v = soroban_sdk::Vec::new(env);
        for _ in 0..n {
            v.push_back(Address::generate(env));
        }
        v
    }

    // -----------------------------------------------------------------------
    // initialize
    // -----------------------------------------------------------------------

    #[test]
    fn initialize_sets_admin_and_seeds_defaults() {
        let (_env, client, admin) = setup();

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
        assert!(client.has_role(&admin, &Role::Admin));
    }

    #[test]
    fn initialize_rejects_zero_threshold() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, GovernanceContract);
        let client = GovernanceContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let mut admin_set: soroban_sdk::Vec<Address> = soroban_sdk::Vec::new(&env);
        admin_set.push_back(admin.clone());
        let result = client.try_initialize(&admin, &0, &admin_set);
        assert_eq!(result, Err(Ok(Error::InvalidArgument)));
    }

    #[test]
    fn initialize_rejects_threshold_exceeding_admin_set_size() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, GovernanceContract);
        let client = GovernanceContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let mut admin_set: soroban_sdk::Vec<Address> = soroban_sdk::Vec::new(&env);
        admin_set.push_back(admin.clone());
        let result = client.try_initialize(&admin, &2, &admin_set);
        assert_eq!(result, Err(Ok(Error::InvalidArgument)));
    }

    // -----------------------------------------------------------------------
    // grant_role / revoke_role
    // -----------------------------------------------------------------------

    #[test]
    fn admin_can_grant_role() {
        let (env, client, admin) = setup();
        let user = Address::generate(&env);

        client.grant_role(&admin, &user, &Role::Upgrader);
        assert!(client.has_role(&user, &Role::Upgrader));
    }

    #[test]
    fn non_admin_cannot_grant_role() {
        let (env, client, _admin) = setup();
        let stranger = Address::generate(&env);
        let user = Address::generate(&env);

        let result = client.try_grant_role(&stranger, &user, &Role::Upgrader);
        assert_eq!(result, Err(Ok(Error::Unauthorized)));
        assert!(!client.has_role(&user, &Role::Upgrader));
    }

    #[test]
    fn admin_can_revoke_role() {
        let (env, client, admin) = setup();
        let user = Address::generate(&env);

        client.grant_role(&admin, &user, &Role::Pauser);
        assert!(client.has_role(&user, &Role::Pauser));

        client.revoke_role(&admin, &user, &Role::Pauser);
        assert!(!client.has_role(&user, &Role::Pauser));
    }

    #[test]
    fn non_admin_cannot_revoke_role() {
        let (env, client, admin) = setup();
        let stranger = Address::generate(&env);
        let user = Address::generate(&env);

        client.grant_role(&admin, &user, &Role::Pauser);

        let result = client.try_revoke_role(&stranger, &user, &Role::Pauser);
        assert_eq!(result, Err(Ok(Error::Unauthorized)));
        assert!(client.has_role(&user, &Role::Pauser));
    }

    // -----------------------------------------------------------------------
    // propose / approve / execute
    // -----------------------------------------------------------------------

    #[test]
    fn propose_creates_proposal_with_zero_approvals() {
        let (env, client, admin) = setup();

        let action = ProposalAction::Pause;
        let proposal_id = client.propose(&admin, &action);

        let proposal = client.get_proposal(&proposal_id);
        assert_eq!(proposal.id, 1);
        assert_eq!(proposal.proposer, admin);
        assert_eq!(proposal.action, ProposalAction::Pause);
        assert_eq!(proposal.approval_count, 0);
        assert_eq!(proposal.status, ProposalStatus::Pending);
    }

    #[test]
    fn non_admin_cannot_propose() {
        let (env, client, _admin) = setup();
        let stranger = Address::generate(&env);

        let action = ProposalAction::Pause;
        let result = client.try_propose(&stranger, &action);
        assert_eq!(result, Err(Ok(Error::Unauthorized)));
    }

    #[test]
    fn approve_records_and_increments_count() {
        let (env, client, admin) = setup();
        let admins = client.get_admin_set();
        let second_admin = admins.get(1).unwrap();

        let action = ProposalAction::Pause;
        let proposal_id = client.propose(&admin, &action);

        client.approve(&admin, &proposal_id);
        let proposal = client.get_proposal(&proposal_id);
        assert_eq!(proposal.approval_count, 1);

        client.approve(&second_admin, &proposal_id);
        let proposal = client.get_proposal(&proposal_id);
        assert_eq!(proposal.approval_count, 2);
    }

    #[test]
    fn duplicate_approval_is_rejected() {
        let (env, client, admin) = setup();

        let action = ProposalAction::Pause;
        let proposal_id = client.propose(&admin, &action);

        client.approve(&admin, &proposal_id);
        let result = client.try_approve(&admin, &proposal_id);
        assert_eq!(result, Err(Ok(Error::AlreadyApproved)));
    }

    #[test]
    fn execute_fails_below_threshold() {
        let (env, client, admin) = setup();

        let action = ProposalAction::Pause;
        let proposal_id = client.propose(&admin, &action);

        client.approve(&admin, &proposal_id);
        let result = client.try_execute(&admin, &proposal_id);
        assert_eq!(result, Err(Ok(Error::BelowThreshold)));
    }

    #[test]
    fn execute_succeeds_at_threshold() {
        let (env, client, admin) = setup();
        let admins = client.get_admin_set();
        let second_admin = admins.get(1).unwrap();

        let action = ProposalAction::Pause;
        let proposal_id = client.propose(&admin, &action);

        client.approve(&admin, &proposal_id);
        client.approve(&second_admin, &proposal_id);

        client.execute(&admin, &proposal_id);

        let proposal = client.get_proposal(&proposal_id);
        assert_eq!(proposal.status, ProposalStatus::Executed);
        assert!(client.is_paused());
    }

    #[test]
    fn execute_fails_on_already_executed() {
        let (env, client, admin) = setup();
        let admins = client.get_admin_set();
        let second_admin = admins.get(1).unwrap();

        let action = ProposalAction::Pause;
        let proposal_id = client.propose(&admin, &action);

        client.approve(&admin, &proposal_id);
        client.approve(&second_admin, &proposal_id);
        client.execute(&admin, &proposal_id);

        let result = client.try_execute(&admin, &proposal_id);
        assert_eq!(result, Err(Ok(Error::AlreadyExecuted)));
    }

    #[test]
    fn execute_with_more_approvals_than_threshold() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, GovernanceContract);
        let client = GovernanceContractClient::new(&env, &contract_id);
        let a1 = Address::generate(&env);
        let a2 = Address::generate(&env);
        let a3 = Address::generate(&env);
        let mut admin_set: soroban_sdk::Vec<Address> = soroban_sdk::Vec::new(&env);
        admin_set.push_back(a1.clone());
        admin_set.push_back(a2.clone());
        admin_set.push_back(a3.clone());
        client.initialize(&a1, &2, &admin_set);

        let action = ProposalAction::Pause;
        let proposal_id = client.propose(&a1, &action);

        client.approve(&a1, &proposal_id);
        client.approve(&a2, &proposal_id);
        client.approve(&a3, &proposal_id);

        client.execute(&a1, &proposal_id);

        let proposal = client.get_proposal(&proposal_id);
        assert_eq!(proposal.status, ProposalStatus::Executed);
        assert!(client.is_paused());
    }

    #[test]
    fn non_admin_cannot_approve() {
        let (env, client, admin) = setup();
        let stranger = Address::generate(&env);

        let action = ProposalAction::Pause;
        let proposal_id = client.propose(&admin, &action);

        let result = client.try_approve(&stranger, &proposal_id);
        assert_eq!(result, Err(Ok(Error::Unauthorized)));
    }

    #[test]
    fn non_admin_cannot_execute() {
        let (env, client, admin) = setup();
        let admins = client.get_admin_set();
        let second_admin = admins.get(1).unwrap();
        let stranger = Address::generate(&env);

        let action = ProposalAction::Pause;
        let proposal_id = client.propose(&admin, &action);

        client.approve(&admin, &proposal_id);
        client.approve(&second_admin, &proposal_id);

        let result = client.try_execute(&stranger, &proposal_id);
        assert_eq!(result, Err(Ok(Error::Unauthorized)));
    }

    // -----------------------------------------------------------------------
    // Proposal actions
    // -----------------------------------------------------------------------

    #[test]
    fn grant_role_via_proposal() {
        let (env, client, admin) = setup();
        let admins = client.get_admin_set();
        let second_admin = admins.get(1).unwrap();
        let user = Address::generate(&env);

        let action = ProposalAction::GrantRole(user.clone(), Role::TreasuryManager);
        let proposal_id = client.propose(&admin, &action);

        client.approve(&admin, &proposal_id);
        client.approve(&second_admin, &proposal_id);
        client.execute(&admin, &proposal_id);

        assert!(client.has_role(&user, &Role::TreasuryManager));
    }

    #[test]
    fn revoke_role_via_proposal() {
        let (env, client, admin) = setup();
        let admins = client.get_admin_set();
        let second_admin = admins.get(1).unwrap();
        let user = Address::generate(&env);

        client.grant_role(&admin, &user, &Role::TreasuryManager);
        assert!(client.has_role(&user, &Role::TreasuryManager));

        let action = ProposalAction::RevokeRole(user.clone(), Role::TreasuryManager);
        let proposal_id = client.propose(&admin, &action);

        client.approve(&admin, &proposal_id);
        client.approve(&second_admin, &proposal_id);
        client.execute(&admin, &proposal_id);

        assert!(!client.has_role(&user, &Role::TreasuryManager));
    }

    #[test]
    fn set_parameter_via_proposal() {
        let (env, client, admin) = setup();
        let admins = client.get_admin_set();
        let second_admin = admins.get(1).unwrap();

        let action = ProposalAction::SetParameter(ParameterKey::AidDefaultExpiry, 120);
        let proposal_id = client.propose(&admin, &action);

        client.approve(&admin, &proposal_id);
        client.approve(&second_admin, &proposal_id);
        client.execute(&admin, &proposal_id);

        assert_eq!(client.aid_default_expiry(), 120);
    }

    #[test]
    fn pause_via_proposal() {
        let (env, client, admin) = setup();
        let admins = client.get_admin_set();
        let second_admin = admins.get(1).unwrap();

        let proposal_id = client.propose(&admin, &ProposalAction::Pause);

        client.approve(&admin, &proposal_id);
        client.approve(&second_admin, &proposal_id);
        client.execute(&admin, &proposal_id);

        assert!(client.is_paused());
    }

    #[test]
    fn unpause_via_proposal() {
        let (env, client, admin) = setup();
        let admins = client.get_admin_set();
        let second_admin = admins.get(1).unwrap();

        // Pause first.
        let pause_id = client.propose(&admin, &ProposalAction::Pause);
        client.approve(&admin, &pause_id);
        client.approve(&second_admin, &pause_id);
        client.execute(&admin, &pause_id);
        assert!(client.is_paused());

        // Unpause.
        let unpause_id = client.propose(&admin, &ProposalAction::Unpause);
        client.approve(&admin, &unpause_id);
        client.approve(&second_admin, &unpause_id);
        client.execute(&admin, &unpause_id);

        assert!(!client.is_paused());
    }

    #[test]
    fn get_proposal_returns_not_found_for_nonexistent_id() {
        let (env, client, _admin) = setup();

        let result = client.try_get_proposal(&999);
        assert_eq!(result, Err(Ok(Error::ProposalNotFound)));
    }

    // -----------------------------------------------------------------------
    // set_admin_set
    // -----------------------------------------------------------------------

    #[test]
    fn admin_can_update_admin_set_and_threshold() {
        let (env, client, admin) = setup();

        let new_admins = make_admin_set(&env, 3);
        client.set_admin_set(&admin, &new_admins, &2);

        assert_eq!(client.get_threshold(), 2);
        assert_eq!(client.get_admin_set().len(), 3);
    }

    #[test]
    fn non_admin_cannot_update_admin_set() {
        let (env, client, _admin) = setup();
        let stranger = Address::generate(&env);

        let mut new_admins: soroban_sdk::Vec<Address> = soroban_sdk::Vec::new(&env);
        new_admins.push_back(Address::generate(&env));
        let result = client.try_set_admin_set(&stranger, &new_admins, &1);
        assert_eq!(result, Err(Ok(Error::Unauthorized)));
    }

    #[test]
    fn set_admin_set_rejects_invalid_threshold() {
        let (env, client, admin) = setup();

        let new_admins = make_admin_set(&env, 2);
        let result = client.try_set_admin_set(&admin, &new_admins, &3);
        assert_eq!(result, Err(Ok(Error::InvalidArgument)));
    }

    // -----------------------------------------------------------------------
    // Events
    // -----------------------------------------------------------------------

    #[test]
    fn propose_emits_event() {
        let (env, client, admin) = setup();

        let _proposal_id = client.propose(&admin, &ProposalAction::Pause);

        let all_events = env.events().all();
        let found = all_events
            .iter()
            .any(|e| e.1 == (symbol_short!("proposal"), symbol_short!("created")).into_val(&env));
        assert!(found, "expected proposal created event");
    }

    #[test]
    fn approve_emits_event() {
        let (env, client, admin) = setup();

        let proposal_id = client.propose(&admin, &ProposalAction::Pause);
        client.approve(&admin, &proposal_id);

        let all_events = env.events().all();
        let found = all_events
            .iter()
            .any(|e| e.1 == (symbol_short!("proposal"), symbol_short!("approved")).into_val(&env));
        assert!(found, "expected proposal approved event");
    }

    #[test]
    fn execute_emits_event() {
        let (env, client, admin) = setup();
        let admins = client.get_admin_set();
        let second_admin = admins.get(1).unwrap();

        let proposal_id = client.propose(&admin, &ProposalAction::Pause);
        client.approve(&admin, &proposal_id);
        client.approve(&second_admin, &proposal_id);
        client.execute(&admin, &proposal_id);

        let all_events = env.events().all();
        let found = all_events
            .iter()
            .any(|e| e.1 == (symbol_short!("proposal"), symbol_short!("executed")).into_val(&env));
        assert!(found, "expected proposal executed event");
    }

    #[test]
    fn grant_role_emits_event() {
        let (env, client, admin) = setup();
        let user = Address::generate(&env);

        client.grant_role(&admin, &user, &Role::Upgrader);

        let all_events = env.events().all();
        let found = all_events
            .iter()
            .any(|e| e.1 == (symbol_short!("role"), symbol_short!("granted")).into_val(&env));
        assert!(found, "expected role granted event");
    }

    #[test]
    fn revoke_role_emits_event() {
        let (env, client, admin) = setup();
        let user = Address::generate(&env);

        client.grant_role(&admin, &user, &Role::Upgrader);
        client.revoke_role(&admin, &user, &Role::Upgrader);

        let all_events = env.events().all();
        let found = all_events
            .iter()
            .any(|e| e.1 == (symbol_short!("role"), symbol_short!("revoked")).into_val(&env));
        assert!(found, "expected role revoked event");
    }

    // -----------------------------------------------------------------------
    // Parameter management tests
    // -----------------------------------------------------------------------

    #[test]
    fn authorized_update_changes_parameter_and_emits_event() {
        let (env, client, admin) = setup();
        let key = ParameterKey::ReferralTierBps(2);
        let value = 350;

        client.set_param(&admin, &key, &value);
        let all_events = env.events().all();
        // The last event is ActionExecuted; find the ParameterChanged event
        let param_event = all_events
            .iter()
            .rev()
            .find(|e| e.1 == (shared::events::PARAMETER_CHANGED,).into_val(&env))
            .expect("expected PARAMETER_CHANGED event");
        assert_eq!(
            ParameterChangedEvent::try_from_val(&env, &param_event.2).unwrap(),
            ParameterChangedEvent { key, value }
        );

        assert_eq!(client.get_param(&ParameterKey::ReferralTierBps(2)), value);
        assert_eq!(client.referral_tier_bps(&2), value);
    }

    #[test]
    fn unauthorized_update_is_rejected() {
        let (env, client, _admin) = setup();
        let stranger = Address::generate(&env);

        assert!(matches!(
            client.try_set_param(&stranger, &ParameterKey::AidDefaultExpiry, &120),
            Err(Ok(Error::Unauthorized))
        ));
        assert_eq!(client.aid_default_expiry(), DEFAULT_AID_DEFAULT_EXPIRY);
    }

    #[test]
    fn out_of_range_values_are_rejected() {
        let (env, client, admin) = setup();

        assert!(matches!(
            client.try_set_param(
                &admin,
                &ParameterKey::AidDefaultExpiry,
                &(MAX_AID_DEFAULT_EXPIRY + 1)
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
        let (env, client, _admin) = setup();

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

    #[test]
    fn test_quorum_percentage_bps_default_and_update() {
        let (env, client, admin) = setup();

        assert_eq!(
            client.quorum_percentage_bps(),
            DEFAULT_QUORUM_PERCENTAGE_BPS
        );

        client.set_param(&admin, &ParameterKey::QuorumPercentageBps, &7_500);
        assert_eq!(client.quorum_percentage_bps(), 7_500);

        // Rejects out of range (bounds: 1..=10_000)
        assert!(matches!(
            client.try_set_param(&admin, &ParameterKey::QuorumPercentageBps, &0),
            Err(Ok(Error::InvalidArgument))
        ));
        assert!(matches!(
            client.try_set_param(&admin, &ParameterKey::QuorumPercentageBps, &10_001),
            Err(Ok(Error::InvalidArgument))
        ));
    }
}
