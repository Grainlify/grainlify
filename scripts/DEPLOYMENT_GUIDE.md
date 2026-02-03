# Grainlify Smart Contract Deployment Guide

This guide covers the complete deployment, upgrade, migration, and rollback process for Grainlify smart contracts on the Stellar/Soroban network.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Quick Start](#quick-start)
3. [Configuration](#configuration)
4. [Deployment](#deployment)
5. [Upgrades](#upgrades)
6. [Migrations](#migrations)
7. [Verification](#verification)
8. [Rollbacks](#rollbacks)
9. [Troubleshooting](#troubleshooting)
10. [Best Practices](#best-practices)

---

## Prerequisites

### Required Tools

1. **Rust Toolchain**
   ```bash
   rustup install stable
   rustup default stable
   rustup target add wasm32-unknown-unknown
   ```

2. **Stellar CLI**
   ```bash
   cargo install --locked stellar-cli
   stellar version
   ```

3. **jq** (JSON processor - recommended)
   ```bash
   # macOS
   brew install jq
   
   # Ubuntu/Debian
   apt-get install jq
   ```

### Account Setup

#### Testnet

```bash
# Generate a new keypair
stellar keys generate --global grainlify-deployer --network testnet

# Get the public address
stellar keys public-key grainlify-deployer

# Fund via Friendbot
stellar keys fund grainlify-deployer --network testnet
# Or manually:
# curl "https://friendbot.stellar.org?addr=<YOUR_PUBLIC_KEY>"
```

#### Mainnet

For mainnet, you need:
- A funded Stellar account
- Secure key storage (hardware wallet recommended)
- Sufficient XLM for deployment fees (~100 XLM recommended)

---

## Quick Start

### Deploy to Testnet

```bash
# 1. Navigate to scripts directory
cd scripts

# 2. Copy and configure environment
cp config/.env.example config/.env
# Edit config/.env with your credentials

# 3. Deploy all contracts
./deploy_testnet.sh

# Or deploy a specific contract
./deploy_testnet.sh grainlify-core
```

### Deploy to Mainnet

```bash
# Ensure ALLOW_MAINNET_DEPLOY=true in .env or environment
./deploy_mainnet.sh
```

---

## Configuration

### Environment Variables

Create `scripts/config/.env` from the example:

```bash
# Network selection
NETWORK=testnet

# Stellar credentials
STELLAR_SECRET_KEY=S...YOUR_SECRET_KEY...
STELLAR_PUBLIC_KEY=G...YOUR_PUBLIC_KEY...

# Admin address (defaults to STELLAR_PUBLIC_KEY)
ADMIN_ADDRESS=

# Token contract for escrow (XLM native asset default)
TOKEN_ADDRESS=CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC

# Deployment settings
DRY_RUN=false
VERBOSE=true
CONFIRM_DEPLOY=true
```

### Network Configuration

Network settings are in `scripts/config/networks.json`:

| Network | RPC URL | Use Case |
|---------|---------|----------|
| testnet | soroban-testnet.stellar.org | Development/Testing |
| mainnet | soroban-mainnet.stellar.org | Production |
| local | localhost:8000 | Local development |

---

## Deployment

### Available Contracts

| Contract | Description | Path |
|----------|-------------|------|
| grainlify-core | Core upgrade system | contracts/grainlify-core |
| program-escrow | Program prize pool escrow | contracts/program-escrow |
| bounty-escrow | Bounty escrow | contracts/bounty_escrow |

### Deployment Commands

```bash
# Deploy all contracts
./deploy.sh -n testnet

# Deploy specific contract
./deploy.sh -n testnet -c grainlify-core

# Dry run (no changes)
./deploy.sh -n testnet -d

# Skip confirmations
./deploy.sh -n testnet -y

# Verbose output
./deploy.sh -n testnet -v
```

### Deployment Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                      Deployment Process                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  1. Build Contract                                               │
│     cargo build --release --target wasm32-unknown-unknown        │
│                        │                                         │
│                        ▼                                         │
│  2. Upload WASM                                                  │
│     stellar contract upload --wasm <file>                        │
│     → Returns: WASM hash                                         │
│                        │                                         │
│                        ▼                                         │
│  3. Deploy Contract                                              │
│     stellar contract deploy --wasm-hash <hash>                   │
│     → Returns: Contract ID                                       │
│                        │                                         │
│                        ▼                                         │
│  4. Initialize Contract                                          │
│     stellar contract invoke -- init --admin <addr>               │
│                        │                                         │
│                        ▼                                         │
│  5. Save Deployment Record                                       │
│     deployments/<network>_deployments.json                       │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Deployment Records

Deployments are recorded in `contracts/deployments/<network>_deployments.json`:

```json
{
  "deployments": [
    {
      "contract_name": "grainlify-core",
      "contract_id": "CABC...",
      "wasm_hash": "abc123...",
      "version": 1,
      "deployed_at": "2024-01-15T10:30:00Z",
      "network": "testnet"
    }
  ]
}
```

---

## Upgrades

### Upgrade Command

```bash
# Basic upgrade
./upgrade.sh -n testnet -c grainlify-core

# Upgrade with specific contract ID
./upgrade.sh -n testnet -c grainlify-core -i CABC...

# Upgrade with existing WASM hash
./upgrade.sh -n testnet -c grainlify-core -w abc123...

# Upgrade and migrate
./upgrade.sh -n testnet -c grainlify-core -m -t 20000
```

### Upgrade Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                       Upgrade Process                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  1. Pre-Upgrade Checks                                           │
│     ├─ Verify current version                                    │
│     ├─ Backup deployment state                                   │
│     └─ Verify admin access                                       │
│                        │                                         │
│                        ▼                                         │
│  2. Build & Install New WASM                                     │
│     cargo build --release                                        │
│     stellar contract install --wasm <file>                       │
│                        │                                         │
│                        ▼                                         │
│  3. Execute Upgrade                                              │
│     stellar contract invoke -- upgrade --new_wasm_hash <hash>    │
│                        │                                         │
│                        ▼                                         │
│  4. Post-Upgrade Verification                                    │
│     ├─ Verify new version                                        │
│     ├─ Health check                                              │
│     └─ Update records                                            │
│                        │                                         │
│                        ▼                                         │
│  5. Migration (Optional)                                         │
│     stellar contract invoke -- migrate --target_version <v>      │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Migrations

### Migration Strategies

| Strategy | Description | Use Case |
|----------|-------------|----------|
| full | Direct migration to target version | Simple migrations |
| incremental | Step-by-step through versions | Complex migrations |
| batch | Process data in batches | Large data sets |

### Migration Commands

```bash
# Full migration
./migrate.sh -n testnet -c grainlify-core -t 20000

# Incremental migration
./migrate.sh -n testnet -c grainlify-core -t 20000 -s incremental

# Batch migration
./migrate.sh -n testnet -c grainlify-core -t 20000 -s batch --batch-size 50

# Dry run
./migrate.sh -n testnet -c grainlify-core -t 20000 -d
```

### Version Numbering

Version numbers use numeric encoding: `MAJOR*10000 + MINOR*100 + PATCH`

| SemVer | Numeric |
|--------|---------|
| 1.0.0 | 10000 |
| 1.1.0 | 10100 |
| 2.0.0 | 20000 |
| 2.1.3 | 20103 |

---

## Verification

### Verification Modes

| Mode | Checks |
|------|--------|
| quick | Contract exists, version |
| standard | + Admin access, function calls |
| full | + State consistency, error handling |

### Verification Commands

```bash
# Quick verification
./verify.sh -n testnet -c grainlify-core --quick

# Standard verification (default)
./verify.sh -n testnet -c grainlify-core

# Full verification
./verify.sh -n testnet --full

# Verify all deployed contracts
./verify.sh -n testnet
```

### Verification Output

```
Verifying: grainlify-core
Contract ID: CABC...
Network:     testnet
Mode:        standard

  ✓ Contract Exists: Contract is deployed
  ✓ Version Check: Version: 10000
  ✓ Admin Access: Admin: GADM...
  ✓ Function: get_version: Returns: 10000

==============================================================================
Verification Summary
==============================================================================
  Passed:   4
  Failed:   0
  Warnings: 0
```

---

## Rollbacks

### Rollback Commands

```bash
# List available rollback points
./rollback.sh -n testnet -c grainlify-core --list

# Rollback to previous version
./rollback.sh -n testnet -c grainlify-core

# Rollback to specific WASM
./rollback.sh -n testnet -c grainlify-core -w abc123...

# Emergency rollback (skip confirmations)
./rollback.sh -n mainnet -c grainlify-core --emergency
```

### Rollback Safety

The rollback script:
1. Creates pre-rollback backup
2. Executes rollback (upgrade to previous WASM)
3. Verifies rollback success
4. Records rollback in deployment history

---

## Troubleshooting

### Common Issues

#### "Contract not found"
```bash
# Verify contract exists using contract info
stellar contract info wasm --id <CONTRACT_ID> --network testnet

# Check deployment records
cat contracts/deployments/testnet_deployments.json
```

#### "Unauthorized" errors
```bash
# Verify you're using the admin account
stellar keys public-key grainlify-deployer

# Check contract admin (requires source account)
stellar contract invoke --id <CONTRACT_ID> --source-account <YOUR_KEY> --network testnet --send=no -- get_admin
```

#### Build failures
```bash
# Ensure wasm target is installed
rustup target add wasm32-unknown-unknown

# Check Rust version
rustup show

# Clean and rebuild
cargo clean
cargo build --release --target wasm32-unknown-unknown
```

### Logs

Check deployment logs in:
- `contracts/deployments/logs/`
- `contracts/deployments/backups/`

---

## Best Practices

### Before Deployment

- [ ] Test contracts thoroughly on testnet
- [ ] Audit contract code
- [ ] Document breaking changes
- [ ] Plan rollback strategy

### For Mainnet

- [ ] Complete testnet deployment cycle
- [ ] Security audit completed
- [ ] Admin keys secured (hardware wallet/multisig)
- [ ] Sufficient XLM for fees
- [ ] Team notified
- [ ] Rollback plan documented

### After Deployment

- [ ] Verify all contracts functioning
- [ ] Update dependent services with new contract IDs
- [ ] Monitor for issues
- [ ] Keep deployment records secure

### Security

- Never commit secret keys to version control
- Use hardware wallets for mainnet admin keys
- Consider multisig for critical operations
- Keep WASM hashes for all versions for potential rollback

---

## Script Reference

| Script | Purpose |
|--------|---------|
| `deploy.sh` | Main deployment script |
| `deploy_testnet.sh` | Testnet deployment convenience |
| `deploy_mainnet.sh` | Mainnet deployment with safety checks |
| `upgrade.sh` | Contract upgrade with validation |
| `migrate.sh` | Data structure migration |
| `verify.sh` | Deployment verification |
| `rollback.sh` | Rollback to previous version |
| `utils.sh` | Shared utility functions |

---

## Support

For issues or questions:
1. Check [Troubleshooting](#troubleshooting) section
2. Review Stellar documentation
3. Open an issue on GitHub
