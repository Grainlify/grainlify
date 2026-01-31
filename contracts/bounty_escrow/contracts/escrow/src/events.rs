//! # Bounty Escrow Events Module
//!
//! This module defines all events emitted by the Bounty Escrow contract.
//! Events provide an audit trail and enable off-chain indexing for monitoring
//! bounty lifecycle states and time-based release schedules.
//!
//! ## Event Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Event Flow Diagram                        │
//! ├─────────────────────────────────────────────────────────────┤
//! │                                                              │
//! │  Contract Init → BountyEscrowInitialized                    │
//! │       ↓                                                      │
//! │  Lock Funds    → FundsLocked                                │
//! │       ↓                                                      │
//! │  ┌──────────┐                                               │
//! │  │ Decision │                                               │
//! │  └────┬─────┘                                               │
//! │       ├─────→ Release → FundsReleased                       │
//! │       ├─────→ Schedule → ScheduleCreated                    │
//! │       │         ↓ → ScheduleReleased                        │
//! │       └─────→ Refund  → FundsRefunded                       │
//! └─────────────────────────────────────────────────────────────┘
//!

use soroban_sdk::{contracttype, symbol_short, Address, Env};

// ============================================================================
// Contract Initialization Event
// ============================================================================

/// Event emitted when the Bounty Escrow contract is initialized.
///
/// # Fields
/// * `admin` - The administrator address with release authorization
/// * `token` - The token contract address (typically XLM/USDC)
/// * `timestamp` - Unix timestamp of initialization
///
/// # Event Topic
/// Symbol: `init`
///
/// # Usage
/// This event is emitted once during contract deployment and signals
/// that the contract is ready to accept bounty escrows.
///
/// # Security Considerations
/// - Only emitted once; subsequent init attempts should fail
/// - Admin address should be a secure backend service
/// - Token address must be a valid Stellar token contract
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BountyEscrowInitialized {
    pub admin: Address,
    pub token: Address,
    pub timestamp: u64,
}

// ============================================================================
// Funds Locked Event
// ============================================================================

/// Event emitted when funds are locked in escrow for a bounty.
///
/// # Fields
/// * `bounty_id` - Unique identifier for the bounty
/// * `amount` - Amount of tokens locked (in stroops for XLM)
/// * `depositor` - Address that deposited the funds
/// * `deadline` - Unix timestamp after which refunds are allowed
///
/// # Event Topic
/// Symbol: `f_lock`
/// Indexed: `bounty_id` (allows filtering by specific bounty)
///
/// # State Transition
/// ```text
/// NONE → LOCKED
/// ```
///
/// # Usage
/// Emitted when a bounty creator locks funds for a task. The depositor
/// transfers tokens to the contract, which holds them until release or refund.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FundsLocked {
    pub bounty_id: u64,
    pub amount: i128,
    pub depositor: Address,
    pub deadline: u64,
}

// ============================================================================
// Funds Released Event
// ============================================================================

/// Event emitted when escrowed funds are released to a contributor.
///
/// # Fields
/// * `bounty_id` - The bounty identifier
/// * `amount` - Amount transferred to recipient
/// * `recipient` - Address receiving the funds (contributor)
/// * `timestamp` - Unix timestamp of release
/// * `remaining_amount` - Remaining amount in escrow after release
///
/// # Event Topic
/// Symbol: `f_rel`
/// Indexed: `bounty_id`
///
/// # State Transition
/// ```text
/// LOCKED → RELEASED (final state)
/// ```
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FundsReleased {
    pub bounty_id: u64,
    pub amount: i128,
    pub recipient: Address,
    pub timestamp: u64,
    pub remaining_amount: i128,
}

// ============================================================================
// Funds Refunded Event
// ============================================================================

/// Event emitted when escrowed funds are refunded to the depositor.
///
/// # Fields
/// * `bounty_id` - The bounty identifier
/// * `amount` - Amount refunded to depositor
/// * `refund_to` - Address receiving the refund (original depositor)
/// * `timestamp` - Unix timestamp of refund
/// * `refund_mode` - Type of refund (Full, Partial, Custom)
/// * `remaining_amount` - Remaining amount in escrow after refund
///
/// # Event Topic
/// Symbol: `f_ref`
/// Indexed: `bounty_id`
///
/// # State Transition
/// ```text
/// LOCKED → REFUNDED (final state)
/// ```
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FundsRefunded {
    pub bounty_id: u64,
    pub amount: i128,
    pub refund_to: Address,
    pub timestamp: u64,
    pub refund_mode: crate::RefundMode,
    pub remaining_amount: i128,
}

// ============================================================================
// Contract Pause Events
// ============================================================================

/// Event emitted when the contract is paused.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractPaused {
    pub paused_by: Address,
    pub timestamp: u64,
}

/// Event emitted when the contract is unpaused.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractUnpaused {
    pub unpaused_by: Address,
    pub timestamp: u64,
}

// ============================================================================
// Emergency Withdrawal Event
// ============================================================================

/// Event emitted during emergency withdrawal of all contract funds.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmergencyWithdrawal {
    pub withdrawn_by: Address,
    pub amount: i128,
    pub recipient: Address,
    pub timestamp: u64,
}

// ============================================================================
// Batch Operation Events
// ============================================================================

/// Event emitted for batch fund locking operations.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchFundsLocked {
    pub count: u32,
    pub total_amount: i128,
    pub timestamp: u64,
}

/// Event emitted for batch fund release operations.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchFundsReleased {
    pub count: u32,
    pub total_amount: i128,
    pub timestamp: u64,
}

// ============================================================================
// Release Schedule Events
// ============================================================================

/// Event emitted when a new release schedule is created.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduleCreated {
    pub bounty_id: u64,
    pub schedule_id: u32,
    pub amount: i128,
    pub timestamp: u64,
    pub created_by: Address,
    pub created_at: u64,
}

/// Event emitted when a release schedule is executed.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduleReleased {
    pub bounty_id: u64,
    pub schedule_id: u32,
    pub amount: i128,
    pub recipient: Address,
    pub executed_by: Address,
    pub executed_at: u64,
}

// ============================================================================
// Fee Events
// ============================================================================

/// Event emitted when fee configuration is updated.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeConfigUpdated {
    pub lock_fee_rate: i128,
    pub release_fee_rate: i128,
    pub fee_recipient: Address,
    pub fee_enabled: bool,
    pub timestamp: u64,
}

/// Event emitted when fees are collected.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeCollected {
    pub operation_type: FeeOperationType,
    pub amount: i128,
    pub fee_rate: i128,
    pub recipient: Address,
    pub timestamp: u64,
}

/// Enum representing fee operation types.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FeeOperationType {
    Lock,
    Release,
}

// ============================================================================
// Event Emission Functions
// ============================================================================

/// Emits a BountyEscrowInitialized event.
pub fn emit_bounty_initialized(env: &Env, event: BountyEscrowInitialized) {
    let topics = (symbol_short!("init"),);
    env.events().publish(topics, event);
}

/// Emits a FundsLocked event.
pub fn emit_funds_locked(env: &Env, event: FundsLocked) {
    let topics = (symbol_short!("f_lock"), event.bounty_id);
    env.events().publish(topics, event);
}

/// Emits a FundsReleased event.
pub fn emit_funds_released(env: &Env, event: FundsReleased) {
    let topics = (symbol_short!("f_rel"), event.bounty_id);
    env.events().publish(topics, event);
}

/// Emits a FundsRefunded event.
pub fn emit_funds_refunded(env: &Env, event: FundsRefunded) {
    let topics = (symbol_short!("f_ref"), event.bounty_id);
    env.events().publish(topics, event);
}

/// Emits a ContractPaused event.
pub fn emit_contract_paused(env: &Env, event: ContractPaused) {
    let topics = (symbol_short!("pause"),);
    env.events().publish(topics, event);
}

/// Emits a ContractUnpaused event.
pub fn emit_contract_unpaused(env: &Env, event: ContractUnpaused) {
    let topics = (symbol_short!("unpause"),);
    env.events().publish(topics, event);
}

/// Emits an EmergencyWithdrawal event.
pub fn emit_emergency_withdrawal(env: &Env, event: EmergencyWithdrawal) {
    let topics = (symbol_short!("ewith"),);
    env.events().publish(topics, event);
}

/// Emits a BatchFundsLocked event.
pub fn emit_batch_funds_locked(env: &Env, event: BatchFundsLocked) {
    let topics = (symbol_short!("b_lock"),);
    env.events().publish(topics, event);
}

/// Emits a BatchFundsReleased event.
pub fn emit_batch_funds_released(env: &Env, event: BatchFundsReleased) {
    let topics = (symbol_short!("b_rel"),);
    env.events().publish(topics, event);
}

/// Emits a ScheduleCreated event.
pub fn emit_schedule_created(env: &Env, event: ScheduleCreated) {
    let topics = (symbol_short!("sched_cre"), event.bounty_id, event.schedule_id);
    env.events().publish(topics, event);
}

/// Emits a ScheduleReleased event.
pub fn emit_schedule_released(env: &Env, event: ScheduleReleased) {
    let topics = (symbol_short!("sched_rel"), event.bounty_id, event.schedule_id);
    env.events().publish(topics, event);
}

/// Emits a FeeConfigUpdated event.
pub fn emit_fee_config_updated(env: &Env, event: FeeConfigUpdated) {
    let topics = (symbol_short!("fee_cfg"),);
    env.events().publish(topics, event);
}

/// Emits a FeeCollected event.
pub fn emit_fee_collected(env: &Env, event: FeeCollected) {
    let topics = (symbol_short!("fee_coll"),);
    env.events().publish(topics, event);
}