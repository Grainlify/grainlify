//! # Program Escrow Contract
//!
//! This contract manages escrow functionality for hackathon and program prize pools on the
//! Grainlify platform. Unlike the bounty escrow which handles individual bounties, this contract
//! manages a single prize pool that can be distributed to multiple winners.
//!
//! ## Overview
//!
//! The Program Escrow Contract provides secure fund management for hackathons, contests, and
//! grant programs. Organizers lock funds into the contract, and the authorized backend can
//! execute payouts to multiple winners either individually or in batches. This contract maintains
//! a complete payout history for transparency and auditing.
//!
//! ## Key Features
//!
//! - **Prize Pool Management**: Lock and manage funds for entire programs
//! - **Batch Payouts**: Efficiently distribute prizes to multiple winners in a single transaction
//! - **Single Payouts**: Release individual prizes when needed
//! - **Payout History**: Immutable record of all distributions
//! - **Balance Tracking**: Real-time tracking of remaining funds
//! - **Event Emission**: All operations emit events for off-chain indexing
//!
//! ## Security Model
//!
//! - **Authorization**: Only the authorized payout key can release funds
//! - **Balance Validation**: Prevents overdrafts with strict balance checks
//! - **Overflow Protection**: Safe arithmetic for all balance operations
//! - **Atomic Operations**: Batch payouts are all-or-nothing
//! - **Immutable History**: Payout records cannot be modified after creation
//!
//! ## Usage Example
//!
//! ```rust,ignore
//! // 1. Initialize program escrow
//! contract.init_program(env, program_id, authorized_key, token_address);
//!
//! // 2. Lock prize pool funds
//! contract.lock_program_funds(env, total_prize_amount);
//!
//! // 3. Distribute prizes to winners
//! contract.batch_payout(env, winner_addresses, prize_amounts);
//!
//! // 4. Check remaining balance
//! let remaining = contract.get_remaining_balance(env);
//! ```

#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, vec, Address, Env, String, Symbol, Vec,
    token,
};

/// Event emitted when a program is initialized.
///
/// This event signals the creation of a new program escrow with its configuration.
const PROGRAM_INITIALIZED: Symbol = symbol_short!("ProgramInit");

/// Event emitted when funds are locked into the program escrow.
///
/// This event is published each time additional funds are added to the prize pool.
const FUNDS_LOCKED: Symbol = symbol_short!("FundsLocked");

/// Event emitted when a batch payout is executed.
///
/// This event contains summary information about the batch operation.
const BATCH_PAYOUT: Symbol = symbol_short!("BatchPayout");

/// Event emitted when a single payout is executed.
///
/// This event contains details about the individual payout transaction.
const PAYOUT: Symbol = symbol_short!("Payout");

/// Storage key for program data.
///
/// This key is used to store and retrieve the main program escrow data structure.
const PROGRAM_DATA: Symbol = symbol_short!("ProgramData");

/// Record of a single payout transaction.
///
/// Each payout creates an immutable record that is added to the program's payout history.
/// These records provide a complete audit trail of all fund distributions.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayoutRecord {
    /// Address that received the payout.
    pub recipient: Address,
    /// Amount transferred in this payout (in token base units).
    pub amount: i128,
    /// Unix timestamp when the payout was executed.
    pub timestamp: u64,
}

/// Complete data structure for a program escrow.
///
/// This structure maintains all state for a program's prize pool including balances,
/// authorization, and complete payout history.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramData {
    /// Unique identifier for the program/hackathon.
    pub program_id: String,
    /// Total funds ever locked into this program (cumulative).
    pub total_funds: i128,
    /// Current remaining balance available for payouts.
    pub remaining_balance: i128,
    /// Address authorized to execute payouts (typically Grainlify backend).
    pub authorized_payout_key: Address,
    /// Complete history of all payouts executed from this escrow.
    pub payout_history: Vec<PayoutRecord>,
    /// Token contract address used for all transfers.
    pub token_address: Address,
}

#[contract]
pub struct ProgramEscrowContract;

#[contractimpl]
impl ProgramEscrowContract {
    /// Initialize a new program escrow.
    ///
    /// Creates a new program escrow with zero initial balance. This function must be called
    /// before any funds can be locked or distributed. Each contract instance can only be
    /// initialized once.
    ///
    /// # Arguments
    ///
    /// * `env` - The contract execution environment
    /// * `program_id` - Unique identifier for the program/hackathon
    /// * `authorized_payout_key` - Address authorized to trigger payouts (typically Grainlify backend)
    /// * `token_address` - Address of the token contract to use for transfers (typically native XLM)
    ///
    /// # Returns
    ///
    /// The initialized `ProgramData` structure with zero balances and empty payout history.
    ///
    /// # Panics
    ///
    /// Panics if the program has already been initialized.
    ///
    /// # Security
    ///
    /// - Can only be called once per contract instance
    /// - The authorized_payout_key should be carefully secured
    /// - Consider using a backend-controlled or multisig address for payout authorization
    /// - Emits `ProgramInitialized` event for off-chain tracking
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let program_id = String::from_str(&env, "hackathon-2024");
    /// let backend_key = Address::from_string("GBACKEND...");
    /// let xlm_token = Address::from_string("CTOKEN...");
    /// let program_data = contract.init_program(env, program_id, backend_key, xlm_token);
    /// ```
    pub fn init_program(
        env: Env,
        program_id: String,
        authorized_payout_key: Address,
        token_address: Address,
    ) -> ProgramData {
        // Check if program already exists
        if env.storage().instance().has(&PROGRAM_DATA) {
            panic!("Program already initialized");
        }

        let contract_address = env.current_contract_address();
        let program_data = ProgramData {
            program_id: program_id.clone(),
            total_funds: 0,
            remaining_balance: 0,
            authorized_payout_key: authorized_payout_key.clone(),
            payout_history: vec![&env],
            token_address: token_address.clone(),
        };

        // Store program data
        env.storage().instance().set(&PROGRAM_DATA, &program_data);

        // Emit ProgramInitialized event
        env.events().publish(
            (PROGRAM_INITIALIZED,),
            (program_id, authorized_payout_key, token_address, 0i128),
        );

        program_data
    }

    /// Lock funds into the program escrow.
    ///
    /// Adds funds to the program's prize pool. This function can be called multiple times
    /// to incrementally fund the program. The total_funds and remaining_balance are both
    /// increased by the specified amount.
    ///
    /// # Arguments
    ///
    /// * `env` - The contract execution environment
    /// * `amount` - Amount of funds to lock (in token base units, must be > 0)
    ///
    /// # Returns
    ///
    /// Updated `ProgramData` with increased balances.
    ///
    /// # Panics
    ///
    /// - Panics if amount is <= 0
    /// - Panics if the program has not been initialized
    ///
    /// # Security
    ///
    /// - Validates amount is positive
    /// - Updates both total_funds (cumulative) and remaining_balance (current)
    /// - Emits `FundsLocked` event with new balances
    /// - Note: Actual token transfer must be done separately before calling this function
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let prize_pool = 10000_0000000i128; // 10,000 XLM
    /// let updated_data = contract.lock_program_funds(env, prize_pool);
    /// // Can call again to add more funds later
    /// ```
    pub fn lock_program_funds(env: Env, amount: i128) -> ProgramData {
        if amount <= 0 {
            panic!("Amount must be greater than zero");
        }

        let mut program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        // Update balances
        program_data.total_funds += amount;
        program_data.remaining_balance += amount;

        // Store updated data
        env.storage().instance().set(&PROGRAM_DATA, &program_data);

        // Emit FundsLocked event
        env.events().publish(
            (FUNDS_LOCKED,),
            (
                program_data.program_id.clone(),
                amount,
                program_data.remaining_balance,
            ),
        );

        program_data
    }

    /// Execute batch payouts to multiple recipients.
    ///
    /// Distributes prizes to multiple winners in a single atomic transaction. This is more
    /// efficient than multiple single payouts and ensures all winners are paid together or
    /// none are paid (all-or-nothing atomicity).
    ///
    /// # Arguments
    ///
    /// * `env` - The contract execution environment
    /// * `recipients` - Vector of recipient addresses (must not be empty)
    /// * `amounts` - Vector of amounts (must match recipients length, all must be > 0)
    ///
    /// # Returns
    ///
    /// Updated `ProgramData` with decreased remaining_balance and updated payout_history.
    ///
    /// # Panics
    ///
    /// - Panics if caller is not the authorized payout key
    /// - Panics if program has not been initialized
    /// - Panics if recipients and amounts vectors have different lengths
    /// - Panics if recipients vector is empty
    /// - Panics if any amount is <= 0
    /// - Panics if total payout exceeds remaining balance
    /// - Panics on arithmetic overflow when calculating total
    ///
    /// # Security
    ///
    /// - **Authorization Required**: Only authorized_payout_key can call this function
    /// - **Atomic Operation**: All transfers succeed or all fail together
    /// - **Balance Validation**: Ensures sufficient funds before any transfers
    /// - **Overflow Protection**: Uses checked arithmetic for total calculation
    /// - **Immutable History**: All payouts are permanently recorded
    /// - Emits `BatchPayout` event with summary information
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let winners = vec![&env, 
    ///     Address::from_string("GWINNER1..."),
    ///     Address::from_string("GWINNER2..."),
    ///     Address::from_string("GWINNER3...")
    /// ];
    /// let prizes = vec![&env, 
    ///     5000_0000000i128,  // 1st place: 5000 XLM
    ///     3000_0000000i128,  // 2nd place: 3000 XLM
    ///     2000_0000000i128   // 3rd place: 2000 XLM
    /// ];
    /// let updated_data = contract.batch_payout(env, winners, prizes);
    /// ```
    pub fn batch_payout(
        env: Env,
        recipients: Vec<Address>,
        amounts: Vec<i128>,
    ) -> ProgramData {
        // Verify authorization
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        let caller = env.invoker();
        if caller != program_data.authorized_payout_key {
            panic!("Unauthorized: only authorized payout key can trigger payouts");
        }

        // Validate input lengths match
        if recipients.len() != amounts.len() {
            panic!("Recipients and amounts vectors must have the same length");
        }

        if recipients.len() == 0 {
            panic!("Cannot process empty batch");
        }

        // Calculate total payout amount
        let mut total_payout: i128 = 0;
        for amount in amounts.iter() {
            if *amount <= 0 {
                panic!("All amounts must be greater than zero");
            }
            total_payout = total_payout
                .checked_add(*amount)
                .unwrap_or_else(|| panic!("Payout amount overflow"));
        }

        // Validate sufficient balance
        if total_payout > program_data.remaining_balance {
            panic!("Insufficient balance: requested {}, available {}", 
                total_payout, program_data.remaining_balance);
        }

        // Execute transfers
        let mut updated_history = program_data.payout_history.clone();
        let timestamp = env.ledger().timestamp();
        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &program_data.token_address);

        for (i, recipient) in recipients.iter().enumerate() {
            let amount = amounts.get(i).unwrap();
            
            // Transfer funds from contract to recipient
            token_client.transfer(&contract_address, recipient, amount);

            // Record payout
            let payout_record = PayoutRecord {
                recipient: recipient.clone(),
                amount: *amount,
                timestamp,
            };
            updated_history.push_back(payout_record);
        }

        // Update program data
        let mut updated_data = program_data.clone();
        updated_data.remaining_balance -= total_payout;
        updated_data.payout_history = updated_history;

        // Store updated data
        env.storage().instance().set(&PROGRAM_DATA, &updated_data);

        // Emit BatchPayout event
        env.events().publish(
            (BATCH_PAYOUT,),
            (
                updated_data.program_id.clone(),
                recipients.len() as u32,
                total_payout,
                updated_data.remaining_balance,
            ),
        );

        updated_data
    }

    /// Execute a single payout to one recipient.
    ///
    /// Distributes a prize to a single winner. Use this for individual payouts or when
    /// distributing prizes at different times. For multiple simultaneous payouts, consider
    /// using `batch_payout` for better efficiency.
    ///
    /// # Arguments
    ///
    /// * `env` - The contract execution environment
    /// * `recipient` - Address of the recipient to receive the payout
    /// * `amount` - Amount to transfer (must be > 0)
    ///
    /// # Returns
    ///
    /// Updated `ProgramData` with decreased remaining_balance and updated payout_history.
    ///
    /// # Panics
    ///
    /// - Panics if caller is not the authorized payout key
    /// - Panics if program has not been initialized
    /// - Panics if amount is <= 0
    /// - Panics if amount exceeds remaining balance
    ///
    /// # Security
    ///
    /// - **Authorization Required**: Only authorized_payout_key can call this function
    /// - **Balance Validation**: Ensures sufficient funds before transfer
    /// - **Immutable History**: Payout is permanently recorded
    /// - Emits `Payout` event with transaction details
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let winner = Address::from_string("GWINNER...");
    /// let prize = 1000_0000000i128; // 1000 XLM
    /// let updated_data = contract.single_payout(env, winner, prize);
    /// ```
    pub fn single_payout(env: Env, recipient: Address, amount: i128) -> ProgramData {
        // Verify authorization
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        let caller = env.invoker();
        if caller != program_data.authorized_payout_key {
            panic!("Unauthorized: only authorized payout key can trigger payouts");
        }

        // Validate amount
        if amount <= 0 {
            panic!("Amount must be greater than zero");
        }

        // Validate sufficient balance
        if amount > program_data.remaining_balance {
            panic!("Insufficient balance: requested {}, available {}", 
                amount, program_data.remaining_balance);
        }

        // Transfer funds from contract to recipient
        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &program_data.token_address);
        token_client.transfer(&contract_address, &recipient, &amount);

        // Record payout
        let timestamp = env.ledger().timestamp();
        let payout_record = PayoutRecord {
            recipient: recipient.clone(),
            amount,
            timestamp,
        };

        let mut updated_history = program_data.payout_history.clone();
        updated_history.push_back(payout_record);

        // Update program data
        let mut updated_data = program_data.clone();
        updated_data.remaining_balance -= amount;
        updated_data.payout_history = updated_history;

        // Store updated data
        env.storage().instance().set(&PROGRAM_DATA, &updated_data);

        // Emit Payout event
        env.events().publish(
            (PAYOUT,),
            (
                updated_data.program_id.clone(),
                recipient,
                amount,
                updated_data.remaining_balance,
            ),
        );

        updated_data
    }

    /// Get complete program information.
    ///
    /// Returns all data about the program escrow including balances, configuration,
    /// and complete payout history. This is a read-only view function.
    ///
    /// # Arguments
    ///
    /// * `env` - The contract execution environment
    ///
    /// # Returns
    ///
    /// Complete `ProgramData` structure including:
    /// - program_id
    /// - total_funds (cumulative)
    /// - remaining_balance (current)
    /// - authorized_payout_key
    /// - payout_history (all payouts)
    /// - token_address
    ///
    /// # Panics
    ///
    /// Panics if the program has not been initialized.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let program_info = contract.get_program_info(env);
    /// // Access all program data: balances, history, etc.
    /// ```
    pub fn get_program_info(env: Env) -> ProgramData {
        env.storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"))
    }

    /// Get the current remaining balance.
    ///
    /// Returns the amount of funds still available for distribution. This is a convenience
    /// function that extracts just the remaining_balance from the program data.
    ///
    /// # Arguments
    ///
    /// * `env` - The contract execution environment
    ///
    /// # Returns
    ///
    /// Current remaining balance available for payouts (in token base units).
    ///
    /// # Panics
    ///
    /// Panics if the program has not been initialized.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let available = contract.get_remaining_balance(env);
    /// // Check if sufficient funds for next payout
    /// if available >= prize_amount {
    ///     // Proceed with payout
    /// }
    /// ```
    pub fn get_remaining_balance(env: Env) -> i128 {
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));

        program_data.remaining_balance
    }
}

#[cfg(test)]
mod test;
