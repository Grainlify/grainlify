# Reentrancy Threat Model - Grainlify

## Executive Summary

This document analyzes all reentrancy attack vectors in the Grainlify smart contract system and documents the guards implemented to prevent them.

## Threat Overview

### What is Reentrancy?

A reentrancy attack occurs when an external call allows the called contract to re-enter the calling contract before the first invocation completes, potentially leading to:
- Double withdrawals
- State inconsistencies
- Unauthorized fund access
- Logic bypass

### Soroban-Specific Considerations

While Soroban's execution model provides some reentrancy protection through:
- Controlled execution environment
- Host function barriers
- Limited callback capabilities

**We still implement explicit guards because:**
1. Defense in depth
2. Future protocol changes
3. Auditability
4. Best practices

## Attack Surface Analysis

### 1. Bounty Escrow Contract

**Entry Points:**
- `create_bounty()` - Creates escrow, transfers tokens
- `claim_bounty()` - Releases funds to recipient
- `refund_bounty()` - Returns funds to creator
- `cancel_bounty()` - Cancels and refunds

**External Calls:**
```rust
// Potential reentrancy vector
token_client.transfer(&from, &escrow, &amount);
// ↓ Could callback to attacker contract
// ↓ Attacker could call claim_bounty() again
```

**Attack Scenario:**
1. Attacker creates bounty with malicious token
2. Malicious token's `transfer()` calls back to attacker
3. Attacker calls `claim_bounty()` again before state updates
4. Double claim achieved ❌

**Mitigation:** Reentrancy guard on all state-changing functions

### 2. Program Escrow Contract

**Entry Points:**
- `deposit_to_program()`
- `withdraw_from_program()`
- `batch_distribute()`

**Reentrancy Risk:** HIGH
- Token transfers in loops
- External contract calls
- State updates after transfers

**Attack Scenario:**
1. Attacker included in batch distribution
2. Malicious contract triggers reentrancy in callback
3. Re-enters `batch_distribute()` or `withdraw_from_program()`
4. Drains escrow ❌

### 3. Token Transfer Callbacks

**Risk Areas:**
- Any function that calls `token.transfer()`
- Functions that accept user-provided addresses
- Batch operations with multiple transfers

## Identified Vulnerabilities

### Critical

1. **Bounty Claim Double Spend**
   - Location: `claim_bounty()`
   - Severity: Critical
   - Status: ⏳ Pending Remediation

2. **Escrow Withdrawal Reentrancy**
   - Location: `withdraw_from_program()`
   - Severity: Critical
   - Status: ⏳ Pending Remediation

### High

3. **Batch Distribution State Corruption**
   - Location: `batch_distribute()`
   - Severity: High
   - Status: ⏳ Pending Remediation

4. **Refund Reentrancy**
   - Location: `refund_bounty()`
   - Severity: High
   - Status: ⏳ Pending Remediation

## Guard Implementation Strategy

### Checks-Effects-Interactions Pattern

```rust
pub fn withdraw(env: Env, user: Address, amount: i128) -> Result<(), Error> {
    // 1. CHECKS - Reentrancy guard + validation
    ReentrancyGuard::enter(&env)?;
    require!(amount > 0, Error::InvalidAmount);
    
    // 2. EFFECTS - Update state BEFORE external calls
    let balance = get_balance(&env, &user);
    require!(balance >= amount, Error::InsufficientBalance);
    set_balance(&env, &user, balance - amount);
    
    // 3. INTERACTIONS - External calls last
    token_client.transfer(&contract, &user, &amount);
    
    // 4. CLEANUP - Exit guard
    ReentrancyGuard::exit(&env);
    
    Ok(())
}
```

## Testing Strategy

1. **Direct Reentrancy Tests** - Same function re-entry
2. **Cross-Function Tests** - Different function re-entry
3. **Cross-Contract Tests** - Multi-contract reentrancy
4. **Nested Call Tests** - Deep call stack reentrancy
5. **Panic Recovery Tests** - Guard cleanup on failure
