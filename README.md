# SorobanAnchor

A Soroban smart contract SDK for Stellar anchors. Handles attestation management, SEP-6 deposit/withdrawal flows, SEP-10 JWT authentication, anchor routing, rate limiting, and transaction state tracking — all in a `no_std` Rust library compiled to WASM.

## What it does

- Registers and revokes attestors with SEP-10 JWT verification
- Submits and retrieves on-chain attestations with replay attack protection
- Normalizes SEP-6 deposit, withdrawal, and transaction status responses across anchors
- Verifies SEP-10 EdDSA JWTs on-chain using stored Ed25519 public keys
- Routes requests across multiple anchors by reputation, fees, and settlement time
- Caches anchor metadata and stellar.toml capabilities with TTL-based expiry
- Tracks transaction state transitions with full audit logging
- Propagates request IDs and tracing spans across operations
- Enforces rate limits and configurable retry/backoff strategies
- Validates anchor domain endpoints and response schemas

## Project structure

```
src/                        # Core library
  lib.rs                    # Public API surface
  contract.rs               # Soroban contract (attestations, sessions, quotes, routing)
  sep6.rs                   # SEP-6 deposit/withdrawal normalization
  sep10_jwt.rs              # SEP-10 JWT verification (EdDSA, no_std)
  domain_validator.rs       # Anchor domain/endpoint validation
  errors.rs                 # Stable error codes
  rate_limiter.rs           # Rate limiting
  response_validator.rs     # Response schema validation
  retry.rs                  # Retry with exponential backoff
  transaction_state_tracker.rs
  deterministic_hash.rs     # Canonical SHA-256 payload hashing

tests/                      # Integration and unit tests
configs/                    # Example anchor configurations (JSON + TOML)
examples/                   # Rust and shell usage examples
scripts/                    # Build, validation, and deploy scripts
docs/                       # Feature and guide documentation
test_snapshots/             # Snapshot fixtures for deterministic tests
```

## Building

```bash
cargo build --release
```

For WASM output (Soroban deployment):

```bash
cargo build --release --target wasm32-unknown-unknown --no-default-features --features wasm
```

## Testing

```bash
cargo test
```

Run the stress-test suite (excluded from normal CI):

```bash
cargo test --features stress-tests
```

## Feature flags

The crate uses four feature flags to control which modules are compiled.

| Flag | Default | Purpose |
|------|---------|---------|
| `std` | ✓ | Enables filesystem-based config loading (`load_runtime_config_file`, `RuntimeConfig`). Disable for pure no_std environments. |
| `wasm` | — | Soroban on-chain deployment target. Excludes all HTTP/host modules (`sep6`, `sep24`, `sep38`, `webhook`, `streaming_monitor`); only the contract, error types, rate limiter, and cryptographic utilities are compiled. |
| `mock-only` | — | Enables the `mock` module with pre-built valid fixtures for every response type. Use in integration tests and CI pipelines that have no live anchor. |
| `stress-tests` | — | Enables `tests/load_simulation_tests.rs` — high-concurrency and throughput tests excluded from normal CI. |

### Build variants

```bash
# Native development (default features)
cargo build

# Soroban on-chain WASM deployment
cargo build --release \
  --target wasm32-unknown-unknown \
  --no-default-features --features wasm

# Testing with mock fixtures (no live anchor)
cargo test --features mock-only

# Testing with mock fixtures and config (std + mock)
cargo test --features std,mock-only

# Full suite including stress tests
cargo test --features std,mock-only,stress-tests

# Library only, no std (no_std verification)
cargo check --no-default-features
```

### Using mock fixtures

```rust
use anchorkit::mock::{mock_deposit_response, mock_firm_quote};
use anchorkit::{initiate_deposit, sep38::request_firm_quote};

// Test the deposit parsing pipeline without a live anchor
let raw = mock_deposit_response();
let deposit = initiate_deposit(raw).unwrap();
assert_eq!(deposit.transaction_id, "mock-txn-001");

// Test SEP-38 quote parsing
let raw_quote = mock_firm_quote();
let quote = request_firm_quote(raw_quote, 1_700_000_000).unwrap();
assert!(!quote.id.is_empty());
```

## CLI

```bash
# Deploy to testnet
anchorkit deploy --network testnet

# Register an attestor
anchorkit register --address GANCHOR123... --services deposits,withdrawals,kyc

# Submit an attestation
anchorkit attest --subject GUSER123... --payload-hash abc123...

# Check environment setup
anchorkit doctor
```

## Key APIs

```rust
// SEP-6: normalize a raw anchor deposit response
let response = initiate_deposit(raw)?;

// SEP-10: verify an anchor JWT on-chain
contract.verify_sep10_token(token, issuer);

// Submit an attestation (replay-protected)
let id = contract.submit_attestation(issuer, subject, timestamp, payload_hash, sig);

// Route across anchors by lowest fee
let best = contract.route(options);

// Track transaction state
tracker.transition(tx_id, TransactionStatus::Completed);
```

## Configuration

Anchor configs live in `configs/` as JSON or TOML. Validate them with:

```bash
./scripts/validate_all.sh
```

Schema reference: `config_schema.json`

## License

MIT
