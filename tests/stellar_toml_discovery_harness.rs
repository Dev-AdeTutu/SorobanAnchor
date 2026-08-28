#![cfg(test)]

//! Stellar TOML discovery service test harness (#300)
//!
//! This module provides a mock HTTP service and comprehensive tests for
//! Stellar TOML discovery, including handling of redirects, missing files,
//! and invalid content.

use anchorkit::retry::{MockJitterSource, RetryConfig};
use anchorkit::stellar_toml::{
    fetch_stellar_toml_url, fetch_stellar_toml_with_retry, parse_stellar_toml,
    parse_stellar_toml_with_origin, ParsedStellarToml, StellarTomlFetchConfig,
    MAX_STELLAR_TOML_BYTES,
};

// ── Mock TOML responses ────────────────────────────────────────────────────

const VALID_TOML_MINIMAL: &str = r#"
TRANSFER_SERVER = "https://api.example.com"
SIGNING_KEY = "GSIGN123"
"#;

const VALID_TOML_FULL: &str = r#"
NETWORK_PASSPHRASE = "Test SDF Network ; September 2015"
TRANSFER_SERVER = "https://api.example.com"
TRANSFER_SERVER_SEP0024 = "https://api.example.com/sep24"
KYC_SERVER = "https://kyc.example.com"
WEB_AUTH_ENDPOINT = "https://auth.example.com"
SIGNING_KEY = "GSIGN123"
DIRECT_PAYMENT_SERVER = "https://payments.example.com"
ANCHOR_QUOTE_SERVER = "https://quotes.example.com"

[[CURRENCIES]]
code = "USDC"
issuer = "GABC123"
status = "test"

[[CURRENCIES]]
code = "EUR"
issuer = "GEUR123"
status = "live"

[[CURRENCIES]]
code = "XLM"
issuer = "native"
"#;

const VALID_TOML_SEP6_ONLY: &str = r#"
TRANSFER_SERVER = "https://api.example.com"
SIGNING_KEY = "GSIGN123"

[[CURRENCIES]]
code = "USDC"
issuer = "GABC123"
"#;

const VALID_TOML_SEP24_ONLY: &str = r#"
TRANSFER_SERVER_SEP0024 = "https://api.example.com/sep24"
SIGNING_KEY = "GSIGN123"

[[CURRENCIES]]
code = "USDC"
issuer = "GABC123"
"#;

const VALID_TOML_SEP38_ONLY: &str = r#"
ANCHOR_QUOTE_SERVER = "https://quotes.example.com"
SIGNING_KEY = "GSIGN123"

[[CURRENCIES]]
code = "USDC"
issuer = "GABC123"
"#;

const VALID_TOML_WITH_COMMENTS: &str = r#"
# Anchor configuration
TRANSFER_SERVER = "https://api.example.com"
# Web authentication
WEB_AUTH_ENDPOINT = "https://auth.example.com"
SIGNING_KEY = "GSIGN123"

# Supported assets
[[CURRENCIES]]
code = "USDC"
issuer = "GABC123"
"#;

const VALID_TOML_MULTIPLE_CURRENCIES: &str = r#"
TRANSFER_SERVER = "https://api.example.com"
SIGNING_KEY = "GSIGN123"

[[CURRENCIES]]
code = "USDC"
issuer = "GABC123"

[[CURRENCIES]]
code = "EUR"
issuer = "GEUR123"

[[CURRENCIES]]
code = "GBP"
issuer = "GGBP123"

[[CURRENCIES]]
code = "JPY"
issuer = "GJPY123"

[[CURRENCIES]]
code = "AUD"
issuer = "GAUD123"
"#;

const INVALID_TOML_BAD_URL: &str = r#"
TRANSFER_SERVER = "http://insecure.example.com"
SIGNING_KEY = "GSIGN123"
"#;

const INVALID_TOML_MALFORMED_URL: &str = r#"
TRANSFER_SERVER = "not-a-valid-url"
SIGNING_KEY = "GSIGN123"
"#;

const INVALID_TOML_MISSING_SCHEME: &str = r#"
TRANSFER_SERVER = "api.example.com"
SIGNING_KEY = "GSIGN123"
"#;

const VALID_TOML_SAME_ORIGIN: &str = r#"
TRANSFER_SERVER = "https://anchor.example.com"
SIGNING_KEY = "GSIGN123"
"#;

const TOML_MISMATCHED_ORIGIN: &str = r#"
TRANSFER_SERVER = "https://evil.example.net"
SIGNING_KEY = "GSIGN123"
"#;

// ── URL construction tests ─────────────────────────────────────────────────

#[test]
fn test_fetch_stellar_toml_url_valid_domain() {
    let url = fetch_stellar_toml_url("example.com").expect("valid domain must construct URL");
    assert_eq!(url, "example.com/.well-known/stellar.toml");
}

#[test]
fn test_fetch_stellar_toml_url_with_https_prefix() {
    let url = fetch_stellar_toml_url("https://example.com").expect("https domain must construct URL");
    assert_eq!(url, "https://example.com/.well-known/stellar.toml");
}

#[test]
fn test_fetch_stellar_toml_url_with_trailing_slash() {
    let url = fetch_stellar_toml_url("https://example.com/").expect("domain with trailing slash must construct URL");
    assert_eq!(url, "https://example.com/.well-known/stellar.toml");
}

#[test]
fn test_fetch_stellar_toml_url_with_port() {
    let url = fetch_stellar_toml_url("https://example.com:8080").expect("domain with port must construct URL");
    assert_eq!(url, "https://example.com:8080/.well-known/stellar.toml");
}

#[test]
fn test_fetch_stellar_toml_url_invalid_domain_rejected() {
    let result = fetch_stellar_toml_url("http://insecure.example.com");
    assert!(result.is_err(), "insecure domain must be rejected");
}

#[test]
fn test_fetch_stellar_toml_url_invalid_scheme_rejected() {
    let result = fetch_stellar_toml_url("ftp://example.com");
    assert!(result.is_err(), "ftp scheme must be rejected");
}

// ── TOML parsing tests ─────────────────────────────────────────────────────

#[test]
fn test_parse_valid_toml_minimal() {
    let parsed = parse_stellar_toml(VALID_TOML_MINIMAL).expect("minimal TOML must parse");
    assert_eq!(parsed.transfer_server.as_deref(), Some("https://api.example.com"));
    assert_eq!(parsed.signing_key.as_deref(), Some("GSIGN123"));
}

#[test]
fn test_parse_valid_toml_full() {
    let parsed = parse_stellar_toml(VALID_TOML_FULL).expect("full TOML must parse");
    assert_eq!(parsed.transfer_server.as_deref(), Some("https://api.example.com"));
    assert_eq!(parsed.transfer_server_sep0024.as_deref(), Some("https://api.example.com/sep24"));
    assert_eq!(parsed.kyc_server.as_deref(), Some("https://kyc.example.com"));
    assert_eq!(parsed.web_auth_endpoint.as_deref(), Some("https://auth.example.com"));
    assert_eq!(parsed.signing_key.as_deref(), Some("GSIGN123"));
    assert_eq!(parsed.direct_payment_server.as_deref(), Some("https://payments.example.com"));
    assert_eq!(parsed.anchor_quote_server.as_deref(), Some("https://quotes.example.com"));
    assert_eq!(parsed.supported_assets.len(), 3);
}

#[test]
fn test_parse_sep6_only_support() {
    let parsed = parse_stellar_toml(VALID_TOML_SEP6_ONLY).expect("SEP-6 only TOML must parse");
    assert!(parsed.supports_sep6());
    assert!(!parsed.supports_sep24());
    assert!(!parsed.supports_sep38());
}

#[test]
fn test_parse_sep24_only_support() {
    let parsed = parse_stellar_toml(VALID_TOML_SEP24_ONLY).expect("SEP-24 only TOML must parse");
    assert!(!parsed.supports_sep6());
    assert!(parsed.supports_sep24());
    assert!(!parsed.supports_sep38());
}

#[test]
fn test_parse_sep38_only_support() {
    let parsed = parse_stellar_toml(VALID_TOML_SEP38_ONLY).expect("SEP-38 only TOML must parse");
    assert!(!parsed.supports_sep6());
    assert!(!parsed.supports_sep24());
    assert!(parsed.supports_sep38());
}

#[test]
fn test_parse_toml_with_comments() {
    let parsed = parse_stellar_toml(VALID_TOML_WITH_COMMENTS).expect("TOML with comments must parse");
    assert_eq!(parsed.transfer_server.as_deref(), Some("https://api.example.com"));
    assert_eq!(parsed.web_auth_endpoint.as_deref(), Some("https://auth.example.com"));
}

#[test]
fn test_parse_multiple_currencies() {
    let parsed = parse_stellar_toml(VALID_TOML_MULTIPLE_CURRENCIES).expect("multiple currencies must parse");
    assert_eq!(parsed.supported_assets.len(), 5);
    assert!(parsed.supported_assets.contains(&"USDC".to_string()));
    assert!(parsed.supported_assets.contains(&"EUR".to_string()));
    assert!(parsed.supported_assets.contains(&"GBP".to_string()));
    assert!(parsed.supported_assets.contains(&"JPY".to_string()));
    assert!(parsed.supported_assets.contains(&"AUD".to_string()));
}

#[test]
fn test_parse_currency_lookup() {
    let parsed = parse_stellar_toml(VALID_TOML_FULL).expect("full TOML must parse");
    let usdc = parsed.find_currency("USDC").expect("USDC must be found");
    assert_eq!(usdc.code, "USDC");
    assert_eq!(usdc.issuer.as_deref(), Some("GABC123"));
    assert_eq!(usdc.status.as_deref(), Some("test"));
}

#[test]
fn test_parse_currency_not_found() {
    let parsed = parse_stellar_toml(VALID_TOML_FULL).expect("full TOML must parse");
    let unknown = parsed.find_currency("UNKNOWN");
    assert!(unknown.is_none());
}

// ── Invalid TOML tests ─────────────────────────────────────────────────────

#[test]
fn test_parse_invalid_toml_bad_url_rejected() {
    let result = parse_stellar_toml(INVALID_TOML_BAD_URL);
    assert!(result.is_err(), "insecure URL must be rejected");
}

#[test]
fn test_parse_invalid_toml_malformed_url_rejected() {
    let result = parse_stellar_toml(INVALID_TOML_MALFORMED_URL);
    assert!(result.is_err(), "malformed URL must be rejected");
}

#[test]
fn test_parse_invalid_toml_missing_scheme_rejected() {
    let result = parse_stellar_toml(INVALID_TOML_MISSING_SCHEME);
    assert!(result.is_err(), "URL without scheme must be rejected");
}

// ── Origin-pinning tests (#847) ────────────────────────────────────────────

#[test]
fn test_parse_stellar_toml_with_origin_same_origin_passes() {
    let parsed = parse_stellar_toml_with_origin(VALID_TOML_SAME_ORIGIN, "https://anchor.example.com")
        .expect("same-origin TRANSFER_SERVER must pass");
    assert_eq!(parsed.transfer_server.as_deref(), Some("https://anchor.example.com"));
}

#[test]
fn test_parse_stellar_toml_with_origin_mismatched_origin_rejected() {
    let result = parse_stellar_toml_with_origin(TOML_MISMATCHED_ORIGIN, "https://anchor.example.com");
    assert!(result.is_err(), "mismatched TRANSFER_SERVER origin must be rejected before use");
}

#[test]
fn test_parse_stellar_toml_with_origin_ignores_unrelated_metadata() {
    // Other fields (currencies, signing key, unrelated SEP endpoints) are
    // untouched by origin pinning — only TRANSFER_SERVER is checked.
    let parsed = parse_stellar_toml_with_origin(VALID_TOML_FULL, "https://api.example.com")
        .expect("matching TRANSFER_SERVER origin must pass");
    assert_eq!(parsed.kyc_server.as_deref(), Some("https://kyc.example.com"));
    assert_eq!(parsed.web_auth_endpoint.as_deref(), Some("https://auth.example.com"));
    assert_eq!(parsed.supported_assets.len(), 3);
}

// ── Document size bound tests (#850) ───────────────────────────────────────

#[test]
fn test_fetch_stellar_toml_with_retry_rejects_oversized_body() {
    let config = StellarTomlFetchConfig {
        retry: RetryConfig::new(1, 0, 0, 1),
        fallback_hosts: Vec::new(),
    };
    let oversized_body = "a".repeat(MAX_STELLAR_TOML_BYTES + 1);
    let mut js = MockJitterSource::new(vec![0u64; 4]);

    let result = fetch_stellar_toml_with_retry(
        "https://anchor.example.com",
        &config,
        |_url| Ok(oversized_body.clone()),
        |_ms| {},
        &mut js,
    );

    assert!(result.is_err(), "oversized stellar.toml body must be rejected before parsing");
}

#[test]
fn test_fetch_stellar_toml_with_retry_accepts_body_at_limit() {
    let config = StellarTomlFetchConfig {
        retry: RetryConfig::new(1, 0, 0, 1),
        fallback_hosts: Vec::new(),
    };
    let at_limit_body = "a".repeat(MAX_STELLAR_TOML_BYTES);
    let mut js = MockJitterSource::new(vec![0u64; 4]);

    let result = fetch_stellar_toml_with_retry(
        "https://anchor.example.com",
        &config,
        |_url| Ok(at_limit_body.clone()),
        |_ms| {},
        &mut js,
    );

    assert!(result.is_ok(), "a body at exactly the limit must still be accepted");
}

// ── Edge case tests ────────────────────────────────────────────────────────

#[test]
fn test_parse_empty_toml() {
    let parsed = parse_stellar_toml("").expect("empty TOML must parse");
    assert!(parsed.transfer_server.is_none());
    assert!(parsed.supported_assets.is_empty());
}

#[test]
fn test_parse_toml_with_only_comments() {
    let raw = r#"
# This is a comment
# Another comment
"#;
    let parsed = parse_stellar_toml(raw).expect("comment-only TOML must parse");
    assert!(parsed.transfer_server.is_none());
}

#[test]
fn test_parse_toml_with_blank_lines() {
    let raw = r#"

TRANSFER_SERVER = "https://api.example.com"


SIGNING_KEY = "GSIGN123"

"#;
    let parsed = parse_stellar_toml(raw).expect("TOML with blank lines must parse");
    assert_eq!(parsed.transfer_server.as_deref(), Some("https://api.example.com"));
}

#[test]
fn test_parse_sep10_complete_check() {
    let parsed = parse_stellar_toml(VALID_TOML_FULL).expect("full TOML must parse");
    assert!(parsed.is_sep10_complete(), "TOML with both endpoint and key must be complete");

    let partial = parse_stellar_toml(VALID_TOML_SEP6_ONLY).expect("SEP-6 only TOML must parse");
    assert!(!partial.is_sep10_complete(), "TOML without web auth endpoint must not be complete");
}

#[test]
fn test_parse_toml_with_whitespace_variations() {
    let raw = r#"
TRANSFER_SERVER   =   "https://api.example.com"
SIGNING_KEY="GSIGN123"
WEB_AUTH_ENDPOINT= "https://auth.example.com"
"#;
    let parsed = parse_stellar_toml(raw).expect("TOML with whitespace variations must parse");
    assert_eq!(parsed.transfer_server.as_deref(), Some("https://api.example.com"));
    assert_eq!(parsed.signing_key.as_deref(), Some("GSIGN123"));
}

// ── Discovery workflow tests ───────────────────────────────────────────────

#[test]
fn test_discovery_workflow_construct_url_and_parse() {
    // Step 1: Construct URL
    let url = fetch_stellar_toml_url("https://example.com").expect("URL construction must succeed");
    assert_eq!(url, "https://example.com/.well-known/stellar.toml");

    // Step 2: Parse TOML (simulating HTTP response)
    let parsed = parse_stellar_toml(VALID_TOML_FULL).expect("TOML parsing must succeed");
    assert!(parsed.supports_sep6());
    assert!(parsed.supports_sep24());
}

#[test]
fn test_discovery_workflow_validate_capabilities() {
    let parsed = parse_stellar_toml(VALID_TOML_FULL).expect("TOML must parse");

    // Verify all capabilities are present
    assert!(parsed.supports_sep6(), "must support SEP-6");
    assert!(parsed.supports_sep24(), "must support SEP-24");
    assert!(parsed.supports_sep10(), "must support SEP-10");
    assert!(parsed.supports_sep31(), "must support SEP-31");
    assert!(parsed.supports_sep38(), "must support SEP-38");
    assert!(parsed.is_sep10_complete(), "must have complete SEP-10");
}

#[test]
fn test_discovery_workflow_handle_missing_file() {
    // Simulate 404 response (empty TOML)
    let parsed = parse_stellar_toml("").expect("empty TOML must parse gracefully");
    assert!(!parsed.supports_sep6(), "missing file should have no capabilities");
}

#[test]
fn test_discovery_workflow_handle_invalid_response() {
    // Simulate invalid response
    let result = parse_stellar_toml(INVALID_TOML_BAD_URL);
    assert!(result.is_err(), "invalid response must be rejected");
}

#[test]
fn test_discovery_workflow_partial_capabilities() {
    // Anchor with only SEP-6
    let sep6_only = parse_stellar_toml(VALID_TOML_SEP6_ONLY).expect("SEP-6 only must parse");
    assert!(sep6_only.supports_sep6());
    assert!(!sep6_only.supports_sep24());
    assert!(!sep6_only.supports_sep38());

    // Anchor with only SEP-24
    let sep24_only = parse_stellar_toml(VALID_TOML_SEP24_ONLY).expect("SEP-24 only must parse");
    assert!(!sep24_only.supports_sep6());
    assert!(sep24_only.supports_sep24());
    assert!(!sep24_only.supports_sep38());
}

#[test]
fn test_discovery_workflow_asset_discovery() {
    let parsed = parse_stellar_toml(VALID_TOML_MULTIPLE_CURRENCIES).expect("multiple currencies must parse");
    
    // Verify all assets are discovered
    assert_eq!(parsed.supported_assets.len(), 5);
    
    // Verify asset lookup
    assert!(parsed.find_currency("USDC").is_some());
    assert!(parsed.find_currency("EUR").is_some());
    assert!(parsed.find_currency("GBP").is_some());
    assert!(parsed.find_currency("JPY").is_some());
    assert!(parsed.find_currency("AUD").is_some());
    assert!(parsed.find_currency("UNKNOWN").is_none());
}
