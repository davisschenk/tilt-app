# Implementation Details

Technical reference for AI agents and contributors working on the Tilt Hydrometer Platform codebase.

---

## Architecture Overview

```
┌─────────────────────┐        HTTP/JSON         ┌──────────────────────────┐
│   Raspberry Pi Zero │  ──────────────────────►  │     Rocket API Server    │
│                     │   POST /api/v1/readings   │                          │
│  ┌───────────────┐  │                           │  ┌────────────────────┐  │
│  │ BLE Scanner   │  │                           │  │  SeaORM + Postgres │  │
│  │ (btleplug)    │  │                           │  └────────────────────┘  │
│  └───────────────┘  │                           │                          │
│  client binary      │                           │  server binary           │
└─────────────────────┘                           └──────────────────────────┘
         ▲
         │ BLE iBeacon
    ┌────┴────┐
    │  Tilt   │  (floating in fermenter)
    │Hydrometer│
    └─────────┘
```

### Crate Layout (Cargo Workspace)

| Crate    | Purpose |
|----------|---------|
| `shared` | Domain types, DTOs, enums — single source of truth for both server and client |
| `server` | Rocket web API, SeaORM entities/migrations, services, validation guards |
| `client` | BLE scanner, Tilt iBeacon parser, HTTP uploader, CLI interface |

---

## Tilt BLE Protocol

Tilt hydrometers broadcast as **Apple iBeacon** BLE advertisements. Each color has a fixed 128-bit UUID that differs only in byte 5:

| Color  | UUID                                     |
|--------|------------------------------------------|
| Red    | `A495BB10-C5B1-4B44-B512-1370F02D74DE`  |
| Green  | `A495BB20-C5B1-4B44-B512-1370F02D74DE`  |
| Black  | `A495BB30-C5B1-4B44-B512-1370F02D74DE`  |
| Purple | `A495BB40-C5B1-4B44-B512-1370F02D74DE`  |
| Orange | `A495BB50-C5B1-4B44-B512-1370F02D74DE`  |
| Blue   | `A495BB60-C5B1-4B44-B512-1370F02D74DE`  |
| Yellow | `A495BB70-C5B1-4B44-B512-1370F02D74DE`  |
| Pink   | `A495BB80-C5B1-4B44-B512-1370F02D74DE`  |

**Data encoding:**
- **Major** (u16, big-endian) = temperature in °F
- **Minor** (u16, big-endian) = specific gravity × 1000 (divide by 1000.0 for SG)
- **TX Power** (i8) = transmit power in dBm
- **RSSI** (i8) = received signal strength

Reference: [kvurd.com/blog/tilt-hydrometer-ibeacon-data-format](https://kvurd.com/blog/tilt-hydrometer-ibeacon-data-format/)

---

## Server (`server/`)

### Tech Stack
- **Rocket v0.5** (async, features: `json`, `secrets`)
- **SeaORM** (async ORM with `sea-orm-migration` for versioned schema migrations)
- **PostgreSQL 16**
- **rocket_cors** for CORS fairing
- **serde / serde_json**, **chrono** (with `serde`), **uuid** (v4 + serde)
- **dotenvy** for `.env` loading
- **tracing + tracing-subscriber** for structured logging

### Database Schema

**hydrometers**
| Column              | Type         | Notes                          |
|---------------------|--------------|--------------------------------|
| id                  | UUID (PK)    | v4, generated                  |
| color               | Enum         | TiltColor                      |
| name                | VARCHAR      | Optional user-friendly alias   |
| temp_offset_f       | FLOAT8       | Calibration offset, default 0  |
| gravity_offset      | FLOAT8       | Calibration offset, default 0  |
| created_at          | TIMESTAMPTZ  | Auto-set                       |

**brews**
| Column              | Type         | Notes                          |
|---------------------|--------------|--------------------------------|
| id                  | UUID (PK)    | v4, generated                  |
| name                | VARCHAR      | Required                       |
| style               | VARCHAR      | Optional (e.g., "IPA")         |
| og                  | FLOAT8       | Original gravity               |
| fg                  | FLOAT8       | Final gravity (measured)       |
| target_fg           | FLOAT8       | Target final gravity           |
| abv                 | FLOAT8       | Computed or manual             |
| status              | Enum         | Active / Completed / Archived  |
| start_date          | TIMESTAMPTZ  | When brew started              |
| end_date            | TIMESTAMPTZ  | Nullable, when completed       |
| notes               | TEXT         | Free-form markdown             |
| hydrometer_id       | UUID (FK)    | References hydrometers.id      |
| created_at          | TIMESTAMPTZ  | Auto-set                       |
| updated_at          | TIMESTAMPTZ  | Auto-updated                   |

**readings**
| Column              | Type         | Notes                          |
|---------------------|--------------|--------------------------------|
| id                  | UUID (PK)    | v4, generated                  |
| brew_id             | UUID (FK)    | Nullable, references brews.id  |
| hydrometer_id       | UUID (FK)    | References hydrometers.id      |
| temperature_f       | FLOAT8       | From iBeacon major field       |
| gravity             | FLOAT8       | From iBeacon minor / 1000.0    |
| rssi                | SMALLINT     | Nullable, signal strength      |
| recorded_at         | TIMESTAMPTZ  | When the reading was taken      |
| created_at          | TIMESTAMPTZ  | When the server stored it      |

### API Endpoints (mounted at `/api/v1/`)

**Hydrometers**
- `GET    /hydrometers`           — List all registered hydrometers
- `POST   /hydrometers`           — Register a new hydrometer
- `GET    /hydrometers/<id>`      — Get hydrometer details
- `PUT    /hydrometers/<id>`      — Update name / calibration offsets
- `DELETE /hydrometers/<id>`      — Remove hydrometer

**Brews**
- `GET    /brews`                 — List brews (filterable by status)
- `POST   /brews`                 — Create a new brew session
- `GET    /brews/<id>`            — Get brew details with summary stats
- `PUT    /brews/<id>`            — Update brew metadata / status
- `DELETE /brews/<id>`            — Archive or delete a brew

**Readings**
- `POST   /readings`              — Submit one or more readings (batch)
- `GET    /readings?brew_id=&hydrometer_id=&since=&until=&limit=` — Query readings with filters

### Key Design Patterns

1. **Validation in guards, not routes.** All POST/PUT payloads use custom `FromForm` / `Json<T>` with `#[derive(Deserialize, Validate)]` structs. Route handlers receive already-validated data. Invalid input yields 422 automatically via Rocket catchers.

2. **Thin service layer.** `routes/` → `services/` → SeaORM entities. Routes handle HTTP concerns; services handle business logic and DB access.

3. **Typed JSON errors.** A custom Rocket responder wraps all errors as `{ "error": "..." }` with the correct HTTP status code. Catchers for 404, 422, 500 return the same shape.

4. **CORS via fairing.** `rocket_cors` configured globally, not per-route. Catch-all OPTIONS route for preflight handling.

5. **Static file serving.** The built React frontend is served via `FileServer` with an SPA fallback route for client-side routing.

6. **No `.unwrap()` in production.** Use `?` propagation, `anyhow::Result`, or explicit error mapping.

---

## Client (`client/`)

### Tech Stack
- **btleplug** — pure Rust async BLE scanning
- **reqwest** (features: `json`, `rustls-tls`) — HTTP client
- **tokio** — async runtime
- **clap** (derive) — CLI argument parsing
- **tracing + tracing-subscriber** — structured logging
- **serde / serde_json**, **chrono**

### Behavior

1. **Scan** — Continuously listens for BLE advertisements using `btleplug`
2. **Filter** — Matches manufacturer-specific data against the 8 known Tilt UUIDs
3. **Parse** — Extracts temperature (major), gravity (minor), RSSI from iBeacon payload
4. **Batch** — Collects readings over a configurable interval (default 15 seconds)
5. **Upload** — POSTs batch to `{server_url}/api/v1/readings`
6. **Retry** — Exponential backoff on HTTP failure; in-memory bounded `VecDeque` buffer when server is unreachable
7. **Repeat**

### Simulator Mode

The client includes a simulator (`--simulate`) that generates synthetic readings using sine waves with per-color phase offsets. Useful for development without BLE hardware.

### Deployment

Runs as a **systemd service** on the Raspberry Pi Zero W. Cross-compiled from the dev machine targeting `arm-unknown-linux-gnueabihf`.

---

## Shared Crate (`shared/`)

Houses all types that cross the client ↔ server boundary:

- **`TiltColor`** — Enum with 8 variants, each carrying its iBeacon UUID constant. Serializes to/from camelCase strings.
- **`TiltReading`** — DTO: `color`, `temperature_f`, `gravity`, `rssi`, `recorded_at`
- **`CreateReadingsBatch`** — Vec of readings for the batch POST endpoint
- **`Brew`** / **`CreateBrew`** / **`UpdateBrew`** — API DTOs
- **`Hydrometer`** / **`CreateHydrometer`** / **`UpdateHydrometer`** — API DTOs
- **`BrewStatus`** — Enum: Active, Completed, Archived

All types derive `Serialize, Deserialize` and use `#[serde(rename_all = "camelCase")]`.

---

## Web Frontend (`web/`)

### Tech Stack
- **React 19** via **Vite**
- **TypeScript** (strict mode)
- **TailwindCSS v4** with `@tailwindcss/typography`
- **shadcn/ui** component primitives (Radix UI)
- **Lucide React** icons
- **React Router v7** for client-side routing
- **TanStack Query v5** for server state management
- **Recharts** for fermentation charts
- **@uiw/react-md-editor** for markdown notes
- **date-fns** for date formatting

### Key Patterns
- Centralized API client in `web/src/lib/api.ts` with typed responses
- One custom hook per API resource (`useBrews()`, `useHydrometers()`, `useReadings()`)
- Dark/light/system theme via `ThemeProvider` with CSS variable overrides
- Served as static files from the Rocket server (no separate web server needed)

---

## Environment Variables

| Variable             | Description                              | Default               |
|----------------------|------------------------------------------|-----------------------|
| `DATABASE_URL`       | PostgreSQL connection string             | (required)            |
| `ROCKET_SECRET_KEY`  | Secret key for Rocket sessions           | (required in prod)    |
| `ROCKET_PORT`        | Server port                              | `8000`                |
| `ROCKET_ADDRESS`     | Bind address                             | `127.0.0.1`           |
| `RUST_LOG`           | Log level filter                         | `info`                |
| `WEB_DIST_DIR`       | Path to built web frontend               | `web/dist`            |
| `DB_PASSWORD`        | Database password (used by Docker)       | `password`            |
