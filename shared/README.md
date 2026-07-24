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

## Maintenance rules

1. Never change the numeric value of a published error variant.
2. Never assign the same numeric value to multiple variants.
3. Put module-specific errors inside the module's assigned range.
4. Use the `900-999` range only for errors shared by multiple contracts.
5. Update this document whenever a new error code is introduced.
6. Update the uniqueness and stability tests when adding a shared variant.

---

## Storage helpers

Soroban charges rent on ledger entries and expires them when their TTL reaches
zero.  The helpers in `shared::storage` centralise all storage access so that
every contract bumps TTLs consistently without re-implementing the same
boilerplate.

### Storage-type guidance

Choose the right storage tier based on how long the data must survive and how
it is scoped:

| Data category | Storage type | Rationale |
|---|---|---|
| Contract config (admin address, token, fee rate…) | `instance` | Tied to the contract instance; evicted only when the contract itself is evicted. No per-entry TTL management needed. |
| Pause flag | `instance` | Config-level; must always be readable if the contract exists. |
| Aid records | `persistent` | Long-lived financial records that must survive many ledger closures until claimed or refunded. |
| Referral edges & accrued balances | `persistent` | Per-address data that outlives a single transaction. |
| Role assignments | `persistent` | Role grants must persist until explicitly revoked. |
| One-time claim nonces | `temporary` | Only needed until the claim window closes; can expire naturally, saving rent. |
| Oracle verification proofs | `temporary` | Short-lived attestation; the contract does not need to keep them forever. |

### TTL constants

All `extend_ttl` calls inside the helpers use these defaults.  Tune them to
match the expected ledger cadence for your deployment environment.

| Constant | Value (ledgers) | ≈ Wall-clock (5 s/ledger) |
|---|---|---|
| `PERSISTENT_BUMP_AMOUNT` | 120 960 | 7 days |
| `PERSISTENT_TTL_THRESHOLD` | 103 680 | 6 days — bumped when < 1 day remains |
| `TEMPORARY_BUMP_AMOUNT` | 17 280 | 1 day |
| `TEMPORARY_TTL_THRESHOLD` | 8 640 | 12 hours — bumped when < 12 h remains |

### API reference

#### Instance storage

```rust
use shared::{instance_get, instance_has, instance_remove, instance_set};

// Write config (no TTL needed — scoped to the contract instance).
instance_set(&env, &MyKey::Admin, &admin_address);

// Read config.
let admin: Option<Address> = instance_get(&env, &MyKey::Admin);

// Existence check.
if instance_has(&env, &MyKey::Token) { … }

// Delete.
instance_remove(&env, &MyKey::Admin);
```

#### Persistent storage

```rust
use shared::{
    persistent_extend_ttl, persistent_get, persistent_has,
    persistent_remove, persistent_set,
};

// Write a long-lived record (TTL extended immediately).
persistent_set(&env, &DataKey::Aid(aid_id), &record);

// Read a record (TTL extended on hit; None returned on miss, no panic).
let record: Option<AidRecord> = persistent_get(&env, &DataKey::Aid(aid_id));

// Existence check (does NOT extend TTL — use persistent_get when you
// intend to read the value right after).
if persistent_has(&env, &DataKey::Aid(aid_id)) { … }

// Explicit TTL extension (useful when updating a record in-place via a
// separate code path that does not go through persistent_set).
persistent_extend_ttl(&env, &DataKey::Aid(aid_id));

// Delete (e.g. after full settlement to reclaim storage rent).
persistent_remove(&env, &DataKey::Aid(aid_id));
```

#### Temporary storage

```rust
use shared::{temporary_get, temporary_has, temporary_remove, temporary_set};

// Write a short-lived nonce (TTL set immediately).
temporary_set(&env, &DataKey::ClaimNonce(aid_id, hash), &true);

// Read (TTL extended on hit; None if expired or absent).
let used: Option<bool> = temporary_get(&env, &DataKey::ClaimNonce(aid_id, hash));

// Existence check (does NOT extend TTL).
if temporary_has(&env, &DataKey::ClaimNonce(aid_id, hash)) { … }

// Delete before natural expiry if you want to reclaim rent early.
temporary_remove(&env, &DataKey::ClaimNonce(aid_id, hash));
```

#### Pause flag helpers

```rust
use shared::{is_paused, set_paused};

// Check at the top of every state-changing entry point.
if is_paused(&env) {
    return Err(MyError::Paused);
}

// Admin entry point to pause/resume.
set_paused(&env, true);   // pause
set_paused(&env, false);  // resume
```

### Key-design convention

Define a contract-local `DataKey` enum annotated with `#[contracttype]` for
each module.  Place the keys that go to persistent storage in the enum variants
that encode the identity of the record (e.g. `Aid(u64)`) and the keys that go
to instance storage in unit variants (e.g. `AidCounter`).  Document the storage
tier in each variant's doc comment.

```rust
use soroban_sdk::contracttype;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// persistent — one record per aid disbursement.
    Aid(u64),
    /// instance — monotonically-increasing counter.
    AidCounter,
}
```

This convention ensures keys are type-safe, self-documenting, and
non-colliding across storage tiers.
