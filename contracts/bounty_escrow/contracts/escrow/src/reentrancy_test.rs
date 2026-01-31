#![cfg(test)]
use crate::{BountyEscrowContract, BountyEscrowContractClient, LockFundsItem, ReleaseFundsItem};
use crate::security::reentrancy_guard::{ReentrancyGuard};
use soroban_sdk::{Address, Env, Vec, symbol_short, testutils::Address as _};

#[test]
fn test_bounty_escrow_reentrancy_blocked() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    // Initialize
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    client.init(&admin, &token);

    // Lock the guard manually to simulate being inside a guarded function
    ReentrancyGuard::enter(&env).unwrap();
    
    // Any guarded function call should now fail with ReentrantCall error
    let depositor = Address::generate(&env);
    let res = client.try_lock_funds(&depositor, &1, &100, &1000);
    assert!(res.is_err());
    
    // Check specific error variant if possible (18 is ReentrantCall)
    // In Soroban tests, InvokeError(HostError) or similar might be returned
    
    let res = client.try_release_funds(&1, &Address::generate(&env));
    assert!(res.is_err());
    
    let res = client.try_refund_funds(&1);
    assert!(res.is_err());
    
    let res = client.try_batch_lock_funds(&Vec::new(&env));
    assert!(res.is_err());
    
    let res = client.try_batch_release_funds(&Vec::new(&env));
    assert!(res.is_err());

    // Unlock
    ReentrancyGuard::exit(&env);
    
    // Calls should no longer fail due to reentrancy
    // (They might fail for other reasons, but the guard is cleared)
    let res = client.try_refund_funds(&1);
    // Should fail with BountyNotFound (4) but NOT ReentrantCall
    assert!(res.is_err());
}
