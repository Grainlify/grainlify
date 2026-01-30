# MEV and Front-Running Analysis

## Overview

This document analyzes potential MEV (Maximal Extractable Value) and front-running risks in the Grainlify escrow contracts and documents mitigations where applicable.

## Front-Running and MEV Risks

### 1. Bounty Escrow Contract

#### High-Risk Operations

**1.1 Permissionless Refunds (`refund`)**
- **Risk Level**: Medium
- **Description**: After deadline passes, anyone can call `refund()` to return funds to the depositor
- **Front-Running Vector**: 
  - Attacker monitors mempool for refund transactions
  - Submits their own refund transaction with higher gas price
  - Extracts value by being first to execute
- **Impact**: Low - funds only go to rightful owner (depositor or approved recipient)
- **Mitigation**: 
  - Acceptable risk: No funds can be stolen, only execution priority
  - Optional: Add minimum delay after deadline before refunds allowed
  - Optional: Implement commit-reveal scheme (complex, may not be practical)

**1.2 Large Payouts (`release_funds`, `batch_release_funds`)**
- **Risk Level**: Low-Medium
- **Description**: Admin-controlled releases to contributors
- **Front-Running Vector**:
  - Large payout transactions visible in mempool
  - Attacker could front-run with their own transaction
  - However, requires admin auth, so risk is limited
- **Impact**: Low - requires admin compromise
- **Mitigation**:
  - Admin-controlled operations reduce risk
  - Rate limiting already implemented
  - Payout caps can be configured via `ConfigLimits.max_bounty_amount`
  - Consider: Per-transaction payout limits

**1.3 Batch Operations (`batch_release_funds`)**
- **Risk Level**: Low
- **Description**: Multiple payouts in single transaction
- **Front-Running Vector**: Similar to single payouts
- **Impact**: Low - batch size limited to 100
- **Mitigation**:
  - `MAX_BATCH_SIZE` limits exposure
  - Total amount validation prevents overflow

### 2. Program Escrow Contract

#### High-Risk Operations

**2.1 Batch Payouts (`batch_payout`)**
- **Risk Level**: Low-Medium
- **Description**: Large batch payouts to multiple recipients
- **Front-Running Vector**:
  - Large batch transactions visible in mempool
  - Potential for sandwich attacks if recipients trade immediately
- **Impact**: Medium - depends on payout size and recipient behavior
- **Mitigation**:
  - Requires `authorized_payout_key` auth (backend-controlled)
  - Rate limiting implemented
  - Consider: Per-recipient payout caps
  - Consider: Minimum delay between batch operations

**2.2 Single Payouts (`single_payout`)**
- **Risk Level**: Low
- **Description**: Individual prize payouts
- **Front-Running Vector**: Similar to batch, but smaller scale
- **Impact**: Low - single recipient
- **Mitigation**:
  - Auth-controlled
  - Amount validation

**2.3 Automatic Schedule Releases (`release_prog_schedule_automatic`)**
- **Risk Level**: Low
- **Description**: Time-based automatic releases
- **Front-Running Vector**: 
  - Anyone can trigger after timestamp
  - Could be front-run for priority
- **Impact**: Low - funds go to predetermined recipient
- **Mitigation**:
  - Acceptable risk: No value extraction possible
  - Recipient is fixed in schedule

## Implemented Mitigations

### 1. Access Control
- **Admin-Only Operations**: Critical operations require admin/auth key
- **Rate Limiting**: Prevents rapid-fire attacks
- **Pause Mechanism**: Emergency stop capability

### 2. Operational Limits
- **Batch Size Limits**: `MAX_BATCH_SIZE = 100` prevents unbounded operations
- **Configurable Limits**: `ConfigLimits` allows setting max amounts
- **Amount Validation**: All operations validate amounts against balances

### 3. Design Patterns
- **Checks-Effects-Interactions**: State updates before external calls
- **Reentrancy Guards**: Prevents reentrancy attacks
- **Atomic Operations**: Batch operations are all-or-nothing

## Recommended Additional Mitigations

### 1. Payout Caps (Implemented)
- Add `max_payout_per_transaction` to `ConfigLimits`
- Enforce per-transaction limits on large payouts
- Reduces MEV extraction opportunity

### 2. Optional Delays (Documented, Not Implemented)
- Minimum delay between large operations
- Time-lock for refunds after deadline
- Trade-off: Security vs UX

### 3. Commit-Reveal Scheme (Not Recommended)
- Complex to implement
- Poor UX
- May not be necessary given current risk profile

## Risk Assessment Summary

| Operation | Risk Level | Front-Runnable | Value Extractable | Mitigation Status |
|-----------|-----------|----------------|-------------------|-------------------|
| `refund` (after deadline) | Medium | Yes | No (funds to owner) | Acceptable risk |
| `release_funds` | Low | Yes (if admin compromised) | Low | Auth + rate limiting |
| `batch_release_funds` | Low | Yes (if admin compromised) | Low | Auth + batch limits |
| `batch_payout` | Low-Medium | Yes | Medium | Auth + rate limiting |
| `single_payout` | Low | Yes | Low | Auth |
| `release_prog_schedule_automatic` | Low | Yes | No | Acceptable risk |

## Acceptable Risks

1. **Permissionless Refunds**: After deadline, anyone can trigger refund. This is by design to prevent funds from being stuck. No value can be extracted as funds only go to the depositor.

2. **Automatic Releases**: Anyone can trigger time-based releases. Recipient is predetermined, so no value extraction.

3. **Large Payouts**: Admin-controlled operations have inherent trust. If admin is compromised, front-running is the least concern.

## Best Practices for Operators

1. **Use Private Transaction Channels**: For large payouts, consider using private mempools or direct contract calls
2. **Monitor Mempool**: Watch for suspicious front-running attempts
3. **Configure Limits**: Set appropriate `max_bounty_amount` and payout caps
4. **Rate Limiting**: Leverage existing rate limiting for high-frequency operations
5. **Batch Strategically**: Use batch operations to reduce per-transaction overhead, but be aware of visibility

## Conclusion

The contracts have low to medium front-running risk, with most high-value operations protected by access control. The permissionless operations (refunds, automatic releases) have acceptable risk profiles as they cannot extract value from users. Additional mitigations like payout caps can be configured via `ConfigLimits` if needed.
