# SorobanAnchor Production Runbook

This document describes the complete lifecycle for building, deploying, validating, upgrading, and recovering SorobanAnchor contracts in production.

---

## Table of Contents

1. [Pre-Deployment Preparation](#pre-deployment-preparation)
2. [Build and Packaging](#build-and-packaging)
3. [Deployment](#deployment)
4. [Post-Deployment Validation](#post-deployment-validation)
5. [Upgrade Procedure](#upgrade-procedure)
6. [Failure Recovery / Rollback](#failure-recovery--rollback)
7. [Troubleshooting Common Issues](#troubleshooting-common-issues)

---

## Pre-Deployment Preparation

### Prerequisites

Ensure the following tools are installed:
- Rust 1.75+ with `wasm32-unknown-unknown` target
- Python 3.7+ (for config validation)
- `soroban-cli` (for contract deployment)
- Binaryen (optional, for WASM optimization)

### Environment Variables

Set the following environment variables:
```bash
# For testnet
export SOROBAN_NETWORK=testnet
export SOROBAN_RPC_URL=https://soroban-testnet.stellar.org:443
export SOROBAN_NETWORK_PASSPHRASE="Test SDF Network ; September 2015"

# For mainnet
# export SOROBAN_NETWORK=mainnet
# export SOROBAN_RPC_URL=https://soroban-mainnet.stellar.org:443
# export SOROBAN_NETWORK_PASSPHRASE="Public Global Stellar Network ; September 2015"

export ANCHOR_ADMIN_SECRET=<your-admin-secret-key>
```

### Configuration Validation

Run pre-deployment validation:
```bash
./scripts/pre_deploy_validate.sh
```

This validates all config files against the schema and checks dependencies.

---

## Build and Packaging

### Build Steps

1. Clean previous builds:
```bash
cargo clean
```

2. Build release artifacts:
```bash
make release
```
This executes `scripts/package_release.sh` which:
- Installs required Rust targets
- Builds native CLI
- Builds optimized WASM contract
- Creates release bundle

3. Validate the release bundle:
```bash
make release-validate
```

### Artifacts

The release bundle in `dist/anchorkit-<VERSION>/` contains:
- `anchorkit` - CLI binary
- `anchorkit.wasm` - Optimized Soroban contract
- `schemas/config_schema.json` - Config schema
- `configs/` - Example anchor configurations
- `docs/` - Documentation

---

## Deployment

### Initial Deployment

1. Verify the WASM checksum:
```bash
sha256sum dist/anchorkit-<VERSION>/anchorkit.wasm
```

2. Deploy using the CLI:
```bash
./target/release/anchorkit deploy --network $SOROBAN_NETWORK
```

3. Initialize the contract with admin address:
```bash
# Using soroban-cli
soroban contract invoke \
  --id <deployed-contract-id> \
  --source $ANCHOR_ADMIN_SECRET \
  --network $SOROBAN_NETWORK \
  -- \
  initialize \
  --admin <admin-address>
```

### Record Deployment Details

Save the following information:
- Contract ID
- Deployment block height
- WASM SHA-256 checksum
- Admin address

---

## Post-Deployment Validation

1. Verify contract initialization:
```bash
soroban contract invoke \
  --id <contract-id> \
  --source $ANCHOR_ADMIN_SECRET \
  --network $SOROBAN_NETWORK \
  -- \
  get_admin
```

2. Run health checks:
```bash
./target/release/anchorkit doctor
```

3. Test basic functionality:
```bash
# Test registering an attestor (dry run)
./target/release/anchorkit register --address <test-attestor> --services deposits --dry-run
```

---

## Upgrade Procedure

### Pre-Upgrade Steps

1. Create a configuration snapshot (if applicable):
```rust
// Using the service management API
let snapshot_id = ServiceManager::create_snapshot(
    &env,
    &anchor,
    &current_services,
    "pre_upgrade_2024_06_01",
);
```

2. Build the new version:
```bash
make release
```

3. Verify the new WASM checksum against published release notes.

### Perform Upgrade

1. Deploy the new WASM:
```bash
soroban contract install \
  --wasm dist/anchorkit-<NEW-VERSION>/anchorkit.wasm \
  --source $ANCHOR_ADMIN_SECRET \
  --network $SOROBAN_NETWORK
```

2. Upgrade the contract:
```bash
soroban contract invoke \
  --id <contract-id> \
  --source $ANCHOR_ADMIN_SECRET \
  --network $SOROBAN_NETWORK \
  -- \
  upgrade \
  --wasm_hash <new-wasm-hash>
```

### Post-Upgrade Validation

Repeat all steps in [Post-Deployment Validation](#post-deployment-validation).

---

## Failure Recovery / Rollback

### Immediate Actions

If a deployment or upgrade causes issues:

1. **Pause affected services** (using service management API):
```rust
ServiceManager::disable_all_services(&env, &anchor, &all_services);
```

2. **Collect diagnostic information**:
   - Error logs
   - Transaction hashes
   - Contract state snapshots

### Rollback Procedure

1. Locate the previous release bundle in `dist/`.

2. Reinstall the previous WASM:
```bash
soroban contract install \
  --wasm dist/anchorkit-<OLD-VERSION>/anchorkit.wasm \
  --source $ANCHOR_ADMIN_SECRET \
  --network $SOROBAN_NETWORK
```

3. Rollback the contract:
```bash
soroban contract invoke \
  --id <contract-id> \
  --source $ANCHOR_ADMIN_SECRET \
  --network $SOROBAN_NETWORK \
  -- \
  upgrade \
  --wasm_hash <old-wasm-hash>
```

4. Restore service configuration from snapshot:
```rust
ServiceManager::rollback_to_snapshot(&env, snapshot_id);
```

5. Verify rollback with validation steps.

---

## Troubleshooting Common Issues

### Issue: Deployment Fails with "Invalid WASM"

**Solution**:
1. Check that you're using `--no-default-features --features wasm`
2. Verify the WASM is optimized with `wasm-opt -Oz`
3. Check the WASM size (Soroban has size limits)

### Issue: Contract Invocation Fails with "Unauthorized"

**Solution**:
1. Verify the source account has admin privileges
2. Check the SEP-10 JWT (if applicable) is valid and not expired
3. Verify the multi-signature setup (if used)

### Issue: Service State Not Persisting

**Solution**:
1. Check that `enable_service()` returns `true`
2. Verify the anchor address is correct
3. Check contract storage limits

### Issue: Config Validation Fails

**Solution**:
1. Run `./scripts/validate_all.sh` for detailed errors
2. Check config against `config_schema.json`
3. Ensure required fields are present

---

## References

- [README.md](../README.md)
- [Governance and Security](./governance-and-security.md)
- [Service Management](./service-management.md)
- [Contract Functions](./CONTRACT_FUNCTIONS.md)

