//! Minimal stellar.toml capability parser.
//!
//! Parses the key=value fields relevant to anchor capability discovery
//! (SEP-6, SEP-24, SEP-10, SEP-31, SEP-38, KYC) from a raw stellar.toml string.
//! No external TOML crate is required; only `alloc` is used.
//!
//! ## Parsing model (issue #237)
//!
//! The parser is *section-aware*. It tracks the current table header so that
//! currency-scoped keys (`code`, `issuer`, `status`, `name`) are only collected
//! while inside a `[[CURRENCIES]]` block. This prevents unrelated nested or
//! namespaced sections (for example `[INTERACTIVE_DEPOSITS]`, `[[DOCUMENTATION]]`,
//! or anchor-specific feature tables) from leaking stray `code = "..."` lines
//! into the discovered asset list.
//!
//! Top-level SEP endpoint keys (`TRANSFER_SERVER`, `WEB_AUTH_ENDPOINT`, …) are
//! recognised regardless of the surrounding section so that real-world files
//! that declare them after a table still parse. URL-valued fields are validated
//! strictly with [`validate_anchor_domain`]; an invalid URL is a hard error.
//! Every other field is optional: when absent it parses to `None`/empty rather
//! than failing, so incomplete-but-acceptable configurations are tolerated.

extern crate alloc;
use alloc::{string::String, vec::Vec};

use crate::domain_validator::validate_anchor_domain;
use crate::errors::AnchorKitError;

/// A single currency/asset declared in a `[[CURRENCIES]]` table.
///
/// Only `code` is required for an entry to be retained; `issuer` and `status`
/// are optional and default to `None` when omitted.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedCurrency {
    pub code: String,
    pub issuer: Option<String>,
    pub status: Option<String>,
}

/// Parsed representation of the anchor-relevant fields in a stellar.toml file.
/// All URL fields are validated with [`validate_anchor_domain`] before being stored.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedStellarToml {
    pub network_passphrase: Option<String>,
    pub transfer_server: Option<String>,
    pub transfer_server_sep0024: Option<String>,
    pub kyc_server: Option<String>,
    pub web_auth_endpoint: Option<String>,
    pub signing_key: Option<String>,
    /// SEP-31 direct payment server endpoint.
    pub direct_payment_server: Option<String>,
    /// SEP-38 anchor quote (RFQ) server endpoint.
    pub anchor_quote_server: Option<String>,
    /// Asset codes declared in `[[CURRENCIES]]` sections, de-duplicated in
    /// declaration order. Retained for backwards compatibility; richer detail
    /// is available via [`currencies`](Self::currencies).
    pub supported_assets: Vec<String>,
    /// Structured currency definitions parsed from `[[CURRENCIES]]` sections.
    pub currencies: Vec<ParsedCurrency>,
}

impl ParsedStellarToml {
    /// Returns `true` if the anchor declares SEP-6 support.
    pub fn supports_sep6(&self) -> bool {
        self.transfer_server.is_some()
    }

    /// Returns `true` if the anchor declares SEP-24 support.
    pub fn supports_sep24(&self) -> bool {
        self.transfer_server_sep0024.is_some()
    }

    /// Returns `true` if the anchor declares SEP-10 (web auth) support.
    pub fn supports_sep10(&self) -> bool {
        self.web_auth_endpoint.is_some()
    }

    /// Returns `true` if the anchor declares SEP-31 (direct payment) support.
    pub fn supports_sep31(&self) -> bool {
        self.direct_payment_server.is_some()
    }

    /// Returns `true` if the anchor declares SEP-38 (anchor quote) support.
    pub fn supports_sep38(&self) -> bool {
        self.anchor_quote_server.is_some()
    }

    /// Returns `true` only when the file declares a *complete* SEP-10 setup.
    ///
    /// SEP-10 challenge verification requires both a `WEB_AUTH_ENDPOINT` and a
    /// `SIGNING_KEY`; a file advertising only one of the two cannot be used for
    /// authentication. Use this when strictness matters, and
    /// [`supports_sep10`](Self::supports_sep10) for a looser advertised-endpoint
    /// check.
    pub fn is_sep10_complete(&self) -> bool {
        self.web_auth_endpoint.is_some() && self.signing_key.is_some()
    }

    /// Look up a parsed currency by its asset code.
    pub fn find_currency(&self, code: &str) -> Option<&ParsedCurrency> {
        self.currencies.iter().find(|c| c.code == code)
    }
}

/// Constructs the well-known stellar.toml URL for a given domain.
///
/// # Errors
/// Returns `Err` if `domain` fails [`validate_anchor_domain`].
pub fn fetch_stellar_toml_url(domain: &str) -> Result<String, AnchorKitError> {
    validate_anchor_domain(domain)?;
    let mut url = String::from(domain);
    // Strip trailing slash before appending path
    if url.ends_with('/') {
        url.pop();
    }
    url.push_str("/.well-known/stellar.toml");
    Ok(url)
}

/// Which table the parser is currently inside.
enum Section {
    /// Top level, or any non-currency table (its fields are ignored).
    Other,
    /// Inside a `[[CURRENCIES]]` block.
    Currencies,
}

/// Parse a raw stellar.toml string into a [`ParsedStellarToml`].
///
/// Top-level SEP endpoint keys and `[[CURRENCIES]]` entries are extracted.
/// Currency-scoped keys are only collected while inside a `[[CURRENCIES]]`
/// table, so nested or namespaced sections cannot pollute the asset list.
/// All URL fields are validated; an invalid URL causes an error.
///
/// # Errors
/// Returns `Err` if any URL field contains an invalid value.
pub fn parse_stellar_toml(raw: &str) -> Result<ParsedStellarToml, AnchorKitError> {
    let mut network_passphrase: Option<String> = None;
    let mut transfer_server: Option<String> = None;
    let mut transfer_server_sep0024: Option<String> = None;
    let mut kyc_server: Option<String> = None;
    let mut web_auth_endpoint: Option<String> = None;
    let mut signing_key: Option<String> = None;
    let mut direct_payment_server: Option<String> = None;
    let mut anchor_quote_server: Option<String> = None;
    let mut currencies: Vec<ParsedCurrency> = Vec::new();

    let mut section = Section::Other;
    let mut current: Option<ParsedCurrency> = None;

    for line in raw.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }

        // Section header, e.g. `[[CURRENCIES]]`, `[DOCUMENTATION]`.
        if line.starts_with('[') {
            // Flush any in-progress currency before switching tables.
            flush_currency(&mut current, &mut currencies);
            let header = line.trim_matches(|c| c == '[' || c == ']').trim();
            if header.eq_ignore_ascii_case("CURRENCIES") {
                section = Section::Currencies;
                current = Some(ParsedCurrency {
                    code: String::new(),
                    issuer: None,
                    status: None,
                });
            } else {
                section = Section::Other;
            }
            continue;
        }

        if let Some((key, value)) = parse_kv(line) {
            match section {
                Section::Currencies => {
                    let cur = current.get_or_insert(ParsedCurrency {
                        code: String::new(),
                        issuer: None,
                        status: None,
                    });
                    match key {
                        "code" => cur.code = value,
                        "issuer" => cur.issuer = Some(value),
                        "status" => cur.status = Some(value),
                        _ => {}
                    }
                }
                Section::Other => match key {
                    "NETWORK_PASSPHRASE" => network_passphrase = Some(value),
                    "TRANSFER_SERVER" => {
                        validate_anchor_domain(&value)?;
                        transfer_server = Some(value);
                    }
                    "TRANSFER_SERVER_SEP0024" => {
                        validate_anchor_domain(&value)?;
                        transfer_server_sep0024 = Some(value);
                    }
                    "KYC_SERVER" => {
                        validate_anchor_domain(&value)?;
                        kyc_server = Some(value);
                    }
                    "WEB_AUTH_ENDPOINT" => {
                        validate_anchor_domain(&value)?;
                        web_auth_endpoint = Some(value);
                    }
                    "DIRECT_PAYMENT_SERVER" => {
                        validate_anchor_domain(&value)?;
                        direct_payment_server = Some(value);
                    }
                    "ANCHOR_QUOTE_SERVER" => {
                        validate_anchor_domain(&value)?;
                        anchor_quote_server = Some(value);
                    }
                    "SIGNING_KEY" => {
                        signing_key = Some(value);
                    }
                    _ => {}
                },
            }
        }
    }
    // Flush the final currency block, if any.
    flush_currency(&mut current, &mut currencies);

    // Derive the de-duplicated code list for backwards compatibility.
    let mut supported_assets: Vec<String> = Vec::new();
    for c in currencies.iter() {
        if !supported_assets.contains(&c.code) {
            supported_assets.push(c.code.clone());
        }
    }

    Ok(ParsedStellarToml {
        network_passphrase,
        transfer_server,
        transfer_server_sep0024,
        kyc_server,
        web_auth_endpoint,
        signing_key,
        direct_payment_server,
        anchor_quote_server,
        supported_assets,
        currencies,
    })
}

/// Push `current` into `currencies` if it carries a non-empty asset code,
/// leaving `current` as `None`. Currency blocks without a `code` are dropped.
fn flush_currency(current: &mut Option<ParsedCurrency>, currencies: &mut Vec<ParsedCurrency>) {
    if let Some(c) = current.take() {
        if !c.code.is_empty() {
            currencies.push(c);
        }
    }
}

/// Extract (key, value) from a line of the form `KEY = "value"` or `KEY = value`.
/// Returns `None` if the line is not a key=value assignment.
fn parse_kv(line: &str) -> Option<(&str, String)> {
    let eq = line.find('=')?;
    let key = line[..eq].trim();
    let raw_val = line[eq + 1..].trim();
    // Strip surrounding quotes if present
    let value = if raw_val.starts_with('"') && raw_val.ends_with('"') && raw_val.len() >= 2 {
        &raw_val[1..raw_val.len() - 1]
    } else {
        raw_val
    };
    // Skip inline comments after the value
    let value = value.split('#').next().unwrap_or(value).trim();
    if key.is_empty() {
        return None;
    }
    Some((key, String::from(value)))
}
