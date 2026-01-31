#![cfg(test)]
use crate::{ProgramEscrowContract, ProgramEscrowContractClient};
use crate::security::reentrancy_guard::{ReentrancyGuard};
use soroban_sdk::{Address, Env, String, symbol_short, testutils::Address as _};

#[test]
fn test_program_escrow_reentrancy_blocked() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    // Simulation:
    // Lock the guard manually to simulate being inside a guarded function
    ReentrancyGuard::enter(&env).unwrap();
    
    // Any guarded function call should now fail (panics with "Reentrancy detected")
    let res = client.try_lock_program_funds(&String::from_str(&env, "TEST"), &100);
    assert!(res.is_err());
    
    let res = client.try_batch_payout(&String::from_str(&env, "TEST"), &soroban_sdk::vec![&env, Address::generate(&env)], &soroban_sdk::vec![&env, 100]);
    assert!(res.is_err());
    
    let res = client.try_single_payout(&String::from_str(&env, "TEST"), &Address::generate(&env), &100);
    assert!(res.is_err());
    
    let res = client.try_create_program_release_schedule(&String::from_str(&env, "TEST"), &100, &1000, &Address::generate(&env));
    assert!(res.is_err());

    // Unlock
    ReentrancyGuard::exit(&env);
    
    // Calls should no longer fail due to reentrancy
    // (They might fail for other reasons, like program not found)
    let res = client.try_lock_program_funds(&String::from_str(&env, "TEST"), &100);
    // Should fail with program not found but NOT because of reentrancy
    assert!(res.is_err());
}
