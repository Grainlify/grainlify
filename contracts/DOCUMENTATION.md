# Grainlify Smart Contracts - Developer Documentation

This document provides comprehensive guidance for developers integrating with or maintaining the Grainlify smart contracts on the Stellar Soroban platform.

## Table of Contents

- [Overview](#overview)
- [Contract Ecosystem](#contract-ecosystem)
- [Contract Interaction Patterns](#contract-interaction-patterns)
- [Event Monitoring](#event-monitoring)
- [Error Handling](#error-handling)
- [Integration Examples](#integration-examples)
- [Testing Strategies](#testing-strategies)
- [Best Practices](#best-practices)

## Overview

Grainlify uses three main smart contracts to manage bounties and program prize pools on the Stellar network:

1. **Bounty Escrow Contract** - Individual bounty escrow management
2. **Program Escrow Contract** - Hackathon/program prize pool management
3. **Grainlify Core Contract** - Contract upgradeability and versioning

All contracts are written in Rust using the Soroban SDK and deployed on the Stellar network.

## Contract Ecosystem

### Architecture Flow

```
┌─────────────────┐
│  Grainlify      │
│  Backend        │
│  (Admin Key)    │
└────────┬────────┘
         │
         │ Manages & Authorizes
         │
         ▼
┌────────────────────────────────────┐
│   Smart Contracts (Soroban)        │
│                                    │
│  ┌──────────────────────────────┐ │
│  │  Bounty Escrow Contract      │ │
│  │  - Lock funds per bounty     │ │
│  │  - Release to contributors   │ │
│  │  - Deadline-based refunds    │ │
│  └──────────────────────────────┘ │
│                                    │
│  ┌──────────────────────────────┐ │
│  │  Program Escrow Contract     │ │
│  │  - Lock prize pools          │ │
│  │  - Batch payouts to winners  │ │
│  │  - Payout history tracking   │ │
│  └──────────────────────────────┘ │
│                                    │
│  ┌──────────────────────────────┐ │
│  │  Grainlify Core Contract     │ │
│  │  - Contract upgrades         │ │
│  │  - Version management        │ │
│  └──────────────────────────────┘ │
└────────────────────────────────────┘
         │
         │ Transfers
         ▼
┌────────────────────┐
│  User Wallets      │
│  (Contributors)    │
└────────────────────┘
```

## Contract Interaction Patterns

### Bounty Escrow Workflow

#### 1. Initialize Contract (One-time)

```rust
use soroban_sdk::{Address, Env};

// Deploy and initialize
let admin = Address::from_string("GADMIN...");
let xlm_token = Address::from_string("CTOKEN...");
contract.init(env, admin, xlm_token);
```

#### 2. Lock Funds for a Bounty

```rust
// Project maintainer locks funds
let depositor = Address::from_string("GMAINTAINER...");
let bounty_id = 12345u64;
let amount = 1000_0000000i128; // 1000 XLM
let deadline = env.ledger().timestamp() + 2592000; // 30 days

contract.lock_funds(env, depositor, bounty_id, amount, deadline)?;
```

#### 3. Release Funds to Contributor (Admin Only)

```rust
// Backend verifies work and releases funds
let bounty_id = 12345u64;
let contributor = Address::from_string("GCONTRIBUTOR...");

contract.release_funds(env, bounty_id, contributor)?;
```

#### 4. Refund After Deadline

```rust
// Anyone can trigger refund after deadline
let bounty_id = 12345u64;
contract.refund(env, bounty_id)?;
// Funds returned to original depositor
```

### Program Escrow Workflow

#### 1. Initialize Program

```rust
let program_id = String::from_str(&env, "hackathon-2024");
let backend_key = Address::from_string("GBACKEND...");
let xlm_token = Address::from_string("CTOKEN...");

let program_data = contract.init_program(
    env, 
    program_id, 
    backend_key, 
    xlm_token
);
```

#### 2. Lock Prize Pool Funds

```rust
// Can be called multiple times to add funds
let prize_pool = 50000_0000000i128; // 50,000 XLM
contract.lock_program_funds(env, prize_pool)?;
```

#### 3. Distribute Prizes (Batch)

```rust
use soroban_sdk::vec;

let winners = vec![&env,
    Address::from_string("GWINNER1..."),
    Address::from_string("GWINNER2..."),
    Address::from_string("GWINNER3..."),
];

let prizes = vec![&env,
    10000_0000000i128,  // 1st: 10,000 XLM
    5000_0000000i128,   // 2nd: 5,000 XLM
    2500_0000000i128,   // 3rd: 2,500 XLM
];

contract.batch_payout(env, winners, prizes)?;
```

#### 4. Query Program State

```rust
// Get complete program info
let info = contract.get_program_info(env);

// Or just get remaining balance
let remaining = contract.get_remaining_balance(env);
```

## Event Monitoring

All contracts emit events for off-chain tracking and indexing.

### Bounty Escrow Events

```rust
// FundsLocked
event: ("funds_locked", bounty_id)
data: (depositor, amount, deadline)

// FundsReleased
event: ("funds_released", bounty_id)
data: (contributor, amount)

// FundsRefunded
event: ("funds_refunded", bounty_id)
data: (depositor, amount)
```

### Program Escrow Events

```rust
// ProgramInitialized
event: "ProgramInit"
data: (program_id, authorized_key, token_address, initial_balance)

// FundsLocked
event: "FundsLocked"
data: (program_id, amount, new_remaining_balance)

// BatchPayout
event: "BatchPayout"
data: (program_id, recipient_count, total_amount, new_remaining_balance)

// Payout
event: "Payout"
data: (program_id, recipient, amount, new_remaining_balance)
```

### Event Monitoring Example

```typescript
// Off-chain event listener (pseudo-code)
sorobanClient.on('event', (event) => {
  if (event.topic.includes('funds_released')) {
    const [bountyId] = event.topic;
    const [contributor, amount] = event.data;
    
    // Update database
    await db.bounties.update(bountyId, {
      status: 'paid',
      contributor: contributor,
      paidAmount: amount,
      paidAt: new Date()
    });
  }
});
```

## Error Handling

### Bounty Escrow Errors

| Error Code | Error Name | Description | Resolution |
|------------|------------|-------------|------------|
| 1 | `AlreadyInitialized` | Contract already initialized | Cannot re-initialize |
| 2 | `NotInitialized` | Contract not initialized | Call `init` first |
| 3 | `BountyExists` | Bounty ID already exists | Use unique bounty ID |
| 4 | `BountyNotFound` | No bounty with this ID | Check bounty ID |
| 5 | `FundsNotLocked` | Funds already released/refunded | Cannot modify |
| 6 | `DeadlineNotPassed` | Refund deadline not reached | Wait for deadline |
| 7 | `Unauthorized` | Caller not authorized | Use admin key |

### Error Handling Example

```rust
match contract.release_funds(env, bounty_id, contributor) {
    Ok(()) => {
        // Success - funds released
    },
    Err(Error::BountyNotFound) => {
        // Handle: bounty doesn't exist
    },
    Err(Error::FundsNotLocked) => {
        // Handle: already released or refunded
    },
    Err(Error::Unauthorized) => {
        // Handle: not admin
    },
    Err(e) => {
        // Handle other errors
    }
}
```

## Integration Examples

### Backend Integration (TypeScript/Node.js)

```typescript
import { SorobanRpc, Contract, Keypair } from '@stellar/stellar-sdk';

class GrainlifyContractClient {
  private contract: Contract;
  private adminKeypair: Keypair;
  
  constructor(contractId: string, adminSecret: string) {
    this.contract = new Contract(contractId);
    this.adminKeypair = Keypair.fromSecret(adminSecret);
  }
  
  async releaseBountyFunds(
    bountyId: number, 
    contributorAddress: string
  ): Promise<void> {
    const tx = this.contract
      .call('release_funds', bountyId, contributorAddress)
      .build();
    
    tx.sign(this.adminKeypair);
    
    const result = await this.server.sendTransaction(tx);
    await result.wait();
  }
  
  async batchPayoutWinners(
    winners: string[], 
    amounts: bigint[]
  ): Promise<void> {
    const tx = this.contract
      .call('batch_payout', winners, amounts)
      .build();
    
    tx.sign(this.adminKeypair);
    
    const result = await this.server.sendTransaction(tx);
    await result.wait();
  }
}
```

### Frontend Integration (React)

```typescript
import { useSorobanReact } from '@soroban-react/core';

function BountyDetails({ bountyId }: { bountyId: number }) {
  const { contractClient } = useSorobanReact();
  const [escrowInfo, setEscrowInfo] = useState(null);
  
  useEffect(() => {
    async function fetchEscrowInfo() {
      const info = await contractClient.call(
        'get_escrow_info', 
        bountyId
      );
      setEscrowInfo(info);
    }
    
    fetchEscrowInfo();
  }, [bountyId]);
  
  return (
    <div>
      <h2>Bounty #{bountyId}</h2>
      <p>Amount: {escrowInfo?.amount} stroops</p>
      <p>Status: {escrowInfo?.status}</p>
      <p>Deadline: {new Date(escrowInfo?.deadline * 1000).toLocaleString()}</p>
    </div>
  );
}
```

## Testing Strategies

### Unit Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};
    
    #[test]
    fn test_lock_and_release_funds() {
        let env = Env::default();
        let contract = BountyEscrowContractClient::new(&env, &contract_id);
        
        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);
        let token = create_token_contract(&env);
        
        // Initialize
        contract.init(&admin, &token.address);
        
        // Lock funds
        let bounty_id = 1u64;
        let amount = 1000i128;
        let deadline = env.ledger().timestamp() + 1000;
        
        contract.lock_funds(&depositor, &bounty_id, &amount, &deadline);
        
        // Release funds
        contract.release_funds(&bounty_id, &contributor);
        
        // Verify
        let info = contract.get_escrow_info(&bounty_id);
        assert_eq!(info.status, EscrowStatus::Released);
    }
}
```

### Integration Testing

Test on Stellar testnet before deploying to mainnet:

```bash
# Deploy to testnet
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/bounty_escrow.wasm \
  --network testnet

# Initialize
soroban contract invoke \
  --id CONTRACT_ID \
  --network testnet \
  -- init \
  --admin GADMIN... \
  --token CTOKEN...
```

## Best Practices

### Security

1. **Admin Key Management**
   - Store admin keys in secure key management systems (e.g., AWS KMS, HashiCorp Vault)
   - Consider using multisig for production admin operations
   - Rotate keys periodically

2. **Authorization Checks**
   - Always verify authorization before privileged operations
   - Use the backend as the sole admin for fund releases
   - Implement additional off-chain verification before releasing funds

3. **Amount Validation**
   - Validate amounts are positive before locking funds
   - Check sufficient balance before payouts
   - Use safe arithmetic to prevent overflows

### Performance

1. **Batch Operations**
   - Use `batch_payout` for multiple winners instead of individual calls
   - Reduces transaction fees and improves efficiency

2. **Event Indexing**
   - Index contract events for fast off-chain queries
   - Don't rely solely on contract storage for historical data

3. **Storage Management**
   - Be aware of storage TTL for persistent data
   - Extend TTL for long-running escrows

### Maintenance

1. **Version Tracking**
   - Always update version numbers after upgrades
   - Document breaking changes between versions
   - Test upgrades thoroughly on testnet

2. **Monitoring**
   - Monitor contract events in real-time
   - Set up alerts for failed transactions
   - Track contract balance vs. expected escrows

3. **Documentation**
   - Keep this documentation updated with contract changes
   - Document all integration patterns
   - Maintain changelog for contract versions

## Support and Resources

- **Soroban Documentation**: https://soroban.stellar.org/docs
- **Stellar SDK**: https://github.com/stellar/js-stellar-sdk
- **Contract Source**: See individual contract files in this directory
- **Architecture Overview**: See [ARCHITECTURE.md](./ARCHITECTURE.md)
- **Security Guidelines**: See [SECURITY.md](./SECURITY.md)

## Changelog

### Version 1.0.0 (Current)
- Initial release with comprehensive documentation
- Bounty Escrow Contract with deadline-based refunds
- Program Escrow Contract with batch payout support
- Grainlify Core Contract for upgradeability
