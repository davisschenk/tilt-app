# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Features

- **Web flasher & device setup** — New *Device Setup* page flashes and configures an ESP32 over Web Serial: pick a WiFi network, save, and the device joins automatically. An API key is created per device.
- **Serial provisioning** — The ESP32 firmware now stores WiFi, server URL and API key in NVS and accepts config over serial, so one generic image works for everyone (`cfg.toml` is optional).
- **Bundled firmware** — CI builds the firmware and the Docker image ships it (`FIRMWARE_DIR`); the server exposes `/api/v1/firmware/manifest` and serves it for OTA, with a relative URL.
- **Accounts** — Per-user data ownership, account export and deletion, privacy and terms pages, a landing page with first-run onboarding, and generic `OIDC_*` settings for any OpenID Connect provider.
- **Measurement preferences** — Account-wide °C/°F and SG/°P for display and entry. APIs, webhooks' raw values and CSV exports stay in °F and SG.
- **Duplicate batches** — Start a new brew from an existing one's yeast, targets, batch size, nutrients and notes, on a free hydrometer.
- **Scanner health** — ESP32 heartbeats (about once a minute) show Wi-Fi/Bluetooth signal, upload backlog and reading gaps, separately from hydrometer reception.
- **Batch reports** — Publish a read-only snapshot link for a brew (refresh or revoke it, print to PDF). Notes, tasting notes and photos are opt-in.

### Operations

- **Health checks** — `/api/v1/health` (liveness) and `/api/v1/health/ready` (pings the database, 503 when it's unreachable). The production compose healthcheck uses readiness.
- **Prometheus metrics** — `GET /metrics`, enabled by setting `METRICS_TOKEN` (bearer auth; 404 when unset): request latency by route and status, readings accepted/capped, rate-limit refusals, webhook deliveries, database pool usage.
- **Readings retention** — Opt-in `READINGS_RETENTION_DAYS` deletes readings that aren't part of any brew once they're older than N days, hourly and in small batches. Brew history is never deleted.
- **Backups, TLS and releases** — Nightly verified Postgres and photo backups (optionally copied off-host with rclone), an optional bundled Caddy for HTTPS, pinned image versions, JSON logs, and automated versioned releases with signed firmware.
- **Dependency auditing** — `cargo audit` and `npm audit` run on every PR, weekly, and before each release; Dependabot proposes grouped updates.

### Security

- **OTA hardening** — Devices now only install strictly newer firmware whose Ed25519-signed manifest and SHA-256 verify (key via `OTA_SIGNING_KEY_FILE` / `OTA_SIGNING_KEY` CI secret). Removed the `OTA_FIRMWARE_URL`/`OTA_FIRMWARE_VERSION` overrides.
- **OTA reliability** — Fixed a stack overflow in the download buffer, enabled bootloader rollback (marks the image valid after the first scan cycle), and grew both OTA slots to 0x1F0000. **Existing boards need one USB/web reflash** to pick up the new partition table and bootloader.

- **Fail-closed auth** — With `AUTH_MODE=oidc` (the default) the server refuses to start without its OIDC settings, `ROCKET_SECRET_KEY` and `FRONTEND_URL`; the session cookie is secured.
- **Webhook SSRF** — Alert webhooks can't target loopback or private addresses.
- **Attachments** — Uploads are served sandboxed with `Content-Disposition`, closing a stored-XSS path; public report photos use the same protections.
- **Content-Security-Policy** — `script-src` no longer allows inline scripts; added `object-src`, `base-uri`, `frame-ancestors` and `form-action`; HSTS is only sent over HTTPS. Every end-to-end test now fails on any CSP violation.
- **Rate limits** — Per-user limits and resource caps. Per-IP limits now apply only to sign-in and reading uploads, so browsing the web UI no longer uses up the quota your scanner's uploads depend on. The server refuses to start if a limited route has no limiter, and idle limiter entries are pruned.
- **Dependencies** — Updated rustls, quinn, h2, tokio, react-router and others for published advisories; dropped HTTP/2 support in Rocket (removes the vulnerable h2 0.3); upgraded ECharts to 6 and removed the unused chart.js packages.

### Fixes

- Charts now render on phones (the brew chart was 0×0 at phone width) and resize correctly after loading.
- Requests no longer panic on edge-case input (downsampling, plateau alerts, limiter setup).
- Reading uploads return 500 instead of silently storing readings without their brew when the active-brew lookup fails.
- Account export now includes published reports and scanner ids; a test checks that every table is covered by export and deletion.

### Changes

- Firmware version now comes from `esp32-client/Cargo.toml`; `firmware_version` was removed from `cfg.toml`.
- `just esp32-release` produces the merged and OTA images plus a manifest (the old `make release` copied the ELF, which could not be used for OTA).
- Faster readings queries with composite indexes and streaming downsampling.
- Discord/Slack alert text follows the account's units; the metric label reads "Temperature (°F)" again for default settings.
- `just audit` runs both dependency audits locally; clippy now lints test code too.

### Upgrade notes

- **Deploy the server before updating ESP32 firmware.** Migrations run at startup and add preferences, scanners, reports and an optional scanner link on readings; older firmware keeps working.
- New optional settings: `METRICS_TOKEN`, `READINGS_RETENTION_DAYS` (both off when unset).
- If you scrape `/metrics` through the bundled Caddy, the endpoint is public apart from the token, so use a long random token.
- Reverse proxies must set `X-Forwarded-Proto: https` for HSTS to be sent (Caddy and Cloudflare do).

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
