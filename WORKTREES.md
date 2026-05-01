# Git Worktrees — Multi-Agent Development

This repo is fully worktree-aware. Multiple AI agents (or humans) can work on independent branches simultaneously, each with its own isolated database, ports, and environment.

## How isolation works

| Resource | Isolation mechanism |
|---|---|
| Git branch | Each worktree checks out its own branch |
| `.env` | Each worktree has its own `.env` with unique ports/DB |
| PostgreSQL DB | Different `DB_NAME` per worktree (e.g. `tilt_feat1`) |
| PostgreSQL port | Different `DB_PORT` per worktree (e.g. `5433`) |
| API server port | Different `PORT` / `ROCKET_PORT` per worktree |
| Build cache | Shared `target/` in the main worktree (via `.cargo/config.toml`) |
| Git hooks | Installed to the common `.git/hooks/` — shared by all worktrees |

## Quick start

### Create a worktree for a new feature

```bash
# From the main worktree
just wt-add feat/my-feature ../tilt-feat 5433 8001
```

This will:
1. Create a new git worktree at `../tilt-feat` on branch `feat/my-feature`
2. Copy `.env.example` → `../tilt-feat/.env` with unique `DB_NAME`, `DB_PORT`, `PORT`, `ROCKET_PORT`, and `DATABASE_URL`

### Start the stack in the new worktree

```bash
cd ../tilt-feat
just db-up          # starts postgres on port 5433 with DB tilt_feat_my_feature
just db-migrate     # runs migrations against the isolated DB
just server         # starts the API on port 8001
# in another terminal:
just web            # starts the Vite dev server (update VITE_API_URL if needed)
```

### List all active worktrees

```bash
just wt-list
# or: git worktree list
```

### Remove a worktree

```bash
just wt-remove ../tilt-feat
```

This stops its Docker stack (including the DB volume) and removes the worktree.

## Port allocation convention

Use this table to avoid conflicts when running multiple worktrees:

| Worktree | DB_PORT | PORT / ROCKET_PORT |
|---|---|---|
| main (`tilt-app/`) | 5432 | 8000 |
| worktree 1 | 5433 | 8001 |
| worktree 2 | 5434 | 8002 |
| worktree 3 | 5435 | 8003 |

## Shared build cache

All worktrees share the `target/` directory of the main worktree (set in `.cargo/config.toml`). This means:
- Rebuilds are fast — unchanged crates are not recompiled
- Only one worktree should run `cargo build` at a time to avoid lock contention

To give a worktree its own isolated build cache, set `CARGO_TARGET_DIR` in its shell or `.env`:

```bash
export CARGO_TARGET_DIR=/tmp/tilt-feat-target
```

## Notes for AI agents

- Each agent should operate inside a single worktree directory
- Never touch another worktree's `.env`, DB, or running server
- `prd.json` and `progress.md` are gitignored — each worktree/agent maintains its own copy
- Always run `just db-up && just db-migrate` before starting the server in a fresh worktree
- The `VITE_API_URL` env var in the web dev server must point to the correct API port for that worktree
