FROM rust:1.88.0-bookworm AS builder

RUN apt-get update && apt-get install -y \
    git \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/ord

COPY . .

RUN cargo build --bin ord --release

FROM debian:bookworm-slim

# Install runtime dependencies including jemalloc
RUN apt-get update && apt-get install -y \
    openssl \
    libjemalloc2 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder \
    /usr/src/ord/target/release/ord \
    /usr/local/bin

# Use jemalloc via LD_PRELOAD (no source code changes needed)
ENV LD_PRELOAD=/usr/lib/x86_64-linux-gnu/libjemalloc.so.2
ENV RUST_BACKTRACE=1
ENV RUST_LOG=info