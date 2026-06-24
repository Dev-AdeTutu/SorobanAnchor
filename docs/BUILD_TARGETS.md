# AnchorKit Build Targets

This document describes all available build targets for AnchorKit. Use these targets to build, test, format, and lint the codebase.

## Quick Reference

```bash
make fmt               # Auto-fix code formatting
make lint              # Run linting checks
make test              # Run all tests
make build             # Build release binary
make wasm              # Build WASM target
```

## Build Targets

### `make build`

Build the release binary for the native platform.

```bash
make build
```

**What it does:**
- Compiles with `--release` flag
- Optimizes for production use
- Outputs to `target/release/anchorkit`

### `make test`

Run all unit and integration tests.

```bash
make test
```

**What it does:**
- Runs all tests in `tests/` directory
- Runs doc tests
- Runs tests in all modules

**Options:**
```bash
cargo test -- --nocapture    # Show test output
cargo test test_name         # Run specific test
cargo test --lib             # Run library tests only
```

### `make wasm`

Build the WASM target for Soroban deployment.

```bash
make wasm
```

**What it does:**
- Compiles for `wasm32-unknown-unknown` target
- Disables default features (std)
- Enables `wasm` feature
- Optimizes for minimal binary size
- Displays final WASM binary size

**Output:**
```
target/wasm32-unknown-unknown/release/anchorkit.wasm
```

## Formatting Targets (rustfmt)

### `make fmt`

Auto-fix code formatting for all targets.

```bash
make fmt
```

**What it does:**
- Runs `cargo fmt --all`
- Modifies files in-place to match `rustfmt.toml` rules
- Applies to all Rust files in the project

**Configuration:** See `rustfmt.toml` for formatting rules

### `make fmt-check`

Check code formatting without modifying files.

```bash
make fmt-check
```

**What it does:**
- Runs `cargo fmt --all -- --check`
- Reports formatting issues without modifying
- Useful for CI/CD pipelines

**Exit codes:**
- 0 = All files properly formatted
- 1 = Formatting issues found

## Linting Targets (clippy)

### `make lint`

Run clippy on all targets with strict warnings-as-errors policy.

```bash
make lint
```

**What it does:**
- Runs `cargo clippy --all-targets --all-features -- -D warnings`
- Checks all targets (lib, bins, tests, examples)
- Treats all warnings as errors (fails on any warning)
- Enables all features for comprehensive checking

**Configuration:** See `.clippy.toml` for linting rules

## Quality Check Scripts

Alternative to Makefile targets, use provided scripts:

### Unix/Linux/macOS

```bash
bash scripts/quality-check.sh all      # All targets
bash scripts/quality-check.sh native   # Native only
bash scripts/quality-check.sh wasm     # WASM only
```

### Windows

```bash
scripts\quality-check.bat all          # All targets
scripts\quality-check.bat native       # Native only
scripts\quality-check.bat wasm         # WASM only
```

## Typical Workflows

### Before committing code

```bash
make check
```

Runs formatting check, linting, and tests. Fix any issues and commit.

### Auto-fix formatting issues

```bash
make fmt
make check
```

Auto-fix formatting, then run all checks.

### Quick native-only check

```bash
make lint-native
make test
```

Check native code without WASM overhead.

### Validate WASM code

```bash
make check-wasm
make wasm
```

Check WASM formatting and linting, then build.

### Full validation before push

```bash
make check
make wasm
```

Run all checks and build both native and WASM targets.

## Feature Flags

### Default features

```bash
cargo build --release
```

Enables: `std` (standard library support)

### WASM features

```bash
cargo build --release --target wasm32-unknown-unknown --no-default-features --features wasm
```

Disables: `std`
Enables: `wasm`

### All features

```bash
cargo build --all-features
```

Enables all features for comprehensive testing.

## Troubleshooting

### Clippy is slow

Run on specific targets:

```bash
make lint-native    # Skip WASM
cargo clippy --lib  # Library only
```

### Formatting conflicts

If `rustfmt` and `clippy` disagree:

```bash
make fmt            # Run rustfmt first
make lint           # Then run clippy
```

### WASM build fails

Ensure WASM target is installed:

```bash
rustup target add wasm32-unknown-unknown
```

### Tests fail

Run with output:

```bash
cargo test -- --nocapture
```

## Configuration Files

- **`rustfmt.toml`** — Formatting rules
- **`.clippy.toml`** — Linting rules
- **`Cargo.toml`** — Build configuration and dependencies
- **`Makefile`** — Build targets

## See Also

- [CONTRIBUTING.md](CONTRIBUTING.md) — Contributor guidelines
- [CODE_QUALITY.md](CODE_QUALITY.md) — Code quality standards
- [README.md](../README.md) — Project overview
