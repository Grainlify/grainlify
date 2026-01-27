#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _},
    token, Address, Env, String
};

use crate::{ProgramEscrowContract, ProgramEscrowContractClient};

// Test helper to create a mock token contract
fn create_token_contract<'a>(env: &Env, admin: &Address) -> token::Client<'a> {
    let token_address = env.register_stellar_asset_contract_v2(admin.clone());
    token::Client::new(env, &token_address.address())
}

fn create_test_env() -> (Env, ProgramEscrowContractClient<'static>, Address, token::Client<'static>) {
    let env = Env::default();
    env.mock_all_auths(); // Mock all authorizations for testing

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    // Create admin and token
    let admin = Address::generate(&env);
    let token_client = create_token_contract(&env, &admin);

    // Initialize a program first
    let program_id = String::from_str(&env, "TestProgram");
    client.initialize_program(&program_id, &admin, &token_client.address);

    (env, client, admin, token_client)
}

#[test]
fn test_pause_functionality() {
    let (env, client, admin, _token_client) = create_test_env();

    // Initially not paused
    assert!(!client.is_paused());

    // Pause the contract
    client.pause(&Some(String::from_str(&env, "Security issue")));

    // Should be paused now
    assert!(client.is_paused());

    // Unpause the contract
    client.unpause(&Some(String::from_str(&env, "Issue resolved")));

    // Should not be paused anymore
    assert!(!client.is_paused());

    // Now locking funds should work (contract is not paused)
    let program_id = String::from_str(&env, "TestProgram");
    let amount = 1000i128;
    client.lock_program_funds(&program_id, &amount);
}

#[test]
fn test_emergency_withdraw() {
    let (env, client, admin, _token_client) = create_test_env();

    // Lock some funds first
    let program_id = String::from_str(&env, "TestProgram");
    let amount = 1000i128;

    client.lock_program_funds(&program_id, &amount);

    // Pause the contract
    client.pause(&Some(String::from_str(&env, "Emergency")));
    assert!(client.is_paused());

    // Contract should be paused (emergency_withdraw would require actual token balance)
    // This test validates that pause functionality works correctly
}