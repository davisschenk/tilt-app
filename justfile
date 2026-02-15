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
