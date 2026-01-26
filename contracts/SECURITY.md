# Security Documentation - Grainlify Smart Contracts

This document outlines the security model, considerations, and best practices for the Grainlify smart contract ecosystem.

## Table of Contents

- [Security Model Overview](#security-model-overview)
- [Authorization Model](#authorization-model)
- [Attack Surface Analysis](#attack-surface-analysis)
- [Known Limitations](#known-limitations)
- [Security Best Practices](#security-best-practices)
- [Audit Preparation](#audit-preparation)
- [Incident Response](#incident-response)
- [Upgrade Safety](#upgrade-safety)

## Security Model Overview

The Grainlify smart contracts implement a **centralized authorization model** with the Grainlify backend acting as a trusted intermediary. This design prioritizes:

1. **Off-chain Verification**: Work verification happens off-chain via GitHub integration
2. **On-chain Enforcement**: Fund custody and transfers are enforced on-chain
3. **Admin Control**: A single admin key (backend) authorizes all fund releases
4. **Deadline Protection**: Time-locked refunds prevent permanent fund loss

### Trust Assumptions

- **Backend Security**: The Grainlify backend is trusted to verify work correctly
- **Admin Key Security**: The admin private key is securely stored and managed
- **Stellar Network**: The underlying Stellar network operates correctly
- **Token Contract**: The XLM token contract functions as expected

## Authorization Model

### Bounty Escrow Contract

| Function | Authorization Required | Who Can Call |
|----------|----------------------|--------------|
| `init` | None (first-time only) | Anyone (once) |
| `lock_funds` | Depositor signature | Depositor |
| `release_funds` | Admin signature | Admin only |
| `refund` | None (deadline-based) | Anyone (after deadline) |
| `get_escrow_info` | None (read-only) | Anyone |
| `get_balance` | None (read-only) | Anyone |

**Key Security Features:**
- ✅ Depositor must authorize fund locking (prevents unauthorized deposits)
- ✅ Only admin can release funds (prevents unauthorized withdrawals)
- ✅ Refunds are time-locked (prevents premature refunds)
- ✅ Refunds are permissionless after deadline (prevents stuck funds)

### Program Escrow Contract

| Function | Authorization Required | Who Can Call |
|----------|----------------------|--------------|
| `init_program` | None (first-time only) | Anyone (once) |
| `lock_program_funds` | None | Anyone |
| `batch_payout` | Authorized payout key | Admin only |
| `single_payout` | Authorized payout key | Admin only |
| `get_program_info` | None (read-only) | Anyone |
| `get_remaining_balance` | None (read-only) | Anyone |

**Key Security Features:**
- ✅ Only authorized key can distribute funds (prevents unauthorized payouts)
- ✅ Balance validation prevents overdrafts
- ✅ Overflow protection on total calculations
- ✅ Immutable payout history (audit trail)

### Grainlify Core Contract

| Function | Authorization Required | Who Can Call |
|----------|----------------------|--------------|
| `init` | None (first-time only) | Anyone (once) |
| `upgrade` | Admin signature | Admin only |
| `get_version` | None (read-only) | Anyone |
| `set_version` | Admin signature | Admin only |

**Key Security Features:**
- ✅ Only admin can upgrade contract code
- ✅ Version tracking for migration safety
- ✅ Storage persists across upgrades

## Attack Surface Analysis

### Potential Attack Vectors

#### 1. Admin Key Compromise

**Risk Level**: 🔴 CRITICAL

**Description**: If the admin private key is compromised, an attacker could:
- Release all locked bounty funds to arbitrary addresses
- Drain program prize pools
- Upgrade contracts to malicious code

**Mitigations**:
- Store admin key in hardware security module (HSM) or secure key management service
- Implement multi-signature requirement for production
- Monitor all admin transactions for anomalies
- Use time-locked or threshold signatures for large transfers

#### 2. Reentrancy Attacks

**Risk Level**: 🟢 LOW

**Description**: Reentrancy attacks where a malicious contract calls back into the escrow contract during execution.

**Mitigations**:
- ✅ Soroban's execution model prevents reentrancy by design
- ✅ State updates happen before external calls (checks-effects-interactions pattern)
- ✅ No callbacks to untrusted contracts

#### 3. Integer Overflow/Underflow

**Risk Level**: 🟢 LOW

**Description**: Arithmetic operations could overflow or underflow, causing incorrect balances.

**Mitigations**:
- ✅ Rust's default checked arithmetic prevents silent overflows
- ✅ Program escrow uses `checked_add` for total calculations
- ✅ Panics on overflow rather than wrapping

#### 4. Deadline Manipulation

**Risk Level**: 🟡 MEDIUM

**Description**: Incorrect deadline handling could allow premature refunds or prevent legitimate refunds.

**Mitigations**:
- ✅ Deadlines use Soroban's ledger timestamp (cannot be manipulated)
- ✅ Refund logic strictly enforces `now >= deadline`
- ⚠️ Ensure depositors set reasonable deadlines (off-chain validation)

#### 5. Denial of Service (DoS)

**Risk Level**: 🟡 MEDIUM

**Description**: Attacker could create many small escrows to bloat storage or make batch operations expensive.

**Mitigations**:
- ✅ Soroban's resource limits prevent unbounded operations
- ✅ Storage costs discourage spam
- ⚠️ Consider minimum escrow amounts in production

#### 6. Front-Running

**Risk Level**: 🟢 LOW

**Description**: Attacker observes pending transactions and submits competing transactions with higher fees.

**Mitigations**:
- ✅ Admin-only release operations prevent front-running of payouts
- ✅ Depositor authorization prevents unauthorized fund locking
- ⚠️ Refunds are permissionless but only benefit the original depositor

## Known Limitations

### 1. Centralized Admin Control

**Limitation**: Single admin key controls all fund releases.

**Impact**: 
- Admin key compromise = total fund loss
- Admin key loss = funds stuck (until deadline for bounties)

**Recommendations**:
- Implement multisig for production
- Use time-locked transactions for large payouts
- Consider DAO governance for admin operations

### 2. No Dispute Resolution

**Limitation**: Contracts have no on-chain dispute resolution mechanism.

**Impact**:
- Disputes must be resolved off-chain
- Admin has final authority on releases

**Recommendations**:
- Implement clear dispute resolution process off-chain
- Document all decisions for transparency
- Consider third-party arbitration for large bounties

### 3. Token Contract Dependency

**Limitation**: Contracts depend on external token contract (XLM).

**Impact**:
- Token contract bugs could affect escrows
- Token contract upgrades could break compatibility

**Recommendations**:
- Only use well-audited token contracts
- Test thoroughly with target token contract
- Monitor token contract for upgrades

### 4. Storage TTL Management

**Limitation**: Persistent storage has time-to-live (TTL) limits.

**Impact**:
- Long-running escrows could expire if TTL not extended
- Data loss if TTL expires

**Recommendations**:
- Implement TTL monitoring and extension
- Set conservative TTL values
- Document TTL requirements for integrators

## Security Best Practices

### For Developers

1. **Input Validation**
   ```rust
   // Always validate inputs
   if amount <= 0 {
       panic!("Amount must be positive");
   }
   ```

2. **Authorization Checks**
   ```rust
   // Always check authorization first
   admin.require_auth();
   // Then proceed with operation
   ```

3. **State Updates Before External Calls**
   ```rust
   // Update state first
   escrow.status = EscrowStatus::Released;
   env.storage().persistent().set(&key, &escrow);
   
   // Then make external call
   token_client.transfer(&contract, &recipient, &amount);
   ```

4. **Safe Arithmetic**
   ```rust
   // Use checked arithmetic
   let total = total_payout
       .checked_add(amount)
       .unwrap_or_else(|| panic!("Overflow"));
   ```

### For Integrators

1. **Admin Key Management**
   - Use hardware security modules (HSM) for production
   - Implement key rotation policies
   - Never commit keys to version control
   - Use environment variables or secrets management

2. **Transaction Monitoring**
   - Monitor all contract transactions
   - Set up alerts for unusual activity
   - Log all admin operations
   - Implement rate limiting

3. **Testing**
   - Test on testnet extensively before mainnet
   - Test all error conditions
   - Test with realistic amounts and scenarios
   - Perform load testing

4. **Upgrade Safety**
   - Test upgrades on testnet first
   - Have rollback plan ready
   - Communicate upgrades to users
   - Monitor post-upgrade behavior

## Audit Preparation

### Pre-Audit Checklist

- [ ] All functions have comprehensive documentation
- [ ] All error conditions are documented
- [ ] Security considerations are documented
- [ ] Test coverage is >90%
- [ ] Integration tests cover all workflows
- [ ] Known limitations are documented
- [ ] Deployment scripts are tested
- [ ] Admin key management is documented

### Audit Scope

**In Scope:**
- All contract logic and state management
- Authorization and access control
- Arithmetic operations and overflow handling
- Storage management and TTL handling
- Event emission and data integrity

**Out of Scope:**
- Off-chain backend verification logic
- Frontend integration code
- Stellar network security
- Token contract implementation

### Audit Questions to Address

1. Can funds be stolen or locked permanently?
2. Can unauthorized parties release funds?
3. Are there any reentrancy vulnerabilities?
4. Can arithmetic operations overflow/underflow?
5. Is the upgrade mechanism secure?
6. Are deadlines enforced correctly?
7. Can storage be corrupted or lost?

## Incident Response

### Severity Levels

**CRITICAL** 🔴
- Funds at risk of theft
- Admin key compromised
- Contract upgrade vulnerability

**HIGH** 🟠
- Funds locked but not stolen
- Authorization bypass
- Data corruption

**MEDIUM** 🟡
- Incorrect state updates
- Event emission failures
- Performance degradation

**LOW** 🟢
- Documentation errors
- Minor UI issues
- Non-critical bugs

### Response Procedures

#### Admin Key Compromise

1. **Immediate Actions** (within 1 hour)
   - Revoke compromised key access
   - Deploy new admin key
   - Pause all operations if possible
   - Notify users

2. **Investigation** (within 24 hours)
   - Review all transactions from compromised key
   - Identify affected escrows
   - Assess total impact

3. **Recovery** (within 1 week)
   - Upgrade contracts if necessary
   - Restore affected escrows
   - Implement additional security measures

#### Contract Bug Discovery

1. **Assessment** (within 2 hours)
   - Determine severity and impact
   - Identify affected contracts/escrows
   - Estimate potential losses

2. **Mitigation** (within 24 hours)
   - Pause affected operations if possible
   - Prepare contract fix
   - Test fix thoroughly on testnet

3. **Deployment** (within 1 week)
   - Deploy fixed contract
   - Migrate affected state if necessary
   - Communicate with users

## Upgrade Safety

### Upgrade Process

1. **Preparation**
   - Write and test new contract code
   - Document all changes
   - Prepare migration scripts if needed
   - Test on testnet extensively

2. **Deployment**
   - Upload new WASM to Stellar
   - Get WASM hash
   - Call `upgrade` function with new hash
   - Update version number

3. **Verification**
   - Verify new code is active
   - Test all functions
   - Monitor for issues
   - Communicate upgrade to users

### Upgrade Risks

⚠️ **Storage Compatibility**: New code must be compatible with existing storage layout

⚠️ **Breaking Changes**: Function signatures must remain compatible or require migration

⚠️ **State Migration**: Complex state changes require careful migration logic

### Rollback Strategy

If an upgrade fails:
1. Deploy previous WASM version
2. Call `upgrade` with old WASM hash
3. Verify rollback successful
4. Investigate failure cause

**Note**: Rollback may not be possible if state has been migrated incompatibly.

## Contact and Reporting

For security issues or questions:
- **Email**: security@grainlify.io (if available)
- **GitHub**: Open a private security advisory
- **Emergency**: Contact admin key holders directly

**Responsible Disclosure**: Please report security vulnerabilities privately before public disclosure.
