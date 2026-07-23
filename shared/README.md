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