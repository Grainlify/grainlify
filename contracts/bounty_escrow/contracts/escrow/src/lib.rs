//! # Bounty Escrow Contract
//!
//! This contract manages escrow functionality for individual bounties on the Grainlify platform.
//! Each bounty has its own escrow record that locks funds until a contributor completes the work.
//!
//! ## Overview
//!
//! The Bounty Escrow Contract provides a secure, non-custodial vault for bounty funds on the
//! Stellar network using Soroban smart contracts. Project maintainers can lock funds for specific
//! bounties, and the Grainlify backend (holding the admin key) releases funds to contributors
//! after verifying their work through GitHub PR merges and quality scoring.
//!
//! ## Key Features
//!
//! - **Secure Fund Locking**: Funds are transferred to the contract and locked for specific bounties
//! - **Admin-Controlled Release**: Only the authorized admin (backend) can release funds to contributors
//! - **Deadline-Based Refunds**: Depositors can reclaim funds after the deadline expires
//! - **Event Emission**: All state changes emit events for off-chain indexing
//! - **Persistent Storage**: Escrow data persists with proper TTL management
//!
//! ## Security Model
//!
//! - **Authorization**: Release operations require admin signature; refunds require deadline expiration
//! - **Reentrancy**: Protected by Soroban's execution model
//! - **Storage Safety**: Uses persistent storage with appropriate TTL for long-lived escrows
//!
//! ## Usage Example
//!
//! ```rust,ignore
//! // 1. Initialize contract (one-time)
//! contract.init(env, admin_address, xlm_token_address);
//!
//! // 2. Lock funds for a bounty
//! contract.lock_funds(env, depositor, bounty_id, amount, deadline);
//!
//! // 3. Release to contributor (admin only)
//! contract.release_funds(env, bounty_id, contributor_address);
//!
//! // 4. Or refund after deadline
//! contract.refund(env, bounty_id);
//! ```

#![no_std]
mod events;
mod test_bounty_escrow;

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, token, Address, Env, Vec};
use events::{BountyEscrowInitialized, FundsLocked, FundsReleased, FundsRefunded, BatchFundsLocked, BatchFundsReleased, emit_bounty_initialized, emit_funds_locked, emit_funds_released, emit_funds_refunded, emit_batch_funds_locked, emit_batch_funds_released};

/// Contract errors that can occur during escrow operations.
///
/// Each error variant represents a specific failure condition with a unique error code
/// for debugging and error handling.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// Contract has already been initialized and cannot be initialized again.
    AlreadyInitialized = 1,
    /// Contract has not been initialized yet. Call `init` first.
    NotInitialized = 2,
    /// A bounty with this ID already exists. Each bounty ID must be unique.
    BountyExists = 3,
    /// No bounty found with the specified ID.
    BountyNotFound = 4,
    /// Funds are not in the Locked state. They may have already been released or refunded.
    FundsNotLocked = 5,
    /// The deadline has not passed yet. Refunds are only allowed after the deadline.
    DeadlineNotPassed = 6,
    /// The caller is not authorized to perform this operation.
    Unauthorized = 7,
    InvalidBatchSize = 8,
    BatchSizeMismatch = 9,
    DuplicateBountyId = 10,
}

/// Represents the current state of escrowed funds.
///
/// The status transitions are one-way: Locked → Released or Locked → Refunded.
/// Once funds are released or refunded, the status cannot be changed.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EscrowStatus {
    /// Funds are locked in escrow and awaiting release or refund.
    Locked,
    /// Funds have been released to the contributor.
    Released,
    /// Funds have been refunded to the original depositor.
    Refunded,
    PartiallyRefunded,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RefundMode {
    Full,
    Partial,
    Custom,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundRecord {
    pub amount: i128,
    pub recipient: Address,
    pub mode: RefundMode,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundApproval {
    pub bounty_id: u64,
    pub amount: i128,
    pub recipient: Address,
    pub mode: RefundMode,
    pub approved_by: Address,
    pub approved_at: u64,
}

/// Escrow data structure containing all information about a bounty's escrowed funds.
///
/// This structure is stored persistently and tracks the lifecycle of escrowed funds
/// from deposit through release or refund.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Escrow {
    /// Address of the account that deposited the funds.
    pub depositor: Address,
    /// Amount of tokens locked in escrow (in stroops for XLM).
    pub amount: i128,
    /// Current status of the escrow (Locked, Released, or Refunded).
    pub status: EscrowStatus,
    /// Unix timestamp after which refunds are allowed.
    pub deadline: u64,
    pub refund_history: Vec<RefundRecord>,
    pub remaining_amount: i128,
}

/// Storage keys used for contract data persistence.
///
/// These keys organize different types of data in the contract's storage.
#[contracttype]
pub enum DataKey {
    /// Stores the admin address authorized to release funds.
    Admin,
    /// Stores the token contract address (typically XLM).
    Token,
    /// Stores escrow data for a specific bounty ID.
    Escrow(u64), // bounty_id
    RefundApproval(u64), // bounty_id -> RefundApproval
}


/// The main bounty escrow contract.
///
/// This contract must be initialized before use with the `init` function.
#[contract]
pub struct BountyEscrowContract;

#[contractimpl]
impl BountyEscrowContract {
    /// Initialize the contract with the admin address and token contract address.
    ///
    /// This function must be called exactly once before any other operations can be performed.
    /// The admin address will have exclusive authority to release escrowed funds to contributors.
    ///
    /// # Arguments
    ///
    /// * `env` - The contract execution environment
    /// * `admin` - Address authorized to release funds (typically the Grainlify backend)
    /// * `token` - Address of the token contract to use for transfers (typically native XLM)
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If initialization succeeds
    /// * `Err(Error::AlreadyInitialized)` - If the contract has already been initialized
    ///
    /// # Security
    ///
    /// - This function can only be called once
    /// - The admin address should be carefully secured as it controls all fund releases
    /// - Consider using a multisig or backend-controlled address for the admin
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let admin = Address::from_string("GADMIN...");
    /// let xlm_token = Address::from_string("CTOKEN...");
    /// contract.init(env, admin, xlm_token)?;
    /// ```
    pub fn init(env: Env, admin: Address, token: Address) -> Result<(), Error> {
        // Apply rate limiting
        anti_abuse::check_rate_limit(&env, admin.clone());

        let start = env.ledger().timestamp();
        let caller = admin.clone();

        // Prevent re-initialization
        if env.storage().instance().has(&DataKey::Admin) {
            monitoring::track_operation(&env, symbol_short!("init"), caller, false);
            return Err(Error::AlreadyInitialized);
        }

        // Store configuration
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Token, &token);

        // Emit initialization event
        emit_bounty_initialized(
            &env,
            BountyEscrowInitialized {
                admin: admin.clone(),
                token,
                timestamp: env.ledger().timestamp(),
            },
        );

        // Track successful operation
        monitoring::track_operation(&env, symbol_short!("init"), caller, true);

        // Track performance
        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("init"), duration);

        Ok(())
    }

    /// Lock funds in escrow for a specific bounty.
    ///
    /// Transfers tokens from the depositor to the contract and creates an escrow record.
    /// The funds remain locked until either released by the admin or refunded after the deadline.
    ///
    /// # Arguments
    ///
    /// * `env` - The contract execution environment
    /// * `depositor` - Address depositing the funds (must authorize this transaction)
    /// * `bounty_id` - Unique identifier for the bounty (must not already exist)
    /// * `amount` - Amount of tokens to lock (in stroops for XLM, must be > 0)
    /// * `deadline` - Unix timestamp after which refunds are allowed
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If funds are successfully locked
    /// * `Err(Error::NotInitialized)` - If contract hasn't been initialized
    /// * `Err(Error::BountyExists)` - If a bounty with this ID already exists
    ///
    /// # Panics
    ///
    /// Panics if the token transfer fails (e.g., insufficient balance or allowance).
    ///
    /// # Security
    ///
    /// - Requires authorization from the depositor address
    /// - Each bounty_id must be unique to prevent overwrites
    /// - Funds are immediately transferred to the contract
    /// - Emits `funds_locked` event for off-chain tracking
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let depositor = Address::from_string("GDEPOSIT...");
    /// let bounty_id = 12345u64;
    /// let amount = 1000_0000000i128; // 1000 XLM
    /// let deadline = env.ledger().timestamp() + 2592000; // 30 days
    /// contract.lock_funds(env, depositor, bounty_id, amount, deadline)?;
    /// ```
    pub fn lock_funds(
        env: Env,
        depositor: Address,
        bounty_id: u64,
        amount: i128,
        deadline: u64,
    ) -> Result<(), Error> {
        // Apply rate limiting
        anti_abuse::check_rate_limit(&env, depositor.clone());

        let start = env.ledger().timestamp();
        let caller = depositor.clone();

        // Verify depositor authorization
        depositor.require_auth();

        // Ensure contract is initialized
        if env.storage().instance().has(&DataKey::ReentrancyGuard) {
            panic!("Reentrancy detected");
        }
        env.storage()
            .instance()
            .set(&DataKey::ReentrancyGuard, &true);

        if amount <= 0 {
            monitoring::track_operation(&env, symbol_short!("lock"), caller, false);
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::InvalidAmount);
        }

        if deadline <= env.ledger().timestamp() {
            monitoring::track_operation(&env, symbol_short!("lock"), caller, false);
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::InvalidDeadline);
        }
        if !env.storage().instance().has(&DataKey::Admin) {
            monitoring::track_operation(&env, symbol_short!("lock"), caller, false);
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::NotInitialized);
        }

        // Prevent duplicate bounty IDs
        if env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            monitoring::track_operation(&env, symbol_short!("lock"), caller, false);
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::BountyExists);
        }

        // Get token contract and transfer funds
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);

        // Transfer funds from depositor to contract
        client.transfer(&depositor, &env.current_contract_address(), &amount);

        // Create escrow record
        let escrow = Escrow {
            depositor: depositor.clone(),
            amount,
            status: EscrowStatus::Locked,
            deadline,
            refund_history: vec![&env],
            remaining_amount: amount,
        };

        // Store in persistent storage with extended TTL
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        // Emit event for off-chain indexing
        emit_funds_locked(
            &env,
            FundsLocked {
                bounty_id,
                amount,
                depositor: depositor.clone(),
                deadline,
            },
        );

        env.storage().instance().remove(&DataKey::ReentrancyGuard);

        // Track successful operation
        monitoring::track_operation(&env, symbol_short!("lock"), caller, true);

        // Track performance
        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("lock"), duration);

        Ok(())
    }

    /// Release escrowed funds to a contributor.
    ///
    /// Transfers the locked funds to the specified contributor address and marks the escrow
    /// as released. Only the admin can call this function, typically after the Grainlify
    /// backend has verified the contributor's work.
    ///
    /// # Arguments
    ///
    /// * `env` - The contract execution environment
    /// * `bounty_id` - ID of the bounty to release funds for
    /// * `contributor` - Address to receive the escrowed funds
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If funds are successfully released
    /// * `Err(Error::NotInitialized)` - If contract hasn't been initialized
    /// * `Err(Error::BountyNotFound)` - If no escrow exists for this bounty_id
    /// * `Err(Error::FundsNotLocked)` - If funds have already been released or refunded
    ///
    /// # Panics
    ///
    /// Panics if the caller is not the admin address.
    ///
    /// # Security
    ///
    /// - **Admin-only**: Requires authorization from the admin address
    /// - Validates escrow exists and is in Locked state
    /// - Status change is permanent (cannot be reversed)
    /// - Emits `funds_released` event for off-chain tracking
    /// - The admin should verify contributor work off-chain before calling
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let bounty_id = 12345u64;
    /// let contributor = Address::from_string("GCONTRIB...");
    /// contract.release_funds(env, bounty_id, contributor)?;
    /// ```
    pub fn release_funds(env: Env, bounty_id: u64, contributor: Address) -> Result<(), Error> {
        let start = env.ledger().timestamp();

        // Ensure contract is initialized
        if env.storage().instance().has(&DataKey::ReentrancyGuard) {
            panic!("Reentrancy detected");
        }
        env.storage()
            .instance()
            .set(&DataKey::ReentrancyGuard, &true);
        if !env.storage().instance().has(&DataKey::Admin) {
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::NotInitialized);
        }

        // Verify admin authorization
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        
        // Apply rate limiting
        anti_abuse::check_rate_limit(&env, admin.clone());

        admin.require_auth();

        // Verify bounty exists
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            monitoring::track_operation(&env, symbol_short!("release"), admin.clone(), false);
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::BountyNotFound);
        }

        // Get and verify escrow state
        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();

        if escrow.status != EscrowStatus::Locked {
            monitoring::track_operation(&env, symbol_short!("release"), admin.clone(), false);
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::FundsNotLocked);
        }

        // Transfer funds to contributor
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);
        escrow.status = EscrowStatus::Released;
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        // Transfer funds to contributor
        client.transfer(
            &env.current_contract_address(),
            &contributor,
            &escrow.amount,
        );

        // Emit release event
        emit_funds_released(
            &env,
            FundsReleased {
                bounty_id,
                amount: escrow.amount,
                recipient: contributor.clone(),
                timestamp: env.ledger().timestamp(),
            },
        );

        env.storage().instance().remove(&DataKey::ReentrancyGuard);

        // Track successful operation
        monitoring::track_operation(&env, symbol_short!("release"), admin, true);

        // Track performance
        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("release"), duration);
        Ok(())
    }

    /// Refund escrowed funds to the original depositor after the deadline.
    ///
    /// Returns locked funds to the depositor if the deadline has passed. This function is
    /// permissionless and can be called by anyone once the deadline expires, ensuring funds
    /// are never permanently locked even if the depositor loses their key.
    ///
    /// # Arguments
    ///
    /// * `env` - The contract execution environment
    /// * `bounty_id` - ID of the bounty to refund
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If funds are successfully refunded
    /// * `Err(Error::BountyNotFound)` - If no escrow exists for this bounty_id
    /// * `Err(Error::FundsNotLocked)` - If funds have already been released or refunded
    /// * `Err(Error::DeadlineNotPassed)` - If the current time is before the deadline
    ///
    /// # Security
    ///
    /// - **Permissionless**: Anyone can trigger refund after deadline (prevents stuck funds)
    /// - **Time-locked**: Only works after the deadline timestamp
    /// - Validates escrow exists and is in Locked state
    /// - Funds always return to the original depositor (not the caller)
    /// - Emits `funds_refunded` event for off-chain tracking
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // After deadline has passed
    /// let bounty_id = 12345u64;
    /// contract.refund(env, bounty_id)?;
    /// // Funds are returned to original depositor
    /// ```
    pub fn refund(env: Env, bounty_id: u64) -> Result<(), Error> {
        // We'll allow anyone to trigger the refund if conditions are met, 
        // effectively making it permissionless but conditional.
        // OR we can require depositor auth. Let's make it permissionless to ensure funds aren't stuck if depositor key is lost,
        // but strictly logic bound.
        // However, usually refund is triggered by depositor. Let's stick to logic.
        
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }

        let escrow: Escrow = env.storage().persistent().get(&DataKey::Escrow(bounty_id)).unwrap();

        if escrow.status != EscrowStatus::Locked && escrow.status != EscrowStatus::PartiallyRefunded {
            return Err(Error::FundsNotLocked);
        }

        if amount <= 0 || amount > escrow.remaining_amount {
            return Err(Error::InvalidAmount);
        }

        let approval = RefundApproval {
            bounty_id,
            amount,
            recipient: recipient.clone(),
            mode: mode.clone(),
            approved_by: admin.clone(),
            approved_at: env.ledger().timestamp(),
        };

        env.storage().persistent().set(&DataKey::RefundApproval(bounty_id), &approval);

        Ok(())
    }

    /// Refund funds with support for Full, Partial, and Custom refunds.
    /// - Full: refunds all remaining funds to depositor
    /// - Partial: refunds specified amount to depositor
    /// - Custom: refunds specified amount to specified recipient (requires admin approval if before deadline)
    pub fn refund(
        env: Env,
        bounty_id: u64,
        amount: Option<i128>,
        recipient: Option<Address>,
        mode: RefundMode,
    ) -> Result<(), Error> {
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            let caller = env.current_contract_address();
            monitoring::track_operation(&env, symbol_short!("refund"), caller, false);
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::BountyNotFound);
        }

        // Get and verify escrow state
        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();
        let caller = escrow.depositor.clone();

        if escrow.status != EscrowStatus::Locked && escrow.status != EscrowStatus::PartiallyRefunded {
            return Err(Error::FundsNotLocked);
        }

        // Verify deadline has passed
        let now = env.ledger().timestamp();
        let is_before_deadline = now < escrow.deadline;

        // Determine refund amount and recipient
        let refund_amount: i128;
        let refund_recipient: Address;

        match mode {
            RefundMode::Full => {
                refund_amount = escrow.remaining_amount;
                refund_recipient = escrow.depositor.clone();
                if is_before_deadline {
                    return Err(Error::DeadlineNotPassed);
                }
            }
            RefundMode::Partial => {
                refund_amount = amount.unwrap_or(escrow.remaining_amount);
                refund_recipient = escrow.depositor.clone();
                if is_before_deadline {
                    return Err(Error::DeadlineNotPassed);
                }
            }
            RefundMode::Custom => {
                refund_amount = amount.ok_or(Error::InvalidAmount)?;
                refund_recipient = recipient.ok_or(Error::InvalidAmount)?;
                
                // Custom refunds before deadline require admin approval
                if is_before_deadline {
                    if !env.storage().persistent().has(&DataKey::RefundApproval(bounty_id)) {
                        return Err(Error::RefundNotApproved);
                    }
                    let approval: RefundApproval = env.storage()
                        .persistent()
                        .get(&DataKey::RefundApproval(bounty_id))
                        .unwrap();
                    
                    // Verify approval matches request
                    if approval.amount != refund_amount 
                        || approval.recipient != refund_recipient 
                        || approval.mode != mode {
                        return Err(Error::RefundNotApproved);
                    }
                    
                    // Clear approval after use
                    env.storage().persistent().remove(&DataKey::RefundApproval(bounty_id));
                }
            }
        }

        // Validate amount
        if refund_amount <= 0 || refund_amount > escrow.remaining_amount {
            return Err(Error::InvalidAmount);
        }

        // Transfer funds back to depositor
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);

        // Check contract balance
        let contract_balance = client.balance(&env.current_contract_address());
        if contract_balance < refund_amount {
            return Err(Error::InsufficientFunds);
        }

        // Transfer funds
        client.transfer(&env.current_contract_address(), &refund_recipient, &refund_amount);

        // Update escrow state
        escrow.remaining_amount -= refund_amount;
        
        // Add to refund history
        let refund_record = RefundRecord {
            amount: refund_amount,
            recipient: refund_recipient.clone(),
            mode: mode.clone(),
            timestamp: env.ledger().timestamp(),
        };
        escrow.refund_history.push_back(refund_record);

        // Update status
        if escrow.remaining_amount == 0 {
            escrow.status = EscrowStatus::Refunded;
        } else {
            escrow.status = EscrowStatus::PartiallyRefunded;
        }

        env.storage().persistent().set(&DataKey::Escrow(bounty_id), &escrow);

        // Emit refund event
        emit_funds_refunded(
            &env,
            FundsRefunded {
                bounty_id,
                amount: refund_amount,
                refund_to: refund_recipient,
                timestamp: env.ledger().timestamp(),
                refund_mode: mode.clone(),
                remaining_amount: escrow.remaining_amount,
            },
        );

        env.storage().instance().remove(&DataKey::ReentrancyGuard);

        // Track successful operation
        monitoring::track_operation(&env, symbol_short!("refund"), caller, true);

        // Track performance
        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("refund"), duration);

        Ok(())
    }

    /// Retrieve complete escrow information for a bounty.
    ///
    /// Returns all stored data about an escrow including depositor, amount, status, and deadline.
    /// This is a read-only view function that doesn't modify state.
    ///
    /// # Arguments
    ///
    /// * `env` - The contract execution environment
    /// * `bounty_id` - ID of the bounty to query
    ///
    /// # Returns
    ///
    /// * `Ok(Escrow)` - The complete escrow data structure
    /// * `Err(Error::BountyNotFound)` - If no escrow exists for this bounty_id
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let bounty_id = 12345u64;
    /// let escrow_info = contract.get_escrow_info(env, bounty_id)?;
    /// // escrow_info contains: depositor, amount, status, deadline
    /// ```
    pub fn get_escrow_info(env: Env, bounty_id: u64) -> Result<Escrow, Error> {
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }
        Ok(env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap())
    }

    /// Get the contract's current token balance.
    ///
    /// Returns the total balance of tokens held by the contract. This should equal the sum
    /// of all locked escrows. Useful for auditing and verification.
    ///
    /// # Arguments
    ///
    /// * `env` - The contract execution environment
    ///
    /// # Returns
    ///
    /// * `Ok(i128)` - The contract's token balance
    /// * `Err(Error::NotInitialized)` - If contract hasn't been initialized
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let balance = contract.get_balance(env)?;
    /// // balance is in stroops for XLM (1 XLM = 10_000_000 stroops)
    /// ```
    pub fn get_balance(env: Env) -> Result<i128, Error> {
        if !env.storage().instance().has(&DataKey::Token) {
            return Err(Error::NotInitialized);
        }
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);
        Ok(client.balance(&env.current_contract_address()))
    }

    /// Batch lock funds for multiple bounties in a single transaction.
    /// This improves gas efficiency by reducing transaction overhead.
    /// 
    /// # Arguments
    /// * `items` - Vector of LockFundsItem containing bounty_id, depositor, amount, and deadline
    /// 
    /// # Returns
    /// Number of successfully locked bounties
    /// 
    /// # Errors
    /// * InvalidBatchSize - if batch size exceeds MAX_BATCH_SIZE or is zero
    /// * BountyExists - if any bounty_id already exists
    /// * NotInitialized - if contract is not initialized
    /// 
    /// # Note
    /// This operation is atomic - if any item fails, the entire transaction reverts.
    pub fn batch_lock_funds(env: Env, items: Vec<LockFundsItem>) -> Result<u32, Error> {
        // Validate batch size
        let batch_size = items.len() as u32;
        if batch_size == 0 {
            return Err(Error::InvalidBatchSize);
        }
        if batch_size > MAX_BATCH_SIZE {
            return Err(Error::InvalidBatchSize);
        }

        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);
        let contract_address = env.current_contract_address();
        let timestamp = env.ledger().timestamp();

        // Validate all items before processing (all-or-nothing approach)
        for item in items.iter() {
            // Check if bounty already exists
            if env.storage().persistent().has(&DataKey::Escrow(item.bounty_id)) {
                return Err(Error::BountyExists);
            }

            // Validate amount
            if item.amount <= 0 {
                return Err(Error::InvalidBatchSize);
            }

            // Check for duplicate bounty_ids in the batch
            let mut count = 0u32;
            for other_item in items.iter() {
                if other_item.bounty_id == item.bounty_id {
                    count += 1;
                }
            }
            if count > 1 {
                return Err(Error::DuplicateBountyId);
            }
        }

        // Collect unique depositors and require auth once for each
        // This prevents "frame is already authorized" errors when same depositor appears multiple times
        let mut seen_depositors: Vec<Address> = Vec::new(&env);
        for item in items.iter() {
            let mut found = false;
            for seen in seen_depositors.iter() {
                if seen.clone() == item.depositor {
                    found = true;
                    break;
                }
            }
            if !found {
                seen_depositors.push_back(item.depositor.clone());
                item.depositor.require_auth();
            }
        }

        // Process all items (atomic - all succeed or all fail)
        let mut locked_count = 0u32;
        for item in items.iter() {
            // Transfer funds from depositor to contract
            client.transfer(&item.depositor, &contract_address, &item.amount);

            // Create escrow record
            let escrow = Escrow {
                depositor: item.depositor.clone(),
                amount: item.amount,
                status: EscrowStatus::Locked,
                deadline: item.deadline,
            };

            // Store escrow
            env.storage().persistent().set(&DataKey::Escrow(item.bounty_id), &escrow);

            // Emit individual event for each locked bounty
            emit_funds_locked(
                &env,
                FundsLocked {
                    bounty_id: item.bounty_id,
                    amount: item.amount,
                    depositor: item.depositor.clone(),
                    deadline: item.deadline,
                },
            );

            locked_count += 1;
        }

        // Emit batch event
        emit_batch_funds_locked(
            &env,
            BatchFundsLocked {
                count: locked_count,
                total_amount: items.iter().map(|i| i.amount).sum(),
                timestamp,
            },
        );

        Ok(locked_count)
    }

    /// Batch release funds to multiple contributors in a single transaction.
    /// This improves gas efficiency by reducing transaction overhead.
    /// 
    /// # Arguments
    /// * `items` - Vector of ReleaseFundsItem containing bounty_id and contributor address
    /// 
    /// # Returns
    /// Number of successfully released bounties
    /// 
    /// # Errors
    /// * InvalidBatchSize - if batch size exceeds MAX_BATCH_SIZE or is zero
    /// * BountyNotFound - if any bounty_id doesn't exist
    /// * FundsNotLocked - if any bounty is not in Locked status
    /// * Unauthorized - if caller is not admin
    /// 
    /// # Note
    /// This operation is atomic - if any item fails, the entire transaction reverts.
    pub fn batch_release_funds(env: Env, items: Vec<ReleaseFundsItem>) -> Result<u32, Error> {
        // Validate batch size
        let batch_size = items.len() as u32;
        if batch_size == 0 {
            return Err(Error::InvalidBatchSize);
        }
        if batch_size > MAX_BATCH_SIZE {
            return Err(Error::InvalidBatchSize);
        }

        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);
        let contract_address = env.current_contract_address();
        let timestamp = env.ledger().timestamp();

        // Validate all items before processing (all-or-nothing approach)
        let mut total_amount: i128 = 0;
        for item in items.iter() {
            // Check if bounty exists
            if !env.storage().persistent().has(&DataKey::Escrow(item.bounty_id)) {
                return Err(Error::BountyNotFound);
            }

            let escrow: Escrow = env.storage()
                .persistent()
                .get(&DataKey::Escrow(item.bounty_id))
                .unwrap();

            // Check if funds are locked
            if escrow.status != EscrowStatus::Locked {
                return Err(Error::FundsNotLocked);
            }

            // Check for duplicate bounty_ids in the batch
            let mut count = 0u32;
            for other_item in items.iter() {
                if other_item.bounty_id == item.bounty_id {
                    count += 1;
                }
            }
            if count > 1 {
                return Err(Error::DuplicateBountyId);
            }

            total_amount = total_amount
                .checked_add(escrow.amount)
                .ok_or(Error::InvalidBatchSize)?;
        }

        // Process all items (atomic - all succeed or all fail)
        let mut released_count = 0u32;
        for item in items.iter() {
            let mut escrow: Escrow = env.storage()
                .persistent()
                .get(&DataKey::Escrow(item.bounty_id))
                .unwrap();

            // Transfer funds to contributor
            client.transfer(&contract_address, &item.contributor, &escrow.amount);

            // Update escrow status
            escrow.status = EscrowStatus::Released;
            env.storage().persistent().set(&DataKey::Escrow(item.bounty_id), &escrow);

            // Emit individual event for each released bounty
            emit_funds_released(
                &env,
                FundsReleased {
                    bounty_id: item.bounty_id,
                    amount: escrow.amount,
                    recipient: item.contributor.clone(),
                    timestamp,
                },
            );

            released_count += 1;
        }

        // Emit batch event
        emit_batch_funds_released(
            &env,
            BatchFundsReleased {
                count: released_count,
                total_amount,
                timestamp,
            },
        );

        Ok(released_count)
    }
}

#[cfg(test)]
mod test;
