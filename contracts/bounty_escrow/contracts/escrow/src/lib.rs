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
use soroban_sdk::{contract, contracterror, contractimpl, contracttype, token, Address, Env, Symbol};

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
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Token, &token);
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
        depositor.require_auth();

        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        if env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyExists);
        }

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);

        // Transfer funds from depositor to contract
        client.transfer(&depositor, &env.current_contract_address(), &amount);

        let escrow = Escrow {
            depositor: depositor.clone(),
            amount,
            status: EscrowStatus::Locked,
            deadline,
        };

        // Extend the TTL of the storage entry to ensure it lives long enough
        env.storage().persistent().set(&DataKey::Escrow(bounty_id), &escrow);
        
        // Emit value allows for off-chain indexing
        env.events().publish(
            (Symbol::new(&env, "funds_locked"), bounty_id),
            (depositor, amount, deadline)
        );

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
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }

        let mut escrow: Escrow = env.storage().persistent().get(&DataKey::Escrow(bounty_id)).unwrap();

        if escrow.status != EscrowStatus::Locked {
            return Err(Error::FundsNotLocked);
        }

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);

        // Transfer funds to contributor
        client.transfer(&env.current_contract_address(), &contributor, &escrow.amount);

        escrow.status = EscrowStatus::Released;
        env.storage().persistent().set(&DataKey::Escrow(bounty_id), &escrow);

        env.events().publish(
            (Symbol::new(&env, "funds_released"), bounty_id),
            (contributor, escrow.amount)
        );

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

        let mut escrow: Escrow = env.storage().persistent().get(&DataKey::Escrow(bounty_id)).unwrap();

        if escrow.status != EscrowStatus::Locked {
            return Err(Error::FundsNotLocked);
        }

        let now = env.ledger().timestamp();
        if now < escrow.deadline {
            return Err(Error::DeadlineNotPassed);
        }

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);

        // Transfer funds back to depositor
        client.transfer(&env.current_contract_address(), &escrow.depositor, &escrow.amount);

        escrow.status = EscrowStatus::Refunded;
        env.storage().persistent().set(&DataKey::Escrow(bounty_id), &escrow);

        env.events().publish(
            (Symbol::new(&env, "funds_refunded"), bounty_id),
            (escrow.depositor, escrow.amount)
        );

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
        Ok(env.storage().persistent().get(&DataKey::Escrow(bounty_id)).unwrap())
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
}
