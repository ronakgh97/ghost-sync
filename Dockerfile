FROM rust:1.93.1-slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    clang \
    cmake \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY examples ./examples

RUN cargo build --example gs-daemon --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/examples/gs-daemon /app/gs-daemon

EXPOSE 7777
EXPOSE 7878

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s CMD test -f /app/gs-daemon|| exit 1

CMD ["/app/gs-daemon", "run"]
