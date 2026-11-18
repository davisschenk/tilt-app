# Tilt Hydrometer Platform

A full-stack application for monitoring fermentation with [Tilt Wireless Hydrometers](https://tilthydrometer.com/). Track gravity, temperature, and brew sessions in real time from a modern web dashboard.

## Features

- **Real-time fermentation monitoring** — Gravity and temperature readings from Tilt hydrometers displayed on live charts
- **Brew session management** — Create, track, and archive brew sessions with OG/FG, style, and notes
- **Multi-hydrometer support** — Monitor up to 8 Tilt colors simultaneously with per-color charts
- **Fermentation event log** — Log events (dry hop, cold crash, etc.) with photo attachments
- **TOSNA nutrient scheduling** — Automated nutrient addition schedule with reminders
- **Alert system** — Gravity/temperature threshold alerts via webhook (Discord, Slack, etc.)
- **Dark/light theme** — System-aware theme with manual toggle
- **Single binary deployment** — Server, API, and web frontend served from one Rocket binary
- **Docker ready** — Multi-stage Dockerfile for easy self-hosting

## How It Works

```
  Tilt Hydrometer        ESP32 / Scanner          Server + Web UI
  (in fermenter)         (BLE scanner)            (your network)
 ┌──────────────┐       ┌──────────────┐        ┌──────────────────┐
 │  BLE iBeacon │──────►│  esp32-client│──HTTP──►│  Rocket API      │
 │  broadcast   │  BLE  │  (firmware)  │  JSON   │  PostgreSQL      │
 └──────────────┘       └──────────────┘         │  React Dashboard │
                                                  └──────────────────┘
```

The **ESP32 client** is the recommended scanner — it runs on a cheap ESP32 board, scans for Tilt BLE advertisements, and uploads readings to the server over WiFi. See [`esp32-client/README.md`](esp32-client/README.md) for setup instructions.

## Tech Stack

| Component      | Technology                                           |
|----------------|------------------------------------------------------|
| **Server**     | Rust, Rocket v0.5, SeaORM, PostgreSQL 16             |
| **Frontend**   | React 19, TypeScript, TailwindCSS v4, shadcn/ui      |
| **Charts**     | Recharts                                             |
| **ESP32 client** | Rust, esp-idf-svc, esp32-nimble                   |
| **Auth**       | OIDC (tested with [Authentik](https://goauthentik.io/)) |
| **Infra**      | Docker, cargo-chef                                   |

## Quick Start

### Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) 22+
- [Docker](https://docs.docker.com/get-docker/) with Compose
- [just](https://github.com/casey/just) (`cargo install just`)
- [sea-orm-cli](https://www.sea-ql.org/SeaORM/) (`cargo install sea-orm-cli`)
- An OIDC provider (e.g. [Authentik](https://goauthentik.io/), Auth0, Keycloak)

### Development

```bash
git clone https://github.com/davisschenk/tilt-app.git
cd tilt-app

# Install git hooks and create .env from template
just setup

# Edit .env — configure the OIDC_* values (any OpenID Connect provider)
nano .env

# Start database and run migrations
just db-up
just db-migrate

# Start the server (terminal 1)
just server

# Start the web dev server with hot reload (terminal 2)
just web
```

Visit **http://localhost:5173** for the dev frontend, or **http://localhost:8000** for the production-built version (after `just serve`).

### Available Commands

| Command           | Description                                          |
|-------------------|------------------------------------------------------|
| `just setup`      | First-time setup: install hooks, create .env         |
| `just serve`      | Build web + server, then run everything              |
| `just server`     | Run just the Rocket server                           |
| `just web`        | Run the Vite dev server (hot reload)                 |
| `just db-up`      | Start PostgreSQL via Docker                          |
| `just db-migrate` | Run database migrations                              |
| `just db-reset`   | Reset database (down, up, migrate)                   |
| `just test`       | Run all Rust + web tests                             |
| `just build`      | Build everything for production                      |
| `just ci`         | Full CI pipeline: fmt check + lint + tests           |

## Production Deployment

### Environment Variables

Copy `.env.example` to `.env` and fill in the required values:

| Variable | Description |
|----------|-------------|
| `DB_PASSWORD` | PostgreSQL password |
| `ROCKET_SECRET_KEY` | Secret for signing cookies (`openssl rand -base64 32`) |
| `OIDC_ISSUER_URL` | OIDC issuer URL |
| `OIDC_CLIENT_ID` | OIDC client ID |
| `OIDC_CLIENT_SECRET` | OIDC client secret |
| `OIDC_REDIRECT_URL` | OAuth2 callback URL (e.g. `https://yourdomain.com/api/v1/auth/callback`) |
| `FRONTEND_URL` | Your public URL for CORS (e.g. `https://yourdomain.com`) |
| `UPLOAD_DIR` | Path for photo attachment storage (default: `./uploads`) |

> **Security note:** `AUTH_MODE` defaults to `oidc`, so an unconfigured server refuses to start rather than running open. `AUTH_MODE=disabled` (local development only) also requires `AUTH_DISABLED_ACK=true`. The older `AUTHENTIK_*` variable names still work as fallbacks for the `OIDC_*` ones.

### Running a public instance (self-service sign-up)

TiltBrew delegates accounts to your OpenID Connect provider, so who can sign up is configured there, not in the app. Each account gets its own private brews, devices, alerts and API keys.

With Authentik, for example: create an *enrollment flow* (prompt stage for username/email/password, an email-verification stage, and a user-write stage), then select it as the **Enrollment flow** on your login page's identification stage. Authentik then shows a "Sign up" link, and the landing page's **Sign in or create an account** button takes people there. Add a captcha stage to the enrollment flow before opening it to the internet. Other providers (Auth0, Keycloak, ...) have equivalent self-registration settings. Set `FRONTEND_URL`, `ROCKET_SECRET_KEY` and https `OIDC_REDIRECT_URL` first; the server refuses to start in `oidc` mode without them.

### Docker Compose

```bash
cp .env.example .env
# Edit .env with your values
docker compose -f docker-compose.prod.yml up -d --build
```

Exposes port `8000` (override with `PORT=` env var). Put any reverse proxy (nginx, Caddy, Traefik, Cloudflare Tunnel, etc.) in front of it.

## ESP32 Client

See [`esp32-client/README.md`](esp32-client/README.md) for full setup, flashing, and configuration instructions.

## Project Structure

```
tilt-app/
├── server/                 # Rocket API server + SeaORM migrations
├── shared/                 # Common types and DTOs
├── web/                    # React 19 frontend (Vite + TypeScript)
├── esp32-client/           # ESP32 BLE scanner firmware
├── docker-compose.yml      # Development
├── docker-compose.prod.yml # Production
└── justfile                # Command runner recipes
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

[AGPL-3.0](LICENSE) — If you host a modified version of this software as a service, you must release your modifications under the same license.

## Automated releases

In GitHub Actions, open **Release → Run workflow**, select **master**, and choose
the version increment: **patch** (default), **minor**, or **major**. The workflow
calculates the next version from the highest existing stable `vX.Y.Z` tag;
for example, patch changes `1.0.0` to `1.0.1`, minor to `1.1.0`, and major to
`2.0.0`. Release runs are serialized so concurrent requests cannot choose the
same version.

The workflow runs the Rust formatting, lint and test checks and builds the web
frontend. It then builds signed ESP32 firmware, bundles the same firmware into
the server image, and publishes `ghcr.io/davisschenk/tilt-app:<version>` and
`latest`. Finally it creates the `v<version>` GitHub release with generated
release notes, flash and OTA images, `manifest.json`, and `SHA256SUMS`.

The repository's `OTA_SIGNING_KEY` Actions secret must contain the existing
Ed25519 private key in PEM format. Keep that key stable so installed devices
continue to trust OTA updates. Release builds embed the calculated version in the
firmware and its manifest; local builds use the ESP32 crate version.

Merges to master publish development images as `edge` and `sha-<commit>`,
leaving `latest` for tested releases. Creating a GitHub
release by hand does not start a second image build. A release does not deploy
or restart the homelab service; deployment remains managed by the homelab repo.
If publishing fails before release creation, rerun the failed jobs for the same
workflow run. Completed releases are immutable by convention; use a new version
for subsequent changes.
