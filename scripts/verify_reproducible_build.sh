#!/bin/bash

set -e

SOURCE_DATE_EPOCH=${SOURCE_DATE_EPOCH:-1717200000}
WASM_TARGET=wasm32-unknown-unknown
WASM_OUT=target/$WASM_TARGET/release/anchorkit.wasm

echo "=== Reproducible Build Verification ==="
echo "Source Date Epoch: $SOURCE_DATE_EPOCH"

# Build 1
echo ""
echo "Building first time..."
export SOURCE_DATE_EPOCH
cargo clean
cargo build --release --target $WASM_TARGET --no-default-features --features wasm
HASH1=$(sha256sum $WASM_OUT | awk '{print $1}')
echo "First build hash: $HASH1"
cp $WASM_OUT build1.wasm

# Build 2
echo ""
echo "Building second time..."
cargo clean
cargo build --release --target $WASM_TARGET --no-default-features --features wasm
HASH2=$(sha256sum $WASM_OUT | awk '{print $1}')
echo "Second build hash: $HASH2"
cp $WASM_OUT build2.wasm

# Compare
echo ""
if [ "$HASH1" = "$HASH2" ]; then
    echo "✅ SUCCESS: Builds are reproducible!"
    echo "   Hash: $HASH1"
    rm -f build1.wasm build2.wasm
    exit 0
else
    echo "❌ FAILURE: Builds are NOT reproducible!"
    echo "   First hash:  $HASH1"
    echo "   Second hash: $HASH2"
    echo "   Artifacts preserved as build1.wasm and build2.wasm"
    exit 1
fi
