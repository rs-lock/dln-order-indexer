FROM rust:1.89-bookworm AS builder
WORKDIR /app

RUN apt-get update && apt-get install -y \
    cmake libsasl2-dev libssl-dev pkg-config \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates libssl3 libsasl2-2 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/ingestion /usr/local/bin/
