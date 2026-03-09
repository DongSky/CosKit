#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

echo "=== CosKit (Rust/Tauri) macOS Build ==="
echo ""

# Check prerequisites
command -v node >/dev/null 2>&1 || { echo "Error: node not found. Install Node.js first."; exit 1; }
command -v cargo >/dev/null 2>&1 || { source "$HOME/.cargo/env" 2>/dev/null || true; }
command -v cargo >/dev/null 2>&1 || { echo "Error: cargo not found. Install Rust: https://rustup.rs"; exit 1; }

# Install npm dependencies (Tauri CLI)
echo "[1/3] Installing npm dependencies..."
npm install

# Build Tauri app (release mode)
echo "[2/3] Building Tauri app (release)..."
npx tauri build

# Show output
echo ""
echo "[3/3] Build complete!"
echo ""
echo "Output:"
find src-tauri/target/release/bundle -name '*.dmg' -o -name '*.app' 2>/dev/null | head -5
echo ""
echo "All bundles are in: src-tauri/target/release/bundle/"
