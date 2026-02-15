# Tilt Hydrometer Platform

A full-stack application for monitoring fermentation with [Tilt Wireless Hydrometers](https://tilthydrometer.com/). Track gravity, temperature, and brew sessions in real time from a modern web dashboard.

## Features

- **Real-time fermentation monitoring** — Gravity and temperature readings from Tilt hydrometers displayed on live charts
- **Brew session management** — Create, track, and archive brew sessions with OG/FG, style, and markdown notes
- **Multi-hydrometer support** — Monitor up to 8 Tilt colors simultaneously with per-color charts
- **Dark/light theme** — System-aware theme with manual toggle
- **Single binary deployment** — Server, API, and web frontend all served from one Rocket binary
- **Docker ready** — Multi-stage Dockerfile with Cloudflare tunnel support for homelab hosting

## How It Works

```
  Tilt Hydrometer          Raspberry Pi             Server + Web UI
  (in fermenter)           (BLE scanner)            (your network)
 ┌──────────────┐        ┌──────────────┐        ┌──────────────────┐
 │  BLE iBeacon │───────►│    Client    │──HTTP──►│  Rocket API      │
 │  broadcast   │  BLE   │    binary    │  JSON   │  PostgreSQL      │
 └──────────────┘        └──────────────┘        │  React Dashboard │
                                                  └──────────────────┘
```

The **client** runs on a Raspberry Pi, scans for Tilt BLE advertisements, and uploads readings to the **server**. The server stores data in PostgreSQL and serves both the REST API and the React web dashboard.

## Tech Stack

| Component    | Technology                                           |
|--------------|------------------------------------------------------|
| **Server**   | Rust, Rocket v0.5, SeaORM, PostgreSQL 16             |
| **Client**   | Rust, btleplug (BLE), reqwest, clap                  |
| **Frontend** | React 19, TypeScript, TailwindCSS v4, shadcn/ui      |
| **Charts**   | Recharts                                             |
| **Infra**    | Docker, cargo-chef, Cloudflare Tunnel                |

## Quick Start

### Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) 22+
- [Docker](https://docs.docker.com/get-docker/) (for PostgreSQL)
- [just](https://github.com/casey/just) (command runner)
- [sea-orm-cli](https://www.sea-ql.org/SeaORM/) (`cargo install sea-orm-cli`)

### Development

```bash
# Clone and enter the project
git clone https://github.com/davisschenk/tilt-hydrometer-web.git
cd tilt-hydrometer-web

# Copy and configure environment
cp .env.example .env

# Start database and run migrations
just db-up
just db-migrate

# Start the server (builds web frontend + serves everything)
just serve
```

Visit **http://localhost:8000** to see the dashboard.

To simulate Tilt readings without hardware (in a separate terminal):

```bash
just client-sim
```

### Available Commands

| Command           | Description                                          |
|-------------------|------------------------------------------------------|
| `just serve`      | Build web + server, then run everything               |
| `just server`     | Run just the Rocket server                           |
| `just web`        | Run the Vite dev server (hot reload)                 |
| `just client-sim` | Simulate Tilt readings (Red + Blue)                  |
| `just db-up`      | Start PostgreSQL via Docker                          |
| `just db-migrate` | Run database migrations                              |
| `just db-reset`   | Reset database (down, up, migrate)                   |
| `just test`       | Run all Rust + web tests                             |
| `just build`      | Build everything for production                      |

## Production Deployment

### Docker Compose

```bash
cp .env.example .env
# Edit .env: set DB_PASSWORD and ROCKET_SECRET_KEY
docker compose -f docker-compose.prod.yml up -d --build
```

The production compose file (`docker-compose.prod.yml`) includes:
- PostgreSQL on an internal network
- The server exposed on the `cloudflare` external network for tunnel access

### Client on Raspberry Pi

Cross-compile the client for the Pi:

```bash
cross build --release --target arm-unknown-linux-gnueabihf -p client
```

Copy the binary to the Pi and run it as a systemd service:

```bash
tilt-client --server-url http://your-server:8000 --scan-interval 15
```

## Project Structure

```
tilt-hydrometer-web/
├── client/          # BLE scanner + uploader (Raspberry Pi)
├── server/          # Rocket API server + migrations
├── shared/          # Common types and DTOs
├── web/             # React frontend (Vite + TypeScript)
├── docker-compose.yml       # Development
├── docker-compose.prod.yml  # Production (Cloudflare)
└── justfile                 # Command runner recipes
```

## License

MIT
