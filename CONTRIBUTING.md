# Contributing

## Development Setup

### Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) 22+
- [Docker](https://docs.docker.com/get-docker/) with Compose
- [just](https://github.com/casey/just) (`cargo install just`)
- [sea-orm-cli](https://www.sea-ql.org/SeaORM/) (`cargo install sea-orm-cli`)

### First-time Setup

```bash
git clone https://github.com/davisschenk/tilt-app.git
cd tilt-app

# Install git hooks and create .env
just setup

# Edit .env — at minimum set AUTHENTIK_* values for auth
# For local dev you can use any OIDC provider or Authentik

# Start Postgres and run migrations
just db-up
just db-migrate

# Start server + web dev server in separate terminals
just server   # terminal 1 — Rocket on :8000
just web      # terminal 2 — Vite on :5173
```

### Running Tests

```bash
just test          # Rust workspace + web type-check
just test-rust     # Rust only
just test-web      # Web type-check only
```

### Code Style

```bash
just fmt           # Format Rust code
just lint          # Run clippy
```

The pre-commit hook (installed by `just setup`) runs `cargo fmt --check` and `cargo clippy` automatically.

### Database Migrations

```bash
# Generate a new migration
sea-orm-cli migrate generate <name>

# Apply migrations
just db-migrate

# Regenerate SeaORM entities after schema changes
just db-entities
```

### Project Layout

```
tilt-app/
├── server/          # Rocket API + SeaORM migrations
├── shared/          # Common DTOs shared between server and clients
├── web/             # React 19 + TypeScript frontend (Vite)
├── esp32-client/    # ESP32 BLE scanner firmware (separate toolchain)
├── docker-compose.yml           # Development
├── docker-compose.prod.yml      # Generic production
├── docker-compose.cloudflare.yml # Production with Cloudflare tunnel
└── justfile                     # Command runner
```

### Ralph Workflow

This project uses the Ralph backlog methodology. `prd.json` and `progress.md` are **never committed** — they are local working state files excluded by `.gitignore`.

## Submitting Changes

1. Fork the repo and create a feature branch
2. Make your changes with tests where applicable
3. Run `just ci` (format check + lint + tests) — this must pass
4. Open a PR with a clear description of what changed and why
