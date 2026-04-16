FROM node:22-slim AS web-builder
WORKDIR /app/web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/ .
RUN npm run build

FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY server/Cargo.toml server/Cargo.toml
COPY server/migration/Cargo.toml server/migration/Cargo.toml
COPY shared/Cargo.toml shared/Cargo.toml
COPY client/Cargo.toml client/Cargo.toml
COPY server/src server/src
COPY server/migration/src server/migration/src
COPY shared/src shared/src
COPY client/src client/src
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json -p server -p shared
COPY Cargo.toml Cargo.lock ./
COPY server/ server/
COPY shared/ shared/
COPY client/Cargo.toml client/Cargo.toml
COPY client/src/ client/src/
RUN cargo build --release --bin server

FROM debian:trixie-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/server /usr/local/bin/server
COPY --from=web-builder /app/web/dist /app/web/dist
COPY server/Rocket.toml /app/Rocket.toml
ENV WEB_DIST_DIR=/app/web/dist
ENTRYPOINT ["/usr/local/bin/server"]
