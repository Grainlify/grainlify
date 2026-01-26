//! # Grainlify Core Contract
//!
//! This contract provides core upgradeability functionality for the Grainlify platform.
//! It manages contract versioning and WASM code upgrades with admin authorization.
//!
//! ## Overview
//!
//! The Grainlify Core Contract implements a simple but secure upgrade mechanism for Soroban
//! smart contracts. It maintains an admin address that has exclusive authority to upgrade
//! the contract's WASM code and manage version numbers.
//!
//! ## Key Features
//!
//! - **Admin-Controlled Upgrades**: Only the admin can upgrade contract code
//! - **Version Tracking**: Maintains version number for migration management
//! - **One-Time Initialization**: Contract can only be initialized once
//! - **Secure Authorization**: All privileged operations require admin signature
//!
//! ## Security Model
//!
//! - **Single Admin**: One address controls all upgrade operations
//! - **Initialization Lock**: Cannot be re-initialized after first setup
//! - **Version Management**: Tracks contract version for safe migrations
//!
//! ## Usage Example
//!
//! ```rust,ignore
//! // 1. Initialize contract (one-time)
//! contract.init(env, admin_address);
//!
//! // 2. Check current version
//! let version = contract.get_version(env);
//!
//! // 3. Upgrade contract (admin only)
//! let new_wasm_hash = BytesN::from_array(&env, &[...]);
//! contract.upgrade(env, new_wasm_hash);
//!
//! // 4. Update version after upgrade
//! contract.set_version(env, 2);
//! ```

#![no_std]

mod multisig;
use multisig::MultiSig;
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Address, BytesN, Env, Symbol, Vec,
};

// ==================== MONITORING MODULE ====================
mod monitoring {
    use soroban_sdk::{contracttype, symbol_short, Address, Env, String, Symbol};

    // Storage keys
    const OPERATION_COUNT: &str = "op_count";
    const USER_COUNT: &str = "usr_count";
    const ERROR_COUNT: &str = "err_count";

    // Event: Operation metric
    #[contracttype]
    #[derive(Clone, Debug)]
    pub struct OperationMetric {
        pub operation: Symbol,
        pub caller: Address,
        pub timestamp: u64,
        pub success: bool,
    }

    // Event: Performance metric
    #[contracttype]
    #[derive(Clone, Debug)]
    pub struct PerformanceMetric {
        pub function: Symbol,
        pub duration: u64,
        pub timestamp: u64,
    }

    // Data: Health status
    #[contracttype]
    #[derive(Clone, Debug)]
    pub struct HealthStatus {
        pub is_healthy: bool,
        pub last_operation: u64,
        pub total_operations: u64,
        pub contract_version: String,
    }

    // Data: Analytics
    #[contracttype]
    #[derive(Clone, Debug)]
    pub struct Analytics {
        pub operation_count: u64,
        pub unique_users: u64,
        pub error_count: u64,
        pub error_rate: u32,
    }

    // Data: State snapshot
    #[contracttype]
    #[derive(Clone, Debug)]
    pub struct StateSnapshot {
        pub timestamp: u64,
        pub total_operations: u64,
        pub total_users: u64,
        pub total_errors: u64,
    }

    // Data: Performance stats
    #[contracttype]
    #[derive(Clone, Debug)]
    pub struct PerformanceStats {
        pub function_name: Symbol,
        pub call_count: u64,
        pub total_time: u64,
        pub avg_time: u64,
        pub last_called: u64,
    }

    // Track operation
    pub fn track_operation(env: &Env, operation: Symbol, caller: Address, success: bool) {
        let key = Symbol::new(env, OPERATION_COUNT);
        let count: u64 = env.storage().persistent().get(&key).unwrap_or(0);
        env.storage().persistent().set(&key, &(count + 1));

        if !success {
            let err_key = Symbol::new(env, ERROR_COUNT);
            let err_count: u64 = env.storage().persistent().get(&err_key).unwrap_or(0);
            env.storage().persistent().set(&err_key, &(err_count + 1));
        }

        env.events().publish(
            (symbol_short!("metric"), symbol_short!("op")),
            OperationMetric {
                operation,
                caller,
                timestamp: env.ledger().timestamp(),
                success,
            },
        );
    }

    // Track performance
    pub fn emit_performance(env: &Env, function: Symbol, duration: u64) {
        let count_key = (Symbol::new(env, "perf_cnt"), function.clone());
        let time_key = (Symbol::new(env, "perf_time"), function.clone());

        let count: u64 = env.storage().persistent().get(&count_key).unwrap_or(0);
        let total: u64 = env.storage().persistent().get(&time_key).unwrap_or(0);

        env.storage().persistent().set(&count_key, &(count + 1));
        env.storage()
            .persistent()
            .set(&time_key, &(total + duration));

        env.events().publish(
            (symbol_short!("metric"), symbol_short!("perf")),
            PerformanceMetric {
                function,
                duration,
                timestamp: env.ledger().timestamp(),
            },
        );
    }

    // Health check
    pub fn health_check(env: &Env) -> HealthStatus {
        let key = Symbol::new(env, OPERATION_COUNT);
        let ops: u64 = env.storage().persistent().get(&key).unwrap_or(0);

        HealthStatus {
            is_healthy: true,
            last_operation: env.ledger().timestamp(),
            total_operations: ops,
            contract_version: String::from_str(env, "1.0.0"),
        }
    }

    // Get analytics
    pub fn get_analytics(env: &Env) -> Analytics {
        let op_key = Symbol::new(env, OPERATION_COUNT);
        let usr_key = Symbol::new(env, USER_COUNT);
        let err_key = Symbol::new(env, ERROR_COUNT);

        let ops: u64 = env.storage().persistent().get(&op_key).unwrap_or(0);
        let users: u64 = env.storage().persistent().get(&usr_key).unwrap_or(0);
        let errors: u64 = env.storage().persistent().get(&err_key).unwrap_or(0);

        let error_rate = if ops > 0 {
            ((errors as u128 * 10000) / ops as u128) as u32
        } else {
            0
        };

        Analytics {
            operation_count: ops,
            unique_users: users,
            error_count: errors,
            error_rate,
        }
    }

    // Get state snapshot
    pub fn get_state_snapshot(env: &Env) -> StateSnapshot {
        let op_key = Symbol::new(env, OPERATION_COUNT);
        let usr_key = Symbol::new(env, USER_COUNT);
        let err_key = Symbol::new(env, ERROR_COUNT);

        StateSnapshot {
            timestamp: env.ledger().timestamp(),
            total_operations: env.storage().persistent().get(&op_key).unwrap_or(0),
            total_users: env.storage().persistent().get(&usr_key).unwrap_or(0),
            total_errors: env.storage().persistent().get(&err_key).unwrap_or(0),
        }
    }

    // Get performance stats
    pub fn get_performance_stats(env: &Env, function_name: Symbol) -> PerformanceStats {
        let count_key = (Symbol::new(env, "perf_cnt"), function_name.clone());
        let time_key = (Symbol::new(env, "perf_time"), function_name.clone());
        let last_key = (Symbol::new(env, "perf_last"), function_name.clone());

        let count: u64 = env.storage().persistent().get(&count_key).unwrap_or(0);
        let total: u64 = env.storage().persistent().get(&time_key).unwrap_or(0);
        let last: u64 = env.storage().persistent().get(&last_key).unwrap_or(0);

        let avg = if count > 0 { total / count } else { 0 };

        PerformanceStats {
            function_name,
            call_count: count,
            total_time: total,
            avg_time: avg,
            last_called: last,
        }
    }
}
// ==================== END MONITORING MODULE ====================


// ============================================================================
// Contract Definition
// ============================================================================

/// The main Grainlify core contract for upgradability.
#[contract]
pub struct GrainlifyContract;

/// Storage keys for contract data.
///
/// These keys organize different types of persistent data in the contract's storage.
#[contracttype]
#[derive(Clone)]
enum DataKey {
    /// Stores the admin address authorized to perform upgrades.
    Admin,
    /// Stores the current contract version number.
    Version,

    
    // NEW: store wasm hash per proposal
    UpgradeProposal(u64),
}

/// Current contract version.
///
/// This constant represents the version of the deployed WASM code. After upgrading,
/// the admin should call `set_version` to update the stored version to match.
const VERSION: u3env.storage().instance().get(&DataKey::Version).unwrap_or(0) = 1;

    // ========================================================================
    // Initialization
    // ========================================================================

    /// Initializes the contract with an admin address.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `admin` - Address authorized to perform upgrades
    ///
    /// # Panics
    /// * If contract is already initialized
    ///
    /// # State Changes
    /// - Sets Admin address in instance storage
    /// - Sets initial Version number
    ///
    /// # Security Considerations
    /// - Can only be called once (prevents admin takeover)
    /// - Admin address is immutable after initialization
    /// - Admin should be a secure address (hardware wallet/multi-sig)
    /// - No authorization required for initialization (first-caller pattern)
    ///
    /// # Example
    /// ```rust
    /// use soroban_sdk::{Address, Env};
    ///
    /// let env = Env::default();
    /// let admin = Address::generate(&env);
    ///
    /// // Initialize contract
    /// contract.init(&env, &admin);
    ///
    /// // Subsequent init attempts will panic
    /// // contract.init(&env, &another_admin); // ❌ Panics!
    /// ```
    ///
    /// # Gas Cost
    /// Low - Two storage writes
    ///
    /// # Production Deployment
    /// ```bash
    /// # Deploy contract
    /// stellar contract deploy \
    ///   --wasm target/wasm32-unknown-unknown/release/grainlify.wasm \
    ///   --source ADMIN_SECRET_KEY
    ///
    /// # Initialize with admin address
    /// stellar contract invoke \
    ///   --id CONTRACT_ID \
    ///   --source ADMIN_SECRET_KEY \
    ///   -- init \
    ///   --admin GADMIN_ADDRESS
    /// ```
 
#[contractimpl]
impl GrainlifyContract {
    /// Initialize the contract with an admin address.
    ///
    /// Sets up the contract for first use by storing the admin address and initial version.
    /// This function can only be called once.
    ///
    /// # Arguments
    ///
    /// * `env` - The contract execution environment
    /// * `admin` - Address that will have upgrade and version management privileges
    ///
    /// # Panics
    ///
    /// Panics with "Already initialized" if the contract has already been initialized.
    ///
    /// # Security
    ///
    /// - Can only be called once
    /// - The admin address controls all future upgrades
    /// - Consider using a multisig or DAO-controlled address for production
    /// - Initial version is set to the VERSION constant (currently 1)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let admin = Address::from_string("GADMIN...");
    /// contract.init(env, admin);
    /// // Contract is now initialized and ready for use
    /// ```
    pub fn init(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            monitoring::track_operation(&env, symbol_short!("init"), admin.clone(), false);
            panic!("Already initialized");
        }

        // Store admin address (immutable after this point)
        env.storage().instance().set(&DataKey::Admin, &admin);

        // Set initial version
        env.storage().instance().set(&DataKey::Version, &VERSION);

        // Track successful operation
        monitoring::track_operation(&env, symbol_short!("init"), admin, true);

        // Track performance
        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("init"), duration);
    }

    /// Upgrade the contract to new WASM code.
    ///
    /// Replaces the current contract's WASM code with new code identified by its hash.
    /// Only the admin can perform upgrades. After upgrading, consider calling `set_version`
    /// to update the version number.
    ///
    /// # Arguments
    ///
    /// * `env` - The contract execution environment
    /// * `new_wasm_hash` - 32-byte hash of the new WASM code to deploy
    ///
    /// # Panics
    ///
    /// Panics if the caller is not the admin address.
    ///
    /// # Security
    ///
    /// - **Admin-only**: Requires authorization from the admin address
    /// - **Irreversible**: Once upgraded, the old code is replaced
    /// - **Testing Required**: Always test upgrades on testnet first
    /// - **Version Management**: Update version number after upgrade
    /// - **State Preservation**: Contract storage persists across upgrades
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Upload new WASM and get its hash
    /// let new_wasm_hash = BytesN::from_array(&env, &[0x12, 0x34, ...]);
    /// contract.upgrade(env, new_wasm_hash);
    /// // Contract now runs the new code
    /// contract.set_version(env, 2); // Update version
    /// ```
    pub fn upgrade(env: Env, new_wasm_hash: BytesN<3env.storage().instance().get(&DataKey::Version).unwrap_or(0)>) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        // Perform WASM upgrade
        env.deployer().update_current_contract_wasm(new_wasm_hash);

        // Track successful operation
        monitoring::track_operation(&env, symbol_short!("upgrade"), admin, true);

        // Track performance
        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("upgrade"), duration);
    }

    /// Get the current contract version number.
    ///
    /// Returns the stored version number, or 0 if no version has been set.
    /// This is a read-only view function.
    ///
    /// # Arguments
    ///
    /// * `env` - The contract execution environment
    ///
    /// # Returns
    ///
    /// The current version number, or 0 if not set.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let version = contract.get_version(env);
    /// // version is 1 after initialization, or updated value after set_version
    /// ```
    pub fn get_version(env: Env) -> u3env.storage().instance().get(&DataKey::Version).unwrap_or(0) {
        env.storage().instance().get(&DataKey::Version).unwrap_or(0)
    }
    
    /// Update the contract version number.
    ///
    /// Sets a new version number in storage. This should be called after upgrading the
    /// contract to keep the version number in sync with the deployed code.
    ///
    /// # Arguments
    ///
    /// * `env` - The contract execution environment
    /// * `new_version` - The new version number to store
    ///
    /// # Panics
    ///
    /// Panics if the caller is not the admin address.
    ///
    /// # Security
    ///
    /// - **Admin-only**: Requires authorization from the admin address
    /// - **Manual Update**: Version must be manually updated after upgrades
    /// - **Migration Tracking**: Use version numbers to track migration state
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // After upgrading to new code
    /// contract.set_version(env, 2);
    /// // Version is now 2
    /// ```
    pub fn set_version(env: Env, new_version: u3env.storage().instance().get(&DataKey::Version).unwrap_or(0)) {
         let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
         admin.require_auth();
         env.storage().instance().set(&DataKey::Version, &new_version);
    }
}


// ============================================================================
// Testing Module
// ============================================================================
#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};

    #[test]
    fn multisig_init_works() {
        let env = Env::default();
        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &contract_id);

        let mut signers = soroban_sdk::Vec::new(&env);
        signers.push_back(Address::generate(&env));
        signers.push_back(Address::generate(&env));
        signers.push_back(Address::generate(&env));

        client.init(&signers, &2u32);
    }

    #[test]
    fn test_set_version() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.init_admin(&admin);

        client.set_version(&2);
        assert_eq!(client.get_version(), 2);
    }
}

