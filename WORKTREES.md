# Worktree-Native Development

This repo is fully worktree-aware. The same commands work in the main repo and in every linked worktree. Multiple AI agents (or humans) can work on independent branches in parallel — each with its own database, ports, environment, and build artifacts.

## TL;DR

```bash
# Spawn a new worktree.
just wt-add feat/my-feature

# Bring its stack up (auto port allocation, seeded DB).
cd ../tilt-feat_my_feature
just up               # host mode (DB in docker, server/web on host)
just up --docker      # full docker (single container, web baked in)

# Use it.
curl http://localhost:<PORT>/api/v1/health     # PORT printed by `just up`

# When done.
just down --reset     # stops, removes volumes, clears ports
cd -
just wt-remove ../tilt-feat_my_feature
```

## The three modes

| Mode            | Verb               | DB     | Server               | Web                  | Hot reload |
|-----------------|--------------------|--------|----------------------|----------------------|-----------:|
| Dev             | `just dev`         | docker | host (`cargo run`)   | host (`vite`)        | yes        |
| Up (host)       | `just up`          | docker | host                 | served by Rocket     | no         |
| Up (docker)     | `just up --docker` | docker | docker               | baked into binary    | no         |

- `just dev` is what you want for active coding — fastest iteration.
- `just up` is what you want to sanity-check a branch before pushing.
- `just up --docker` is what agents and demos use — fully isolated, prod-shaped.

## What gets isolated per worktree

| Resource           | Isolation                                                                                              |
|--------------------|--------------------------------------------------------------------------------------------------------|
| Git branch         | Each worktree on its own branch (`git worktree`).                                                      |
| `.env`             | Each worktree has its own.                                                                             |
| Compose project    | `tilt-<slug>` — `tilt-main` for primary repo, `tilt-wt_<branch>` for worktrees.                        |
| Postgres DB        | Different `DB_NAME` (e.g., `tilt_wt_my_feat`) on a different `DB_PORT`.                                |
| API host port      | Different `PORT` per worktree (auto-allocated by `just up`).                                           |
| Volumes            | Compose project name namespaces them (`tilt-wt_my_feat_pgdata` vs `tilt-main_pgdata`).                 |
| Host build cache   | Per-worktree `target/` directory via `CARGO_TARGET_DIR=./target` in `.env`.                            |
| Docker build cache | **Shared** across worktrees via named BuildKit cache mounts (`tilt-cargo-*-v1`, `tilt-npm-v1`).        |

## Auth

`AUTH_MODE` controls auth — `disabled` (no auth, dev user injected) or `oidc` (enforced). Worktrees default to `disabled`. The server logs the active mode at startup:

```
auth: disabled (dev user injected — do NOT use in production)
auth: oidc https://auth.example.com/...
```

`AUTH_MODE=oidc` with missing `AUTHENTIK_*` env vars causes the server to refuse to start. Production compose defaults to `oidc`.

## Port allocation

Worktrees get free ports automatically on first `just up`. Ports stick (recorded in `.env`) until you `just down --reset`. There's no manual port table.

## Cross-worktree commands

| Command                | What it does                                           |
|------------------------|--------------------------------------------------------|
| `just wt-list`         | All worktrees + their allocated ports + running state. |
| `just status --all`    | Every running tilt stack on this host.                 |
| `just down --all`      | Stops every running tilt stack on this host.           |
| `just wt-prune`        | Removes worktrees whose branch is gone from origin.    |

## Seeded data

`just up` runs `cargo run -p server --bin seed` after migrations. The minimal profile creates:

- 3 hydrometers (Red active, Black archived, Green nameless)
- 1 active brew on Red (West Coast IPA, 24h of readings)
- 1 completed brew on Black (Imperial Stout, 14d of readings)
- 2 brew events on the active brew
- 1 alert rule + 1 alert target stub

Stable seed UUIDs: see `server/src/seed/mod.rs::ids`. Re-running is a no-op unless you pass `--force` (or `just seed-force`).

## Build cache hygiene

Symptoms → fix:

| Symptom                                               | Fix                          |
|-------------------------------------------------------|------------------------------|
| `cargo: failed to load source for dependency`         | `just cache-purge rust`      |
| `npm ERR! ... ENOTEMPTY`                              | `just cache-purge node`      |
| Builds succeed but binary crashes oddly               | `just cache-purge hard`      |

The cache mounts have stable IDs versioned `-v1`. To force a clean cut-over for the whole project (e.g., after upgrading the Rust toolchain), bump the suffix in `server/Dockerfile` to `-v2`.

## Notes for AI agents

- One agent per worktree. Don't touch another worktree's `.env`, DB, or running stack.
- Agents default to `just up --docker` — fully isolated, prod-shaped.
- `prd.json` and `progress.md` are gitignored — each worktree maintains its own.
- The agent's golden path: `just wt-add <branch> && cd ../tilt-<slug> && just up && curl http://localhost:<PORT>/api/v1/health`.
