use soroban_sdk::{contracttype, symbol_short, Address, Env, String, Vec};

#[contracttype]
#[derive(Clone, Debug)]
pub struct ProgramInitializedEvent {
    pub admin: Address,
    pub program_id: String,
    pub total_funds: i128,
    pub timestamp: u64,
}

pub fn emit_program_initialized(env: &Env, event: ProgramInitializedEvent) {
    let topics = (symbol_short!("p_init"),);
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct FundsLockedEvent {
    pub program_id: String,
    pub amount: i128,
    pub remaining_balance: i128,
    pub timestamp: u64,
}

pub fn emit_funds_locked(env: &Env, event: FundsLockedEvent) {
    let topics = (symbol_short!("f_lock"),);
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct BatchPayoutEvent {
    pub program_id: String,
    pub amounts: Vec<i128>,
    pub recipients: Vec<Address>,
    pub timestamp: u64,
}

pub fn emit_batch_payout(env: &Env, event: BatchPayoutEvent) {
    let topics = (symbol_short!("b_pay"),);
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct PayoutEvent {
    pub program_id: String,
    pub amount: i128,
    pub recipient: Address,
    pub timestamp: u64,
}

pub fn emit_payout(env: &Env, event: PayoutEvent) {
    let topics = (symbol_short!("payout"),);
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct UpdateAdminEvent {
    pub admin: Address,
    pub new_admin: Address,
    pub timestamp: u64,
}

pub fn emit_update_admin(env: &Env, event: UpdateAdminEvent) {
    let topics = (symbol_short!("upd_adm"),);
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct UpdateAuthorizedKeyEvent {
    pub old_authorized_payout_key: Address,
    pub new_authorized_payout_key: Address,
    pub timestamp: u64,
}

pub fn emit_update_authorized_key(env: &Env, event: UpdateAuthorizedKeyEvent) {
    let topics = (symbol_short!("upd_apk"),);
    env.events().publish(topics, event.clone());
}
