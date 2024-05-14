FROM rust:1.78-slim AS builder

ARG FEATURES=postgres

WORKDIR /usr/src/bytecodealliance/registry

# musl-dev libpq-dev
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt update && apt upgrade --no-install-recommends -y && \
    apt install --no-install-recommends pkg-config libssl-dev libpq-dev -y

# Build diesel CLI
RUN cargo install diesel_cli --no-default-features --features postgres

COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/src/bytecodealliance/registry/target \
    cargo build --release --workspace --features "$FEATURES" && \
    cp target/release/warg target/release/warg-server /usr/local/bin

FROM debian:stable-slim AS migration

WORKDIR /app

# Install libpq5
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt update && apt upgrade --no-install-recommends -y && \
    apt install --no-install-recommends -y libpq5

# Copy diesl CLI from builder
COPY --from=builder /usr/local/cargo/bin/diesel /usr/local/bin

# Copy migration required files from source
COPY ./crates/server/diesel.toml ./
COPY ./crates/server/src/datastore/postgres ./src/datastore/postgres

CMD ["/usr/local/bin/diesel", "migration", "run", "--migration-dir", "./src/datastore/postgres/migrations"]

FROM debian:stable-slim AS warg

RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt update && apt upgrade --no-install-recommends -y && \
    apt install --no-install-recommends openssl -y

# Configure container user and group
RUN groupadd -r warg && useradd --no-log-init -r -g warg warg
USER warg

COPY --from=builder --chown=warg /usr/local/bin/warg /usr/local/bin/

ENTRYPOINT ["/usr/local/bin/warg"]

FROM debian:stable-slim

# Create container user and group
RUN groupadd -g 1000 -r warg-server && useradd --no-log-init -u 1000 -r -g warg-server warg-server

# Create volume for content directory
ENV CONTENT_DIR=/var/lib/warg-server/data
VOLUME $CONTENT_DIR
RUN mkdir -p "$CONTENT_DIR" && chown warg-server:warg-server "$CONTENT_DIR"

# Configure port settings
EXPOSE 8090

RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt update && apt upgrade --no-install-recommends -y && \
    apt install --no-install-recommends libpq5 -y

USER warg-server

COPY --chown=warg-server --chmod=700 crates/server/entrypoint.sh /

COPY --from=builder --chown=warg-server /usr/local/bin/warg-server /usr/local/bin/

ENTRYPOINT ["/entrypoint.sh"]
