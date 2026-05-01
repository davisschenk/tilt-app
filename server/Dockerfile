FROM node:22-slim AS web-builder
WORKDIR /app/web
COPY web/package.json web/package-lock.json ./
RUN --mount=type=cache,id=tilt-npm-v1,target=/root/.npm,sharing=locked \
    npm ci
COPY web/ .
RUN npm run build

FROM lukemathwalker/cargo-chef:latest-rust-1-bookworm AS chef
WORKDIR /app

FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY server/Cargo.toml server/Cargo.toml
COPY server/migration/Cargo.toml server/migration/Cargo.toml
COPY shared/Cargo.toml shared/Cargo.toml
# Cargo chef prepare only needs Cargo.toml files, not src — keeping src out
# prevents source changes from invalidating the dependency cache layer
RUN mkdir -p server/src server/migration/src shared/src \
    && echo 'fn main() {}' > server/src/main.rs \
    && touch server/migration/src/lib.rs shared/src/lib.rs
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev
COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,id=tilt-cargo-registry-v1,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=tilt-cargo-git-v1,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=tilt-cargo-target-v1,target=/app/target,sharing=locked \
    cargo chef cook --release --recipe-path recipe.json -p server -p shared
COPY Cargo.toml Cargo.lock ./
COPY server/ server/
COPY shared/ shared/
RUN --mount=type=cache,id=tilt-cargo-registry-v1,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=tilt-cargo-git-v1,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=tilt-cargo-target-v1,target=/app/target,sharing=locked \
    cargo build --release --bin server \
    && cp target/release/server /usr/local/bin/server

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl3 \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /usr/local/bin/server /usr/local/bin/server
COPY --from=web-builder /app/web/dist /app/web/dist
COPY server/Rocket.toml /app/Rocket.toml
ENV WEB_DIST_DIR=/app/web/dist
ENTRYPOINT ["/usr/local/bin/server"]
