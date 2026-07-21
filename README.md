# ⭐ Alian Structure Smart Contracts

> **Open-source Soroban smart contracts powering transparent, secure and verifiable humanitarian aid distribution on the Stellar blockchain.**

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/Rust-1.80+-orange.svg)
![Soroban](https://img.shields.io/badge/Soroban-Latest-blue.svg)
![Stellar](https://img.shields.io/badge/Stellar-Blockchain-blue.svg)
![Status](https://img.shields.io/badge/status-active%20development-brightgreen.svg)

---

# Table of Contents

- Overview
- Why Smart Contracts
- Core Features
- Architecture
- Contract Modules
- Technology Stack
- Project Structure
- Development Setup
- Build
- Testing
- Deployment
- Security
- Upgrade Strategy
- Contract Interfaces
- Events
- Storage Layout
- Contribution Guide
- License

---

# Overview

The **Alian Structure Contracts** repository contains every on-chain component responsible for securely executing humanitarian aid transactions on the Stellar blockchain using **Soroban**.

These contracts serve as the trust layer of the platform.

Instead of relying on centralized intermediaries, every payment, verification, referral reward, and settlement is executed transparently on-chain.

The contracts are designed to be:

- Secure
- Auditable
- Upgradeable
- Gas Efficient
- Modular
- Production Ready

---

# Why Smart Contracts?

Traditional donation systems rely heavily on centralized infrastructure.

Alian Structure replaces this model by executing critical operations directly on-chain.

The contracts provide:

- Transparent aid settlements
- Immutable transaction records
- Referral commission distribution
- Treasury management
- Multi-signature administration
- Emergency controls
- Upgrade governance

Every important financial action becomes publicly verifiable.

---

# Core Features

## Direct Aid Settlement

Transfers donor funds directly to verified recipients.

Features

- Atomic transfers
- Escrow support
- Expiration handling
- Claim verification
- Replay protection

---

## Claim Links

Generate secure claim identifiers.

Supports

- One-time claims
- Expiration dates
- Maximum claims
- Signature validation
- Hash verification

---

## Referral Rewards

Automatically distributes affiliate commissions.

Supports

- Multi-tier referrals
- Percentage configuration
- Reward limits
- Treasury payouts

---

## Treasury

Central treasury responsible for:

- Holding protocol reserves
- Reward distribution
- Administrative transfers
- Emergency withdrawals

---

## Governance

Administrative controls include

- Contract upgrades
- Parameter updates
- Treasury permissions
- Emergency pause
- Role assignments

---

## Identity Verification

Stores verification references without exposing private user information.

Supports

- Hash verification
- Metadata pointers
- AI verification references
- Off-chain oracle integration

---

# Smart Contract Architecture

```

┌────────────────────────────┐
│        Frontend            │
└────────────┬───────────────┘
│
▼
┌────────────────────────────┐
│       Backend API          │
└────────────┬───────────────┘
│
▼
┌────────────────────────────┐
│      Soroban Contracts     │
├────────────────────────────┤
│ Aid Contract               │
│ Treasury Contract          │
│ Referral Contract          │
│ Governance Contract        │
│ Oracle Contract            │
│ Registry Contract          │
└────────────┬───────────────┘
│
▼
Stellar Ledger

```

---

# Contract Modules

## aid-contract

Responsible for

- Aid creation
- Aid claiming
- Settlement
- Escrow
- Refunds

---

## treasury-contract

Responsible for

- Treasury balances
- Reward distribution
- Protocol funds
- Emergency reserve

---

## referral-contract

Responsible for

- Referral registration
- Commission calculations
- Tier rewards
- Reward claims

---

## governance-contract

Responsible for

- Upgrade authorization
- Admin roles
- Parameter management
- Contract registry

---

## oracle-contract

Responsible for

- AI verification references
- External signatures
- Verification proofs

---

## registry-contract

Responsible for

- Contract discovery
- Address registry
- Version tracking

---

# Technology Stack

| Technology | Purpose |
|------------|---------|
| Rust | Smart contract language |
| Soroban SDK | Contract development |
| Stellar CLI | Deployment |
| Cargo | Package manager |
| Soroban RPC | Network interaction |
| GitHub Actions | CI/CD |

---

# Project Structure

```

contracts/

├── aid-contract/
├── treasury-contract/
├── referral-contract/
├── governance-contract/
├── oracle-contract/
├── registry-contract/
│
├── shared/
│ ├── errors.rs
│ ├── events.rs
│ ├── storage.rs
│ ├── auth.rs
│ ├── math.rs
│ └── utils.rs
│
├── scripts/
│ ├── deploy.sh
│ ├── upgrade.sh
│ ├── initialize.sh
│ └── verify.sh
│
├── tests/
├── Cargo.toml
├── Cargo.lock
└── README.md

```

---

# Development Setup

## Install Rust

```
rustup update
```

Install Soroban CLI

```
cargo install --locked soroban-cli
```

Clone repository

```
git clone https://github.com/SourceXXL/alian_structure-contracts.git

cd alian_structure-contracts
```

---

# Build

```
cargo build --release
```

Build optimized WASM

```
cargo build \
--target wasm32v1-none \
--release
```

---

# Testing

Run all tests

```
cargo test
```

Run integration tests

```
cargo test --test integration
```

Generate coverage

```
cargo llvm-cov
```

---

# Deployment

Deploy to Testnet

```
soroban contract deploy \
--wasm target/wasm32v1-none/release/aid_contract.wasm \
--source admin
```

Initialize

```
soroban contract invoke \
--id CONTRACT_ID \
-- initialize
```

---

# Security

The contracts implement

- Reentrancy protection
- Signature verification
- Overflow-safe arithmetic
- Access control
- Replay protection
- Input validation
- Treasury limits
- Emergency pause
- Time-based expirations
- Storage validation

---

# Upgrade Strategy

Supports controlled upgrades through governance.

Only authorized administrators may:

- Upgrade contracts
- Register new implementations
- Pause protocol
- Resume protocol
- Update treasury
- Modify protocol parameters

---

# Storage Layout

Persistent storage includes

Aid Records

- Aid ID
- Donor
- Recipient
- Amount
- Status
- Timestamp

Referral Records

- Wallet
- Referrer
- Commission
- Tier

Treasury

- Balance
- Rewards
- Fees

Governance

- Admins
- Roles
- Versions
- Registry

---

# Events

Contracts emit events for:

AidCreated

AidClaimed

AidSettled

AidRefunded

CommissionPaid

TreasuryDeposit

TreasuryWithdrawal

ContractPaused

ContractResumed

ContractUpgraded

---

# Future Roadmap

- Cross-chain settlement
- Stellar Asset support
- Multi-token donations
- DAO governance
- Zero Knowledge verification
- On-chain reputation
- Human identity proofs
- Streaming donations
- Batch settlements

---

# Contributing

We welcome contributions from Rust and Soroban developers.

Workflow

1. Fork repository

2. Create feature branch

3. Write tests

4. Submit Pull Request

Every contract contribution must include:

- Unit tests

- Documentation

- Security considerations

- Gas optimization review

---

# License

Licensed under the MIT License.

See LICENSE for details.

---

# Built With

- Stellar
- Soroban
- Rust
- Open Source Community

Building transparent humanitarian infrastructure for everyone.
