# Reproducible Builds Guide

This document provides guidance and tools to ensure reproducible builds of AnchorKit, increasing security and production trust.

## Prerequisites

### Toolchain Versions

The following toolchain and dependency versions are recommended for reproducible builds:

- **Rust**: 1.80.0 (or compatible)
- **Cargo**: Version matching Rust
- **Target**: wasm32-unknown-unknown for Soroban contracts

To install the required toolchain:
```bash
rustup install 1.80.0
rustup default 1.80.0
rustup target add wasm32-unknown-unknown
```

### Environment Variables

To stabilize build outputs, set the following environment variables:

- `SOURCE_DATE_EPOCH`: Unix timestamp for deterministic file timestamps
- `CARGO_TARGET_DIR`: Optional, fixed target directory

## Reproducible Build Configuration

### Cargo Profile Settings

The `release` profile in `Cargo.toml` is already configured for reproducibility:
- `codegen-units = 1`: Ensures single code generation unit
- `lto = true`: Link-time optimization
- `strip = "symbols"`: Consistent symbol stripping

## Using the Reproducible Build Targets

### Build with Reproducibility Checks

```bash
# Build WASM contract with reproducibility checks
make reproducible-wasm

# Verify artifact consistency across two builds
make verify-reproducible
```

### Manual Reproducibility Check

To manually verify reproducibility:

```bash
# First build
export SOURCE_DATE_EPOCH=1717200000
cargo build --release --target wasm32-unknown-unknown --no-default-features --features wasm
cp target/wasm32-unknown-unknown/release/anchorkit.wasm build1.wasm

# Clean and rebuild
cargo clean
cargo build --release --target wasm32-unknown-unknown --no-default-features --features wasm
cp target/wasm32-unknown-unknown/release/anchorkit.wasm build2.wasm

# Compare hashes
sha256sum build1.wasm build2.wasm
```

## Scripts

- `scripts/verify_reproducible_build.sh`: Unix script to verify build reproducibility
- `scripts/verify_reproducible_build.ps1`: Windows PowerShell equivalent

## Troubleshooting

If builds are not reproducible:
1. Ensure `SOURCE_DATE_EPOCH` is set consistently
2. Verify the same Rust toolchain version is used
3. Check for any filesystem path differences
4. Clear Cargo cache with `cargo clean` between builds
