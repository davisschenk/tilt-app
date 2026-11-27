# Contributing

## Development Setup

### Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) 26+
- [Docker](https://docs.docker.com/get-docker/) with Compose
- [just](https://github.com/casey/just) (`cargo install just`)
- [sea-orm-cli](https://www.sea-ql.org/SeaORM/) (`cargo install sea-orm-cli`)

### First-time Setup

```bash
git clone https://github.com/davisschenk/tilt-app.git
cd tilt-app

# Install git hooks and create .env
just setup

# Bring up the full stack — DB starts, migrations run, seed data loaded,
# server prints a URL. Auth is disabled by default (AUTH_MODE=disabled).
just up

# For active development with hot reload, use this instead:
just dev          # starts DB; then run `just server` and `just web` in separate terminals
```

For OIDC-protected dev (testing the real auth flow), set `AUTH_MODE=oidc` in `.env` and fill in the `AUTHENTIK_*` block. See [`WORKTREES.md`](./WORKTREES.md) for the full mode reference.

### Working on multiple branches at once

This repo is worktree-aware — see [`WORKTREES.md`](./WORKTREES.md). Quick form:

```bash
just wt-add feat/my-thing
cd ../tilt-feat_my_thing
just up
```

Each worktree gets its own DB, port, and environment, so you can have multiple stacks running simultaneously.

### Running Tests

```bash
just test          # Rust workspace + web type-check + web unit tests
just test-rust     # Rust only
just test-web      # Web type-check and Vitest unit tests
just coverage      # Rust + web tests with the coverage gates CI enforces
just e2e           # Playwright E2E + accessibility suite against the real stack
```

**Coverage gates (CI fails below these):**

| Area | Gate | Where |
|------|------|-------|
| Rust (server + shared) | 60% lines | `just coverage-rust` (`cargo-llvm-cov`; needs `DATABASE_URL`) |
| Web | 70% lines/statements, 65% functions, 60% branches | `web/vitest.config.ts` (`just coverage-web`) |

Generated SeaORM entities, `src/bin/`, migrations, and the shadcn `components/ui` primitives are excluded. If a new
file genuinely can't be exercised in jsdom, exclude it narrowly in `vitest.config.ts` with a comment — don't lower the
global floors.

**Web unit/component tests** live next to the code (`*.test.ts(x)`) and use Vitest, Testing Library and
[MSW](https://mswjs.io). Network calls are intercepted by MSW with `onUnhandledFrame: "error"`, so any request without a
handler fails the test. Helpers:

- `renderWithProviders(ui, { route })` (`src/test/render.tsx`) wraps a fresh `QueryClient`, router, theme and auth provider.
- `src/test/fixtures.ts` builds typed brews, hydrometers, readings, alert rules/targets.
- `server.use(http.get("/api/v1/...", ...))` (`src/test/msw/server.ts`) overrides handlers per test.
- `src/test/mocks/echarts-for-react.tsx` stands in for the canvas charts; assert on `chart.props.option` / `series()`.
- `mockNavigation()` (`src/test/navigation.ts`) records `window.location.href` assignments without breaking relative fetches.

**E2E and accessibility** (`web/e2e`) run Playwright against the real stack: Rocket with `AUTH_MODE=disabled` serving
the built `web/dist`, on a Postgres that is reseeded (`seed --profile minimal --force`) before every run.

```bash
just e2e-db-up                         # throwaway Postgres on 127.0.0.1:55432 (or set E2E_DATABASE_URL)
cd web && npx playwright install chromium   # once
just e2e                               # build web, boot the server, run everything
just e2e e2e/accessibility.spec.ts     # a single spec
just e2e-db-down
```

`e2e/accessibility.spec.ts` runs [axe-core](https://github.com/dequelabs/axe-core) (WCAG 2.0/2.1 A + AA) on every page in
light and dark themes, on the main dialogs and menus, and checks keyboard focus handling. Serious and critical
violations fail the test. Webhook targets must be public addresses in E2E too (the SSRF guard rejects loopback).

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
├── docker-compose.prod.yml      # Production (generic, exposes port 8000)
└── justfile                     # Command runner
```

### Ralph Workflow

This project uses the Ralph backlog methodology. `prd.json` and `progress.md` are **never committed** — they are local working state files excluded by `.gitignore`.

## Submitting Changes

1. Fork the repo and create a feature branch
2. Make your changes with tests where applicable
3. Run `just ci` (format check + lint + tests) — this must pass
4. Open a PR with a clear description of what changed and why
