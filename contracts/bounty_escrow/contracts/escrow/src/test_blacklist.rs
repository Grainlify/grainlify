#[cfg(test)]
mod blacklist_tests {
    use crate::BountyEscrowContract;
    use soroban_sdk::{testutils::Address as _, Address, Env};

    fn create_test_contract(env: &Env) -> (Address, BountyEscrowContract) {
        let contract_id = env.register_contract(None, BountyEscrowContract);
        (contract_id, BountyEscrowContract)
    }

    #[test]
    fn test_lock_funds_blacklist_blocked() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token = Address::generate(&env);
        let depositor = Address::generate(&env);

        let (contract_id, _) = create_test_contract(&env);

        env.as_contract(&contract_id, || {
            // Initialize contract
            let _ = BountyEscrowContract::init(env.clone(), admin.clone(), token.clone());

            // Add depositor to blacklist
            let _ = BountyEscrowContract::set_blacklist(env.clone(), depositor.clone(), true, None);

            // Try to lock funds - should fail
            let result = BountyEscrowContract::lock_funds(
                env.clone(),
                depositor.clone(),
                1,
                1000,
                env.ledger().timestamp() + 10000,
            );

            assert!(
                result.is_err(),
                "Lock funds should fail for blacklisted address"
            );
        });
    }

    #[test]
    fn test_whitelist_mode_enforcement() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token = Address::generate(&env);
        let depositor = Address::generate(&env);

        let (contract_id, _) = create_test_contract(&env);

        env.as_contract(&contract_id, || {
            // Initialize contract
            let _ = BountyEscrowContract::init(env.clone(), admin.clone(), token.clone());

            // Enable whitelist mode
            let _ = BountyEscrowContract::set_whitelist_mode(env.clone(), true);

            // Try to lock with non-whitelisted address - should fail
            let result = BountyEscrowContract::lock_funds(
                env.clone(),
                depositor.clone(),
                1,
                1000,
                env.ledger().timestamp() + 10000,
            );
            assert!(
                result.is_err(),
                "Lock funds should fail for non-whitelisted address in whitelist mode"
            );
        });
    }
}
