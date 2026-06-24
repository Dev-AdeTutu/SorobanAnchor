#!/bin/bash
# Verification helper for anchor info discovery tests.
# Current implementation lives in src/contract.rs and tests/anchor_info_discovery_tests.rs.

echo "Running anchor info discovery tests..."
echo ""

cargo test --test anchor_info_discovery_tests
