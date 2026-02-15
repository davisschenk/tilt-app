---
trigger: always_on
---

<tech_stack>
## Language & Toolchain
- **Rust** (stable, latest edition) — entire codebase
- **Cargo workspaces** — monorepo with crates: `server`, `client`, `shared`
- Target: `server` runs on x86_64 Linux (Docker); `client` cross-compiles to `arm-unknown-linux-gnueabihf` (Raspberry Pi Zero W)

## Server Crate (`server/`)
- **Rocket v0.5** — async web framework (features: `json`, `secrets`)
- **SeaORM** — async ORM for PostgreSQL (with `sea-orm-migration` for versioned migrations)
- **PostgreSQL 16** — primary data store, accessed via `sqlx` (SeaORM's async Postgres driver)
- **rocket_cors** — CORS fairing
- **serde / serde_json** — serialization everywhere
- **chrono** (feature `serde`) — timestamps with timezone
- **uuid** (feature `v4`, `serde`) — primary key generation
- **dotenvy** — environment variable loading
- **tracing + tracing-subscriber** — structured logging
- **tokio** (pulled in by Rocket) — async runtime

## Client Crate (`client/`)
- **btleplug** — cross-platform BLE scanning (pure Rust, async/tokio)
- **reqwest** (features: `json`, `rustls-tls`) — HTTP client to push readings to server
- **tokio** — async runtime
- **tracing + tracing-subscriber** — structured logging
- **serde / serde_json** — serialization
- **chrono** — timestamping readings locally before upload
- **clap** (derive) — CLI argument parsing (server URL, scan interval, etc.)

## Shared Crate (`shared/`)
- Common types used by both `server` and `client`:
  - `TiltColor` enum (Red, Green, Black, Purple, Orange, Blue, Yellow, Pink) with UUID constants
  - `TiltReading` — the canonical reading DTO (color, temperature_f, gravity, timestamp)
  - `Brew` summary types, API request/response DTOs
  - `Hydrometer` registration types
- **serde** (Serialize, Deserialize) on all shared types
- **chrono**, **uuid** re-exported as needed

## Database Schema (SeaORM entities)
- **hydrometers** — id (UUID PK), color (enum), name (optional alias), calibration offsets, created_at
- **brews** — id (UUID PK), name, style, og, fg, target_fg, abv, status (enum: Active/Completed/Archived), start_date, end_date, notes, hydrometer_id (FK), created_at, updated_at
- **readings** — id (UUID PK), brew_id (FK, nullable), hydrometer_id (FK), temperature_f (f64), gravity (f64), rssi (i8, nullable), recorded_at (timestamptz), created_at

## Infrastructure
- **Docker Compose** — Postgres + server container
- **Dockerfile** — multi-stage build (cargo-chef for caching)
- **.env** — `DATABASE_URL`, `ROCKET_SECRET_KEY`, `ROCKET_PORT`, `RUST_LOG`

## Tilt BLE Protocol Reference
- Tilt hydrometers broadcast as **Apple iBeacon** BLE advertisements
- Each color has a fixed UUID (only byte 5 differs):
  - Red: `A495BB10-C5B1-4B44-B512-1370F02D74DE`
  - Green: `A495BB20-...`  Black: `A495BB30-...`  Purple: `A495BB40-...`
  - Orange: `A495BB50-...`  Blue: `A495BB60-...`  Yellow: `A495BB70-...`  Pink: `A495BB80-...`
- **Major** (u16, big-endian) = temperature in °F
- **Minor** (u16, big-endian) = specific gravity × 1000 (divide by 1000.0 for SG)
- TX Power (i8) and RSSI (i8) also available
</tech_stack>

<coding_conventions>
## Architecture Rules
1. **Validation in Rocket FromForm / FromData guards, NEVER in route handlers.** Route functions receive already-validated data or Rocket returns a 422 automatically.
2. **All database access goes through SeaORM entities and repositories** — no raw SQL. Thin service layer between routes and ORM.
3. **shared crate is the single source of truth** for DTOs and domain enums. Server and client both depend on it.
4. **No `.unwrap()` in production code.** Use `?`, `anyhow::Result`, or Rocket catchers for error responses.
5. **Errors returned as typed JSON** (`{ "error": "..." }`) with appropriate HTTP status codes via Rocket responders.

## Code Style
- Derive macros over manual impls: `#[derive(Debug, Clone, Serialize, Deserialize, FromForm)]`
- Use `#[serde(rename_all = "camelCase")]` on API-facing types for JSON convention
- Prefer `impl Into<T>` / `From<T>` conversions between ORM entities and API DTOs
- Group imports: std → external crates → crate-internal, separated by blank lines
- Module structure: `routes/`, `services/`, `models/`, `guards/` directories in server
- One file per route group (e.g., `routes/brews.rs`, `routes/readings.rs`, `routes/hydrometers.rs`)
- Logging via `tracing` macros (`info!`, `warn!`, `error!`), never `println!`

## Rocket-Specific Patterns
- Custom **FromForm** structs for all POST/PUT payloads with field-level validation (ranges, non-empty strings, valid enums)
- Use Rocket **fairings** for DB pool setup (`sea_orm_rocket` or manual managed state)
- Use Rocket **catchers** (`#[catch(404)]`, `#[catch(422)]`, `#[catch(500)]`) returning JSON
- Mount routes under versioned API prefix: `/api/v1/`
- CORS configured via fairing, not per-route

## Client (Pi) Patterns
- Scan loop: scan BLE → filter by Tilt UUIDs → parse major/minor → batch POST to server
- Configurable scan interval (default 15s) and server URL via CLI args or env vars
- Graceful retry with exponential backoff on HTTP failures
- Local buffer/queue if server unreachable (in-memory VecDeque, bounded)
- Run as systemd service on the Pi

## Testing
- `#[cfg(test)]` unit tests in each module
- Integration tests using Rocket's `local::asynchronous::Client`
- SeaORM mock connection for service-layer tests
- `cargo test --workspace` must pass before every commit

## Dependency Management
- **NEVER manually edit Cargo.toml to add dependencies.** Always use `cargo add <crate>` or `cargo add --dev <crate>` with appropriate `--features` flags.
- To create a new crate, use `cargo init <path>` (binary) or `cargo init --lib <path>` (library). Never hand-write Cargo.toml from scratch.
- When adding a path dependency (e.g., shared), use `cargo add --path ../shared`.

## SeaORM CLI
- **Install:** `cargo install sea-orm-cli`
- **Generate a new migration:** `sea-orm-cli migrate generate <name>` — creates a timestamped migration file under the migration crate.
- **Run migrations:** `sea-orm-cli migrate up` — requires `DATABASE_URL` in env or `.env`.
- **Rollback:** `sea-orm-cli migrate down` — rolls back the last applied migration.
- **Generate entities from DB:** `sea-orm-cli generate entity -o server/src/models/entities --with-serde both` — regenerates Rust entity files from the live database schema. Always use `--with-serde both` to include Serialize + Deserialize derives.
- Never hand-write SeaORM entity files; always generate them from the database and then adjust relations or derives as needed.
</coding_conventions>