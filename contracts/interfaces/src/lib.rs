#![no_std]
use soroban_sdk::{contracttype, Address, Env, IntoVal, Symbol};

/// Trait for locking funds in an escrow.
pub trait EscrowLock {
    /// Locks funds for a specific bounty or program.
    fn lock_funds(env: Env, depositor: Address, id: u64, amount: i128, deadline: u64);
}

/// Trait for release and refund operations.
pub trait EscrowRelease {
    /// Releases funds to a recipient.
    fn release_funds(env: Env, id: u64, recipient: Address);

    /// Refunds funds to the depositor.
    fn refund(
        env: Env,
        id: u64,
        amount: Option<i128>,
        recipient: Option<Address>,
        mode: RefundMode,
    );

    /// Gets the current balance for an escrow ID.
    fn get_balance(env: Env, id: u64) -> i128;
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
pub struct FeeConfig {
    pub lock_fee_rate: i128, // Basis points (1 = 0.01%)
    pub payout_fee_rate: i128,
    pub fee_recipient: Address,
    pub fee_enabled: bool,
}

/// Trait for managing contract fees.
pub trait ConfigurableFee {
    /// Updates the fee configuration.
    fn set_fee_config(env: Env, config: FeeConfig);

    /// Gets the current fee configuration.
    fn get_fee_config(env: Env) -> FeeConfig;
}

/// Trait for pausing and unpausing contract operations.
pub trait Pausable {
    /// Pauses the contract.
    fn pause(env: Env);

    /// Unpauses the contract.
    fn unpause(env: Env);

    /// Checks if the contract is paused.
    fn is_paused(env: Env) -> bool;
}

/// Helper client for interacting with Escrow contracts from within other contracts.
pub struct EscrowClient {
    pub env: Env,
    pub address: Address,
}

impl EscrowClient {
    pub fn new(env: &Env, address: &Address) -> Self {
        Self {
            env: env.clone(),
            address: address.clone(),
        }
    }

    pub fn lock_funds(&self, depositor: Address, id: u64, amount: i128, deadline: u64) {
        self.env.invoke_contract::<()>(
            &self.address,
            &Symbol::new(&self.env, "lock_funds"),
            (depositor, id, amount, deadline).into_val(&self.env),
        );
    }

    pub fn release_funds(&self, id: u64, recipient: Address) {
        self.env.invoke_contract::<()>(
            &self.address,
            &Symbol::new(&self.env, "release_funds"),
            (id, recipient).into_val(&self.env),
        );
    }

    pub fn refund(
        &self,
        id: u64,
        amount: Option<i128>,
        recipient: Option<Address>,
        mode: RefundMode,
    ) {
        self.env.invoke_contract::<()>(
            &self.address,
            &Symbol::new(&self.env, "refund"),
            (id, amount, recipient, mode).into_val(&self.env),
        );
    }

    pub fn get_balance(&self, id: u64) -> i128 {
        self.env.invoke_contract::<i128>(
            &self.address,
            &Symbol::new(&self.env, "get_balance"),
            (id,).into_val(&self.env),
        )
    }
}

/// Helper client for interacting with configurable fee contracts.
pub struct FeeClient {
    pub env: Env,
    pub address: Address,
}

impl FeeClient {
    pub fn new(env: &Env, address: &Address) -> Self {
        Self {
            env: env.clone(),
            address: address.clone(),
        }
    }

    pub fn get_config(&self) -> FeeConfig {
        self.env.invoke_contract::<FeeConfig>(
            &self.address,
            &Symbol::new(&self.env, "get_fee_config"),
            ().into_val(&self.env),
        )
    }

    pub fn set_config(&self, config: FeeConfig) {
        self.env.invoke_contract::<()>(
            &self.address,
            &Symbol::new(&self.env, "set_fee_config"),
            (config,).into_val(&self.env),
        );
    }
}
