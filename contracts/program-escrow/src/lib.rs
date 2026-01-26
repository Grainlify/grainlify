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
    contract, contractimpl, contracttype, symbol_short, token, vec, Address, Env, String, Symbol,
    Vec,
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

/// Storage key type for individual programs
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Program(String), // program_id -> ProgramData
}

// ============================================================================
// Contract Implementation
// ============================================================================

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
        // Apply rate limiting
        anti_abuse::check_rate_limit(&env, authorized_payout_key.clone());

        let start = env.ledger().timestamp();
        let caller = authorized_payout_key.clone();

        // Validate program_id
        if program_id.len() == 0 {
            monitoring::track_operation(&env, symbol_short!("init_prg"), caller, false);
            panic!("Program ID cannot be empty");
        }

        // Check if program already exists
        let program_key = DataKey::Program(program_id.clone());
        if env.storage().instance().has(&program_key) {
            monitoring::track_operation(&env, symbol_short!("init_prg"), caller, false);
            panic!("Program already exists");
        }

        // Create program data
        let program_data = ProgramData {
            program_id: program_id.clone(),
            total_funds: 0,
            remaining_balance: 0,
            authorized_payout_key: authorized_payout_key.clone(),
            payout_history: vec![&env],
            token_address: token_address.clone(),
        };

        // Store program data
        env.storage().instance().set(&program_key, &program_data);

        // Update registry
        let mut registry: Vec<String> = env
            .storage()
            .instance()
            .get(&PROGRAM_REGISTRY)
            .unwrap_or(vec![&env]);
        registry.push_back(program_id.clone());
        env.storage().instance().set(&PROGRAM_REGISTRY, &registry);

        // Emit registration event
        env.events().publish(
            (PROGRAM_REGISTERED,),
            (program_id, authorized_payout_key, token_address, 0i128),
        );

        // Track successful operation
        monitoring::track_operation(&env, symbol_short!("init_prg"), caller, true);

        // Track performance
        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("init_prg"), duration);

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
            monitoring::track_operation(&env, symbol_short!("lock"), caller.clone(), false);
            panic!("Amount must be greater than zero");
        }

        // Get program data
        let program_key = DataKey::Program(program_id.clone());
        let mut program_data: ProgramData = env
            .storage()
            .instance()
            .get(&program_key)
            .unwrap_or_else(|| {
                monitoring::track_operation(&env, symbol_short!("lock"), caller.clone(), false);
                panic!("Program not found")
            });

        // Update balances
        program_data.total_funds += amount;
        program_data.remaining_balance += amount;

        // Store updated data
        env.storage().instance().set(&program_key, &program_data);

        // Emit event
        env.events().publish(
            (FUNDS_LOCKED,),
            (program_id, amount, program_data.remaining_balance),
        );

        // Track successful operation
        monitoring::track_operation(&env, symbol_short!("lock"), caller, true);

        // Track performance
        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("lock"), duration);

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
        program_id: String,
        recipients: Vec<Address>,
        amounts: Vec<i128>,
    ) -> ProgramData {
        // Apply rate limiting to the contract itself or the program
        // We can't easily get the caller here without getting program data first
        
        // Get program data
        let program_key = DataKey::Program(program_id.clone());
        let program_data: ProgramData = env
            .storage()
            .instance()
            .get(&program_key)
            .unwrap_or_else(|| panic!("Program not found"));

        // Apply rate limiting to the authorized payout key
        anti_abuse::check_rate_limit(&env, program_data.authorized_payout_key.clone());

        // Verify authorization - CRITICAL
        program_data.authorized_payout_key.require_auth();

        // Validate inputs
        if recipients.len() != amounts.len() {
            panic!("Recipients and amounts vectors must have the same length");
        }

        if recipients.is_empty() {
            panic!("Cannot process empty batch");
        }

        // Calculate total with overflow protection
        let mut total_payout: i128 = 0;
        for amount in amounts.iter() {
            if amount <= 0 {
                panic!("All amounts must be greater than zero");
            }
            total_payout = total_payout
                .checked_add(amount)
                .unwrap_or_else(|| panic!("Payout amount overflow"));
        }

        // Validate balance
        if total_payout > program_data.remaining_balance {
            panic!(
                "Insufficient balance: requested {}, available {}",
                total_payout, program_data.remaining_balance
            );
        }

        // Execute transfers
        let mut updated_history = program_data.payout_history.clone();
        let timestamp = env.ledger().timestamp();
        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &program_data.token_address);

        for (i, recipient) in recipients.iter().enumerate() {
            let amount = amounts.get(i.try_into().unwrap()).unwrap();

            // Transfer tokens
            token_client.transfer(&contract_address, &recipient, &amount);

            // Record payout
            let payout_record = PayoutRecord {
                recipient: recipient.clone(),
                amount,
                timestamp,
            };
            updated_history.push_back(payout_record);
        }

        // Update program data
        let mut updated_data = program_data.clone();
        updated_data.remaining_balance -= total_payout;
        updated_data.payout_history = updated_history;

        // Store updated data
        env.storage().instance().set(&program_key, &updated_data);

        // Emit event
        env.events().publish(
            (BATCH_PAYOUT,),
            (
                program_id,
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
            .get(&program_key)
            .unwrap_or_else(|| panic!("Program not found"));

        // Apply rate limiting to the authorized payout key
        anti_abuse::check_rate_limit(&env, program_data.authorized_payout_key.clone());

        program_data.authorized_payout_key.require_auth();
        // Verify authorization
        // let caller = env.invoker();
        // if caller != program_data.authorized_payout_key {
        //     panic!("Unauthorized: only authorized payout key can trigger payouts");
        // }

        // Validate amount
        if amount <= 0 {
            panic!("Amount must be greater than zero");
        }

        // Validate balance
        if amount > program_data.remaining_balance {
            panic!(
                "Insufficient balance: requested {}, available {}",
                amount, program_data.remaining_balance
            );
        }

        // Transfer tokens
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
        env.storage().instance().set(&program_key, &updated_data);

        // Emit event
        env.events().publish(
            (PAYOUT,),
            (
                program_id,
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
            .get(&program_key)
            .unwrap_or_else(|| panic!("Program not found"))
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
            .get(&program_key)
            .unwrap_or_else(|| panic!("Program not found"));

        program_data.remaining_balance
    }

    /// Gets the total number of programs registered.
    ///
    /// # Returns
    /// * `u32` - Count of registered programs
    pub fn get_program_count(env: Env) -> u32 {
        let registry: Vec<String> = env
            .storage()
            .instance()
            .get(&PROGRAM_REGISTRY)
            .unwrap_or(vec![&env]);

        registry.len()
    }

    // ========================================================================
    // Monitoring & Analytics Functions
    // ========================================================================

    /// Health check - returns contract health status
    pub fn health_check(env: Env) -> monitoring::HealthStatus {
        monitoring::health_check(&env)
    }

    /// Get analytics - returns usage analytics
    pub fn get_analytics(env: Env) -> monitoring::Analytics {
        monitoring::get_analytics(&env)
    }

    /// Get state snapshot - returns current state
    pub fn get_state_snapshot(env: Env) -> monitoring::StateSnapshot {
        monitoring::get_state_snapshot(&env)
    }

    /// Get performance stats for a function
    pub fn get_performance_stats(env: Env, function_name: Symbol) -> monitoring::PerformanceStats {
        monitoring::get_performance_stats(&env, function_name)
    }

    // ========================================================================
    // Anti-Abuse Administrative Functions
    // ========================================================================

    /// Sets the administrative address for anti-abuse configuration.
    /// Can only be called once or by the existing admin.
    pub fn set_admin(env: Env, new_admin: Address) {
        if let Some(current_admin) = anti_abuse::get_admin(&env) {
            current_admin.require_auth();
        }
        anti_abuse::set_admin(&env, new_admin);
    }

    /// Updates the rate limit configuration.
    /// Only the admin can call this.
    pub fn update_rate_limit_config(
        env: Env,
        window_size: u64,
        max_operations: u32,
        cooldown_period: u64,
    ) {
        let admin = anti_abuse::get_admin(&env).expect("Admin not set");
        admin.require_auth();

        anti_abuse::set_config(
            &env,
            anti_abuse::AntiAbuseConfig {
                window_size,
                max_operations,
                cooldown_period,
            },
        );
    }

    /// Adds or removes an address from the whitelist.
    /// Only the admin can call this.
    pub fn set_whitelist(env: Env, address: Address, whitelisted: bool) {
        let admin = anti_abuse::get_admin(&env).expect("Admin not set");
        admin.require_auth();

        anti_abuse::set_whitelist(&env, address, whitelisted);
    }

    /// Checks if an address is whitelisted.
    pub fn is_whitelisted(env: Env, address: Address) -> bool {
        anti_abuse::is_whitelisted(&env, address)
    }

    /// Gets the current rate limit configuration.
    pub fn get_rate_limit_config(env: Env) -> anti_abuse::AntiAbuseConfig {
        anti_abuse::get_config(&env)
    }
}

/// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _},
        token, Address, Env, String,
    };

    // Test helper to create a mock token contract
    fn create_token_contract<'a>(env: &Env, admin: &Address) -> token::Client<'a> {
        let token_address = env.register_stellar_asset_contract(admin.clone());
        token::Client::new(env, &token_address)
    }

    // ========================================================================
    // Program Registration Tests
    // ========================================================================

    #[test]
    fn test_register_single_program() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let backend = Address::generate(&env);
        let token = Address::generate(&env);
        let prog_id = String::from_str(&env, "Hackathon2024");

        // Register program
        let program = client.initialize_program(&prog_id, &backend, &token);

        // Verify program data
        assert_eq!(program.program_id, prog_id);
        assert_eq!(program.authorized_payout_key, backend);
        assert_eq!(program.token_address, token);
        assert_eq!(program.total_funds, 0);
        assert_eq!(program.remaining_balance, 0);
        assert_eq!(program.payout_history.len(), 0);

        // Verify it exists
        assert!(client.program_exists(&prog_id));
        assert_eq!(client.get_program_count(), 1);
    }

    #[test]
    fn test_multiple_programs_isolation() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let backend1 = Address::generate(&env);
        let backend2 = Address::generate(&env);
        let backend3 = Address::generate(&env);
        let token = Address::generate(&env);

        // Register three programs
        let prog1 = String::from_str(&env, "ETHGlobal2024");
        let prog2 = String::from_str(&env, "Stellar2024");
        let prog3 = String::from_str(&env, "BuildathonQ1");

        client.initialize_program(&prog1, &backend1, &token);
        client.initialize_program(&prog2, &backend2, &token);
        client.initialize_program(&prog3, &backend3, &token);

        // Verify all exist
        assert!(client.program_exists(&prog1));
        assert!(client.program_exists(&prog2));
        assert!(client.program_exists(&prog3));
        assert_eq!(client.get_program_count(), 3);

        // Verify complete isolation
        let info1 = client.get_program_info(&prog1);
        let info2 = client.get_program_info(&prog2);
        let info3 = client.get_program_info(&prog3);

        assert_eq!(info1.program_id, prog1);
        assert_eq!(info2.program_id, prog2);
        assert_eq!(info3.program_id, prog3);

        assert_eq!(info1.authorized_payout_key, backend1);
        assert_eq!(info2.authorized_payout_key, backend2);
        assert_eq!(info3.authorized_payout_key, backend3);

        // Verify list programs
        let programs = client.list_programs();
        assert_eq!(programs.len(), 3);
    }

    #[test]
    #[should_panic(expected = "Program already exists")]
    fn test_duplicate_program_registration() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let backend = Address::generate(&env);
        let token = Address::generate(&env);
        let prog_id = String::from_str(&env, "Hackathon2024");

        // Register once - should succeed
        client.initialize_program(&prog_id, &backend, &token);

        // Register again - should panic
        client.initialize_program(&prog_id, &backend, &token);
    }

    #[test]
    #[should_panic(expected = "Program ID cannot be empty")]
    fn test_empty_program_id() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let backend = Address::generate(&env);
        let token = Address::generate(&env);
        let empty_id = String::from_str(&env, "");

        client.initialize_program(&empty_id, &backend, &token);
    }

    #[test]
    #[should_panic(expected = "Program not found")]
    fn test_get_nonexistent_program() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let prog_id = String::from_str(&env, "DoesNotExist");
        client.get_program_info(&prog_id);
    }

    // ========================================================================
    // Fund Locking Tests
    // ========================================================================

    #[test]
    fn test_lock_funds_single_program() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);
        let token_client = create_token_contract(&env, &admin);

        let backend = Address::generate(&env);
        let prog_id = String::from_str(&env, "Hackathon2024");

        // Register program
        client.initialize_program(&prog_id, &backend, &token_client.address);

        // Lock funds
        let amount = 10_000_0000000i128; // 10,000 USDC
        let updated = client.lock_program_funds(&prog_id, &amount);

        assert_eq!(updated.total_funds, amount);
        assert_eq!(updated.remaining_balance, amount);
    }

    #[test]
    fn test_lock_funds_multiple_programs_isolation() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);
        let token_client = create_token_contract(&env, &admin);

        let backend1 = Address::generate(&env);
        let backend2 = Address::generate(&env);

        let prog1 = String::from_str(&env, "Program1");
        let prog2 = String::from_str(&env, "Program2");

        // Register programs
        client.initialize_program(&prog1, &backend1, &token_client.address);
        client.initialize_program(&prog2, &backend2, &token_client.address);

        // Lock different amounts in each program
        let amount1 = 5_000_0000000i128;
        let amount2 = 10_000_0000000i128;

        client.lock_program_funds(&prog1, &amount1);
        client.lock_program_funds(&prog2, &amount2);

        // Verify isolation - funds don't mix
        let info1 = client.get_program_info(&prog1);
        let info2 = client.get_program_info(&prog2);

        assert_eq!(info1.total_funds, amount1);
        assert_eq!(info1.remaining_balance, amount1);
        assert_eq!(info2.total_funds, amount2);
        assert_eq!(info2.remaining_balance, amount2);
    }

    #[test]
    fn test_lock_funds_cumulative() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);
        let token_client = create_token_contract(&env, &admin);

        let backend = Address::generate(&env);
        let prog_id = String::from_str(&env, "Hackathon2024");

        client.initialize_program(&prog_id, &backend, &token_client.address);

        // Lock funds multiple times
        client.lock_program_funds(&prog_id, &1_000_0000000);
        client.lock_program_funds(&prog_id, &2_000_0000000);
        client.lock_program_funds(&prog_id, &3_000_0000000);

        let info = client.get_program_info(&prog_id);
        assert_eq!(info.total_funds, 6_000_0000000);
        assert_eq!(info.remaining_balance, 6_000_0000000);
    }

    #[test]
    #[should_panic(expected = "Amount must be greater than zero")]
    fn test_lock_zero_funds() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let backend = Address::generate(&env);
        let token = Address::generate(&env);
        let prog_id = String::from_str(&env, "Hackathon2024");

        client.initialize_program(&prog_id, &backend, &token);
        client.lock_program_funds(&prog_id, &0);
    }

    // ========================================================================
    // Batch Payout Tests
    // ========================================================================

    #[test]
    #[should_panic(expected = "Recipients and amounts vectors must have the same length")]
    fn test_batch_payout_mismatched_lengths() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);
        let token_client = create_token_contract(&env, &admin);

        let backend = Address::generate(&env);
        let prog_id = String::from_str(&env, "Test");

        client.initialize_program(&prog_id, &backend, &token_client.address);
        client.lock_program_funds(&prog_id, &10_000_0000000);

        let recipients = soroban_sdk::vec![&env, Address::generate(&env), Address::generate(&env)];
        let amounts = soroban_sdk::vec![&env, 1_000_0000000i128]; // Mismatch!

        client.batch_payout(&prog_id, &recipients, &amounts);
    }

    #[test]
    #[should_panic(expected = "Insufficient balance")]
    fn test_batch_payout_insufficient_balance() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);
        let token_client = create_token_contract(&env, &admin);

        let backend = Address::generate(&env);
        let prog_id = String::from_str(&env, "Test");

        client.initialize_program(&prog_id, &backend, &token_client.address);
        client.lock_program_funds(&prog_id, &5_000_0000000);

        let recipients = soroban_sdk::vec![&env, Address::generate(&env)];
        let amounts = soroban_sdk::vec![&env, 10_000_0000000i128]; // More than available!

        client.batch_payout(&prog_id, &recipients, &amounts);
    }

    #[test]
    fn test_program_count() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        assert_eq!(client.get_program_count(), 0);

        let backend = Address::generate(&env);
        let token = Address::generate(&env);

        client.initialize_program(&String::from_str(&env, "P1"), &backend, &token);
        assert_eq!(client.get_program_count(), 1);

        client.initialize_program(&String::from_str(&env, "P2"), &backend, &token);
        assert_eq!(client.get_program_count(), 2);

        client.initialize_program(&String::from_str(&env, "P3"), &backend, &token);
        assert_eq!(client.get_program_count(), 3);
    }

    // ========================================================================
    // Anti-Abuse Tests
    // ========================================================================

    #[test]
    #[should_panic(expected = "Operation in cooldown period")]
    fn test_anti_abuse_cooldown_panic() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(1000);
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.set_admin(&admin);
        client.update_rate_limit_config(&3600, &10, &60);

        let backend = Address::generate(&env);
        let token = Address::generate(&env);
        
        client.initialize_program(&String::from_str(&env, "P1"), &backend, &token);
        
        // Advance time by 30s (less than 60s cooldown)
        env.ledger().with_mut(|li| li.timestamp += 30);
        
        client.initialize_program(&String::from_str(&env, "P2"), &backend, &token);
    }

    #[test]
    #[should_panic(expected = "Rate limit exceeded")]
    fn test_anti_abuse_limit_panic() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(1000);
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.set_admin(&admin);
        client.update_rate_limit_config(&3600, &2, &0); // 2 ops max, no cooldown

        let backend = Address::generate(&env);
        let token = Address::generate(&env);
        
        client.initialize_program(&String::from_str(&env, "P1"), &backend, &token);
        client.initialize_program(&String::from_str(&env, "P2"), &backend, &token);
        client.initialize_program(&String::from_str(&env, "P3"), &backend, &token); // Should panic
    }

    #[test]
    fn test_anti_abuse_whitelist() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(1000);
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.set_admin(&admin);
        client.update_rate_limit_config(&3600, &1, &60); // 1 op max

        let backend = Address::generate(&env);
        let token = Address::generate(&env);
        
        client.set_whitelist(&backend, &true);
        
        client.initialize_program(&String::from_str(&env, "P1"), &backend, &token);
        client.initialize_program(&String::from_str(&env, "P2"), &backend, &token); // Should work because whitelisted
    }

    #[test]
    fn test_anti_abuse_config_update() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.set_admin(&admin);
        
        client.update_rate_limit_config(&7200, &5, &120);
        
        let config = client.get_rate_limit_config();
        assert_eq!(config.window_size, 7200);
        assert_eq!(config.max_operations, 5);
        assert_eq!(config.cooldown_period, 120);
    }
}
