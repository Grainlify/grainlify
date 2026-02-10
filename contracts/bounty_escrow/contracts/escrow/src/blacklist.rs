//! # Participant Blacklist and Whitelist Module
//!
//! This module provides compliance and abuse prevention features through:
//! - **Blacklist**: Block specific addresses from locking/receiving funds (e.g., sanctioned addresses)
//! - **Whitelist Mode**: Restrict to only verified participants (optional enforcement)
//!
//! ## Security Model
//!
//! - **Admin-only access**: Only contract admin can manage blacklist/whitelist
//! - **Persistent storage**: List changes are permanent across calls
//! - **Efficient lookups**: O(1) existence checks for both lists
//! - **Audit trail**: Events emitted for all list modifications

use soroban_sdk::{contracttype, symbol_short, Address, Env, Map, String};

// ============================================================================
// Data Structures
// ============================================================================

/// Blacklist status configuration
#[contracttype]
#[derive(Clone, Debug)]
pub struct BlacklistConfig {
    /// Whether blacklist enforcement is active
    pub enabled: bool,
    /// Whether whitelist mode is active (if true, only whitelisted addresses can participate)
    pub whitelist_mode: bool,
}

// ============================================================================
// Events
// ============================================================================

/// Event emitted when an address is added to the blacklist
#[contracttype]
#[derive(Clone, Debug)]
pub struct AddressBlacklisted {
    pub address: Address,
    pub reason: Option<String>,
    pub timestamp: u64,
}

/// Event emitted when an address is removed from the blacklist
#[contracttype]
#[derive(Clone, Debug)]
pub struct AddressUnblacklisted {
    pub address: Address,
    pub timestamp: u64,
}

/// Event emitted when an address is added to the whitelist
#[contracttype]
#[derive(Clone, Debug)]
pub struct AddressWhitelisted {
    pub address: Address,
    pub timestamp: u64,
}

/// Event emitted when an address is removed from the whitelist
#[contracttype]
#[derive(Clone, Debug)]
pub struct AddressUnwhitelisted {
    pub address: Address,
    pub timestamp: u64,
}

/// Event emitted when whitelist mode is toggled
#[contracttype]
#[derive(Clone, Debug)]
pub struct WhitelistModeToggled {
    pub enabled: bool,
    pub timestamp: u64,
}

// ============================================================================
// Emit Functions
// ============================================================================

pub fn emit_address_blacklisted(env: &Env, address: Address, reason: Option<String>) {
    env.events().publish(
        (symbol_short!("blklist"), symbol_short!("add")),
        AddressBlacklisted {
            address,
            reason,
            timestamp: env.ledger().timestamp(),
        },
    );
}

pub fn emit_address_unblacklisted(env: &Env, address: Address) {
    env.events().publish(
        (symbol_short!("blklist"), symbol_short!("rm")),
        AddressUnblacklisted {
            address,
            timestamp: env.ledger().timestamp(),
        },
    );
}

pub fn emit_address_whitelisted(env: &Env, address: Address) {
    env.events().publish(
        (symbol_short!("whtlist"), symbol_short!("add")),
        AddressWhitelisted {
            address,
            timestamp: env.ledger().timestamp(),
        },
    );
}

pub fn emit_address_unwhitelisted(env: &Env, address: Address) {
    env.events().publish(
        (symbol_short!("whtlist"), symbol_short!("rm")),
        AddressUnwhitelisted {
            address,
            timestamp: env.ledger().timestamp(),
        },
    );
}

pub fn emit_whitelist_mode_toggled(env: &Env, enabled: bool) {
    env.events().publish(
        (symbol_short!("whtlist"), symbol_short!("mode")),
        WhitelistModeToggled {
            enabled,
            timestamp: env.ledger().timestamp(),
        },
    );
}

// ============================================================================
// Public Functions
// ============================================================================

/// Adds an address to the blacklist
pub fn add_to_blacklist(env: &Env, address: Address, reason: Option<String>) {
    let blacklist: Map<Address, Option<String>> = env
        .storage()
        .persistent()
        .get(&symbol_short!("blklist"))
        .unwrap_or(Map::new(env));

    let mut new_blacklist = blacklist;
    new_blacklist.set(address.clone(), reason.clone());

    env.storage()
        .persistent()
        .set(&symbol_short!("blklist"), &new_blacklist);

    emit_address_blacklisted(env, address, reason);
}

/// Removes an address from the blacklist
pub fn remove_from_blacklist(env: &Env, address: Address) {
    let blacklist: Map<Address, Option<String>> = env
        .storage()
        .persistent()
        .get(&symbol_short!("blklist"))
        .unwrap_or(Map::new(env));

    if blacklist.contains_key(address.clone()) {
        let mut new_blacklist = blacklist;
        new_blacklist.remove(address.clone());
        env.storage()
            .persistent()
            .set(&symbol_short!("blklist"), &new_blacklist);
        emit_address_unblacklisted(env, address);
    }
}

/// Checks if an address is blacklisted
pub fn is_blacklisted(env: &Env, address: &Address) -> bool {
    let blacklist: Map<Address, Option<String>> = env
        .storage()
        .persistent()
        .get(&symbol_short!("blklist"))
        .unwrap_or(Map::new(env));

    blacklist.contains_key(address.clone())
}

/// Adds an address to the whitelist
pub fn add_to_whitelist(env: &Env, address: Address) {
    let whitelist: Map<Address, bool> = env
        .storage()
        .persistent()
        .get(&symbol_short!("whtlist"))
        .unwrap_or(Map::new(env));

    let mut new_whitelist = whitelist;
    new_whitelist.set(address.clone(), true);

    env.storage()
        .persistent()
        .set(&symbol_short!("whtlist"), &new_whitelist);

    emit_address_whitelisted(env, address);
}

/// Removes an address from the whitelist
pub fn remove_from_whitelist(env: &Env, address: Address) {
    let whitelist: Map<Address, bool> = env
        .storage()
        .persistent()
        .get(&symbol_short!("whtlist"))
        .unwrap_or(Map::new(env));

    if whitelist.contains_key(address.clone()) {
        let mut new_whitelist = whitelist;
        new_whitelist.remove(address.clone());
        env.storage()
            .persistent()
            .set(&symbol_short!("whtlist"), &new_whitelist);
        emit_address_unwhitelisted(env, address);
    }
}

/// Checks if an address is whitelisted
pub fn is_whitelisted(env: &Env, address: &Address) -> bool {
    let whitelist: Map<Address, bool> = env
        .storage()
        .persistent()
        .get(&symbol_short!("whtlist"))
        .unwrap_or(Map::new(env));

    whitelist.get(address.clone()).unwrap_or(false)
}

/// Enables or disables whitelist mode
pub fn set_whitelist_mode(env: &Env, enabled: bool) {
    env.storage()
        .persistent()
        .set(&symbol_short!("wht_mode"), &enabled);

    emit_whitelist_mode_toggled(env, enabled);
}

/// Checks if whitelist mode is enabled
pub fn is_whitelist_mode_enabled(env: &Env) -> bool {
    env.storage()
        .persistent()
        .get(&symbol_short!("wht_mode"))
        .unwrap_or(false)
}

/// Validates if an address can participate (not blacklisted and passes whitelist check if enabled)
pub fn is_participant_allowed(env: &Env, address: &Address) -> bool {
    // Check blacklist first (always enforced)
    if is_blacklisted(env, address) {
        return false;
    }

    // Check whitelist if enabled
    if is_whitelist_mode_enabled(env) {
        return is_whitelisted(env, address);
    }

    // Otherwise allowed
    true
}
