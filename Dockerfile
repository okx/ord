FROM rust:1.84.0-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    musl-tools \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /usr/src/ord

COPY . .

RUN cargo build --bin ord --release \
    --target x86_64-unknown-linux-musl \
    --features openssl

FROM debian:bookworm-slim

COPY --from=builder \
    /usr/src/ord/target/x86_64-unknown-linux-musl/release/ord \
    /usr/local/bin

RUN apt-get update && apt-get install -y openssl

ENV RUST_BACKTRACE=1
ENV RUST_LOG=info