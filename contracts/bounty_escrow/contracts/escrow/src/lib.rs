//! # Bounty Escrow Smart Contract
//!
//! A trustless escrow system for bounty payments on the Stellar blockchain.
//! This contract enables secure fund locking, conditional release to contributors,
//! and automatic refunds after deadlines.
//!
//! ## Overview
//!
//! The Bounty Escrow contract manages the complete lifecycle of bounty payments:
//! 1. **Initialization**: Set up admin and token contract
//! 2. **Lock Funds**: Depositor locks tokens for a bounty with a deadline
//! 3. **Release**: Admin releases funds to contributor upon task completion
//! 4. **Refund**: Automatic refund to depositor if deadline passes
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                  Contract Architecture                       │
//! ├─────────────────────────────────────────────────────────────┤
//! │                                                              │
//! │  ┌──────────────┐                                           │
//! │  │  Depositor   │─────┐                                     │
//! │  └──────────────┘     │                                     │
//! │                       ├──> lock_funds()                     │
//! │  ┌──────────────┐     │         │                           │
//! │  │    Admin     │─────┘         ▼                           │
//! │  └──────────────┘          ┌─────────┐                      │
//! │         │                  │ ESCROW  │                      │
//! │         │                  │ LOCKED  │                      │
//! │         │                  └────┬────┘                      │
//! │         │                       │                           │
//! │         │        ┌──────────────┴───────────────┐           │
//! │         │        │                              │           │
//! │         ▼        ▼                              ▼           │
//! │   release_funds()                          refund()         │
//! │         │                                       │           │
//! │         ▼                                       ▼           │
//! │  ┌──────────────┐                      ┌──────────────┐    │
//! │  │ Contributor  │                      │  Depositor   │    │
//! │  └──────────────┘                      └──────────────┘    │
//! │    (RELEASED)                            (REFUNDED)        │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Security Model
//!
//! ### Trust Assumptions
//! - **Admin**: Trusted entity (backend service) authorized to release funds
//! - **Depositor**: Self-interested party; funds protected by deadline mechanism
//! - **Contributor**: Receives funds only after admin approval
//! - **Contract**: Trustless; operates according to programmed rules
//!
//! ### Key Security Features
//! 1. **Single Initialization**: Prevents admin takeover
//! 2. **Unique Bounty IDs**: No duplicate escrows
//! 3. **Authorization Checks**: All state changes require proper auth
//! 4. **Deadline Protection**: Prevents indefinite fund locking
//! 5. **State Machine**: Enforces valid state transitions
//! 6. **Atomic Operations**: Transfer + state update in single transaction
//!
//! ## Usage Example
//!
//! ```rust
//! use soroban_sdk::{Address, Env};
//!
//! // 1. Initialize contract (one-time setup)
//! let admin = Address::from_string("GADMIN...");
//! let token = Address::from_string("CUSDC...");
//! escrow_client.init(&admin, &token);
//!
//! // 2. Depositor locks 1000 USDC for bounty #42
//! let depositor = Address::from_string("GDEPOSIT...");
//! let amount = 1000_0000000; // 1000 USDC (7 decimals)
//! let deadline = current_timestamp + (30 * 24 * 60 * 60); // 30 days
//! escrow_client.lock_funds(&depositor, &42, &amount, &deadline);
//!
//! // 3a. Admin releases to contributor (happy path)
//! let contributor = Address::from_string("GCONTRIB...");
//! escrow_client.release_funds(&42, &contributor);
//!
//! // OR
//!
//! // 3b. Refund to depositor after deadline (timeout path)
//! // (Can be called by anyone after deadline passes)
//! escrow_client.refund(&42);
//! ```


    #![no_std]
mod events;
mod test_bounty_escrow;
#[cfg(test)]
mod test_query;

use events::{
    emit_batch_funds_locked, emit_batch_funds_released, emit_bounty_initialized,
    emit_contract_paused, emit_contract_unpaused, emit_emergency_withdrawal, emit_funds_locked,
    emit_funds_refunded, emit_funds_released, emit_schedule_created, emit_schedule_released,
    BatchFundsLocked, BatchFundsReleased, BountyEscrowInitialized, ContractPaused,
    ContractUnpaused, EmergencyWithdrawal, FundsLocked, FundsRefunded, FundsReleased,
    ScheduleCreated, ScheduleReleased,
};
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, token, vec, Address, Env,
    Vec,
};

// ==================== MONITORING MODULE ====================
#[allow(dead_code)]
mod monitoring {
    use soroban_sdk::{contracttype, symbol_short, Address, Env, String, Symbol};

    const OPERATION_COUNT: &str = "op_count";
    #[allow(dead_code)]
    const USER_COUNT: &str = "usr_count";
    const ERROR_COUNT: &str = "err_count";

    #[contracttype]
    #[derive(Clone, Debug)]
    pub struct OperationMetric {
        pub operation: Symbol,
        pub caller: Address,
        pub timestamp: u64,
        pub success: bool,
    }

    #[contracttype]
    #[derive(Clone, Debug)]
    pub struct PerformanceMetric {
        pub function: Symbol,
        pub duration: u64,
        pub timestamp: u64,
    }

    #[contracttype]
    #[derive(Clone, Debug)]
    pub struct HealthStatus {
        pub is_healthy: bool,
        pub last_operation: u64,
        pub total_operations: u64,
        pub contract_version: String,
    }

    #[contracttype]
    #[derive(Clone, Debug)]
    pub struct Analytics {
        pub operation_count: u64,
        pub unique_users: u64,
        pub error_count: u64,
        pub error_rate: u32,
    }

    #[contracttype]
    #[derive(Clone, Debug)]
    pub struct StateSnapshot {
        pub timestamp: u64,
        pub total_operations: u64,
        pub total_users: u64,
        pub total_errors: u64,
    }

    #[contracttype]
    #[derive(Clone, Debug)]
    pub struct PerformanceStats {
        pub function_name: Symbol,
        pub call_count: u64,
        pub total_time: u64,
        pub avg_time: u64,
        pub last_called: u64,
    }

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

    #[allow(dead_code)]
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

    #[allow(dead_code)]
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

    #[allow(dead_code)]
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

    #[allow(dead_code)]
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

// ==================== ANTI-ABUSE MODULE ====================
#[allow(dead_code)]
mod anti_abuse {
    use soroban_sdk::{contracttype, symbol_short, Address, Env};

    #[contracttype]
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct AntiAbuseConfig {
        pub window_size: u64,
        pub max_operations: u32,
        pub cooldown_period: u64,
    }

    #[contracttype]
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct AddressState {
        pub last_operation_timestamp: u64,
        pub window_start_timestamp: u64,
        pub operation_count: u32,
    }

    #[contracttype]
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub enum AntiAbuseKey {
        Config,
        State(Address),
        Whitelist(Address),
        Admin,
    }

    pub fn get_config(env: &Env) -> AntiAbuseConfig {
        env.storage()
            .instance()
            .get(&AntiAbuseKey::Config)
            .unwrap_or(AntiAbuseConfig {
                window_size: 3600,
                max_operations: 10,
                cooldown_period: 60,
            })
    }

    #[allow(dead_code)]
    pub fn set_config(env: &Env, config: AntiAbuseConfig) {
        env.storage().instance().set(&AntiAbuseKey::Config, &config);
    }

    pub fn is_whitelisted(env: &Env, address: Address) -> bool {
        env.storage()
            .instance()
            .has(&AntiAbuseKey::Whitelist(address))
    }

    #[allow(dead_code)]
    pub fn set_whitelist(env: &Env, address: Address, whitelisted: bool) {
        if whitelisted {
            env.storage()
                .instance()
                .set(&AntiAbuseKey::Whitelist(address), &true);
        } else {
            env.storage()
                .instance()
                .remove(&AntiAbuseKey::Whitelist(address));
        }
    }

    #[allow(dead_code)]
    pub fn get_admin(env: &Env) -> Option<Address> {
        env.storage().instance().get(&AntiAbuseKey::Admin)
    }

    #[allow(dead_code)]
    pub fn set_admin(env: &Env, admin: Address) {
        env.storage().instance().set(&AntiAbuseKey::Admin, &admin);
    }

    pub fn check_rate_limit(env: &Env, address: Address) {
        if is_whitelisted(env, address.clone()) {
            return;
        }

        let config = get_config(env);
        let now = env.ledger().timestamp();
        let key = AntiAbuseKey::State(address.clone());

        let mut state: AddressState =
            env.storage()
                .persistent()
                .get(&key)
                .unwrap_or(AddressState {
                    last_operation_timestamp: 0,
                    window_start_timestamp: now,
                    operation_count: 0,
                });

        if state.last_operation_timestamp > 0
            && now
                < state
                    .last_operation_timestamp
                    .saturating_add(config.cooldown_period)
        {
            env.events().publish(
                (symbol_short!("abuse"), symbol_short!("cooldown")),
                (address.clone(), now),
            );
            panic!("Operation in cooldown period");
        }

        if now
            >= state
                .window_start_timestamp
                .saturating_add(config.window_size)
        {
            state.window_start_timestamp = now;
            state.operation_count = 1;
        } else {
            if state.operation_count >= config.max_operations {
                env.events().publish(
                    (symbol_short!("abuse"), symbol_short!("limit")),
                    (address.clone(), now),
                );
                panic!("Rate limit exceeded");
            }
            state.operation_count += 1;
        }

        state.last_operation_timestamp = now;
        env.storage().persistent().set(&key, &state);
        env.storage().persistent().extend_ttl(&key, 17280, 17280);
    }
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    BountyExists = 3,
    BountyNotFound = 4,
    FundsNotLocked = 5,
    DeadlineNotPassed = 6,
    Unauthorized = 7,
    InvalidFeeRate = 8,
    FeeRecipientNotSet = 9,
    InvalidBatchSize = 10,
    ContractPaused = 11,
    DuplicateBountyId = 12,
    InvalidAmount = 13,
    InvalidDeadline = 14,
    InsufficientFunds = 16,
    RefundNotApproved = 17,
    BatchSizeMismatch = 18,
    ScheduleNotFound = 19,
    ScheduleNotReady = 20,
    ScheduleAlreadyReleased = 21,
    InvalidScheduleTimestamp = 22,
    TotalScheduleExceedsAmount = 23,
    ScheduleIndexOutOfBounds = 24,
    InvalidScheduleAmount = 25,
}

// ============================================================================
// Release Schedule Data Structures
// ============================================================================

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScheduleStatus {
    Pending,
    Released,
    Cancelled,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseSchedule {
    pub amount: i128,
    pub timestamp: u64,
    pub status: ScheduleStatus,
    pub schedule_id: u32,
    pub released_at: Option<u64>,
    pub released_by: Option<Address>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduleHistoryRecord {
    pub schedule_id: u32,
    pub amount: i128,
    pub timestamp: u64,
    pub status: ScheduleStatus,
    pub executed_at: Option<u64>,
    pub executed_by: Option<Address>,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EscrowStatus {
    Locked,
    Released,
    Refunded,
    PartiallyRefunded,
    PartiallyReleased,
    Scheduled, // New status for escrows with release schedules
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RefundMode {
    Full,
    Partial,
    Custom,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayoutRecord {
    pub amount: i128,
    pub recipient: Address,
    pub timestamp: u64,
    pub schedule_id: Option<u32>,
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

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Escrow {
    pub depositor: Address,
    pub amount: i128,
    pub status: EscrowStatus,
    pub deadline: u64,
    pub refund_history: Vec<RefundRecord>,
    pub payout_history: Vec<PayoutRecord>,
    pub remaining_amount: i128,
    pub release_schedules: Vec<ReleaseSchedule>,
    pub next_schedule_id: u32,
    pub schedule_history: Vec<ScheduleHistoryRecord>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LockFundsItem {
    pub bounty_id: u64,
    pub depositor: Address,
    pub amount: i128,
    pub deadline: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseFundsItem {
    pub bounty_id: u64,
    pub contributor: Address,
}

const MAX_BATCH_SIZE: u32 = 100;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeConfig {
    pub lock_fee_rate: i128,
    pub release_fee_rate: i128,
    pub fee_recipient: Address,
    pub fee_enabled: bool,
}

const BASIS_POINTS: i128 = 10_000;
const MAX_FEE_RATE: i128 = 1_000;

#[contracttype]
pub enum DataKey {
    Admin,
    Token,
    Escrow(u64),
    FeeConfig,
    RefundApproval(u64),
    ReentrancyGuard,
    IsPaused,
    BountyRegistry,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowFilter {
    pub status: Option<u32>,
    pub depositor: Option<Address>,
    pub min_amount: Option<i128>,
    pub max_amount: Option<i128>,
    pub start_time: Option<u64>,
    pub end_time: Option<u64>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pagination {
    pub start_index: u64,
    pub limit: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowStats {
    pub total_bounties: u64,
    pub total_locked_amount: i128,
    pub total_released_amount: i128,
    pub total_refunded_amount: i128,
    pub total_scheduled_amount: i128,
    pub pending_schedules: u32,
}

#[contract]
pub struct BountyEscrowContract;

#[contractimpl]
impl BountyEscrowContract {
    // ========================================================================
    // Initialization
    // ========================================================================

    pub fn init(env: Env, admin: Address, token: Address) -> Result<(), Error> {
        anti_abuse::check_rate_limit(&env, admin.clone());
        let start = env.ledger().timestamp();
        let caller = admin.clone();

        if env.storage().instance().has(&DataKey::Admin) {
            monitoring::track_operation(&env, symbol_short!("init"), caller, false);
            return Err(Error::AlreadyInitialized);
        }

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Token, &token);

        let fee_config = FeeConfig {
            lock_fee_rate: 0,
            release_fee_rate: 0,
            fee_recipient: admin.clone(),
            fee_enabled: false,
        };
        env.storage()
            .instance()
            .set(&DataKey::FeeConfig, &fee_config);

        emit_bounty_initialized(
            &env,
            BountyEscrowInitialized {
                admin: admin.clone(),
                token,
                timestamp: env.ledger().timestamp(),
            },
        );

        monitoring::track_operation(&env, symbol_short!("init"), caller, true);
        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("init"), duration);

        Ok(())
    }

    fn calculate_fee(amount: i128, fee_rate: i128) -> i128 {
        if fee_rate == 0 {
            return 0;
        }
        amount
            .checked_mul(fee_rate)
            .and_then(|x| x.checked_div(BASIS_POINTS))
            .unwrap_or(0)
    }

    fn get_fee_config_internal(env: &Env) -> FeeConfig {
        env.storage()
            .instance()
            .get(&DataKey::FeeConfig)
            .unwrap_or_else(|| FeeConfig {
                lock_fee_rate: 0,
                release_fee_rate: 0,
                fee_recipient: env.storage().instance().get(&DataKey::Admin).unwrap(),
                fee_enabled: false,
            })
    }

    pub fn update_fee_config(
        env: Env,
        lock_fee_rate: Option<i128>,
        release_fee_rate: Option<i128>,
        fee_recipient: Option<Address>,
        fee_enabled: Option<bool>,
    ) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let mut fee_config = Self::get_fee_config_internal(&env);

        if let Some(rate) = lock_fee_rate {
            if !(0..=MAX_FEE_RATE).contains(&rate) {
                return Err(Error::InvalidFeeRate);
            }
            fee_config.lock_fee_rate = rate;
        }

        if let Some(rate) = release_fee_rate {
            if !(0..=MAX_FEE_RATE).contains(&rate) {
                return Err(Error::InvalidFeeRate);
            }
            fee_config.release_fee_rate = rate;
        }

        if let Some(recipient) = fee_recipient {
            fee_config.fee_recipient = recipient;
        }

        if let Some(enabled) = fee_enabled {
            fee_config.fee_enabled = enabled;
        }

        env.storage()
            .instance()
            .set(&DataKey::FeeConfig, &fee_config);

        Ok(())
    }

    pub fn get_fee_config(env: Env) -> FeeConfig {
        Self::get_fee_config_internal(&env)
    }

    // ========================================================================
    // Release Schedule Functions
    // ========================================================================

    /// Create release schedules for a bounty (admin only)
    /// Supports multiple schedules for milestone-based payouts
    pub fn create_release_schedules(
        env: Env,
        bounty_id: u64,
        schedules: Vec<(i128, u64)>,
    ) -> Result<Vec<u32>, Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }

        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();

        if escrow.status != EscrowStatus::Locked && escrow.status != EscrowStatus::Scheduled {
            return Err(Error::FundsNotLocked);
        }

        let now = env.ledger().timestamp();
        let mut total_scheduled_amount: i128 = 0;
        let mut created_schedule_ids = Vec::new(&env);

        for (amount, timestamp) in schedules.iter() {
            if amount <= 0 {
                return Err(Error::InvalidScheduleAmount);
            }

            if timestamp <= now {
                return Err(Error::InvalidScheduleTimestamp);
            }

            total_scheduled_amount = total_scheduled_amount
                .checked_add(amount)
                .ok_or(Error::InvalidScheduleAmount)?;
        }

        let current_total_scheduled: i128 = escrow
            .release_schedules
            .iter()
            .filter(|s| s.status == ScheduleStatus::Pending)
            .map(|s| s.amount)
            .sum();

        if current_total_scheduled
            .checked_add(total_scheduled_amount)
            .ok_or(Error::InvalidScheduleAmount)?
            > escrow.remaining_amount
        {
            return Err(Error::TotalScheduleExceedsAmount);
        }

        for (amount, timestamp) in schedules.iter() {
            let schedule_id = escrow.next_schedule_id;
            escrow.next_schedule_id += 1;

            let schedule = ReleaseSchedule {
                amount: amount,
                timestamp: timestamp,
                status: ScheduleStatus::Pending,
                schedule_id,
                released_at: None,
                released_by: None,
            };

            escrow.release_schedules.push_back(schedule.clone());

            let history_record = ScheduleHistoryRecord {
                schedule_id,
                amount: amount,
                timestamp: timestamp,
                status: ScheduleStatus::Pending,
                executed_at: None,
                executed_by: None,
            };
            escrow.schedule_history.push_back(history_record);

            created_schedule_ids.push_back(schedule_id);

            emit_schedule_created(
                &env,
                ScheduleCreated {
                    bounty_id,
                    schedule_id,
                    amount: amount,
                    timestamp: timestamp,
                    created_by: admin.clone(),
                    created_at: now,
                },
            );
        }

        escrow.status = EscrowStatus::Scheduled;
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        Ok(created_schedule_ids)
    }

    /// Get all release schedules for a bounty
    pub fn get_release_schedules(env: Env, bounty_id: u64) -> Result<Vec<ReleaseSchedule>, Error> {
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }

        let escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();

        Ok(escrow.release_schedules)
    }

    /// Get pending release schedules (ready for execution)
    pub fn get_pending_schedules(env: Env, bounty_id: u64) -> Result<Vec<ReleaseSchedule>, Error> {
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }

        let escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();

        let mut pending = Vec::new(&env);
        let now = env.ledger().timestamp();

        for schedule in escrow.release_schedules.iter() {
            if schedule.status == ScheduleStatus::Pending && schedule.timestamp <= now {
                pending.push_back(schedule.clone());
            }
        }

        Ok(pending)
    }

    /// Execute a specific release schedule (can be called by anyone)
    /// This allows automatic execution when timestamp is reached
    pub fn execute_schedule(
        env: Env,
        bounty_id: u64,
        schedule_index: u32,
        recipient: Address,
    ) -> Result<(), Error> {
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }

        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();

        if escrow.status != EscrowStatus::Scheduled {
            return Err(Error::FundsNotLocked);
        }

        let schedule_count = escrow.release_schedules.len();
        if schedule_index >= schedule_count {
            return Err(Error::ScheduleIndexOutOfBounds);
        }

        let mut schedule = escrow.release_schedules.get(schedule_index).unwrap().clone();

        if schedule.status != ScheduleStatus::Pending {
            return Err(Error::ScheduleAlreadyReleased);
        }

        let now = env.ledger().timestamp();
        if schedule.timestamp > now {
            return Err(Error::ScheduleNotReady);
        }

        if schedule.amount > escrow.remaining_amount {
            return Err(Error::InsufficientFunds);
        }

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);

        let fee_config = Self::get_fee_config_internal(&env);
        let fee_amount = if fee_config.fee_enabled && fee_config.release_fee_rate > 0 {
            Self::calculate_fee(schedule.amount, fee_config.release_fee_rate)
        } else {
            0
        };
        let net_amount = schedule.amount - fee_amount;

        client.transfer(&env.current_contract_address(), &recipient, &net_amount);

        if fee_amount > 0 {
            client.transfer(
                &env.current_contract_address(),
                &fee_config.fee_recipient,
                &fee_amount,
            );
        }

        escrow.remaining_amount -= schedule.amount;

        let payout_record = PayoutRecord {
            amount: schedule.amount,
            recipient: recipient.clone(),
            timestamp: now,
            schedule_id: Some(schedule.schedule_id),
        };
        escrow.payout_history.push_back(payout_record);

        schedule.status = ScheduleStatus::Released;
        schedule.released_at = Some(now);
        schedule.released_by = Some(recipient.clone());

        escrow.release_schedules.set(schedule_index, schedule.clone());

        for i in 0..escrow.schedule_history.len() {
            let mut record = escrow.schedule_history.get(i).unwrap().clone();
            if record.schedule_id == schedule.schedule_id {
                record.status = ScheduleStatus::Released;
                record.executed_at = Some(now);
                record.executed_by = Some(recipient.clone());
                escrow.schedule_history.set(i, record);
                break;
            }
        }

        let has_pending_schedules = escrow
            .release_schedules
            .iter()
            .any(|s| s.status == ScheduleStatus::Pending);

        if !has_pending_schedules && escrow.remaining_amount == 0 {
            escrow.status = EscrowStatus::Released;
        }

        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        let caller = env.current_contract_address();

        emit_schedule_released(
            &env,
            ScheduleReleased {
                bounty_id,
                schedule_id: schedule.schedule_id,
                amount: schedule.amount,
                recipient: recipient.clone(),
                executed_by: caller.clone(),
                executed_at: now,
            },
        );

        monitoring::track_operation(&env, symbol_short!("exec_sch"), caller, true);

        Ok(())
    }

    /// Execute all ready release schedules in batch
    pub fn execute_all_ready_schedules(
        env: Env,
        bounty_id: u64,
        recipient: Address,
    ) -> Result<u32, Error> {
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }

        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();

        if escrow.status != EscrowStatus::Scheduled {
            return Err(Error::FundsNotLocked);
        }

        let now = env.ledger().timestamp();
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);
        let fee_config = Self::get_fee_config_internal(&env);

        let mut executed_count = 0u32;

        for i in 0..escrow.release_schedules.len() {
            let mut schedule = escrow.release_schedules.get(i).unwrap().clone();

            if schedule.status == ScheduleStatus::Pending && schedule.timestamp <= now {
                if schedule.amount > escrow.remaining_amount {
                    continue;
                }

                let fee_amount = if fee_config.fee_enabled && fee_config.release_fee_rate > 0 {
                    Self::calculate_fee(schedule.amount, fee_config.release_fee_rate)
                } else {
                    0
                };
                let net_amount = schedule.amount - fee_amount;

                client.transfer(&env.current_contract_address(), &recipient, &net_amount);

                if fee_amount > 0 {
                    client.transfer(
                        &env.current_contract_address(),
                        &fee_config.fee_recipient,
                        &fee_amount,
                    );
                }

                escrow.remaining_amount -= schedule.amount;

                let payout_record = PayoutRecord {
                    amount: schedule.amount,
                    recipient: recipient.clone(),
                    timestamp: now,
                    schedule_id: Some(schedule.schedule_id),
                };
                escrow.payout_history.push_back(payout_record);

                schedule.status = ScheduleStatus::Released;
                schedule.released_at = Some(now);
                schedule.released_by = Some(recipient.clone());

                escrow.release_schedules.set(i, schedule.clone());

                for j in 0..escrow.schedule_history.len() {
                    let mut record = escrow.schedule_history.get(j).unwrap().clone();
                    if record.schedule_id == schedule.schedule_id {
                        record.status = ScheduleStatus::Released;
                        record.executed_at = Some(now);
                        record.executed_by = Some(recipient.clone());
                        escrow.schedule_history.set(j, record);
                        break;
                    }
                }

                executed_count += 1;

                emit_schedule_released(
                    &env,
                    ScheduleReleased {
                        bounty_id,
                        schedule_id: schedule.schedule_id,
                        amount: schedule.amount,
                        recipient: recipient.clone(),
                        executed_by: env.current_contract_address(),
                        executed_at: now,
                    },
                );
            }
        }

        let has_pending_schedules = escrow
            .release_schedules
            .iter()
            .any(|s| s.status == ScheduleStatus::Pending);

        if !has_pending_schedules && escrow.remaining_amount == 0 {
            escrow.status = EscrowStatus::Released;
        }

        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        monitoring::track_operation(
            &env,
            symbol_short!("exec_all"),
            env.current_contract_address(),
            true,
        );

        Ok(executed_count)
    }

    /// Cancel a pending release schedule (admin only)
    pub fn cancel_schedule(env: Env, bounty_id: u64, schedule_index: u32) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::NotInitialized);
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }

        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();

        let schedule_count = escrow.release_schedules.len();
        if schedule_index >= schedule_count {
            return Err(Error::ScheduleIndexOutOfBounds);
        }

        let mut schedule = escrow.release_schedules.get(schedule_index).unwrap().clone();

        if schedule.status != ScheduleStatus::Pending {
            return Err(Error::ScheduleAlreadyReleased);
        }

        schedule.status = ScheduleStatus::Cancelled;
        escrow.release_schedules.set(schedule_index, schedule.clone());

        for i in 0..escrow.schedule_history.len() {
            let mut record = escrow.schedule_history.get(i).unwrap().clone();
            if record.schedule_id == schedule.schedule_id {
                record.status = ScheduleStatus::Cancelled;
                escrow.schedule_history.set(i, record);
                break;
            }
        }

        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        Ok(())
    }

    /// Get schedule history for a bounty
    pub fn get_schedule_history(
        env: Env,
        bounty_id: u64,
    ) -> Result<Vec<ScheduleHistoryRecord>, Error> {
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }

        let escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();

        Ok(escrow.schedule_history)
    }

    // ========================================================================
    // Modified Existing Functions for Schedule Support
    // ========================================================================

    pub fn lock_funds(
        env: Env,
        depositor: Address,
        bounty_id: u64,
        amount: i128,
        deadline: u64,
    ) -> Result<(), Error> {
        anti_abuse::check_rate_limit(&env, depositor.clone());
        let start = env.ledger().timestamp();
        let caller = depositor.clone();

        if Self::is_paused_internal(&env) {
            monitoring::track_operation(&env, symbol_short!("lock"), caller, false);
            return Err(Error::ContractPaused);
        }

        depositor.require_auth();

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

        if env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            monitoring::track_operation(&env, symbol_short!("lock"), caller, false);
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::BountyExists);
        }

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);

        let fee_config = Self::get_fee_config_internal(&env);
        let fee_amount = if fee_config.fee_enabled && fee_config.lock_fee_rate > 0 {
            Self::calculate_fee(amount, fee_config.lock_fee_rate)
        } else {
            0
        };
        let net_amount = amount - fee_amount;

        client.transfer(&depositor, &env.current_contract_address(), &net_amount);

        if fee_amount > 0 {
            client.transfer(&depositor, &fee_config.fee_recipient, &fee_amount);
        }

        let escrow = Escrow {
            depositor: depositor.clone(),
            amount: net_amount,
            status: EscrowStatus::Locked,
            deadline,
            refund_history: vec![&env],
            payout_history: vec![&env],
            remaining_amount: amount,
            release_schedules: vec![&env],
            next_schedule_id: 0,
            schedule_history: vec![&env],
        };

        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        let mut registry: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::BountyRegistry)
            .unwrap_or(vec![&env]);
        registry.push_back(bounty_id);
        env.storage()
            .instance()
            .set(&DataKey::BountyRegistry, &registry);

        emit_funds_locked(
            &env,
            FundsLocked {
                bounty_id,
                amount: net_amount,
                depositor: depositor.clone(),
                deadline,
            },
        );

        env.storage().instance().remove(&DataKey::ReentrancyGuard);

        monitoring::track_operation(&env, symbol_short!("lock"), caller, true);
        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("lock"), duration);

        Ok(())
    }

    // ========================================================================
// Pause and Emergency Functions
// ========================================================================

/// Check if contract is paused (internal helper)
fn is_paused_internal(env: &Env) -> bool {
    env.storage()
        .persistent()
        .get::<_, bool>(&DataKey::IsPaused)
        .unwrap_or(false)
}

/// Get pause status (view function)
pub fn is_paused(env: Env) -> bool {
    Self::is_paused_internal(&env)
}

/// Pause the contract (admin only)
/// Prevents new fund locks, releases, and refunds
pub fn pause(env: Env) -> Result<(), Error> {
    if !env.storage().instance().has(&DataKey::Admin) {
        return Err(Error::NotInitialized);
    }

    let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    admin.require_auth();

    if Self::is_paused_internal(&env) {
        return Ok(()); // Already paused, idempotent
    }

    env.storage().persistent().set(&DataKey::IsPaused, &true);

    events::emit_contract_paused(
        &env,
        events::ContractPaused {
            paused_by: admin.clone(),
            timestamp: env.ledger().timestamp(),
        },
    );

    Ok(())
}

/// Unpause the contract (admin only)
/// Resumes normal operations
pub fn unpause(env: Env) -> Result<(), Error> {
    if !env.storage().instance().has(&DataKey::Admin) {
        return Err(Error::NotInitialized);
    }

    let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    admin.require_auth();

    if !Self::is_paused_internal(&env) {
        return Ok(()); // Already unpaused, idempotent
    }

    env.storage().persistent().set(&DataKey::IsPaused, &false);

    events::emit_contract_unpaused(
        &env,
        events::ContractUnpaused {
            unpaused_by: admin.clone(),
            timestamp: env.ledger().timestamp(),
        },
    );

    Ok(())
}
    pub fn release_funds(
        env: Env,
        bounty_id: u64,
        contributor: Address,
        amount: Option<i128>,
    ) -> Result<(), Error> {
        let start = env.ledger().timestamp();

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

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();

        if Self::is_paused_internal(&env) {
            monitoring::track_operation(&env, symbol_short!("release"), admin.clone(), false);
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::ContractPaused);
        }

        anti_abuse::check_rate_limit(&env, admin.clone());
        admin.require_auth();

        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            monitoring::track_operation(&env, symbol_short!("release"), admin.clone(), false);
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::BountyNotFound);
        }

        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();

        if escrow.status != EscrowStatus::Locked && escrow.status != EscrowStatus::PartiallyReleased
        {
            monitoring::track_operation(&env, symbol_short!("release"), admin.clone(), false);
            env.storage().instance().remove(&DataKey::ReentrancyGuard);
            return Err(Error::FundsNotLocked);
        }

        let payout_amount = match amount {
            Some(amt) => {
                if amt <= 0 {
                    monitoring::track_operation(
                        &env,
                        symbol_short!("release"),
                        admin.clone(),
                        false,
                    );
                    env.storage().instance().remove(&DataKey::ReentrancyGuard);
                    return Err(Error::InvalidAmount);
                }
                if amt > escrow.remaining_amount {
                    monitoring::track_operation(
                        &env,
                        symbol_short!("release"),
                        admin.clone(),
                        false,
                    );
                    env.storage().instance().remove(&DataKey::ReentrancyGuard);
                    return Err(Error::InvalidAmount);
                }
                amt
            }
            None => escrow.remaining_amount,
        };

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);

        let fee_config = Self::get_fee_config_internal(&env);
        let fee_amount = if fee_config.fee_enabled && fee_config.release_fee_rate > 0 {
            Self::calculate_fee(payout_amount, fee_config.release_fee_rate)
        } else {
            0
        };
        let net_amount = payout_amount - fee_amount;

        let contract_balance = client.balance(&env.current_contract_address());
        if contract_balance < net_amount + fee_amount {
            return Err(Error::InsufficientFunds);
        }

        client.transfer(&env.current_contract_address(), &contributor, &net_amount);

        if fee_amount > 0 {
            client.transfer(
                &env.current_contract_address(),
                &fee_config.fee_recipient,
                &fee_amount,
            );
        }

        escrow.remaining_amount -= payout_amount;

        let payout_record = PayoutRecord {
            amount: payout_amount,
            recipient: contributor.clone(),
            timestamp: env.ledger().timestamp(),
            schedule_id: None,
        };
        escrow.payout_history.push_back(payout_record);

        if escrow.remaining_amount == 0 {
            escrow.status = EscrowStatus::Released;
        } else {
            escrow.status = EscrowStatus::PartiallyReleased;
        }

        env.storage()
            .persistent()
            .set(&DataKey::Escrow(bounty_id), &escrow);

        emit_funds_released(
            &env,
            FundsReleased {
                bounty_id,
                amount: net_amount,
                recipient: contributor.clone(),
                timestamp: env.ledger().timestamp(),
                remaining_amount: escrow.remaining_amount,
            },
        );

        env.storage().instance().remove(&DataKey::ReentrancyGuard);

        monitoring::track_operation(&env, symbol_short!("release"), admin, true);
        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("release"), duration);
        Ok(())
    }

    
    // ========================================================================
    // View Functions
    // ========================================================================

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

    pub fn get_balance(env: Env) -> Result<i128, Error> {
        if !env.storage().instance().has(&DataKey::Token) {
            return Err(Error::NotInitialized);
        }
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let client = token::Client::new(&env, &token_addr);
        Ok(client.balance(&env.current_contract_address()))
    }

    pub fn get_refund_history(env: Env, bounty_id: u64) -> Result<Vec<RefundRecord>, Error> {
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }
        let escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();
        Ok(escrow.refund_history)
    }

    pub fn get_payout_history(env: Env, bounty_id: u64) -> Result<Vec<PayoutRecord>, Error> {
        if !env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
            return Err(Error::BountyNotFound);
        }
        let escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(bounty_id))
            .unwrap();
        Ok(escrow.payout_history)
    }

    pub fn get_stats(env: Env) -> EscrowStats {
        let registry: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::BountyRegistry)
            .unwrap_or(vec![&env]);

        let mut total_locked: i128 = 0;
        let mut total_released: i128 = 0;
        let mut total_refunded: i128 = 0;
        let mut total_scheduled: i128 = 0;
        let mut pending_schedules: u32 = 0;

        for i in 0..registry.len() {
            let bounty_id = registry.get(i).unwrap();
            if env.storage().persistent().has(&DataKey::Escrow(bounty_id)) {
                let escrow: Escrow = env
                    .storage()
                    .persistent()
                    .get(&DataKey::Escrow(bounty_id))
                    .unwrap();

                match escrow.status {
                    EscrowStatus::Locked => {
                        total_locked += escrow.remaining_amount;
                    }
                    EscrowStatus::Released => {
                        total_released += escrow.amount;
                    }
                    EscrowStatus::Refunded => {
                        for record in escrow.refund_history.iter() {
                            total_refunded += record.amount;
                        }
                    }
                    EscrowStatus::PartiallyRefunded => {
                        total_locked += escrow.remaining_amount;
                        for record in escrow.refund_history.iter() {
                            total_refunded += record.amount;
                        }
                    }
                    EscrowStatus::PartiallyReleased => {
                        for record in escrow.payout_history.iter() {
                            total_released += record.amount;
                        }
                        total_locked += escrow.remaining_amount;
                    }
                    EscrowStatus::Scheduled => {
                        total_locked += escrow.remaining_amount;
                        for schedule in escrow.release_schedules.iter() {
                            if schedule.status == ScheduleStatus::Pending {
                                total_scheduled += schedule.amount;
                                pending_schedules += 1;
                            }
                        }
                    }
                }
            }
        }

        EscrowStats {
            total_bounties: registry.len() as u64,
            total_locked_amount: total_locked,
            total_released_amount: total_released,
            total_refunded_amount: total_refunded,
            total_scheduled_amount: total_scheduled,
            pending_schedules,
        }
    }
}

#[cfg(test)]
mod test;
