#!/usr/bin/env bash
# Build the ESP32 firmware and package it for the web flasher / OTA.
# Output (../firmware-dist): merged image (flash at 0x0), app image (OTA), manifest.json.
# Requires the ESP Rust toolchain (see README) and `espflash`.
set -euo pipefail

cd "$(dirname "$0")/.."
OUT="../firmware-dist"
CHIP="esp32"
ELF="target/xtensa-esp32-espidf/release/esp32-client"
# ESP-IDF-built bootloader (has app-rollback enabled; espflash's bundled one does not).
BOOTLOADER="target/xtensa-esp32-espidf/release/bootloader.bin"

# Optional Ed25519 signing: OTA_SIGNING_KEY_FILE = PEM private key
#   generate with: openssl genpkey -algorithm ed25519 -out ota-signing.pem
# The matching public key is compiled into the firmware (devices only accept images
# signed with it). Without a key the build is unsigned and its OTA is refused by devices.
SIGN_KEY="${OTA_SIGNING_KEY_FILE:-}"
if [ -n "$SIGN_KEY" ]; then
  OTA_PUBLIC_KEY="$(openssl pkey -in "$SIGN_KEY" -pubout -outform DER | tail -c 32 | od -An -tx1 | tr -d ' \n')"
  export OTA_PUBLIC_KEY
else
  echo "WARNING: OTA_SIGNING_KEY_FILE not set - building UNSIGNED firmware (no OTA)" >&2
fi

cargo build --release

VERSION=${TILT_RELEASE_VERSION:-$(grep -m1 '^version' Cargo.toml | sed 's/.*= *"\(.*\)"/\1/')}
mkdir -p "$OUT"
MERGED="tilt-esp32-$VERSION-merged.bin"
APP="tilt-esp32-$VERSION-app.bin"
rm -f "$OUT"/tilt-esp32-*.bin "$OUT/manifest.json"

espflash save-image --chip "$CHIP" --merge --flash-size 4mb \
  --bootloader "$BOOTLOADER" --partition-table partitions.csv "$ELF" "$OUT/$MERGED"
espflash save-image --chip "$CHIP" --flash-size 4mb \
  --bootloader "$BOOTLOADER" --partition-table partitions.csv "$ELF" "$OUT/$APP"

APP_SHA="$(sha256sum "$OUT/$APP" | cut -d' ' -f1)"
APP_SIZE="$(stat -c %s "$OUT/$APP")"
SIG_FIELD=""
if [ -n "$SIGN_KEY" ]; then
  # Must match signed_message() in src/ota_logic.rs
  MSG="$(mktemp)"
  printf 'tilt-ota-v1:%s:%s:%s' "$VERSION" "$APP_SIZE" "$APP_SHA" > "$MSG"
  SIG="$(openssl pkeyutl -sign -inkey "$SIGN_KEY" -rawin -in "$MSG" | od -An -tx1 | tr -d ' \n')"
  rm -f "$MSG"
  SIG_FIELD=$(printf ',"signature":"%s"' "$SIG")
fi

file_json() {
  local path="$OUT/$1"
  printf '{"name":"%s","sha256":"%s","size":%s}' \
    "$1" "$(sha256sum "$path" | cut -d' ' -f1)" "$(stat -c %s "$path")"
}

printf '{"version":"%s","chip":"%s","builtAt":"%s","files":{"merged":%s,"app":%s}%s}\n' \
  "$VERSION" "$CHIP" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  "$(file_json "$MERGED")" "$(file_json "$APP")" "$SIG_FIELD" > "$OUT/manifest.json"

echo "Firmware $VERSION written to $(cd "$OUT" && pwd)"
ls -lh "$OUT"
