set dotenv-load

# List all available recipes
default:
    @just --list

# Install git hooks (works in main worktree and linked worktrees)
install-hooks:
    #!/usr/bin/env bash
    set -e
    HOOKS_DIR=$(git rev-parse --git-common-dir)/hooks
    mkdir -p "$HOOKS_DIR"
    cp .githooks/pre-commit "$HOOKS_DIR/pre-commit"
    chmod +x "$HOOKS_DIR/pre-commit"
    echo "Git hooks installed to $HOOKS_DIR."

# Start the Postgres database container
db-up:
    docker compose up -d db
    @echo "Waiting for Postgres to be healthy..."
    @until docker compose exec db pg_isready -U tilt -d tilt > /dev/null 2>&1; do sleep 1; done
    @echo "Postgres is ready."

# Stop all Docker containers
db-down:
    docker compose down

# Run SeaORM migrations
db-migrate:
    sea-orm-cli migrate up -d server/migration

# Regenerate SeaORM entities from the live database
db-entities:
    sea-orm-cli generate entity -o server/src/models/entities --with-serde both

# Reset database: stop, start, and re-run migrations
db-reset: db-down db-up db-migrate

# Hard reset: drop and recreate the database, then re-run all migrations from scratch
db-reset-hard: db-up
    docker compose exec db psql -U tilt -d postgres -c "DROP DATABASE IF EXISTS tilt;"
    docker compose exec db psql -U tilt -d postgres -c "CREATE DATABASE tilt;"
    just db-migrate
    @echo "Database wiped and re-migrated."

# Run the Rocket API server
server:
    cargo run -p server

# Run the Vite dev server for the web frontend
web:
    cd web && npm run dev

# Build web frontend and serve everything from Rocket (single server)
serve: build
    cargo run -p server

# Start all dev services (run in separate terminals: just server, just web)
dev: db-up
    @echo "Database is up. Now run these in separate terminals:"
    @echo "  just server"
    @echo "  just web"

# First-time setup: install git hooks and copy example env file
setup:
    just install-hooks
    @[ -f .env ] && echo ".env already exists, skipping" || (cp .env.example .env && echo "Created .env from .env.example — edit it before starting")

# Build the entire project (Rust workspace + web frontend)
build:
    cargo build --workspace
    cd web && npm run build

# Remove all build artifacts
clean:
    cargo clean
    rm -rf web/dist

# Run all Rust tests
test-rust:
    cargo test --workspace

# Type-check the web frontend (build)
test-web:
    cd web && npm run build

# Run all tests (Rust + web)
test: test-rust test-web

# Format all Rust code
fmt:
    cargo fmt --all

# Check Rust formatting (CI-friendly, no changes)
fmt-check:
    cargo fmt --all -- --check

# Run clippy lints on the Rust workspace
lint:
    cargo clippy --workspace -- -D warnings

# Type-check the Rust workspace
check:
    cargo check --workspace

# Build the ESP32 client firmware (requires: . $HOME/export-esp.sh)
esp32-build:
    cd esp32-client && cargo build --release

# Flash the ESP32 client and open serial monitor
esp32-flash: esp32-build
    cd esp32-client && espflash flash --baud 115200 --monitor target/xtensa-esp32-espidf/release/esp32-client

# Open ESP32 serial monitor without reflashing
esp32-monitor:
    cd esp32-client && espflash monitor

# Check ESP32 client compilation (fast feedback)
esp32-check:
    cd esp32-client && cargo check

# Run ESP32 client unit tests on host (tilt.rs, buffer.rs)
esp32-test:
    cd esp32-client && ./test-host.sh

# Remove ESP32 client build artifacts
esp32-clean:
    cd esp32-client && cargo clean

# Full CI pipeline: format check, lint, and test
ci: fmt-check lint test

# Seed the database with deterministic test data.
# Usage: just seed [profile]   profiles: minimal (default), full (not implemented)
seed profile="minimal":
    cargo run -p server --bin seed -- --profile {{profile}}

# Wipe and reseed the database.
seed-force profile="minimal":
    cargo run -p server --bin seed -- --profile {{profile}} --force

# Purge BuildKit cache mounts. Variants: all (default), rust, node, hard.
cache-purge mode="all":
    #!/usr/bin/env bash
    set -e
    case "{{mode}}" in
      all)
        echo "Pruning all tilt-* BuildKit cache mounts..."
        docker buildx prune --filter "type=exec.cachemount" --force >/dev/null
        echo "Done."
        ;;
      rust)
        echo "Pruning tilt-cargo-* cache mounts..."
        for id in tilt-cargo-registry-v1 tilt-cargo-git-v1 tilt-cargo-target-v1; do
          docker buildx prune --filter "type=exec.cachemount" --filter "id=$id" --force >/dev/null || true
        done
        echo "Done."
        ;;
      node)
        echo "Pruning tilt-npm-v1 cache mount..."
        docker buildx prune --filter "type=exec.cachemount" --filter "id=tilt-npm-v1" --force >/dev/null || true
        echo "Done."
        ;;
      hard)
        echo "Pruning ALL BuildKit caches (mounts + image layers). This will be slow to rebuild."
        docker buildx prune --all --force >/dev/null
        echo "Done."
        ;;
      *)
        echo "Unknown cache-purge mode: {{mode}} (expected: all, rust, node, hard)" >&2
        exit 1
        ;;
    esac

# ---------------------------------------------------------------------------
# Git worktree helpers
# Each worktree gets its own branch, .env, Docker project, and DB.
# ---------------------------------------------------------------------------

# Internal: print the stack slug for the current worktree.
_slug:
    #!/usr/bin/env bash
    set -e
    common_dir=$(git rev-parse --git-common-dir)
    git_dir=$(git rev-parse --git-dir)
    if [ "$(readlink -f "$common_dir")" = "$(readlink -f "$git_dir")" ]; then
      echo "main"
    else
      branch=$(git symbolic-ref --short HEAD 2>/dev/null || echo "detached")
      slug=$(echo "$branch" | tr '/-' '__' | tr '[:upper:]' '[:lower:]' | cut -c1-21)
      echo "wt_$slug"
    fi

# Internal: find a free TCP port.
_free-port:
    #!/usr/bin/env bash
    python3 -c 'import socket; s=socket.socket(); s.bind(("", 0)); print(s.getsockname()[1]); s.close()'

# Internal: ensure DB_PORT and PORT are set in .env, allocating free ports if needed.
_ensure-ports:
    #!/usr/bin/env bash
    set -e
    [ -f .env ] || { echo "no .env in this worktree (run 'just setup' or 'just wt-add' first)" >&2; exit 1; }
    set -a; . ./.env; set +a
    if [ -z "${DB_PORT:-}" ]; then
      DB_PORT=$(just _free-port)
      sed -i "s|^DB_PORT=.*|DB_PORT=${DB_PORT}|" .env
      echo "allocated DB_PORT=${DB_PORT}"
    fi
    if [ -z "${PORT:-}" ]; then
      PORT=$(just _free-port)
      sed -i "s|^PORT=.*|PORT=${PORT}|" .env
      sed -i "s|^ROCKET_PORT=.*|ROCKET_PORT=${PORT}|" .env
      echo "allocated PORT=${PORT}"
    fi
    sed -i "s|^DATABASE_URL=.*|DATABASE_URL=postgres://${DB_USER:-tilt}:${DB_PASSWORD:-password}@localhost:${DB_PORT}/${DB_NAME:-tilt}|" .env

# Bring up the full stack for this worktree.
# Default: DB in docker, server + web on host.
# Pass --docker to run server + web inside docker too.
# Pass --rebuild to force a docker image rebuild.
up *args:
    #!/usr/bin/env bash
    set -e
    docker_mode=0
    rebuild=0
    seed_profile="minimal"
    for arg in {{args}}; do
      case "$arg" in
        --docker) docker_mode=1 ;;
        --rebuild) rebuild=1 ;;
        --seed=*) seed_profile="${arg#--seed=}" ;;
      esac
    done

    slug=$(just _slug)
    [ -f .env ] || { echo "no .env — run 'just setup' (main) or 'just wt-add' (worktree)" >&2; exit 1; }
    if grep -q "^COMPOSE_PROJECT_NAME=" .env; then
      sed -i "s|^COMPOSE_PROJECT_NAME=.*|COMPOSE_PROJECT_NAME=${slug}|" .env
    else
      echo "COMPOSE_PROJECT_NAME=${slug}" >> .env
    fi
    just _ensure-ports
    set -a; . ./.env; set +a

    if [ "$docker_mode" = 1 ]; then
      docker compose up -d db
      until docker compose exec db pg_isready -U "${DB_USER:-tilt}" -d "${DB_NAME:-tilt}" > /dev/null 2>&1; do
        sleep 1
      done
      sea-orm-cli migrate up -d server/migration
      cargo run -p server --bin seed -- --profile "$seed_profile" || true

      build_args=()
      [ "$rebuild" = 1 ] && build_args+=(--no-cache)
      DOCKER_BUILDKIT=1 docker compose build "${build_args[@]}" server
      docker compose up -d server
      echo "waiting for server to be healthy..."
      tries=0
      until docker compose ps --format json server 2>/dev/null | grep -q '"Health":"healthy"'; do
        tries=$((tries+1))
        if [ "$tries" -gt 60 ]; then
          echo "server did not become healthy after 120s; check 'docker compose logs server'" >&2
          exit 1
        fi
        sleep 2
      done
      echo ""
      echo "stack '${slug}' up:"
      echo "  → http://localhost:${PORT}"
    else
      docker compose up -d db
      until docker compose exec db pg_isready -U "${DB_USER:-tilt}" -d "${DB_NAME:-tilt}" > /dev/null 2>&1; do
        sleep 1
      done
      sea-orm-cli migrate up -d server/migration
      cargo run -p server --bin seed -- --profile "$seed_profile" || true
      echo ""
      echo "stack '${slug}' up (host mode):"
      echo "  DB     → postgres://localhost:${DB_PORT}/${DB_NAME:-tilt}"
      echo "  API    → run 'just server'    (will bind localhost:${PORT})"
      echo "  Web    → run 'just web'       (Vite dev server)"
    fi

# Bring down this worktree's stack.
# Pass --reset to also remove volumes and clear allocated ports.
# Pass --all to bring down EVERY running tilt stack on this host.
down *args:
    #!/usr/bin/env bash
    set -e
    reset=0
    all=0
    for arg in {{args}}; do
      case "$arg" in
        --reset) reset=1 ;;
        --all)   all=1 ;;
      esac
    done

    if [ "$all" = 1 ]; then
      stacks=$(docker ps -a --filter "label=com.tilt.stack" --format '{{{{.Label "com.tilt.stack"}}}}' | sort -u)
      if [ -z "$stacks" ]; then
        echo "no tilt stacks running."
        exit 0
      fi
      for s in $stacks; do
        echo "→ stopping stack: $s"
        docker compose -p "tilt-$s" down
      done
      exit 0
    fi

    [ -f .env ] || { echo "no .env in this worktree" >&2; exit 1; }
    if [ "$reset" = 1 ]; then
      docker compose down --volumes
      sed -i "s|^DB_PORT=.*|DB_PORT=|" .env
      sed -i "s|^PORT=.*|PORT=|" .env
      sed -i "s|^ROCKET_PORT=.*|ROCKET_PORT=|" .env
      echo "stack down, volumes removed, ports cleared in .env."
    else
      docker compose down
      echo "stack down (volumes preserved)."
    fi

# Show running stack info for this worktree (or all, with --all).
status *args:
    #!/usr/bin/env bash
    set -e
    all=0
    for arg in {{args}}; do
      [ "$arg" = "--all" ] && all=1
    done

    print_row () {
      local slug="$1" project="$2" db_port="$3" api_port="$4" health="$5"
      printf "  %-20s %-22s db:%-6s api:%-6s %s\n" "$slug" "$project" "$db_port" "$api_port" "$health"
    }

    if [ "$all" = 1 ]; then
      echo "running tilt stacks:"
      docker ps --filter "label=com.tilt.stack" --format '{{{{.Label "com.tilt.stack"}}}}' | sort -u | while read -r slug; do
        [ -z "$slug" ] && continue
        project="tilt-$slug"
        db_port=$(docker inspect --format '{{{{(index (index .NetworkSettings.Ports "5432/tcp") 0).HostPort}}}}' "${project}-db-1" 2>/dev/null || echo "?")
        api_port=$(docker inspect --format '{{{{(index (index .NetworkSettings.Ports "8000/tcp") 0).HostPort}}}}' "${project}-server-1" 2>/dev/null || echo "?")
        health=$(docker inspect --format '{{{{.State.Health.Status}}}}' "${project}-server-1" 2>/dev/null || echo "no-server")
        print_row "$slug" "$project" "$db_port" "$api_port" "$health"
      done
      exit 0
    fi

    [ -f .env ] || { echo "no .env in this worktree" >&2; exit 1; }
    set -a; . ./.env; set +a
    slug=$(just _slug)
    project="tilt-${COMPOSE_PROJECT_NAME:-$slug}"
    db_state=$(docker inspect --format '{{{{.State.Status}}}}' "${project}-db-1" 2>/dev/null || echo "stopped")
    server_state=$(docker inspect --format '{{{{.State.Status}}}}' "${project}-server-1" 2>/dev/null || echo "stopped")
    echo "stack '${slug}':"
    echo "  project       : ${project}"
    echo "  DB_PORT       : ${DB_PORT:-unset}    (state: $db_state)"
    echo "  PORT          : ${PORT:-unset}    (server state: $server_state)"
    echo "  AUTH_MODE     : ${AUTH_MODE:-unset}"
    echo "  DB_NAME       : ${DB_NAME:-unset}"

# Add a new worktree on a new branch and seed its .env.
# Usage: just wt-add <branch> [path]
# Path defaults to ../tilt-<slug>. Ports are auto-allocated on first 'just up'.
wt-add branch path="":
    #!/usr/bin/env bash
    set -e
    branch="{{branch}}"
    path="{{path}}"
    if [ -z "$path" ]; then
      slug=$(echo "$branch" | tr '/-' '__' | tr '[:upper:]' '[:lower:]' | cut -c1-21)
      path="../tilt-${slug}"
    fi
    git worktree add "$path" -b "$branch"
    cp .env.example "$path/.env"
    slug=$(echo "$branch" | tr '/-' '__' | tr '[:upper:]' '[:lower:]' | cut -c1-21)
    sed -i "s|^DB_PORT=.*|DB_PORT=|"           "$path/.env"
    sed -i "s|^PORT=.*|PORT=|"                 "$path/.env"
    sed -i "s|^ROCKET_PORT=.*|ROCKET_PORT=|"   "$path/.env"
    sed -i "s|^DB_NAME=.*|DB_NAME=tilt_${slug}|" "$path/.env"
    sed -i "s|^DATABASE_URL=.*|DATABASE_URL=postgres://tilt:password@localhost:0/tilt_${slug}|" "$path/.env"
    if grep -q "^AUTH_MODE=" "$path/.env"; then
      sed -i "s|^AUTH_MODE=.*|AUTH_MODE=disabled|" "$path/.env"
    else
      echo "AUTH_MODE=disabled" >> "$path/.env"
    fi
    grep -q "^COMPOSE_PROJECT_NAME=" "$path/.env" || echo "COMPOSE_PROJECT_NAME=wt_${slug}" >> "$path/.env"
    echo ""
    echo "Worktree at $path on branch $branch"
    echo "  DB name       : tilt_${slug}"
    echo "  Ports         : (allocated on first 'just up')"
    echo "  AUTH_MODE     : disabled"
    echo ""
    echo "Next: cd $path && just up [--docker]"

# Remove a worktree (stops its stack and removes volumes first).
wt-remove path:
    #!/usr/bin/env bash
    set -e
    if [ ! -d "{{path}}" ]; then
      echo "no such worktree: {{path}}" >&2
      exit 1
    fi
    if [ -f "{{path}}/.env" ]; then
      ( cd "{{path}}" && just down --reset ) || true
    fi
    git worktree remove --force "{{path}}"
    echo "Worktree {{path}} removed."

# List worktrees with allocated ports and running state.
wt-list:
    #!/usr/bin/env bash
    set -e
    git worktree list --porcelain | awk '/^worktree /{print $2}' | while read -r wt; do
      [ -z "$wt" ] && continue
      env_file="$wt/.env"
      branch=$(cd "$wt" && git symbolic-ref --short HEAD 2>/dev/null || echo "(detached)")
      if [ -f "$env_file" ]; then
        db_port=$(grep '^DB_PORT=' "$env_file" | cut -d= -f2)
        api_port=$(grep '^PORT=' "$env_file" | cut -d= -f2)
        slug=$(grep '^COMPOSE_PROJECT_NAME=' "$env_file" | cut -d= -f2)
        running="-"
        if [ -n "$slug" ] && docker ps --filter "label=com.tilt.stack=${slug}" -q | grep -q .; then
          running="running"
        fi
      else
        db_port=""; api_port=""; running="(no .env)"
      fi
      printf "  %-40s %-30s db:%-6s api:%-6s %s\n" "$wt" "$branch" "${db_port:-?}" "${api_port:-?}" "$running"
    done

# Remove worktrees whose branch has been merged and deleted on origin.
wt-prune:
    #!/usr/bin/env bash
    set -e
    git fetch --prune --quiet
    git worktree list --porcelain | awk '/^worktree /{print $2}' | while read -r wt; do
      [ -z "$wt" ] && continue
      [ "$wt" = "$(git rev-parse --show-toplevel)" ] && continue
      branch=$(cd "$wt" && git symbolic-ref --short HEAD 2>/dev/null || echo "")
      [ -z "$branch" ] && continue
      if git show-ref --quiet "refs/remotes/origin/$branch"; then
        continue
      fi
      merged="(unknown)"
      if cd "$wt" && git merge-base --is-ancestor HEAD master 2>/dev/null; then
        merged="merged"
      else
        merged="not-merged"
      fi
      cd "$(git rev-parse --show-toplevel)"
      read -r -p "remove worktree $wt (branch $branch, $merged)? [y/N] " ans
      case "$ans" in
        y|Y) just wt-remove "$wt" ;;
        *)   echo "skipped $wt" ;;
      esac
    done
