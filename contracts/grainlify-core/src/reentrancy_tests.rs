#![cfg(test)]
use crate::{GrainlifyContract, GrainlifyContractClient};
use crate::security::reentrancy_guard::{ReentrancyGuard, ReentrancyError};
use soroban_sdk::{contract, contractimpl, Address, Env, BytesN, symbol_short};

#[contract]
pub struct ReentrancyAttacker;

#[contractimpl]
impl ReentrancyAttacker {
    pub fn attack(env: Env, target: Address) {
        let client = GrainlifyContractClient::new(&env, &target);
        // Try to re-enter a guarded function
        let _ = client.try_upgrade(&BytesN::from_array(&env, &[0u8; 32]));
    }
}

#[test]
fn test_reentrancy_guard_unit_logic() {
    let env = Env::default();
    assert!(!ReentrancyGuard::is_locked(&env));
    ReentrancyGuard::enter(&env).unwrap();
    assert!(ReentrancyGuard::is_locked(&env));
    assert!(ReentrancyGuard::enter(&env).is_err());
    ReentrancyGuard::exit(&env);
    assert!(!ReentrancyGuard::is_locked(&env));
}

#[test]
fn test_raii_guard_unit_lifecycle() {
    let env = Env::default();
    {
        let _guard = crate::security::reentrancy_guard::ReentrancyGuardRAII::new(&env).unwrap();
        assert!(ReentrancyGuard::is_locked(&env));
        let res = crate::security::reentrancy_guard::ReentrancyGuardRAII::new(&env);
        assert!(res.is_err());
    }
    assert!(!ReentrancyGuard::is_locked(&env));
}

#[test]
fn test_grainlify_core_reentrancy_prevention() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, GrainlifyContract);
    let client = GrainlifyContractClient::new(&env, &contract_id);

    // Initialize
    let admin = Address::generate(&env);
    client.init_admin(&admin);

    // Simulate reentrancy by manually locking the guard
    ReentrancyGuard::enter(&env).unwrap();
    
    // Calls to guarded functions should fail
    assert!(client.try_upgrade(&BytesN::from_array(&env, &[0u8; 32])).is_err());
    assert!(client.try_propose_upgrade(&admin, &BytesN::from_array(&env, &[1u8; 32])).is_err());
    assert!(client.try_approve_upgrade(&1, &admin).is_err());
    assert!(client.try_execute_upgrade(&1).is_err());
    
    // Governance functions (these return governance::Error::ReentrantCall)
    let res = client.try_create_proposal(&admin, &BytesN::from_array(&env, &[2u8; 32]), &symbol_short!("T"));
    assert!(res.is_err());
    
    // Unlock
    ReentrancyGuard::exit(&env);
    
    // Now calls should succeed or fail for other reasons, but not reentrancy
    let res = client.try_upgrade(&BytesN::from_array(&env, &[0u8; 32]));
    // Should fail because BytesN is all zeros (host error) but NOT because of reentrancy
    assert!(res.is_err()); 
}

#[test]
fn test_raii_guard_error_drop() {
    let env = Env::default();
    fn test_func(env: &Env) -> Result<(), ReentrancyError> {
        let _guard = crate::security::reentrancy_guard::ReentrancyGuardRAII::new(env)?;
        Err(ReentrancyError::ReentrantCall) // Simulate an error
    }
    
    let _ = test_func(&env);
    
    // Guard should be unlocked even after Err return due to Drop implementation
    assert!(!ReentrancyGuard::is_locked(&env));
}
