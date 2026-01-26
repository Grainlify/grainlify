# Grainlify Smart Contracts

Comprehensive smart contract suite for managing bounties and program prize pools on the Stellar Soroban platform.

## 📋 Overview

This directory contains three Soroban smart contracts that power the Grainlify platform:

1. **[Bounty Escrow Contract](./bounty_escrow/)** - Manages individual bounty escrows with deadline-based refunds
2. **[Program Escrow Contract](./program-escrow/)** - Manages hackathon/program prize pools with batch payout support
3. **[Grainlify Core Contract](./grainlify-core/)** - Provides contract upgradeability and version management

All contracts are written in Rust using the Soroban SDK and deployed on the Stellar network.

## 🚀 Quick Start

### Prerequisites

- Rust 1.70+ with `wasm32-unknown-unknown` target
- Soroban CLI (`cargo install soroban-cli`)
- Stellar account with testnet XLM

### Build All Contracts

```bash
# Build bounty escrow
cd bounty_escrow
cargo build --target wasm32-unknown-unknown --release

# Build program escrow
cd ../program-escrow
cargo build --target wasm32-unknown-unknown --release

# Build grainlify core
cd ../grainlify-core
cargo build --target wasm32-unknown-unknown --release
```

### Deploy to Testnet

```bash
# Deploy bounty escrow
soroban contract deploy \
  --wasm bounty_escrow/target/wasm32-unknown-unknown/release/bounty_escrow.wasm \
  --network testnet

# Initialize
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- init \
  --admin <ADMIN_ADDRESS> \
  --token <XLM_TOKEN_ADDRESS>
```

## 📚 Documentation

- **[ARCHITECTURE.md](./ARCHITECTURE.md)** - System architecture and design overview
- **[DOCUMENTATION.md](./DOCUMENTATION.md)** - Developer guide with integration examples
- **[SECURITY.md](./SECURITY.md)** - Security model, best practices, and audit guidelines
- **[VERSIONS.md](./VERSIONS.md)** - Version history and changelog

## 🏗️ Contract Details

### Bounty Escrow Contract

Manages escrow for individual bounties with the following features:

- ✅ Lock funds for specific bounties
- ✅ Admin-controlled fund release to contributors
- ✅ Deadline-based automatic refunds
- ✅ Event emission for off-chain tracking
- ✅ Persistent storage with TTL management

**Key Functions:**
- `init(admin, token)` - Initialize contract
- `lock_funds(depositor, bounty_id, amount, deadline)` - Lock funds
- `release_funds(bounty_id, contributor)` - Release to contributor (admin only)
- `refund(bounty_id)` - Refund after deadline
- `get_escrow_info(bounty_id)` - Query escrow state
- `get_balance()` - Get contract balance

**Location:** `./bounty_escrow/contracts/escrow/src/lib.rs`

### Program Escrow Contract

Manages prize pools for hackathons and programs:

- ✅ Initialize program with prize pool
- ✅ Lock funds incrementally
- ✅ Batch payout to multiple winners
- ✅ Single payout for individual prizes
- ✅ Complete payout history tracking
- ✅ Real-time balance management

**Key Functions:**
- `init_program(program_id, authorized_key, token)` - Initialize program
- `lock_program_funds(amount)` - Add funds to prize pool
- `batch_payout(recipients, amounts)` - Distribute to multiple winners
- `single_payout(recipient, amount)` - Single prize distribution
- `get_program_info()` - Query program state
- `get_remaining_balance()` - Check available funds

**Location:** `./program-escrow/src/lib.rs`

### Grainlify Core Contract

Provides upgradeability for the contract ecosystem:

- ✅ Admin-controlled WASM upgrades
- ✅ Version tracking and management
- ✅ One-time initialization
- ✅ Storage persistence across upgrades

**Key Functions:**
- `init(admin)` - Initialize contract
- `upgrade(new_wasm_hash)` - Upgrade contract code (admin only)
- `get_version()` - Get current version
- `set_version(new_version)` - Update version (admin only)

**Location:** `./grainlify-core/src/lib.rs`

## 🔐 Security

### Authorization Model

- **Bounty Escrow**: Admin-only release, permissionless refund after deadline
- **Program Escrow**: Authorized payout key controls all distributions
- **Core Contract**: Admin-only upgrades and version management

### Key Security Features

- ✅ Authorization checks on all privileged operations
- ✅ Overflow protection with checked arithmetic
- ✅ Reentrancy protection (Soroban execution model)
- ✅ Time-locked refunds prevent fund loss
- ✅ Immutable payout history for auditing
- ✅ Event emission for transparency

### Security Best Practices

1. **Admin Key Management**: Use HSM or secure key management service
2. **Testing**: Thoroughly test on testnet before mainnet deployment
3. **Monitoring**: Monitor all contract events and transactions
4. **Upgrades**: Test upgrades extensively, have rollback plan ready

See [SECURITY.md](./SECURITY.md) for comprehensive security documentation.

## 🧪 Testing

### Run Unit Tests

```bash
# Test bounty escrow
cd bounty_escrow
cargo test

# Test program escrow
cd ../program-escrow
cargo test

# Test grainlify core
cd ../grainlify-core
cargo test
```

### Integration Testing

Deploy to Stellar testnet and test complete workflows:

```bash
# See DOCUMENTATION.md for integration testing examples
```

## 📊 Architecture

```
┌─────────────────────────────────────────────────────────┐
│                  Grainlify Backend                      │
│              (Admin/Payout Authorization)               │
└────────────────────┬────────────────────────────────────┘
                     │
                     │ Manages & Authorizes
                     │
        ┌────────────┼────────────┐
        │            │            │
        ▼            ▼            ▼
┌──────────────┐ ┌──────────────┐ ┌──────────────┐
│   Bounty     │ │   Program    │ │  Grainlify   │
│   Escrow     │ │   Escrow     │ │    Core      │
│  Contract    │ │  Contract    │ │  Contract    │
└──────┬───────┘ └──────┬───────┘ └──────────────┘
       │                │
       │ Transfers      │ Transfers
       │                │
       ▼                ▼
┌─────────────────────────────────┐
│        User Wallets             │
│    (Contributors/Winners)       │
└─────────────────────────────────┘
```

See [ARCHITECTURE.md](./ARCHITECTURE.md) for detailed architecture documentation.

## 🔄 Workflow Examples

### Bounty Workflow

1. **Create Bounty**: Project maintainer locks funds with deadline
2. **Work Completion**: Contributor submits PR, gets merged
3. **Verification**: Backend verifies work quality
4. **Release**: Backend releases funds to contributor
5. **Alternative**: If deadline passes, anyone can trigger refund

### Program Workflow

1. **Initialize**: Create program escrow with authorized payout key
2. **Fund Pool**: Lock prize pool funds (can be done incrementally)
3. **Hackathon**: Participants build and submit projects
4. **Judging**: Backend determines winners and prize amounts
5. **Distribution**: Backend executes batch payout to all winners

## 📦 Dependencies

- `soroban-sdk` - Soroban smart contract SDK
- Rust standard library (no_std compatible)

## 🛠️ Development

### Project Structure

```
contracts/
├── bounty_escrow/          # Bounty escrow contract
│   ├── contracts/
│   │   └── escrow/
│   │       └── src/
│   │           └── lib.rs  # Main contract code
│   ├── Cargo.toml
│   └── README.md
├── program-escrow/         # Program escrow contract
│   ├── src/
│   │   ├── lib.rs          # Main contract code
│   │   └── test.rs         # Tests
│   ├── Cargo.toml
│   └── README.md
├── grainlify-core/         # Core contract
│   ├── src/
│   │   └── lib.rs          # Main contract code
│   ├── Cargo.toml
│   └── README.md
├── ARCHITECTURE.md         # Architecture documentation
├── DOCUMENTATION.md        # Developer documentation
├── SECURITY.md            # Security documentation
├── VERSIONS.md            # Version history
└── README.md              # This file
```

### Code Style

- Follow Rust standard formatting (`cargo fmt`)
- Use `cargo clippy` for linting
- Document all public functions with `///` comments
- Include usage examples in documentation

### Contributing

1. Fork the repository
2. Create a feature branch
3. Make changes with comprehensive tests
4. Update documentation
5. Submit pull request

## 📝 License

See the main project LICENSE file.

## 🤝 Support

- **Documentation**: See [DOCUMENTATION.md](./DOCUMENTATION.md)
- **Security**: See [SECURITY.md](./SECURITY.md)
- **Issues**: Open a GitHub issue
- **Questions**: Contact the development team

## 🔗 Resources

- [Soroban Documentation](https://soroban.stellar.org/docs)
- [Stellar SDK](https://github.com/stellar/js-stellar-sdk)
- [Soroban Examples](https://github.com/stellar/soroban-examples)
- [Grainlify Platform](https://grainlify.io) (if available)

## ✅ Audit Status

**Status**: Not yet audited

**Recommendations**:
- Complete security audit before mainnet deployment
- Test extensively on testnet
- Implement multisig for admin operations
- Monitor all transactions in production

See [SECURITY.md](./SECURITY.md) for audit preparation checklist.

---

**Version**: 1.0.0  
**Last Updated**: 2026-01-26  
**Soroban SDK Version**: Latest compatible version
