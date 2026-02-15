.PHONY: test check fmt clippy migrate run-server run-client

test:
	cargo test --workspace

check:
	cargo check --workspace

fmt:
	cargo fmt --all -- --check

clippy:
	cargo clippy --workspace -- -D warnings

migrate:
	cd server/migration && cargo run

run-server:
	cargo run -p server

run-client:
	cargo run -p client -- --server-url http://localhost:8000
