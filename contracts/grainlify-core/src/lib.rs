//! # Grainlify Contract Upgrade System
//!
//! A minimal, secure contract upgrade pattern for Soroban smart contracts.
//! This contract implements admin-controlled WASM upgrades with version tracking.
//!
//! ## Overview
//!
//! The Grainlify contract provides a foundational upgrade mechanism that allows
//! authorized administrators to update contract logic while maintaining state
//! persistence. This is essential for bug fixes, feature additions, and security
//! patches in production environments.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │              Contract Upgrade Architecture                   │
//! ├─────────────────────────────────────────────────────────────┤
//! │                                                              │
//! │  ┌──────────────┐                                           │
//! │  │    Admin     │                                           │
//! │  └──────┬───────┘                                           │
//! │         │                                                    │
//! │         │ 1. Compile new WASM                               │
//! │         │ 2. Upload to Stellar                              │
//! │         │ 3. Get WASM hash                                  │
//! │         │                                                    │
//! │         ▼                                                    │
//! │  ┌──────────────────┐                                       │
//! │  │  upgrade(hash)   │────────┐                              │
//! │  └──────────────────┘        │                              │
//! │         │                     │                              │
//! │         │ require_auth()      │                              │
//! │         │                     ▼                              │
//! │         │              ┌─────────────┐                       │
//! │         │              │   Verify    │                       │
//! │         │              │   Admin     │                       │
//! │         │              └──────┬──────┘                       │
//! │         │                     │                              │
//! │         │                     ▼                              │
//! │         │              ┌─────────────┐                       │
//! │         └─────────────>│   Update    │                       │
//! │                        │   WASM      │                       │
//! │                        └──────┬──────┘                       │
//! │                               │                              │
//! │                               ▼                              │
//! │                        ┌─────────────┐                       │
//! │                        │ New Version │                       │
//! │                        │  (Optional) │                       │
//! │                        └─────────────┘                       │
//! │                                                              │
//! │  Storage:                                                    │
//! │  ┌────────────────────────────────────┐                     │
//! │  │ Admin: Address                     │                     │
//! │  │ Version: u32                       │                     │
//! │  └────────────────────────────────────┘                     │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Security Model
//!
//! ### Trust Assumptions
//! - **Admin**: Highly trusted entity with upgrade authority
//! - **WASM Code**: New code must be audited before deployment
//! - **State Preservation**: Upgrades preserve existing contract state
//!
//! ### Security Features
//! 1. **Single Admin**: Only one authorized address can upgrade
//! 2. **Authorization Check**: Every upgrade requires admin signature
//! 3. **Version Tracking**: Auditable upgrade history
//! 4. **State Preservation**: Instance storage persists across upgrades
//! 5. **Immutable After Init**: Admin cannot be changed after initialization
//!
//! ### Security Considerations
//! - Admin key should be secured with hardware wallet or multi-sig
//! - New WASM should be audited before upgrade
//! - Consider implementing timelock for high-value contracts
//! - Version updates should follow semantic versioning
//! - Test upgrades on testnet before mainnet deployment
//!
//! ## Upgrade Process
//!
//! ```rust
//! // 1. Initialize contract (one-time)
//! let admin = Address::from_string("GADMIN...");
//! contract.init(&admin);
//!
//! // 2. Develop and test new version locally
//! // ... make changes to contract code ...
//!
//! // 3. Build new WASM
//! // $ cargo build --release --target wasm32-unknown-unknown
//!
//! // 4. Upload WASM to Stellar and get hash
//! // $ stellar contract install --wasm target/wasm32-unknown-unknown/release/contract.wasm
//! // Returns: hash (e.g., "abc123...")
//!
//! // 5. Perform upgrade
//! let wasm_hash = BytesN::from_array(&env, &[0xab, 0xcd, ...]);
//! contract.upgrade(&wasm_hash);
//!
//! // 6. (Optional) Update version number
//! contract.set_version(&2);
//!
//! // 7. Verify upgrade
//! let version = contract.get_version();
//! assert_eq!(version, 2);
//! ```
//!
//! ## State Migration
//!
//! When upgrading contracts that require state migration:
//!
//! ```rust
//! // In new WASM version, add migration function:
//! pub fn migrate(env: Env) {
//!     let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
//!     admin.require_auth();
//!     
//!     // Perform state migration
//!     // Example: Convert old data format to new format
//!     let old_version = env.storage().instance().get(&DataKey::Version).unwrap_or(0);
//!     
//!     if old_version < 2 {
//!         // Migrate from v1 to v2
//!         migrate_v1_to_v2(&env);
//!     }
//!     
//!     // Update version
//!     env.storage().instance().set(&DataKey::Version, &2u32);
//! }
//! ```
//!
//! ## Best Practices
//!
//! 1. **Version Numbering**: Use semantic versioning (MAJOR.MINOR.PATCH)
//! 2. **Testing**: Always test upgrades on testnet first
//! 3. **Auditing**: Audit new code before mainnet deployment
//! 4. **Documentation**: Document breaking changes between versions
//! 5. **Rollback Plan**: Keep previous WASM hash for emergency rollback
//! 6. **Admin Security**: Use multi-sig or timelock for production
//! 7. **State Validation**: Verify state integrity after upgrade
//!
//! ## Common Pitfalls
//!
//! - ❌ Not testing upgrades on testnet
//! - ❌ Losing admin private key
//! - ❌ Breaking state compatibility between versions
//! - ❌ Not documenting migration steps
//! - ❌ Upgrading without proper testing
//! - ❌ Not having a rollback plan

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
        let timestamp = env.ledger().timestamp();
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
                timestamp,
                success,
            },
        );
    }

    // Track performance
    pub fn emit_performance(env: &Env, function: Symbol, duration: u64) {
        let timestamp = env.ledger().timestamp();
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
                timestamp,
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
        let timestamp = env.ledger().timestamp();
        let op_key = Symbol::new(env, OPERATION_COUNT);
        let usr_key = Symbol::new(env, USER_COUNT);
        let err_key = Symbol::new(env, ERROR_COUNT);

        StateSnapshot {
            timestamp,
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

#[contract]
pub struct GrainlifyContract;

// ============================================================================
// Data Structures
// ============================================================================

/// Storage keys for contract data.
#[contracttype]
#[derive(Clone)]
enum DataKey {
    /// Administrator address with upgrade authority
    Admin,

    /// Current version number (increments with upgrades)
    Version,

    // NEW: store wasm hash per proposal
    UpgradeProposal(u64),
}

// ============================================================================
// Constants
// ============================================================================

const VERSION: u32 = 1;

// ============================================================================
// Contract Implementation
// ============================================================================

#[contractimpl]
impl GrainlifyContract {
    // ========================================================================
    // Initialization
    // ========================================================================

    /// Initializes the contract with multisig configuration.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `signers` - List of signer addresses for multisig
    /// * `threshold` - Number of signatures required to execute proposals
    pub fn init(env: Env, signers: Vec<Address>, threshold: u32) {
        if env.storage().instance().has(&DataKey::Version) {
            panic!("Already initialized");
        }

        MultiSig::init(&env, signers, threshold);
        env.storage().instance().set(&DataKey::Version, &VERSION);
    }

    /// Initializes the contract with a single admin address.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `admin` - Address authorized to perform upgrades
    pub fn init_admin(env: Env, admin: Address) {
        let start = env.ledger().timestamp();

        // Prevent re-initialization to protect admin immutability
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

    // ========================================================================
    // Upgrade Functions
    // ========================================================================

    /// Proposes an upgrade with a new WASM hash (multisig version).
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `proposer` - Address proposing the upgrade
    /// * `wasm_hash` - Hash of the new WASM code
    ///
    /// # Returns
    /// * `u64` - The proposal ID
    pub fn propose_upgrade(env: Env, proposer: Address, wasm_hash: BytesN<32>) -> u64 {
        let proposal_id = MultiSig::propose(&env, proposer);

        env.storage()
            .instance()
            .set(&DataKey::UpgradeProposal(proposal_id), &wasm_hash);

        proposal_id
    }

    /// Approves an upgrade proposal (multisig version).
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `proposal_id` - The ID of the proposal to approve
    /// * `signer` - Address approving the proposal
    pub fn approve_upgrade(env: Env, proposal_id: u64, signer: Address) {
        MultiSig::approve(&env, proposal_id, signer);
    }

    /// Executes an upgrade proposal that has met the multisig threshold.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `proposal_id` - The ID of the upgrade proposal to execute
    pub fn execute_upgrade(env: Env, proposal_id: u64) {
        if !MultiSig::can_execute(&env, proposal_id) {
            panic!("Threshold not met");
        }

        let wasm_hash: BytesN<32> = env
            .storage()
            .instance()
            .get(&DataKey::UpgradeProposal(proposal_id))
            .expect("Missing upgrade proposal");

        env.deployer().update_current_contract_wasm(wasm_hash);

        MultiSig::mark_executed(&env, proposal_id);
    }

    /// Upgrades the contract to new WASM code (single admin version).
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `new_wasm_hash` - Hash of the uploaded WASM code (32 bytes)
    pub fn upgrade(env: Env, new_wasm_hash: BytesN<32>) {
        let start = env.ledger().timestamp();

        // Verify admin authorization
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

    // ========================================================================
    // Version Management
    // ========================================================================

    /// Retrieves the current contract version number.
    pub fn get_version(env: Env) -> u32 {
        env.storage().instance().get(&DataKey::Version).unwrap_or(0)
    }

    /// Updates the contract version number.
    pub fn set_version(env: Env, new_version: u32) {
        let start = env.ledger().timestamp();

        // Verify admin authorization
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        // Update version number
        env.storage()
            .instance()
            .set(&DataKey::Version, &new_version);

        // Track successful operation
        monitoring::track_operation(&env, symbol_short!("set_ver"), admin, true);

        // Track performance
        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("set_ver"), duration);
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
}

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

        client.init(&signers, &2);
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
