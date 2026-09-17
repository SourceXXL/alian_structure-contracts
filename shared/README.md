# Shared contract library

The `shared` crate contains types and utilities reused by every contract in the
workspace.

## Error codes

All contracts must return stable and documented numeric error codes. Stable
codes allow backend and frontend applications to map contract failures to
consistent user-facing messages without depending on error strings.

The shared error enum can be imported through either path:

```rust
shared::Error
shared::errors::Error
```

## Reserved ranges

Each contract module owns a separate numeric range. Numeric codes must not be
reused for a different meaning, even when an older variant is no longer used.

| Range | Owner | Purpose |
|---|---|---|
| `100-199` | Aid contract | Aid distribution and claim-specific errors |
| `200-299` | Treasury contract | Balance, transfer, and treasury-specific errors |
| `300-399` | Referral contract | Referral and reward-specific errors |
| `400-499` | Governance contract | Proposal, vote, and governance-specific errors |
| `500-599` | Oracle contract | Price feed and oracle-specific errors |
| `600-699` | Registry contract | Registration and registry-specific errors |
| `700-899` | Reserved | Reserved for future contract modules |
| `900-999` | Shared/common | Errors with the same meaning across contracts |

## Shared error table

| Code | Variant | Meaning |
|---:|---|---|
| `900` | `NotAuthorized` | The caller is not authorized to perform the operation |
| `901` | `AlreadyInitialized` | The contract or component was already initialized |
| `902` | `NotInitialized` | The contract or component has not been initialized |
| `903` | `InvalidAmount` | The supplied amount is invalid |
| `904` | `Expired` | The operation or resource has expired |
| `905` | `AlreadyClaimed` | The resource or entitlement was already claimed |
| `906` | `Paused` | The operation is disabled while the contract is paused |
| `907` | `Overflow` | An arithmetic operation exceeded its supported range |
| `908` | `InvalidInput` | One or more input values are invalid |
| `909` | `NotFound` | The requested resource could not be found |

## Usage

Contracts can import and return the re-exported enum directly:

```rust
use shared::Error;

pub fn example() -> Result<(), Error> {
    Err(Error::InvalidInput)
}
```

The Aid contract initialization function provides the first workspace example.
It returns `Error::AlreadyInitialized` when initialization is attempted more
than once.

## Module error codes

Each module defines its own `contracterror` enum inside its reserved range.
Modules must document their codes here when introduced.

### Oracle contract (`OracleError`, 500-599)

| Code | Variant | Meaning |
|---:|---|---|
| `500` | `VerificationNotFound` | The requested verification record does not exist |
| `501` | `AlreadyDecided` | A decision has already been recorded for this verification |
| `502` | `NotVerifier` | The caller is not authorised to decide verifications |
| `503` | `InvalidEvidence` | The evidence is invalid (empty/oversized metadata URI) or a query cursor is out of range |
| `504` | `Expired` | The attestation has expired and must be re-verified |
| `505` | `Revoked` | The attestation was verified and later revoked |
| `506` | `AlreadyInitialized` | The contract has already been initialized |
| `507` | `Paused` | The operation is not permitted while the contract is paused |

The oracle contract never stores personal data. Verification records carry
only an evidence hash and a metadata URI; off-chain systems resolve the URI.

## Maintenance rules

1. Never change the numeric value of a published error variant.
2. Never assign the same numeric value to multiple variants.
3. Put module-specific errors inside the module's assigned range.
4. Use the `900-999` range only for errors shared by multiple contracts.
5. Update this document whenever a new error code is introduced.
6. Update the uniqueness and stability tests when adding a shared variant.

## Event schemas

Protocol events use stable two-part topic tuples. Off-chain indexers should
match both topic symbols and decode the data using the documented order and
types.

The legacy single-symbol constants and generic `emit` helper remain available
for backward compatibility. New protocol code should use the typed helpers in
`shared::events`.

| Event | Helper | Topics | Data |
|---|---|---|---|
| `AidCreated` | `emit_aid_created` | `("aid", "created")` | `(u64 aid_id, Address donor, Address recipient, i128 amount, u64 created_at, u64 expires_at)` |
| `AidClaimed` | `emit_aid_claimed` | `("aid", "claimed")` | `(u64 aid_id, Address claimant, u64 claimed_at)` |
| `AidSettled` | `emit_aid_settled` | `("aid", "settled")` | `(u64 aid_id, Address recipient, i128 amount, u64 settled_at)` |
| `AidRefunded` | `emit_aid_refunded` | `("aid", "refunded")` | `(u64 aid_id, Address donor, i128 amount, u64 refunded_at)` |
| `CommissionPaid` | `emit_commission_paid` | `("comm", "paid")` | `(Address recipient, i128 amount, u64 paid_at)` |
| `TreasuryDeposit` | `emit_treasury_deposit` | `("treasury", "deposit")` | `(Symbol category, Address depositor, i128 amount, i128 new_balance)` |
| `TreasuryWithdrawal` | `emit_treasury_withdrawal` | `("treasury", "withdraw")` | `(Symbol category, Address recipient, i128 amount, i128 remaining_balance)` |
| `ContractPaused` | `emit_contract_paused` | `("contract", "paused")` | `(Address actor, u64 paused_at)` |
| `ContractResumed` | `emit_contract_resumed` | `("contract", "resumed")` | `(Address actor, u64 resumed_at)` |
| `ContractUpgraded` | `emit_contract_upgraded` | `("contract", "upgraded")` | `(Address actor, BytesN<32> wasm_hash, u64 upgraded_at)` |
| `ModuleInitialized` | `emit_module_initialized` | `("logging", "initialized")` | `(Symbol module, u32 version, Address caller, u64 initialized_at)` |
| `ActionExecuted` | `emit_action_executed` | `("logging", "action")` | `(Symbol module, Symbol action, Address caller, bool success, u64 executed_at)` |
| `PermissionChanged` | `emit_permission_changed` | `("logging", "permission")` | `(Symbol module, Symbol role, Address subject, bool granted, u64 changed_at)` |
| `OracleSubmitted` | `emit_oracle_submitted` | `("oracle", "submitted")` | `(u32 id, Address subject, Symbol kind, BytesN<32> evidence_hash, u64 submitted_at)` |
| `OracleVerified` | `emit_oracle_verified` | `("oracle", "verified")` | `(u32 id, Address subject, Symbol kind, BytesN<32> evidence_hash, Address verifier, u64 decided_at, Option<u64> expires_at)` |
| `OracleRejected` | `emit_oracle_rejected` | `("oracle", "rejected")` | `(u32 id, Address subject, Symbol kind, BytesN<32> evidence_hash, Address verifier, u64 decided_at)` |

### Event stability rules

1. Do not change a published event's topic tuple.
2. Do not reorder, remove, or change the type of existing data fields.
3. Add new event versions instead of silently changing an existing schema.
4. Use the typed helper whenever one exists.
5. Update this table and the event tests whenever a new schema is introduced.
