use soroban_sdk::{contracttype, symbol_short, Address, BytesN, Env, Symbol};

// Legacy single-topic constants retained for backward compatibility.
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
pub const REFERRAL_REGISTERED: Symbol = symbol_short!("ref_reg");
pub const PROPOSAL_CREATED: Symbol = symbol_short!("prop_new");
pub const PROPOSAL_APPROVED: Symbol = symbol_short!("prop_apr");
pub const PROPOSAL_EXECUTED: Symbol = symbol_short!("prop_exc");
pub const ROLE_GRANTED: Symbol = symbol_short!("role_grt");
pub const ROLE_REVOKED: Symbol = symbol_short!("role_rvk");

// Payment event topic constants.
pub const PAYMENT_TRANSFER: Symbol = symbol_short!("pay_xfr");
pub const PAYMENT_FEE: Symbol = symbol_short!("pay_fee");
pub const PAYMENT_ESCROW_CREATED: Symbol = symbol_short!("pay_esc_c");
pub const PAYMENT_ESCROW_RELEASED: Symbol = symbol_short!("pay_esc_r");
pub const PAYMENT_ESCROW_REFUNDED: Symbol = symbol_short!("pay_esc_f");

// Canonical event-logging topic constants.
pub const EVENT_LOG_INITIALIZED: Symbol = symbol_short!("evt_init");
pub const EVENT_LOG_ACTION: Symbol = symbol_short!("evt_act");
pub const EVENT_LOG_PERMISSION: Symbol = symbol_short!("evt_perm");

/// Emits `AidCreated`.
///
/// Topics: `("aid", "created")`
///
/// Data:
/// `(aid_id, donor, recipient, amount, created_at, expires_at)`
pub fn emit_aid_created(
    env: &Env,
    aid_id: u64,
    donor: &Address,
    recipient: &Address,
    amount: i128,
    created_at: u64,
    expires_at: u64,
) {
    env.events().publish(
        (symbol_short!("aid"), symbol_short!("created")),
        (
            aid_id,
            donor.clone(),
            recipient.clone(),
            amount,
            created_at,
            expires_at,
        ),
    );
}

/// Emits `AidClaimed`.
///
/// Topics: `("aid", "claimed")`
///
/// Data: `(aid_id, claimant, claimed_at)`
pub fn emit_aid_claimed(env: &Env, aid_id: u64, claimant: &Address, claimed_at: u64) {
    env.events().publish(
        (symbol_short!("aid"), symbol_short!("claimed")),
        (aid_id, claimant.clone(), claimed_at),
    );
}

/// Emits `AidSettled`.
///
/// Topics: `("aid", "settled")`
///
/// Data: `(aid_id, recipient, amount, settled_at)`
pub fn emit_aid_settled(
    env: &Env,
    aid_id: u64,
    recipient: &Address,
    amount: i128,
    settled_at: u64,
) {
    env.events().publish(
        (symbol_short!("aid"), symbol_short!("settled")),
        (aid_id, recipient.clone(), amount, settled_at),
    );
}

/// Emits `AidRefunded`.
///
/// Topics: `("aid", "refunded")`
///
/// Data: `(aid_id, donor, amount, refunded_at)`
pub fn emit_aid_refunded(env: &Env, aid_id: u64, donor: &Address, amount: i128, refunded_at: u64) {
    env.events().publish(
        (symbol_short!("aid"), symbol_short!("refunded")),
        (aid_id, donor.clone(), amount, refunded_at),
    );
}

/// Emits `CommissionPaid`.
///
/// Topics: `("comm", "paid")`
///
/// Data: `(recipient, amount, paid_at)`
pub fn emit_commission_paid(env: &Env, recipient: &Address, amount: i128, paid_at: u64) {
    env.events().publish(
        (symbol_short!("comm"), symbol_short!("paid")),
        (recipient.clone(), amount, paid_at),
    );
}

/// Emits `TreasuryDeposit`.
///
/// Topics: `("treasury", "deposit")`
///
/// Data: `(category, depositor, amount, new_balance)`
pub fn emit_treasury_deposit(
    env: &Env,
    category: Symbol,
    depositor: &Address,
    amount: i128,
    new_balance: i128,
) {
    env.events().publish(
        (symbol_short!("treasury"), symbol_short!("deposit")),
        (category, depositor.clone(), amount, new_balance),
    );
}

/// Emits `TreasuryWithdrawal`.
///
/// Topics: `("treasury", "withdraw")`
///
/// Data: `(category, recipient, amount, remaining_balance)`
pub fn emit_treasury_withdrawal(
    env: &Env,
    category: Symbol,
    recipient: &Address,
    amount: i128,
    remaining_balance: i128,
) {
    env.events().publish(
        (symbol_short!("treasury"), symbol_short!("withdraw")),
        (category, recipient.clone(), amount, remaining_balance),
    );
}

/// Emits `ContractPaused`.
///
/// Topics: `("contract", "paused")`
///
/// Data: `(actor, paused_at)`
pub fn emit_contract_paused(env: &Env, actor: &Address, paused_at: u64) {
    env.events().publish(
        (symbol_short!("contract"), symbol_short!("paused")),
        (actor.clone(), paused_at),
    );
}

/// Emits `ContractResumed`.
///
/// Topics: `("contract", "resumed")`
///
/// Data: `(actor, resumed_at)`
pub fn emit_contract_resumed(env: &Env, actor: &Address, resumed_at: u64) {
    env.events().publish(
        (symbol_short!("contract"), symbol_short!("resumed")),
        (actor.clone(), resumed_at),
    );
}

/// Emits `ContractUpgraded`.
///
/// Topics: `("contract", "upgraded")`
///
/// Data: `(actor, wasm_hash, upgraded_at)`
pub fn emit_contract_upgraded(
    env: &Env,
    actor: &Address,
    wasm_hash: &BytesN<32>,
    upgraded_at: u64,
) {
    env.events().publish(
        (symbol_short!("contract"), symbol_short!("upgraded")),
        (actor.clone(), wasm_hash.clone(), upgraded_at),
    );
}

// ---------------------------------------------------------------------------
// Canonical Event Logging helpers
// ---------------------------------------------------------------------------

/// Payload for `ModuleInitialized`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleInitializedEvent {
    pub module: Symbol,
    pub version: u32,
    pub caller: Address,
    pub initialized_at: u64,
}

/// Emits `ModuleInitialized`.
///
/// Topics: `("logging", "init")`
///
/// Data: `(Symbol module, u32 version, Address caller, u64 initialized_at)`
pub fn emit_module_initialized(
    env: &Env,
    module: Symbol,
    version: u32,
    caller: &Address,
    initialized_at: u64,
) {
    env.events().publish(
        (symbol_short!("logging"), symbol_short!("init")),
        (module, version, caller.clone(), initialized_at),
    );
}

/// Payload for `ActionExecuted`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionExecutedEvent {
    pub module: Symbol,
    pub action: Symbol,
    pub caller: Address,
    pub success: bool,
    pub executed_at: u64,
}

/// Emits `ActionExecuted`.
///
/// Topics: `("logging", "action")`
///
/// Data: `(Symbol module, Symbol action, Address caller, bool success, u64 executed_at)`
pub fn emit_action_executed(
    env: &Env,
    module: Symbol,
    action: Symbol,
    caller: &Address,
    success: bool,
    executed_at: u64,
) {
    env.events().publish(
        (symbol_short!("logging"), symbol_short!("action")),
        (module, action, caller.clone(), success, executed_at),
    );
}

/// Payload for `PermissionChanged`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PermissionChangedEvent {
    pub module: Symbol,
    pub role: Symbol,
    pub subject: Address,
    pub granted: bool,
    pub changed_at: u64,
}

/// Emits `PermissionChanged`.
///
/// Topics: `("logging", "perm")`
///
/// Data: `(Symbol module, Symbol role, Address subject, bool granted, u64 changed_at)`
pub fn emit_permission_changed(
    env: &Env,
    module: Symbol,
    role: Symbol,
    subject: &Address,
    granted: bool,
    changed_at: u64,
) {
    env.events().publish(
        (symbol_short!("logging"), symbol_short!("perm")),
        (module, role, subject.clone(), granted, changed_at),
    );
}

/// Emits an event using a legacy single-symbol topic.
///
/// New protocol events should use one of the typed helpers above.
pub fn emit<T: soroban_sdk::IntoVal<Env, soroban_sdk::Val>>(env: &Env, topic: Symbol, data: T) {
    env.events().publish((topic,), data);
}

// ---------------------------------------------------------------------------
// Oracle verification event helpers
// ---------------------------------------------------------------------------

/// Emits `OracleSubmitted`.
///
/// Topics: `("oracle", "submitted")`
///
/// Data: `(u32 id, Address subject, Symbol kind, BytesN<32> evidence_hash, u64 submitted_at)`
pub fn emit_oracle_submitted(
    env: &Env,
    id: u32,
    subject: &Address,
    kind: &Symbol,
    evidence_hash: &BytesN<32>,
    submitted_at: u64,
) {
    env.events().publish(
        (symbol_short!("oracle"), symbol_short!("submitted")),
        (
            id,
            subject.clone(),
            kind.clone(),
            evidence_hash.clone(),
            submitted_at,
        ),
    );
}

/// Emits `OracleVerified`.
///
/// Topics: `("oracle", "verified")`
///
/// Data: `(u32 id, Address subject, Symbol kind, BytesN<32> evidence_hash, Address verifier, u64 decided_at, Option<u64> expires_at)`
#[allow(clippy::too_many_arguments)]
pub fn emit_oracle_verified(
    env: &Env,
    id: u32,
    subject: &Address,
    kind: &Symbol,
    evidence_hash: &BytesN<32>,
    verifier: &Address,
    decided_at: u64,
    expires_at: Option<u64>,
) {
    env.events().publish(
        (symbol_short!("oracle"), symbol_short!("verified")),
        (
            id,
            subject.clone(),
            kind.clone(),
            evidence_hash.clone(),
            verifier.clone(),
            decided_at,
            expires_at,
        ),
    );
}

/// Emits `OracleRejected`.
///
/// Topics: `("oracle", "rejected")`
///
/// Data: `(u32 id, Address subject, Symbol kind, BytesN<32> evidence_hash, Address verifier, u64 decided_at)`
pub fn emit_oracle_rejected(
    env: &Env,
    id: u32,
    subject: &Address,
    kind: &Symbol,
    evidence_hash: &BytesN<32>,
    verifier: &Address,
    decided_at: u64,
) {
    env.events().publish(
        (symbol_short!("oracle"), symbol_short!("rejected")),
        (
            id,
            subject.clone(),
            kind.clone(),
            evidence_hash.clone(),
            verifier.clone(),
            decided_at,
        ),
    );
}

// ---------------------------------------------------------------------------
// Upgradeability event helpers
// ---------------------------------------------------------------------------

/// Topics: ("upgrade", "registered")
pub fn emit_contract_registered(
    env: &Env,
    contract_id: &Address,
    name: Symbol,
    version: u32,
    wasm_hash: &BytesN<32>,
    registered_at: u64,
) {
    env.events().publish(
        (symbol_short!("upgrade"), symbol_short!("upg_reg")),
        (
            contract_id.clone(),
            name,
            version,
            wasm_hash.clone(),
            registered_at,
        ),
    );
}

/// Topics: ("upgrade", "proposed")
pub fn emit_upgrade_proposed(
    env: &Env,
    proposal_id: u64,
    contract_id: &Address,
    new_version: u32,
    proposer: &Address,
    proposed_at: u64,
) {
    env.events().publish(
        (symbol_short!("upgrade"), symbol_short!("proposed")),
        (
            proposal_id,
            contract_id.clone(),
            new_version,
            proposer.clone(),
            proposed_at,
        ),
    );
}

/// Topics: ("upgrade", "executed")
pub fn emit_upgrade_executed(
    env: &Env,
    proposal_id: u64,
    contract_id: &Address,
    old_version: u32,
    new_version: u32,
    executor: &Address,
    executed_at: u64,
) {
    env.events().publish(
        (symbol_short!("upgrade"), symbol_short!("executed")),
        (
            proposal_id,
            contract_id.clone(),
            old_version,
            new_version,
            executor.clone(),
            executed_at,
        ),
    );
}

/// Topics: ("upgrade", "hook_set")
pub fn emit_migration_hook_set(env: &Env, contract_id: &Address, hook_addr: &Address, set_at: u64) {
    env.events().publish(
        (symbol_short!("upgrade"), symbol_short!("hook_set")),
        (contract_id.clone(), hook_addr.clone(), set_at),
    );
}

/// Topics: ("upgrade", "rolledback")
pub fn emit_upgrade_rolled_back(
    env: &Env,
    contract_id: &Address,
    from_version: u32,
    to_version: u32,
    executor: &Address,
    rolled_back_at: u64,
) {
    env.events().publish(
        (symbol_short!("upgrade"), symbol_short!("rollback")),
        (
            contract_id.clone(),
            from_version,
            to_version,
            executor.clone(),
            rolled_back_at,
        ),
    );
}

/// Emits `RoleGranted`.
///
/// Topics: `("role", "granted")`
///
/// Data: `(admin, grantee, role_name, timestamp)`
pub fn emit_role_granted(
    env: &Env,
    admin: &Address,
    grantee: &Address,
    role_name: Symbol,
    timestamp: u64,
) {
    env.events().publish(
        (symbol_short!("role"), symbol_short!("granted")),
        (admin.clone(), grantee.clone(), role_name, timestamp),
    );
}

// ---------------------------------------------------------------------------
// NFT Marketplace event helpers
// ---------------------------------------------------------------------------

/// Topics: ("nft", "listed")
#[allow(clippy::too_many_arguments)]
pub fn emit_nft_listed(
    env: &Env,
    listing_id: u64,
    seller: &Address,
    collection: &Address,
    token_id: u64,
    price: i128,
    currency: &Address,
    listed_at: u64,
) {
    env.events().publish(
        (symbol_short!("nft"), symbol_short!("listed")),
        (
            listing_id,
            seller.clone(),
            collection.clone(),
            token_id,
            price,
            currency.clone(),
            listed_at,
        ),
    );
}

/// Topics: ("nft", "sold")
pub fn emit_nft_sold(
    env: &Env,
    listing_id: u64,
    seller: &Address,
    buyer: &Address,
    price: i128,
    sold_at: u64,
) {
    env.events().publish(
        (symbol_short!("nft"), symbol_short!("sold")),
        (listing_id, seller.clone(), buyer.clone(), price, sold_at),
    );
}

/// Topics: ("nft", "offer")
pub fn emit_nft_offer(
    env: &Env,
    offer_id: u64,
    offerer: &Address,
    token_id: u64,
    amount: i128,
    expires_at: u64,
) {
    env.events().publish(
        (symbol_short!("nft"), symbol_short!("offer")),
        (offer_id, offerer.clone(), token_id, amount, expires_at),
    );
}

/// Topics: ("nft", "bid")
pub fn emit_nft_bid(env: &Env, auction_id: u64, bidder: &Address, amount: i128, new_end: u64) {
    env.events().publish(
        (symbol_short!("nft"), symbol_short!("bid")),
        (auction_id, bidder.clone(), amount, new_end),
    );
}

/// Topics: ("nft", "auction")
pub fn emit_nft_auction(
    env: &Env,
    auction_id: u64,
    seller: &Address,
    collection: &Address,
    token_id: u64,
    start_price: i128,
    end_time: u64,
) {
    env.events().publish(
        (symbol_short!("nft"), symbol_short!("auction")),
        (
            auction_id,
            seller.clone(),
            collection.clone(),
            token_id,
            start_price,
            end_time,
        ),
    );
}

/// Topics: ("nft", "settle")
pub fn emit_nft_settle(
    env: &Env,
    auction_id: u64,
    winner: &Address,
    final_price: i128,
    settled_at: u64,
) {
    env.events().publish(
        (symbol_short!("nft"), symbol_short!("settle")),
        (auction_id, winner.clone(), final_price, settled_at),
    );
}

/// Topics: ("nft", "royal")
pub fn emit_royalty_paid(
    env: &Env,
    token_id: u64,
    recipient: &Address,
    amount: i128,
    paid_at: u64,
) {
    env.events().publish(
        (symbol_short!("nft"), symbol_short!("royal")),
        (token_id, recipient.clone(), amount, paid_at),
    );
}

/// Topics: ("nft", "col_reg")
pub fn emit_collection_registered(
    env: &Env,
    collection: &Address,
    admin: &Address,
    registered_at: u64,
) {
    env.events().publish(
        (symbol_short!("nft"), symbol_short!("col_reg")),
        (collection.clone(), admin.clone(), registered_at),
    );
}

/// Emits `RoleRevoked`.
///
/// Topics: `("role", "revoked")`
///
/// Data: `(admin, grantee, role_name, timestamp)`
pub fn emit_role_revoked(
    env: &Env,
    admin: &Address,
    grantee: &Address,
    role_name: Symbol,
    timestamp: u64,
) {
    env.events().publish(
        (symbol_short!("role"), symbol_short!("revoked")),
        (admin.clone(), grantee.clone(), role_name, timestamp),
    );
}

/// Emits `ProposalCreated`.
///
/// Topics: `("proposal", "created")`
///
/// Data: `(proposal_id, proposer, action_description, timestamp)`
pub fn emit_proposal_created(
    env: &Env,
    proposal_id: u64,
    proposer: &Address,
    action: Symbol,
    timestamp: u64,
) {
    env.events().publish(
        (symbol_short!("proposal"), symbol_short!("created")),
        (proposal_id, proposer.clone(), action, timestamp),
    );
}

/// Emits `ProposalApproved`.
///
/// Topics: `("proposal", "approved")`
///
/// Data: `(proposal_id, approver, approval_count, timestamp)`
pub fn emit_proposal_approved(
    env: &Env,
    proposal_id: u64,
    approver: &Address,
    approval_count: u32,
    timestamp: u64,
) {
    env.events().publish(
        (symbol_short!("proposal"), symbol_short!("approved")),
        (proposal_id, approver.clone(), approval_count, timestamp),
    );
}

/// Emits `ProposalExecuted`.
///
/// Topics: `("proposal", "executed")`
///
/// Data: `(proposal_id, executor, approval_count, timestamp)`
pub fn emit_proposal_executed(
    env: &Env,
    proposal_id: u64,
    executor: &Address,
    approval_count: u32,
    timestamp: u64,
) {
    env.events().publish(
        (symbol_short!("proposal"), symbol_short!("executed")),
        (proposal_id, executor.clone(), approval_count, timestamp),
    );
}

#[cfg(test)]
mod tests {
    use super::{
        emit_action_executed, emit_aid_created, emit_module_initialized, emit_permission_changed,
    };
    use soroban_sdk::{
        contract, contractimpl, symbol_short,
        testutils::{Address as _, Events},
        Address, Env, FromVal, IntoVal, Symbol,
    };

    #[contract]
    struct EventTestContract;

    #[contractimpl]
    impl EventTestContract {
        pub fn publish_aid_created(
            env: Env,
            aid_id: u64,
            donor: Address,
            recipient: Address,
            amount: i128,
            created_at: u64,
            expires_at: u64,
        ) {
            emit_aid_created(
                &env, aid_id, &donor, &recipient, amount, created_at, expires_at,
            );
        }

        pub fn publish_module_initialized(
            env: Env,
            module: Symbol,
            version: u32,
            caller: Address,
            initialized_at: u64,
        ) {
            emit_module_initialized(&env, module, version, &caller, initialized_at);
        }

        pub fn publish_action_executed(
            env: Env,
            module: Symbol,
            action: Symbol,
            caller: Address,
            success: bool,
            executed_at: u64,
        ) {
            emit_action_executed(&env, module, action, &caller, success, executed_at);
        }

        pub fn publish_permission_changed(
            env: Env,
            module: Symbol,
            role: Symbol,
            subject: Address,
            granted: bool,
            changed_at: u64,
        ) {
            emit_permission_changed(&env, module, role, &subject, granted, changed_at);
        }
    }

    #[test]
    fn aid_created_has_stable_topics_and_data() {
        let env = Env::default();
        let donor = Address::generate(&env);
        let recipient = Address::generate(&env);
        let contract_id = env.register_contract(None, EventTestContract);
        let client = EventTestContractClient::new(&env, &contract_id);

        client.publish_aid_created(&7, &donor, &recipient, &500, &100, &1_000);

        let events = env.events().all();
        assert_eq!(events.len(), 1);

        let (emitter, topics, data) = events.get(0).unwrap();

        assert_eq!(emitter, contract_id);
        assert_eq!(
            topics,
            (symbol_short!("aid"), symbol_short!("created"),).into_val(&env)
        );
        let decoded_data: (u64, Address, Address, i128, u64, u64) = FromVal::from_val(&env, &data);

        assert_eq!(
            decoded_data,
            (7u64, donor, recipient, 500i128, 100u64, 1_000u64)
        );
    }

    #[test]
    fn module_initialized_has_stable_topics_and_data() {
        let env = Env::default();
        let module = symbol_short!("aid");
        let caller = Address::generate(&env);
        let contract_id = env.register_contract(None, EventTestContract);
        let client = EventTestContractClient::new(&env, &contract_id);

        client.publish_module_initialized(&module, &1, &caller, &1_000);

        let events = env.events().all();
        assert_eq!(events.len(), 1);

        let (emitter, topics, data) = events.get(0).unwrap();

        assert_eq!(emitter, contract_id);
        assert_eq!(
            topics,
            (symbol_short!("logging"), symbol_short!("init"),).into_val(&env)
        );
        let decoded_data: (Symbol, u32, Address, u64) = FromVal::from_val(&env, &data);

        assert_eq!(decoded_data, (module, 1, caller.clone(), 1_000));
    }

    #[test]
    fn action_executed_has_stable_topics_and_data() {
        let env = Env::default();
        let module = symbol_short!("aid");
        let action = symbol_short!("create");
        let caller = Address::generate(&env);
        let contract_id = env.register_contract(None, EventTestContract);
        let client = EventTestContractClient::new(&env, &contract_id);

        client.publish_action_executed(&module, &action, &caller, &true, &1_000);

        let events = env.events().all();
        assert_eq!(events.len(), 1);

        let (_emitter, topics, data) = events.get(0).unwrap();

        assert_eq!(
            topics,
            (symbol_short!("logging"), symbol_short!("action"),).into_val(&env)
        );
        let decoded_data: (Symbol, Symbol, Address, bool, u64) = FromVal::from_val(&env, &data);

        assert_eq!(decoded_data, (module, action, caller.clone(), true, 1_000));
    }

    #[test]
    fn permission_changed_has_stable_topics_and_data() {
        let env = Env::default();
        let module = symbol_short!("treasury");
        let role = symbol_short!("manager");
        let subject = Address::generate(&env);
        let contract_id = env.register_contract(None, EventTestContract);
        let client = EventTestContractClient::new(&env, &contract_id);

        client.publish_permission_changed(&module, &role, &subject, &true, &1_000);

        let events = env.events().all();
        assert_eq!(events.len(), 1);

        let (_emitter, topics, data) = events.get(0).unwrap();

        assert_eq!(
            topics,
            (symbol_short!("logging"), symbol_short!("perm"),).into_val(&env)
        );
        let decoded_data: (Symbol, Symbol, Address, bool, u64) = FromVal::from_val(&env, &data);

        assert_eq!(decoded_data, (module, role, subject.clone(), true, 1_000));
    }
}
