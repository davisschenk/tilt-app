set dotenv-load

# List all available recipes
default:
    @just --list

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

# Run the Rocket API server
server:
    cargo run -p server

# Run the client in simulate mode (no BLE hardware needed)
client-sim:
    cargo run -p client -- --simulate --server-url http://localhost:8000 --scan-interval 5 --sim-colors Red,Blue

# Run the Vite dev server for the web frontend
web:
    cd web && npm run dev

# Build web frontend and serve everything from Rocket (single server)
serve: build
    cargo run -p server

# Start all dev services (run in separate terminals: just server, just client-sim, just web)
dev: db-up
    @echo "Database is up. Now run these in separate terminals:"
    @echo "  just server"
    @echo "  just client-sim"
    @echo "  just web"

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

# Full CI pipeline: format check, lint, and test
ci: fmt-check lint test
