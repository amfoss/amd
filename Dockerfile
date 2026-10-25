# Build Stage
FROM rust:slim-bullseye AS builder
WORKDIR /builder

RUN cargo init
# Compile deps in a separate layer (for caching)
COPY Cargo.toml Cargo.lock ./
RUN apt-get update
RUN apt install -y pkg-config libssl-dev
RUN cargo build --release

# Compile for release
COPY ./src ./src
RUN rm ./target/release/deps/amd*
RUN cargo build --release

# Release Stage
FROM debian:bullseye-slim AS release
RUN apt-get update
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /builder/target/release/amd /usr/local/bin
CMD ["/usr/local/bin/amd"]
