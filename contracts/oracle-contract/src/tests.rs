#![cfg(test)]

extern crate std;

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Events},
    FromVal, IntoVal,
};

// ---------------------------------------------------------------------------
// Fixture
// ---------------------------------------------------------------------------

struct Fixture {
    env: Env,
    admin: Address,
    subject: Address,
    contract_id: Address,
}

fn setup() -> Fixture {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let subject = Address::generate(&env);

    let contract_id = env.register_contract(None, OracleContract);
    let client = OracleContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    Fixture {
        env,
        admin,
        subject,
        contract_id,
    }
}

/// A 32-byte evidence hash filled with `fill` (never all-zero).
fn evidence_hash(env: &Env, fill: u8) -> BytesN<32> {
    let mut buf = [0u8; 32];
    for (i, b) in buf.iter_mut().enumerate() {
        *b = fill.checked_add(i as u8).unwrap_or(fill);
    }
    BytesN::from_array(env, &buf)
}

fn uri(env: &Env, len: usize) -> Bytes {
    let mut bytes = Bytes::new(env);
    for i in 0..len {
        bytes.push_back(b'a' + (i % 26) as u8);
    }
    bytes
}

/// Submit `count` verifications for `subject`, returning the allocated IDs.
fn submit_verifications(
    env: &Env,
    client: &OracleContractClient,
    subject: &Address,
    count: u32,
) -> std::vec::Vec<u32> {
    let mut ids = std::vec::Vec::with_capacity(count as usize);
    for i in 0..count {
        ids.push(client.submit_verification(
            subject,
            &symbol_short!("need"),
            &evidence_hash(env, i as u8 + 1),
            &uri(env, 8),
        ));
    }
    ids
}

// ---------------------------------------------------------------------------
// Submission
// ---------------------------------------------------------------------------

#[test]
fn submit_creates_pending_record_with_event_and_getter_roundtrip() {
    let fx = setup();
    let client = OracleContractClient::new(&fx.env, &fx.contract_id);

    let hash = evidence_hash(&fx.env, 7);
    let kind = symbol_short!("identity");
    let id = client.submit_verification(&fx.subject, &kind, &hash, &uri(&fx.env, 12));

    assert_eq!(id, 1);
    assert_eq!(client.verification_count(), 1);

    let record = client.get_verification(&id).unwrap();
    assert_eq!(record.id, id);
    assert_eq!(record.subject, fx.subject);
    assert_eq!(record.kind, kind);
    assert_eq!(record.status, VerificationStatus::Pending);
    assert_eq!(record.evidence_hash, hash);
    assert_eq!(record.verifier, None);
    assert_eq!(record.decided_at, None);
    assert_eq!(record.expires_at, None);
    assert_eq!(record.submitted_at, fx.env.ledger().timestamp());

    // Exactly two events so far: ModuleInitialized + OracleSubmitted, and the
    // submission event carries the documented topic tuple and data schema.
    let events = fx.env.events().all();
    assert_eq!(events.len(), 2);
    let (_, topics, data) = events.get(1).unwrap();
    assert_eq!(
        topics,
        (symbol_short!("oracle"), symbol_short!("submitted")).into_val(&fx.env)
    );
    let decoded: (u32, Address, Symbol, BytesN<32>, u64) = FromVal::from_val(&fx.env, &data);
    assert_eq!(
        decoded,
        (
            id,
            fx.subject.clone(),
            kind,
            hash,
            fx.env.ledger().timestamp()
        )
    );
}

#[test]
fn submit_ids_increase_monotonically() {
    let fx = setup();
    let client = OracleContractClient::new(&fx.env, &fx.contract_id);
    let ids = submit_verifications(&fx.env, &client, &fx.subject, 3);
    assert_eq!(ids, std::vec![1, 2, 3]);
    assert_eq!(client.verification_count(), 3);
}

#[test]
fn submit_rejects_empty_and_oversized_metadata_uri() {
    let fx = setup();
    let client = OracleContractClient::new(&fx.env, &fx.contract_id);

    assert_eq!(
        client.try_submit_verification(
            &fx.subject,
            &symbol_short!("need"),
            &evidence_hash(&fx.env, 1),
            &uri(&fx.env, 0)
        ),
        Err(Ok(OracleError::InvalidEvidence))
    );

    assert_eq!(
        client.try_submit_verification(
            &fx.subject,
            &symbol_short!("need"),
            &evidence_hash(&fx.env, 1),
            &uri(&fx.env, (MAX_METADATA_URI_LEN + 1) as usize)
        ),
        Err(Ok(OracleError::InvalidEvidence))
    );

    // Boundary: exactly the cap is accepted.
    let id = client.submit_verification(
        &fx.subject,
        &symbol_short!("need"),
        &evidence_hash(&fx.env, 2),
        &uri(&fx.env, MAX_METADATA_URI_LEN as usize),
    );
    assert_eq!(id, 1);
}

// ---------------------------------------------------------------------------
// Decisions
// ---------------------------------------------------------------------------

#[test]
fn admin_verify_flips_status_and_emits_exactly_one_decision_event() {
    let fx = setup();
    let client = OracleContractClient::new(&fx.env, &fx.contract_id);
    let id = client.submit_verification(
        &fx.subject,
        &symbol_short!("need"),
        &evidence_hash(&fx.env, 3),
        &uri(&fx.env, 8),
    );

    let expires_at = fx.env.ledger().timestamp() + 3600;
    client.decide_verification(&fx.admin, &id, &true, &Some(expires_at));

    let record = client.get_verification(&id).unwrap();
    assert_eq!(record.status, VerificationStatus::Verified);
    assert_eq!(record.verifier, Some(fx.admin.clone()));
    assert_eq!(record.decided_at, Some(fx.env.ledger().timestamp()));
    assert_eq!(record.expires_at, Some(expires_at));

    // 1 init + 1 submit + exactly 1 decision event, with the documented schema.
    let events = fx.env.events().all();
    assert_eq!(events.len(), 3);
    let (_, topics, data) = events.get(2).unwrap();
    assert_eq!(
        topics,
        (symbol_short!("oracle"), symbol_short!("verified")).into_val(&fx.env)
    );
    let decoded: (u32, Address, Symbol, BytesN<32>, Address, u64, Option<u64>) =
        FromVal::from_val(&fx.env, &data);
    assert_eq!(
        decoded,
        (
            id,
            fx.subject.clone(),
            symbol_short!("need"),
            evidence_hash(&fx.env, 3),
            fx.admin.clone(),
            fx.env.ledger().timestamp(),
            Some(expires_at)
        )
    );
}

#[test]
fn admin_reject_flips_status_and_emits_rejected_event() {
    let fx = setup();
    let client = OracleContractClient::new(&fx.env, &fx.contract_id);
    let id = client.submit_verification(
        &fx.subject,
        &symbol_short!("need"),
        &evidence_hash(&fx.env, 4),
        &uri(&fx.env, 8),
    );

    client.decide_verification(&fx.admin, &id, &false, &None);

    let record = client.get_verification(&id).unwrap();
    assert_eq!(record.status, VerificationStatus::Rejected);
    assert_eq!(record.verifier, Some(fx.admin.clone()));

    let events = fx.env.events().all();
    let (_, topics, _) = events.get(2).unwrap();
    assert_eq!(
        topics,
        (symbol_short!("oracle"), symbol_short!("rejected")).into_val(&fx.env)
    );
}

#[test]
fn decide_by_non_admin_returns_not_verifier() {
    let fx = setup();
    let client = OracleContractClient::new(&fx.env, &fx.contract_id);
    let stranger = Address::generate(&fx.env);
    let id = client.submit_verification(
        &fx.subject,
        &symbol_short!("need"),
        &evidence_hash(&fx.env, 5),
        &uri(&fx.env, 8),
    );

    assert_eq!(
        client.try_decide_verification(&stranger, &id, &true, &None),
        Err(Ok(OracleError::NotVerifier))
    );

    // Record is untouched.
    assert_eq!(
        client.get_verification(&id).unwrap().status,
        VerificationStatus::Pending
    );
}

#[test]
fn decide_unknown_id_returns_verification_not_found() {
    let fx = setup();
    let client = OracleContractClient::new(&fx.env, &fx.contract_id);

    assert_eq!(
        client.try_decide_verification(&fx.admin, &999, &true, &None),
        Err(Ok(OracleError::VerificationNotFound))
    );
}

#[test]
fn second_decision_returns_already_decided() {
    let fx = setup();
    let client = OracleContractClient::new(&fx.env, &fx.contract_id);
    let id = client.submit_verification(
        &fx.subject,
        &symbol_short!("need"),
        &evidence_hash(&fx.env, 6),
        &uri(&fx.env, 8),
    );
    client.decide_verification(&fx.admin, &id, &true, &None);

    assert_eq!(
        client.try_decide_verification(&fx.admin, &id, &false, &None),
        Err(Ok(OracleError::AlreadyDecided))
    );
}

#[test]
fn submit_and_decide_while_paused_are_rejected() {
    let fx = setup();
    let client = OracleContractClient::new(&fx.env, &fx.contract_id);
    let id = client.submit_verification(
        &fx.subject,
        &symbol_short!("need"),
        &evidence_hash(&fx.env, 8),
        &uri(&fx.env, 8),
    );

    fx.env.as_contract(&fx.contract_id, || {
        shared::storage::set_paused(&fx.env, true);
    });

    assert_eq!(
        client.try_submit_verification(
            &fx.subject,
            &symbol_short!("need"),
            &evidence_hash(&fx.env, 9),
            &uri(&fx.env, 8)
        ),
        Err(Ok(OracleError::Paused))
    );
    assert_eq!(
        client.try_decide_verification(&fx.admin, &id, &true, &None),
        Err(Ok(OracleError::Paused))
    );

    fx.env.as_contract(&fx.contract_id, || {
        shared::storage::set_paused(&fx.env, false);
    });
    client.decide_verification(&fx.admin, &id, &true, &None);
    assert_eq!(
        client.get_verification(&id).unwrap().status,
        VerificationStatus::Verified
    );
}

#[test]
fn reinitialization_is_rejected() {
    let fx = setup();
    let client = OracleContractClient::new(&fx.env, &fx.contract_id);

    assert_eq!(
        client.try_initialize(&fx.admin),
        Err(Ok(OracleError::AlreadyInitialized))
    );
}

// ---------------------------------------------------------------------------
// Pagination
// ---------------------------------------------------------------------------

#[test]
fn list_returns_all_pages_in_order() {
    let fx = setup();
    let client = OracleContractClient::new(&fx.env, &fx.contract_id);
    let ids = submit_verifications(&fx.env, &client, &fx.subject, 5);

    let page = client.list_verifications_by_subject(&fx.subject, &0, &50);
    assert_eq!(page.total, 5);
    assert_eq!(page.records.len(), 5);
    for (i, record) in page.records.iter().enumerate() {
        assert_eq!(record.id, ids[i]);
    }
}

#[test]
fn list_respects_hard_cap_of_fifty() {
    let fx = setup();
    let client = OracleContractClient::new(&fx.env, &fx.contract_id);
    submit_verifications(&fx.env, &client, &fx.subject, 60);

    // Request far above the cap — page is still exactly 50.
    let page = client.list_verifications_by_subject(&fx.subject, &0, &5_000);
    assert_eq!(page.records.len(), 50);
    assert_eq!(page.total, 60);

    // Second page: the remaining 10.
    let page2 = client.list_verifications_by_subject(&fx.subject, &50, &50);
    assert_eq!(page2.records.len(), 10);

    // Cursor at the end: empty page, no error.
    let page3 = client.list_verifications_by_subject(&fx.subject, &60, &50);
    assert_eq!(page3.records.len(), 0);
    assert_eq!(page3.total, 60);
}

#[test]
fn list_limit_zero_defaults_to_full_page() {
    let fx = setup();
    let client = OracleContractClient::new(&fx.env, &fx.contract_id);
    submit_verifications(&fx.env, &client, &fx.subject, 3);

    let page = client.list_verifications_by_subject(&fx.subject, &0, &0);
    assert_eq!(page.records.len(), 3);
}

#[test]
fn list_cursor_beyond_end_is_rejected() {
    let fx = setup();
    let client = OracleContractClient::new(&fx.env, &fx.contract_id);
    submit_verifications(&fx.env, &client, &fx.subject, 2);

    assert_eq!(
        client.try_list_verifications_by_subject(&fx.subject, &3, &50),
        Err(Ok(OracleError::InvalidEvidence))
    );
}

#[test]
fn list_for_unknown_subject_is_empty_not_error() {
    let fx = setup();
    let client = OracleContractClient::new(&fx.env, &fx.contract_id);
    let stranger = Address::generate(&fx.env);

    let page = client.list_verifications_by_subject(&stranger, &0, &50);
    assert_eq!(page.records.len(), 0);
    assert_eq!(page.total, 0);
}

// ---------------------------------------------------------------------------
// Privacy — no PII in storage
// ---------------------------------------------------------------------------

#[test]
fn storage_contains_only_hashes_and_uris_no_pii() {
    let fx = setup();
    let client = OracleContractClient::new(&fx.env, &fx.contract_id);
    let id = client.submit_verification(
        &fx.subject,
        &symbol_short!("need"),
        &evidence_hash(&fx.env, 10),
        &uri(&fx.env, 8),
    );

    let record = client.get_verification(&id).unwrap();

    // The on-chain record must reference evidence, never describe the person:
    // the only free-form field is the URI pointer, and everything else is an
    // address, enum, hash, or counter.
    assert_eq!(record.metadata_uri, uri(&fx.env, 8));
    assert!(record.metadata_uri.len() <= MAX_METADATA_URI_LEN);
    // Evidence hash pins integrity off-chain and is never all-zero here.
    assert_ne!(
        record.evidence_hash,
        BytesN::from_array(&fx.env, &[0u8; 32])
    );
}

// ---------------------------------------------------------------------------
// Error-code range discipline (500-599, unique)
// ---------------------------------------------------------------------------

#[test]
fn error_codes_are_unique_and_within_oracle_range() {
    let codes: std::vec::Vec<u32> = std::vec::Vec::from([
        OracleError::VerificationNotFound as u32,
        OracleError::AlreadyDecided as u32,
        OracleError::NotVerifier as u32,
        OracleError::InvalidEvidence as u32,
        OracleError::Expired as u32,
        OracleError::Revoked as u32,
        OracleError::AlreadyInitialized as u32,
        OracleError::Paused as u32,
    ]);

    for code in &codes {
        assert!(
            (500..600).contains(code),
            "code {} outside the oracle range 500-599",
            code
        );
    }

    let mut sorted = codes.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), codes.len(), "duplicate error codes detected");
}
