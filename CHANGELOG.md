# Changelog

All notable changes to this project will be documented in this file.

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
