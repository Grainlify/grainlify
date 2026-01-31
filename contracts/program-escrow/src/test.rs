use crate::{ProgramEscrowContract, ProgramEscrowContractClient};
use soroban_sdk::{testutils::Address as _, token, Address, Env, String, Vec, vec, symbol_short};
use proptest::prelude::*;

// Helper to create token
fn create_token_contract<'a>(env: &Env, admin: &Address) -> token::Client<'a> {
    let token_address = env.register_stellar_asset_contract(admin.clone());
    token::Client::new(env, &token_address)
}

// Helper to setup program
fn setup_program(env: &Env) -> (ProgramEscrowContractClient<'static>, Address, Address, String) {
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    let token_admin = Address::generate(env);
    let token_client = create_token_contract(env, &token_admin);
    let program_id = String::from_str(env, "hackathon-2024");
    
    client.initialize_program(&program_id, &admin, &token_client.address);
    (client, admin, token_client.address, program_id)
}

// Helper with funds
fn setup_program_with_funds(env: &Env, initial_amount: i128) -> (ProgramEscrowContractClient<'static>, Address, Address, String) {
    let (client, admin, token, program_id) = setup_program(env);
    
    // The contract requires funds to be transferred to it BEFORE locking.
    // In tests, we can mint directly to the contract address.
    let token_client = token::StellarAssetClient::new(env, &token);
    token_client.mint(&client.address, &initial_amount);
    
    client.lock_program_funds(&program_id, &initial_amount);
    (client, admin, token, program_id)
}

#[test]
fn test_lock_program_funds_max_amount() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _, _, program_id) = setup_program(&env);
    
    // Test with large amount
    let max_amount = i128::MAX;
    // We don't necessarily need actual tokens for this state check if contract doesn't check balance
    // But good practice to simulate reality if it did. 
    // Minting i128::MAX might fail in token contract? 
    // Let's just test state update for now.
    client.lock_program_funds(&program_id, &max_amount);
    
    let info = client.get_program_info(&program_id);
    assert_eq!(info.total_funds, max_amount);
    assert_eq!(info.remaining_balance, max_amount);
}

#[test]
fn test_batch_payout_max_chunk() {
    let env = Env::default();
    env.mock_all_auths();
    
    // Using a smaller initial amount to allow passing in i128
    let initial = 1_000_000_000_000i128;
    let (client, admin, _, program_id) = setup_program_with_funds(&env, initial);
    
    // 50 recipients
    let mut recipients = Vec::new(&env);
    let mut amounts = Vec::new(&env);
    let payout_amt = 1_000_000_000i128;
    
    for _ in 0..50 {
        recipients.push_back(Address::generate(&env));
        amounts.push_back(payout_amt);
    }
    
    client.batch_payout(&program_id, &recipients, &amounts);
    
    let info = client.get_program_info(&program_id);
    assert_eq!(info.remaining_balance, initial - (payout_amt * 50));
}

#[test]
#[should_panic(expected = "Amount must be greater than zero")]
fn test_zero_value_payout() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _, _, program_id) = setup_program_with_funds(&env, 1000);
    let recipient = Address::generate(&env);
    
    client.single_payout(&program_id, &recipient, &0);
}

#[test]
fn test_integration_complex_flow() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, token, program_id) = setup_program(&env);
    let token_client = token::StellarAssetClient::new(&env, &token);
    
    // Top up 1
    token_client.mint(&client.address, &1000);
    client.lock_program_funds(&program_id, &1000);
    assert_eq!(client.get_remaining_balance(&program_id), 1000);
    
    // Top up 2
    token_client.mint(&client.address, &500);
    client.lock_program_funds(&program_id, &500);
    assert_eq!(client.get_remaining_balance(&program_id), 1500);
    
    // Single Payout
    let r1 = Address::generate(&env);
    client.single_payout(&program_id, &r1, &300);
    assert_eq!(client.get_remaining_balance(&program_id), 1200);
    
    // Batch Payout
    let r2 = Address::generate(&env);
    let r3 = Address::generate(&env);
    let recipients = vec![&env, r2, r3];
    let amounts = vec![&env, 200, 100];
    
    client.batch_payout(&program_id, &recipients, &amounts);
    assert_eq!(client.get_remaining_balance(&program_id), 900);
}

#[test]
fn test_monitoring_functions() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    
    // Test health check
    let health = client.health_check();
    assert!(health.is_healthy);
    assert_eq!(health.contract_version, String::from_str(&env, "1.0.0"));
    
    // Generate some activity
    let backend = Address::generate(&env);
    let token = Address::generate(&env);
    let prog_id = String::from_str(&env, "MonitoredProg");
    
    client.initialize_program(&prog_id, &backend, &token);
    
    // Test analytics
    let analytics = client.get_analytics();
    assert!(analytics.operation_count > 0);
    
    // Test state snapshot
    let snapshot = client.get_state_snapshot();
    assert!(snapshot.total_operations > 0);
    
    // Test performance stats
    let stats = client.get_performance_stats(&symbol_short!("init_prg"));
    assert!(stats.call_count > 0);
}

proptest! {
    #[test]
    fn test_fuzz_lock_program_funds(amount in 1..i128::MAX) {
        let env = Env::default();
        env.mock_all_auths(); // Essential for token transfers/auth
        let (client, _, _, program_id) = setup_program(&env);
        
        // We accept that this might fail if amount loops/overflows? 
        // Logic in contract uses unchecked addition/subtraction?
        // Code in lib.rs calls checked_add sometimes?
        // Line 1399 test_lock_zero_funds checks <= 0 panic.
        // If amount is positive, it should succeed unless total overflows i128::MAX.
        // Since we start at 0, one lock of MAX should work.
        
        client.lock_program_funds(&program_id, &amount);
        let info = client.get_program_info(&program_id);
        assert_eq!(info.remaining_balance, amount);
    }
}
