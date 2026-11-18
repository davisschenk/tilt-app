# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Features

- **Web flasher & device setup** — New *Device Setup* page flashes and configures an ESP32 over Web Serial: pick a WiFi network, save, and the device joins automatically. An API key is created per device.
- **Serial provisioning** — The ESP32 firmware now stores WiFi, server URL and API key in NVS and accepts config over serial, so one generic image works for everyone (`cfg.toml` is optional).
- **Bundled firmware** — CI builds the firmware and the Docker image ships it (`FIRMWARE_DIR`); the server exposes `/api/v1/firmware/manifest` and serves it for OTA, with a relative URL.

### Changes

- Firmware version now comes from `esp32-client/Cargo.toml`; `firmware_version` was removed from `cfg.toml`.
- `just esp32-release` produces the merged and OTA images plus a manifest (the old `make release` copied the ELF, which could not be used for OTA).

## [1.0.0] — 2026-04-16

First public release.

### Features

- **Real-time fermentation monitoring** — Live gravity and temperature charts from Tilt Wireless Hydrometers
- **Brew session management** — Create, track, and archive sessions with OG/FG, style, and notes
- **Fermentation event log** — Log events (dry hop, cold crash, etc.) with photo attachments
- **TOSNA nutrient scheduling** — Automated nutrient addition schedule for mead/cider fermentation
- **Alert system** — Gravity/temperature threshold alerts delivered via webhook (Discord, Slack, etc.)
- **ESP32 BLE scanner client** — Rust firmware for ESP32 that scans for Tilt advertisements and uploads readings over WiFi
- **OIDC authentication** — Tested with Authentik; compatible with any standards-compliant OIDC provider
- **Single binary deployment** — Server, API, and React frontend served from one Rocket binary
- **Docker ready** — Multi-stage Dockerfile and Docker Compose for straightforward self-hosting
