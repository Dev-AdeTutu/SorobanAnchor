$ErrorActionPreference = "Stop"

$sourceDateEpoch = if ($env:SOURCE_DATE_EPOCH) { $env:SOURCE_DATE_EPOCH } else { 1717200000 }
$wasmTarget = "wasm32-unknown-unknown"
$wasmOut = "target\$wasmTarget\release\anchorkit.wasm"

Write-Host "=== Reproducible Build Verification ==="
Write-Host "Source Date Epoch: $sourceDateEpoch"

# Build 1
Write-Host ""
Write-Host "Building first time..."
$env:SOURCE_DATE_EPOCH = $sourceDateEpoch
cargo clean
cargo build --release --target $wasmTarget --no-default-features --features wasm
$hash1 = (Get-FileHash -Path $wasmOut -Algorithm SHA256).Hash.ToLower()
Write-Host "First build hash: $hash1"
Copy-Item -Path $wasmOut -Destination "build1.wasm"

# Build 2
Write-Host ""
Write-Host "Building second time..."
cargo clean
cargo build --release --target $wasmTarget --no-default-features --features wasm
$hash2 = (Get-FileHash -Path $wasmOut -Algorithm SHA256).Hash.ToLower()
Write-Host "Second build hash: $hash2"
Copy-Item -Path $wasmOut -Destination "build2.wasm"

# Compare
Write-Host ""
if ($hash1 -eq $hash2) {
    Write-Host "✅ SUCCESS: Builds are reproducible!"
    Write-Host "   Hash: $hash1"
    Remove-Item -Path "build1.wasm", "build2.wasm" -ErrorAction SilentlyContinue
    exit 0
} else {
    Write-Host "❌ FAILURE: Builds are NOT reproducible!"
    Write-Host "   First hash:  $hash1"
    Write-Host "   Second hash: $hash2"
    Write-Host "   Artifacts preserved as build1.wasm and build2.wasm"
    exit 1
}
