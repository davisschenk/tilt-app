#!/usr/bin/env bash
# Build the ESP32 firmware and package it for the web flasher / OTA.
# Output (../firmware-dist): merged image (flash at 0x0), app image (OTA), manifest.json.
# Requires the ESP Rust toolchain (see README) and `espflash`.
set -euo pipefail

cd "$(dirname "$0")/.."
OUT="../firmware-dist"
CHIP="esp32"
ELF="target/xtensa-esp32-espidf/release/esp32-client"

cargo build --release

VERSION=$(grep -m1 '^version' Cargo.toml | sed 's/.*= *"\(.*\)"/\1/')
mkdir -p "$OUT"
MERGED="tilt-esp32-$VERSION-merged.bin"
APP="tilt-esp32-$VERSION-app.bin"
rm -f "$OUT"/tilt-esp32-*.bin "$OUT/manifest.json"

espflash save-image --chip "$CHIP" --merge --flash-size 4mb \
  --partition-table partitions.csv "$ELF" "$OUT/$MERGED"
espflash save-image --chip "$CHIP" --flash-size 4mb \
  --partition-table partitions.csv "$ELF" "$OUT/$APP"

file_json() {
  local path="$OUT/$1"
  printf '{"name":"%s","sha256":"%s","size":%s}' \
    "$1" "$(sha256sum "$path" | cut -d' ' -f1)" "$(stat -c %s "$path")"
}

printf '{"version":"%s","chip":"%s","builtAt":"%s","files":{"merged":%s,"app":%s}}\n' \
  "$VERSION" "$CHIP" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  "$(file_json "$MERGED")" "$(file_json "$APP")" > "$OUT/manifest.json"

echo "Firmware $VERSION written to $(cd "$OUT" && pwd)"
ls -lh "$OUT"
