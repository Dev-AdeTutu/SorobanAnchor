//! Tests for audit log retention policies, pruning, and paginated retrieval (#251).

#![cfg(test)]

mod sep10_test_util;

mod audit_log_retention_tests {
    use soroban_sdk::testutils::{Address as _, Ledger, LedgerInfo};
    use soroban_sdk::{Address, Env};

    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    use anchorkit::{AnchorKitContract, AnchorKitContractClient};
    use crate::sep10_test_util::register_attestor_with_sep10;

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

    fn setup() -> (Env, Address, AnchorKitContractClient) {
        let env = make_env();
        set_ledger(&env, 1000);
        let cid = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &cid);
        let admin = Address::generate(&env);
        client.initialize(&admin);
        (env, admin, client)
    }

    fn add_attestor(env: &Env, client: &AnchorKitContractClient) -> Address {
        let attestor = Address::generate(env);
        let sk = SigningKey::generate(&mut OsRng);
        register_attestor_with_sep10(env, client, &attestor, &attestor, &sk);
        attestor
    }

    fn emit_audit_log(
        env: &Env,
        client: &AnchorKitContractClient,
        attestor: &Address,
        ts: u64,
    ) {
        set_ledger(env, ts);
        let session_id = client.create_session(attestor);
        let issuer = attestor.clone();
        let subject = Address::generate(env);
        let payload = soroban_sdk::Bytes::from_slice(env, b"payload_hash_32bytes_exactly____");
        let sig = soroban_sdk::Bytes::from_slice(env, b"payload_hash_32bytes_exactly____");
        client.submit_attestation_with_session(
            &session_id, &issuer, &subject, &ts, &payload, &sig,
        );
        client.close_session(&session_id, attestor);
    }

    // ── Retention policy ──────────────────────────────────────────────────────

    #[test]
    fn test_default_retention_is_zero_unlimited() {
        let (_, _, client) = setup();
        assert_eq!(client.get_audit_log_retention(), 0);
    }

    #[test]
    fn test_set_and_get_retention() {
        let (_, admin, client) = setup();
        client.set_audit_log_retention(&admin, &30);
        assert_eq!(client.get_audit_log_retention(), 30);
    }

    #[test]
    fn test_update_retention_policy() {
        let (_, admin, client) = setup();
        client.set_audit_log_retention(&admin, &90);
        client.set_audit_log_retention(&admin, &7);
        assert_eq!(client.get_audit_log_retention(), 7);
    }

    // ── Audit log count ───────────────────────────────────────────────────────

    #[test]
    fn test_audit_log_count_starts_at_zero() {
        let (_, _, client) = setup();
        assert_eq!(client.get_audit_log_count(), 0);
    }

    #[test]
    fn test_audit_log_count_increments_with_session_ops() {
        let (env, _, client) = setup();
        let attestor = add_attestor(&env, &client);
        emit_audit_log(&env, &client, &attestor, 2000);
        assert!(client.get_audit_log_count() >= 1);
    }

    // ── Pagination ────────────────────────────────────────────────────────────

    #[test]
    fn test_paginated_retrieval_empty_returns_empty_vec() {
        let (_, _, client) = setup();
        let page = client.get_audit_logs_paginated(&0, &10);
        assert_eq!(page.len(), 0);
    }

    #[test]
    fn test_paginated_retrieval_returns_correct_entries() {
        let (env, _, client) = setup();
        let attestor = add_attestor(&env, &client);
        emit_audit_log(&env, &client, &attestor, 2000);
        emit_audit_log(&env, &client, &attestor, 3000);

        let total = client.get_audit_log_count();
        assert!(total >= 2, "expected at least 2 entries, got {total}");

        let page = client.get_audit_logs_paginated(&0, &10);
        assert!(page.len() >= 2);
    }

    #[test]
    fn test_paginated_retrieval_respects_offset() {
        let (env, _, client) = setup();
        let attestor = add_attestor(&env, &client);
        emit_audit_log(&env, &client, &attestor, 2000);
        emit_audit_log(&env, &client, &attestor, 3000);

        let total = client.get_audit_log_count();
        let page0 = client.get_audit_logs_paginated(&0, &1);
        let page1 = client.get_audit_logs_paginated(&1, &1);
        // Both pages must be non-empty if total >= 2
        if total >= 2 {
            assert_eq!(page0.len(), 1);
            assert_eq!(page1.len(), 1);
        }
    }

    #[test]
    fn test_pagination_limit_capped_at_50() {
        let (env, _, client) = setup();
        let attestor = add_attestor(&env, &client);
        emit_audit_log(&env, &client, &attestor, 2000);

        // Requesting 200 should return at most 50
        let page = client.get_audit_logs_paginated(&0, &200);
        assert!(page.len() <= 50);
    }

    #[test]
    fn test_pagination_offset_beyond_total_returns_empty() {
        let (env, _, client) = setup();
        let attestor = add_attestor(&env, &client);
        emit_audit_log(&env, &client, &attestor, 2000);

        let page = client.get_audit_logs_paginated(&9999, &10);
        assert_eq!(page.len(), 0);
    }

    // ── Session-scoped pagination ─────────────────────────────────────────────

    #[test]
    fn test_session_audit_logs_paginated_returns_correct_entries() {
        let (env, _, client) = setup();
        let attestor = add_attestor(&env, &client);
        set_ledger(&env, 2000);
        let session_id = client.create_session(&attestor);

        let subject = Address::generate(&env);
        let payload = soroban_sdk::Bytes::from_slice(&env, b"payload_hash_32bytes_exactly____");
        let sig = soroban_sdk::Bytes::from_slice(&env, b"payload_hash_32bytes_exactly____");
        client.submit_attestation_with_session(
            &session_id, &attestor, &subject, &2000u64, &payload, &sig,
        );
        client.close_session(&session_id, &attestor);

        let page = client.get_session_logs_paginated(&session_id, &0, &10);
        assert!(page.len() >= 1);
    }

    #[test]
    fn test_session_audit_logs_paginated_empty_offset_returns_empty() {
        let (env, _, client) = setup();
        let attestor = add_attestor(&env, &client);
        set_ledger(&env, 2000);
        let session_id = client.create_session(&attestor);
        client.close_session(&session_id, &attestor);

        let page = client.get_session_logs_paginated(&session_id, &9999, &10);
        assert_eq!(page.len(), 0);
    }

    // ── Pruning ───────────────────────────────────────────────────────────────

    #[test]
    fn test_prune_removes_old_entries() {
        let (env, admin, client) = setup();
        let attestor = add_attestor(&env, &client);
        // Create two audit entries at different timestamps
        emit_audit_log(&env, &client, &attestor, 1_000);
        emit_audit_log(&env, &client, &attestor, 10_000);

        let before_prune = client.get_audit_log_count();
        assert!(before_prune >= 2);

        // Prune entries older than timestamp 5_000
        set_ledger(&env, 20_000);
        let pruned = client.prune_audit_logs(&admin, &5_000u64);
        assert!(pruned >= 1, "expected at least one entry pruned");
    }

    #[test]
    fn test_prune_does_not_remove_recent_entries() {
        let (env, admin, client) = setup();
        let attestor = add_attestor(&env, &client);
        emit_audit_log(&env, &client, &attestor, 10_000);

        set_ledger(&env, 20_000);
        // Prune with a threshold before all entries — nothing should be removed
        let pruned = client.prune_audit_logs(&admin, &1u64);
        assert_eq!(pruned, 0, "no entries should be pruned when threshold is too early");
    }

    #[test]
    fn test_prune_with_no_entries_returns_zero() {
        let (env, admin, client) = setup();
        set_ledger(&env, 5_000);
        let pruned = client.prune_audit_logs(&admin, &9_999u64);
        assert_eq!(pruned, 0);
    }
}
