# Build Stage
FROM rust:slim-bookworm AS builder
WORKDIR /builder
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
       pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN cargo init

# Compile dependencies separately for caching
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release

# Compile the application
COPY ./src ./src
RUN rm -f ./target/release/deps/amd*
RUN cargo build --release

# Release Stage
FROM debian:bookworm-slim AS release
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /builder/target/release/amd /usr/local/bin/amd
CMD ["/usr/local/bin/amd"]
