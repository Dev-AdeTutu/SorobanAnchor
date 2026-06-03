#![cfg(test)]

mod capacity_limits_tests {
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        Address, Env, String,
    };

    use anchorkit::contract::{
        AnchorKitContract, AnchorKitContractClient, AnchorMetadata, CapacityConfig,
    };
    use crate::sep10_test_util::register_attestor_with_sep10;
    use ed25519_dalek::SigningKey;

    fn make_env() -> Env {
        let env = Env::default();
        env.mock_all_auths();
        env
    }

    fn set_ledger(env: &Env, timestamp: u64) {
        env.ledger().set(LedgerInfo {
            timestamp,
            protocol_version: 21,
            sequence_number: 0,
            network_id: Default::default(),
            base_reserve: 0,
            min_persistent_entry_ttl: 4096,
            min_temp_entry_ttl: 16,
            max_entry_ttl: 6312000,
        });
    }

    fn sample_metadata(env: &Env, anchor: &Address) -> AnchorMetadata {
        AnchorMetadata {
            anchor: anchor.clone(),
            reputation_score: 9000,
            liquidity_score: 8500,
            uptime_percentage: 9900,
            total_volume: 1_000_000,
            average_settlement_time: 300,
            is_active: true,
        }
    }

    #[test]
    fn test_attestor_capacity_limit_enforced() {
        let env = make_env();
        set_ledger(&env, 0);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.initialize(&admin);

        // Set capacity limit to 2 attestors
        client.set_capacity_config(&CapacityConfig {
            max_attestors: 2,
            max_cache_entries: 10,
        });

        // Register first attestor - should succeed
        let sk1 = SigningKey::from(&[0u8; 32]);
        let attestor1 = Address::generate(&env);
        register_attestor_with_sep10(&env, &client, &attestor1, &attestor1, &sk1);
        assert_eq!(client.get_attestor_count(), 1);

        // Register second attestor - should succeed
        let sk2 = SigningKey::from(&[1u8; 32]);
        let attestor2 = Address::generate(&env);
        register_attestor_with_sep10(&env, &client, &attestor2, &attestor2, &sk2);
        assert_eq!(client.get_attestor_count(), 2);

        // Register third attestor - should fail
        let sk3 = SigningKey::from(&[2u8; 32]);
        let attestor3 = Address::generate(&env);
        let result = client.try_register_attestor(
            &attestor3,
            &String::from_str(&env, ""),
            &attestor3,
            &soroban_sdk::BytesN::from_array(&env, &[0u8; 32]),
        );
        assert!(result.is_err());

        // Revoke second attestor - count should decrease
        client.revoke_attestor(&attestor2);
        assert_eq!(client.get_attestor_count(), 1);

        // Now register third attestor - should succeed
        register_attestor_with_sep10(&env, &client, &attestor3, &attestor3, &sk3);
        assert_eq!(client.get_attestor_count(), 2);
    }

    #[test]
    fn test_cache_capacity_limit_enforced() {
        let env = make_env();
        set_ledger(&env, 0);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.initialize(&admin);

        // Set capacity limit to 3 cache entries
        client.set_capacity_config(&CapacityConfig {
            max_attestors: 10,
            max_cache_entries: 3,
        });

        // Cache first entry - should succeed
        let anchor1 = Address::generate(&env);
        let meta1 = sample_metadata(&env, &anchor1);
        client.cache_metadata(&anchor1, &meta1, &3600u64);
        assert_eq!(client.get_cache_count(), 1);

        // Cache second entry - should succeed
        let anchor2 = Address::generate(&env);
        let meta2 = sample_metadata(&env, &anchor2);
        client.cache_metadata(&anchor2, &meta2, &3600u64);
        assert_eq!(client.get_cache_count(), 2);

        // Cache third entry - should succeed
        let anchor3 = Address::generate(&env);
        let meta3 = sample_metadata(&env, &anchor3);
        client.cache_metadata(&anchor3, &meta3, &3600u64);
        assert_eq!(client.get_cache_count(), 3);

        // Cache fourth entry - should fail
        let anchor4 = Address::generate(&env);
        let meta4 = sample_metadata(&env, &anchor4);
        let result = client.try_cache_metadata(&anchor4, &meta4, &3600u64);
        assert!(result.is_err());
    }

    #[test]
    fn test_same_cache_entry_does_not_increase_count() {
        let env = make_env();
        set_ledger(&env, 0);
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.initialize(&admin);

        client.set_capacity_config(&CapacityConfig {
            max_attestors: 10,
            max_cache_entries: 3,
        });

        // Cache same anchor multiple times
        let anchor = Address::generate(&env);
        let meta = sample_metadata(&env, &anchor);

        // First time - count increases
        client.cache_metadata(&anchor, &meta, &3600u64);
        assert_eq!(client.get_cache_count(), 1);

        // Second time - same entry, count stays same
        let mut updated_meta = meta.clone();
        updated_meta.reputation_score = 9500;
        client.cache_metadata(&anchor, &updated_meta, &3600u64);
        assert_eq!(client.get_cache_count(), 1);
    }
}
