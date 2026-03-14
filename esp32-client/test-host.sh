#!/usr/bin/env bash
# Run unit tests for pure-Rust modules on the host target.
# ESP-IDF modules (ble, wifi, http, config) cannot be tested on host.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TMPDIR="${TMPDIR:-/tmp}"
TEST_DIR="$TMPDIR/esp32-client-test-$$"

cleanup() { rm -rf "$TEST_DIR"; }
trap cleanup EXIT

mkdir -p "$TEST_DIR/src"

cat > "$TEST_DIR/Cargo.toml" << 'EOF'
[package]
name = "esp32-client-test"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
EOF

# Pin to stable toolchain so we don't pick up the esp toolchain
cat > "$TEST_DIR/rust-toolchain.toml" << 'EOF'
[toolchain]
channel = "stable"
EOF

cp "$SCRIPT_DIR/src/tilt.rs" "$TEST_DIR/src/tilt.rs"
cp "$SCRIPT_DIR/src/buffer.rs" "$TEST_DIR/src/buffer.rs"
cat > "$TEST_DIR/src/lib.rs" << 'EOF'
pub mod tilt;
pub mod buffer;
EOF

echo "Running tests in $TEST_DIR ..."
cd "$TEST_DIR" && cargo test "$@"
