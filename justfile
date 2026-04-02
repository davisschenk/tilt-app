set dotenv-load

# List all available recipes
default:
    @just --list

# Install git hooks (run once after cloning)
install-hooks:
    cp .githooks/pre-commit .git/hooks/pre-commit
    chmod +x .git/hooks/pre-commit
    @echo "Git hooks installed."

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

# Cross-compile the client for Raspberry Pi Zero W (arm-unknown-linux-gnueabihf)
cross-client:
    cross build --release --target arm-unknown-linux-gnueabihf -p client

# Configure a Raspberry Pi for the tilt-client (BlueZ experimental flag, etc.)
# Usage: just setup-pi tilt@192.168.1.100
setup-pi host:
    ssh {{host}} "sudo mkdir -p /etc/systemd/system/bluetooth.service.d && printf '[Service]\nExecStart=\nExecStart=/usr/libexec/bluetooth/bluetoothd --experimental\n' | sudo tee /etc/systemd/system/bluetooth.service.d/experimental.conf"
    ssh {{host}} "sudo systemctl daemon-reload && sudo systemctl restart bluetooth"
    @echo "BlueZ experimental mode enabled on {{host}}"

# Deploy the cross-compiled client binary to a Raspberry Pi via SSH
# Usage: just deploy-client tilt@192.168.1.100
deploy-client host:
    just cross-client
    just setup-pi {{host}}
    scp target/arm-unknown-linux-gnueabihf/release/client {{host}}:/tmp/tilt-client
    scp client/tilt-client.service {{host}}:/tmp/tilt-client.service
    ssh {{host}} "sudo mv /tmp/tilt-client /usr/local/bin/tilt-client && sudo chmod +x /usr/local/bin/tilt-client"
    ssh {{host}} "sudo mv /tmp/tilt-client.service /etc/systemd/system/tilt-client.service"
    ssh {{host}} "sudo systemctl daemon-reload && sudo systemctl enable tilt-client && sudo systemctl restart tilt-client"
    @echo "Deployed and started tilt-client on {{host}}"

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
