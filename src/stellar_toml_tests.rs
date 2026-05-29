#![cfg(test)]

use alloc::vec;
use crate::stellar_toml::{fetch_stellar_toml_url, parse_stellar_toml};

const VALID_TOML: &str = r#"
NETWORK_PASSPHRASE = "Test SDF Network ; September 2015"
TRANSFER_SERVER = "https://api.example.com"
TRANSFER_SERVER_SEP0024 = "https://api.example.com/sep24"
KYC_SERVER = "https://kyc.example.com"
WEB_AUTH_ENDPOINT = "https://auth.example.com"
SIGNING_KEY = "GSIGN123"

[[CURRENCIES]]
code = "USDC"
issuer = "GABC123"

[[CURRENCIES]]
code = "XLM"
issuer = "native"
"#;

#[test]
fn test_parse_valid_toml_extracts_all_fields() {
    let parsed = parse_stellar_toml(VALID_TOML).unwrap();
    assert_eq!(parsed.transfer_server.as_deref(), Some("https://api.example.com"));
    assert_eq!(parsed.transfer_server_sep0024.as_deref(), Some("https://api.example.com/sep24"));
    assert_eq!(parsed.kyc_server.as_deref(), Some("https://kyc.example.com"));
    assert_eq!(parsed.web_auth_endpoint.as_deref(), Some("https://auth.example.com"));
    assert_eq!(parsed.signing_key.as_deref(), Some("GSIGN123"));
    assert_eq!(parsed.supported_assets, vec!["USDC", "XLM"]);
}

#[test]
fn test_parse_sep_support_flags() {
    let parsed = parse_stellar_toml(VALID_TOML).unwrap();
    assert!(parsed.supports_sep6());
    assert!(parsed.supports_sep24());
    assert!(parsed.supports_sep10());
}

#[test]
fn test_parse_missing_optional_fields_returns_none() {
    let raw = r#"SIGNING_KEY = "GSIGN123""#;
    let parsed = parse_stellar_toml(raw).unwrap();
    assert!(parsed.transfer_server.is_none());
    assert!(parsed.transfer_server_sep0024.is_none());
    assert!(parsed.kyc_server.is_none());
    assert!(parsed.web_auth_endpoint.is_none());
    assert!(parsed.supported_assets.is_empty());
    assert!(!parsed.supports_sep6());
    assert!(!parsed.supports_sep24());
    assert!(!parsed.supports_sep10());
}

#[test]
fn test_parse_empty_toml_returns_empty_capabilities() {
    let parsed = parse_stellar_toml("").unwrap();
    assert!(parsed.transfer_server.is_none());
    assert!(parsed.supported_assets.is_empty());
}

#[test]
fn test_parse_invalid_url_in_transfer_server_rejected() {
    let raw = r#"TRANSFER_SERVER = "http://insecure.example.com""#;
    assert!(parse_stellar_toml(raw).is_err());
}

#[test]
fn test_parse_invalid_url_in_web_auth_endpoint_rejected() {
    let raw = r#"WEB_AUTH_ENDPOINT = "not-a-url""#;
    assert!(parse_stellar_toml(raw).is_err());
}

#[test]
fn test_parse_invalid_url_in_kyc_server_rejected() {
    let raw = r#"KYC_SERVER = "ftp://kyc.example.com""#;
    assert!(parse_stellar_toml(raw).is_err());
}

#[test]
fn test_parse_comments_and_blank_lines_ignored() {
    let raw = r#"
# This is a comment
TRANSFER_SERVER = "https://api.example.com"

# Another comment
SIGNING_KEY = "GSIGN123"
"#;
    let parsed = parse_stellar_toml(raw).unwrap();
    assert_eq!(parsed.transfer_server.as_deref(), Some("https://api.example.com"));
    assert_eq!(parsed.signing_key.as_deref(), Some("GSIGN123"));
}

#[test]
fn test_parse_duplicate_currency_codes_deduplicated() {
    let raw = r#"
[[CURRENCIES]]
code = "USDC"

[[CURRENCIES]]
code = "USDC"
"#;
    let parsed = parse_stellar_toml(raw).unwrap();
    assert_eq!(parsed.supported_assets.len(), 1);
}

#[test]
fn test_fetch_stellar_toml_url_valid_domain() {
    let url = fetch_stellar_toml_url("https://example.com").unwrap();
    assert_eq!(url, "https://example.com/.well-known/stellar.toml");
}

#[test]
fn test_fetch_stellar_toml_url_strips_trailing_slash() {
    let url = fetch_stellar_toml_url("https://example.com/").unwrap();
    assert_eq!(url, "https://example.com/.well-known/stellar.toml");
}

#[test]
fn test_fetch_stellar_toml_url_rejects_http() {
    assert!(fetch_stellar_toml_url("http://example.com").is_err());
}

#[test]
fn test_fetch_stellar_toml_url_rejects_invalid_domain() {
    assert!(fetch_stellar_toml_url("not-a-domain").is_err());
}

// ---------------------------------------------------------------------------
// Nested / namespaced parsing (#237)
// ---------------------------------------------------------------------------

/// A `code = "..."` line inside a non-currency table must NOT be treated as an
/// asset. Previously the parser matched `code` globally and would leak this.
#[test]
fn test_nested_non_currency_section_does_not_pollute_assets() {
    let raw = r#"
TRANSFER_SERVER = "https://api.example.com"

[INTERACTIVE_DEPOSITS]
enabled = true
code = "SHOULD_NOT_APPEAR"

[[DOCUMENTATION]]
ORG_NAME = "Example"
code = "ALSO_NOT_AN_ASSET"

[[CURRENCIES]]
code = "USDC"
issuer = "GABC123"
"#;
    let parsed = parse_stellar_toml(raw).unwrap();
    assert_eq!(parsed.supported_assets, vec!["USDC"]);
    assert_eq!(parsed.currencies.len(), 1);
    assert_eq!(parsed.currencies[0].code, "USDC");
    assert_eq!(parsed.currencies[0].issuer.as_deref(), Some("GABC123"));
}

#[test]
fn test_currency_issuer_and_status_parsed() {
    let raw = r#"
[[CURRENCIES]]
code = "USDC"
issuer = "GAISSUER"
status = "live"

[[CURRENCIES]]
code = "EURC"
"#;
    let parsed = parse_stellar_toml(raw).unwrap();
    assert_eq!(parsed.currencies.len(), 2);

    let usdc = parsed.find_currency("USDC").unwrap();
    assert_eq!(usdc.issuer.as_deref(), Some("GAISSUER"));
    assert_eq!(usdc.status.as_deref(), Some("live"));

    let eurc = parsed.find_currency("EURC").unwrap();
    assert!(eurc.issuer.is_none());
    assert!(eurc.status.is_none());

    assert_eq!(parsed.supported_assets, vec!["USDC", "EURC"]);
}

#[test]
fn test_currency_block_without_code_is_dropped() {
    let raw = r#"
[[CURRENCIES]]
issuer = "GANOCODE"
status = "test"

[[CURRENCIES]]
code = "USDC"
"#;
    let parsed = parse_stellar_toml(raw).unwrap();
    assert_eq!(parsed.currencies.len(), 1);
    assert_eq!(parsed.currencies[0].code, "USDC");
}

#[test]
fn test_parse_additional_sep_endpoints() {
    let raw = r#"
NETWORK_PASSPHRASE = "Public Global Stellar Network ; September 2015"
DIRECT_PAYMENT_SERVER = "https://sep31.example.com"
ANCHOR_QUOTE_SERVER = "https://sep38.example.com"
"#;
    let parsed = parse_stellar_toml(raw).unwrap();
    assert_eq!(
        parsed.network_passphrase.as_deref(),
        Some("Public Global Stellar Network ; September 2015")
    );
    assert_eq!(parsed.direct_payment_server.as_deref(), Some("https://sep31.example.com"));
    assert_eq!(parsed.anchor_quote_server.as_deref(), Some("https://sep38.example.com"));
    assert!(parsed.supports_sep31());
    assert!(parsed.supports_sep38());
}

#[test]
fn test_additional_sep_endpoints_strict_on_invalid_url() {
    assert!(parse_stellar_toml(r#"DIRECT_PAYMENT_SERVER = "http://insecure.example.com""#).is_err());
    assert!(parse_stellar_toml(r#"ANCHOR_QUOTE_SERVER = "not-a-url""#).is_err());
}

#[test]
fn test_is_sep10_complete_requires_endpoint_and_key() {
    // Both present → complete
    let both = r#"
WEB_AUTH_ENDPOINT = "https://auth.example.com"
SIGNING_KEY = "GSIGN123"
"#;
    assert!(parse_stellar_toml(both).unwrap().is_sep10_complete());

    // Endpoint only → advertised but not complete
    let endpoint_only = r#"WEB_AUTH_ENDPOINT = "https://auth.example.com""#;
    let p = parse_stellar_toml(endpoint_only).unwrap();
    assert!(p.supports_sep10());
    assert!(!p.is_sep10_complete());

    // Signing key only → not complete (and not advertised)
    let key_only = r#"SIGNING_KEY = "GSIGN123""#;
    let p = parse_stellar_toml(key_only).unwrap();
    assert!(!p.supports_sep10());
    assert!(!p.is_sep10_complete());
}

/// A self-hosted, minimal-but-acceptable file: only SEP-24 advertised, no
/// currencies, no SEP-10. Optional fields absent must parse cleanly.
#[test]
fn test_incomplete_but_acceptable_self_hosted_toml() {
    let raw = r#"
# Self-hosted anchor, SEP-24 only
TRANSFER_SERVER_SEP0024 = "https://self.example.com/sep24"
"#;
    let parsed = parse_stellar_toml(raw).unwrap();
    assert!(parsed.supports_sep24());
    assert!(!parsed.supports_sep6());
    assert!(!parsed.supports_sep10());
    assert!(parsed.supported_assets.is_empty());
    assert!(parsed.currencies.is_empty());
    assert!(parsed.network_passphrase.is_none());
}

/// Per TOML semantics a key following a `[[CURRENCIES]]` header is scoped to
/// that table, so it must NOT be misattributed as a top-level endpoint. This is
/// the flip side of section-awareness: real-world files declare top-level
/// endpoints before any table (see VALID_TOML), and a stray key inside a table
/// is ignored rather than leaking into the root.
#[test]
fn test_key_after_currency_header_is_table_scoped() {
    let raw = r#"
TRANSFER_SERVER = "https://api.example.com"

[[CURRENCIES]]
code = "USDC"
KYC_SERVER = "https://kyc.example.com"
"#;
    let parsed = parse_stellar_toml(raw).unwrap();
    assert_eq!(parsed.supported_assets, vec!["USDC"]);
    // Root-level endpoint declared before the table is parsed.
    assert_eq!(parsed.transfer_server.as_deref(), Some("https://api.example.com"));
    // The KYC_SERVER line is scoped to the currency table and ignored, not
    // promoted to a top-level field.
    assert!(parsed.kyc_server.is_none());
}
