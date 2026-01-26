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
use soroban_sdk::{contract, contractimpl, contracttype, Address, BytesN, Env};

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
}

/// Current contract version.
///
/// This constant represents the version of the deployed WASM code. After upgrading,
/// the admin should call `set_version` to update the stored version to match.
const VERSION: u3env.storage().instance().get(&DataKey::Version).unwrap_or(0) = 1;

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
            panic!("Already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Version, &VERSION);
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

        env.deployer().update_current_contract_wasm(new_wasm_hash);
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


