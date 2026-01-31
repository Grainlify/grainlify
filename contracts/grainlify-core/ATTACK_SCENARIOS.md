# Reentrancy Attack Scenarios - Grainlify

## Scenario 1: Bounty Claim Double Spend (Direct Reentrancy)

**Target:** `BountyEscrowContract::claim_bounty`

**Setup:**
1. Attacker creates a bounty with a malicious token.
2. The malicious token's `transfer` function is programmed to call back into `BountyEscrowContract::claim_bounty`.

**Attack Flow:**
1. Attacker calls `claim_bounty(bounty_id, attacker_address, proof)`.
2. Contract verifies proof and calls `token_client.transfer(contract, attacker, amount)`.
3. Malicious token's `transfer` executes and calls `claim_bounty(bounty_id, attacker, proof)` AGAIN.
4. Second call to `claim_bounty` succeeds because the status hasn't been updated yet in the first call.
5. Contract transfers funds AGAIN.
6. Both calls complete, attacker receives `2 * amount`.

**Impact:** Critical (Drains escrow funds)

## Scenario 2: Batch Distribution Draining (Cross-Function Reentrancy)

**Target:** `ProgramEscrowContract::batch_payout`

**Setup:**
1. Attacker is one of the recipients in a batch payout.
2. Attacker uses a malicious contract as their recipient address.

**Attack Flow:**
1. Admin calls `batch_payout(program_id, [..., attacker_contract, ...], [..., amount, ...])`.
2. Contract loops through recipients.
3. When it reaches `attacker_contract`, it calls `token_client.transfer(contract, attacker_contract, amount)`.
4. `attacker_contract`'s hook (if using a token with hooks) calls `ProgramEscrowContract::single_payout` or another `batch_payout`.
5. Because the `remaining_balance` hasn't been updated yet (it's updated AFTER the loop), the re-entrant call sees the original balance.
6. Attacker drains the entire prize pool.

**Impact:** Critical (Drains entire program prize pool)

## Scenario 3: Refund Exploitation

**Target:** `BountyEscrowContract::refund`

**Setup:**
1. Attacker has a bounty that is eligible for refund.

**Attack Flow:**
1. Attacker calls `refund(bounty_id)`.
2. Contract calls `token_client.transfer(contract, depositor, amount)`.
3. Malicious recipient calls `refund(bounty_id)` again.
4. Double refund achieved.

**Impact:** High (Drains funds)
