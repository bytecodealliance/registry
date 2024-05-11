FROM rust:1.78-slim AS builder

ARG FEATURES=postgres

WORKDIR /usr/src/bytecodealliance/registry

# musl-dev libpq-dev
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt update && apt upgrade --no-install-recommends -y && \
    apt install --no-install-recommends pkg-config libssl-dev libpq-dev -y

COPY crates/server .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/src/bytecodealliance/registry/target \
    cargo build --release --workspace --features "$FEATURES" && \
    cp target/release/warg-server /usr/local/bin

FROM debian:stable-slim

# Create volume for content directory
ENV CONTENT_DIR=/var/lib/warg-server/data
RUN mkdir -p "$CONTENT_DIR"
VOLUME $CONTENT_DIR

# Configure port settings
EXPOSE 8090

RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt update && apt upgrade --no-install-recommends -y && \
    apt install --no-install-recommends libpq5 -y

# Configure container user and group
RUN groupadd -r warg-server && useradd --no-log-init -r -g warg-server warg-server
USER warg-server

COPY --chown=warg-server --chmod=700 crates/server/entrypoint.sh /

COPY --from=builder --chown=warg-server /usr/local/bin/warg-server /usr/local/bin/

ENTRYPOINT ["/entrypoint.sh"]
