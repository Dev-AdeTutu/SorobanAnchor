//! Tests for structured compliance check storage and query APIs (#252).

#![cfg(test)]

mod compliance_query_tests {
    use soroban_sdk::testutils::{Address as _, Ledger, LedgerInfo};
    use soroban_sdk::{Address, Env, String};

    use anchorkit::contract::{AnchorKitContract, AnchorKitContractClient};

    fn make_env() -> Env {
        let env = Env::default();
        env.mock_all_auths();
        env
    }

    fn set_ledger(env: &Env, ts: u64) {
        env.ledger().set(LedgerInfo {
            timestamp: ts,
            protocol_version: 21,
            sequence_number: 0,
            network_id: Default::default(),
            base_reserve: 0,
            min_persistent_entry_ttl: 4096,
            min_temp_entry_ttl: 16,
            max_entry_ttl: 6_312_000,
        });
    }

    fn setup(env: &Env) -> (Address, AnchorKitContractClient<'_>) {
        set_ledger(env, 1000);
        let cid = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(env, &cid);
        let admin = Address::generate(env);
        client.initialize(&admin);
        (admin, client)
    }

    fn check_type(env: &Env, s: &str) -> String {
        String::from_str(env, s)
    }

    // ── get_latest_compliance_check ───────────────────────────────────────────

    #[test]
    fn test_latest_check_none_before_any_record() {
        let env = make_env(); let (_, client) = setup(&env);
        let subject = Address::generate(&env);
        let result = client.get_latest_compliance_check(&subject, &check_type(&env, "kyc"));
        assert!(result.is_none());
    }

    #[test]
    fn test_latest_check_returns_most_recent_passed_record() {
        let env = make_env(); let (admin, client) = setup(&env);
        let subject = Address::generate(&env);

        set_ledger(&env, 2000);
        client.record_compliance_check(&subject, &check_type(&env, "kyc"), &false, &None);
        set_ledger(&env, 3000);
        client.record_compliance_check(&subject, &check_type(&env, "kyc"), &true, &None);

        let latest = client.get_latest_compliance_check(&subject, &check_type(&env, "kyc")).unwrap();
        assert_eq!(latest.result, 1u32, "latest should be the passed check");
        assert_eq!(latest.timestamp, 3000);
    }

    #[test]
    fn test_latest_check_returns_most_recent_failed_record() {
        let env = make_env(); let (admin, client) = setup(&env);
        let subject = Address::generate(&env);

        set_ledger(&env, 2000);
        client.record_compliance_check(&subject, &check_type(&env, "aml"), &true, &None);
        set_ledger(&env, 4000);
        client.record_compliance_check(&subject, &check_type(&env, "aml"), &false, &None);

        let latest = client.get_latest_compliance_check(&subject, &check_type(&env, "aml")).unwrap();
        assert_eq!(latest.result, 0u32);
        assert_eq!(latest.timestamp, 4000);
    }

    #[test]
    fn test_latest_check_isolated_by_check_type() {
        let env = make_env(); let (admin, client) = setup(&env);
        let subject = Address::generate(&env);

        set_ledger(&env, 2000);
        client.record_compliance_check(&subject, &check_type(&env, "kyc"), &true, &None);
        client.record_compliance_check(&subject, &check_type(&env, "aml"), &false, &None);

        let kyc = client.get_latest_compliance_check(&subject, &check_type(&env, "kyc")).unwrap();
        let aml = client.get_latest_compliance_check(&subject, &check_type(&env, "aml")).unwrap();
        assert_eq!(kyc.result, 1u32);
        assert_eq!(aml.result, 0u32);
    }

    // ── get_compliance_check_history ─────────────────────────────────────────

    #[test]
    fn test_history_empty_before_any_record() {
        let env = make_env(); let (_, client) = setup(&env);
        let subject = Address::generate(&env);
        let history = client.get_compliance_check_history(&subject, &check_type(&env, "kyc"), &10);
        assert_eq!(history.len(), 0);
    }

    #[test]
    fn test_history_single_entry() {
        let env = make_env(); let (admin, client) = setup(&env);
        let subject = Address::generate(&env);
        set_ledger(&env, 2000);
        client.record_compliance_check(&subject, &check_type(&env, "kyc"), &true, &None);

        let history = client.get_compliance_check_history(&subject, &check_type(&env, "kyc"), &10);
        assert_eq!(history.len(), 1);
        assert_eq!(history.get(0).unwrap().result, 1u32);
    }

    #[test]
    fn test_history_multiple_entries_ordered_oldest_first() {
        let env = make_env(); let (admin, client) = setup(&env);
        let subject = Address::generate(&env);

        set_ledger(&env, 1000);
        client.record_compliance_check(&subject, &check_type(&env, "kyc"), &false, &None);
        set_ledger(&env, 2000);
        client.record_compliance_check(&subject, &check_type(&env, "kyc"), &true, &None);
        set_ledger(&env, 3000);
        client.record_compliance_check(&subject, &check_type(&env, "kyc"), &false, &None);

        let history = client.get_compliance_check_history(&subject, &check_type(&env, "kyc"), &10);
        assert_eq!(history.len(), 3);
        assert_eq!(history.get(0).unwrap().timestamp, 1000);
        assert_eq!(history.get(1).unwrap().timestamp, 2000);
        assert_eq!(history.get(2).unwrap().timestamp, 3000);
    }

    #[test]
    fn test_history_limit_respected() {
        let env = make_env(); let (admin, client) = setup(&env);
        let subject = Address::generate(&env);

        for ts in [1000u64, 2000, 3000, 4000, 5000] {
            set_ledger(&env, ts);
            client.record_compliance_check(&subject, &check_type(&env, "kyc"), &true, &None);
        }

        let history = client.get_compliance_check_history(&subject, &check_type(&env, "kyc"), &3);
        assert_eq!(history.len(), 3, "limit should restrict to 3 most-recent entries");
        // Most-recent should be at the end
        assert_eq!(history.get(2).unwrap().timestamp, 5000);
    }

    #[test]
    fn test_history_isolated_by_check_type() {
        let env = make_env(); let (admin, client) = setup(&env);
        let subject = Address::generate(&env);

        set_ledger(&env, 2000);
        client.record_compliance_check(&subject, &check_type(&env, "kyc"), &true, &None);
        client.record_compliance_check(&subject, &check_type(&env, "aml"), &false, &None);
        client.record_compliance_check(&subject, &check_type(&env, "aml"), &true, &None);

        let kyc_history = client.get_compliance_check_history(&subject, &check_type(&env, "kyc"), &10);
        let aml_history = client.get_compliance_check_history(&subject, &check_type(&env, "aml"), &10);
        assert_eq!(kyc_history.len(), 1);
        assert_eq!(aml_history.len(), 2);
    }

    // ── list_subject_compliance_checks ────────────────────────────────────

    #[test]
    fn test_list_empty_for_new_subject() {
        let env = make_env(); let (_, client) = setup(&env);
        let subject = Address::generate(&env);
        let types = client.list_subject_compliance_checks(&subject);
        assert_eq!(types.len(), 0);
    }

    #[test]
    fn test_list_returns_all_recorded_check_types() {
        let env = make_env(); let (admin, client) = setup(&env);
        let subject = Address::generate(&env);
        set_ledger(&env, 2000);

        client.record_compliance_check(&subject, &check_type(&env, "kyc"), &true, &None);
        client.record_compliance_check(&subject, &check_type(&env, "aml"), &false, &None);
        client.record_compliance_check(&subject, &check_type(&env, "sanctions"), &true, &None);

        let types = client.list_subject_compliance_checks(&subject);
        assert_eq!(types.len(), 3);
    }

    #[test]
    fn test_list_does_not_duplicate_same_check_type() {
        let env = make_env(); let (admin, client) = setup(&env);
        let subject = Address::generate(&env);

        for ts in [1000u64, 2000, 3000] {
            set_ledger(&env, ts);
            client.record_compliance_check(&subject, &check_type(&env, "kyc"), &true, &None);
        }

        let types = client.list_subject_compliance_checks(&subject);
        assert_eq!(types.len(), 1, "same check_type should appear only once in index");
    }

    #[test]
    fn test_list_isolated_by_subject() {
        let env = make_env(); let (admin, client) = setup(&env);
        let subject_a = Address::generate(&env);
        let subject_b = Address::generate(&env);
        set_ledger(&env, 2000);

        client.record_compliance_check(&subject_a, &check_type(&env, "kyc"), &true, &None);
        client.record_compliance_check(&subject_b, &check_type(&env, "aml"), &false, &None);

        let types_a = client.list_subject_compliance_checks(&subject_a);
        let types_b = client.list_subject_compliance_checks(&subject_b);
        assert_eq!(types_a.len(), 1);
        assert_eq!(types_b.len(), 1);
    }

    // ── Compound workflow ─────────────────────────────────────────────────────

    #[test]
    fn test_full_compliance_workflow() {
        let env = make_env(); let (admin, client) = setup(&env);
        let subject = Address::generate(&env);

        // First KYC check: failed
        set_ledger(&env, 1000);
        client.record_compliance_check(&subject, &check_type(&env, "kyc"), &false, &None);

        // AML check: passed
        set_ledger(&env, 1500);
        client.record_compliance_check(&subject, &check_type(&env, "aml"), &true, &None);

        // Second KYC check (re-review): passed
        set_ledger(&env, 2000);
        client.record_compliance_check(&subject, &check_type(&env, "kyc"), &true, &None);

        // Latest KYC should be the passing check
        let latest_kyc = client.get_latest_compliance_check(&subject, &check_type(&env, "kyc")).unwrap();
        assert_eq!(latest_kyc.result, 1u32);
        assert_eq!(latest_kyc.timestamp, 2000);

        // KYC history should contain both entries in order
        let kyc_history = client.get_compliance_check_history(&subject, &check_type(&env, "kyc"), &10);
        assert_eq!(kyc_history.len(), 2);
        assert_eq!(kyc_history.get(0).unwrap().result, 0u32); // failed first
        assert_eq!(kyc_history.get(1).unwrap().result, 1u32); // passed second

        // Subject index should have both check types
        let types = client.list_subject_compliance_checks(&subject);
        assert_eq!(types.len(), 2);
    }
}
