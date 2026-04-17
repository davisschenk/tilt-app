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

# Edit .env — configure AUTHENTIK_* (or equivalent OIDC) values
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
| `AUTHENTIK_ISSUER_URL` | OIDC issuer URL |
| `AUTHENTIK_CLIENT_ID` | OIDC client ID |
| `AUTHENTIK_CLIENT_SECRET` | OIDC client secret |
| `AUTHENTIK_REDIRECT_URL` | OAuth2 callback URL (e.g. `https://yourdomain.com/api/v1/auth/callback`) |
| `FRONTEND_URL` | Your public URL for CORS (e.g. `https://yourdomain.com`) |
| `UPLOAD_DIR` | Path for photo attachment storage (default: `./uploads`) |

> **Security note:** If `AUTHENTIK_ISSUER_URL` is left unset, the server starts with authentication **disabled** — all API routes are accessible without login. This is a convenience for local development only. Always configure OIDC before exposing the server on a network.

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
