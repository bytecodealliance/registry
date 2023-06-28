# Create shared chef base image for planner and builder targets.
# Chef is used to optimize caching of Rust dependency building.
# Separate stages are used to avoid error-prone cleaning of temporary files
# during the chef planner stage that generates a recipe
FROM rust:1.69-bullseye AS chef
RUN rustup update stable --no-self-update \
    && rustup default stable \
    && rustup target add wasm32-wasi \
    && rustup target add wasm32-unknown-unknown
RUN cargo install cargo-chef --locked

# Create Chef's recipe.json which captures dependency build information.
FROM chef AS planner
WORKDIR /usr/src/bytecodealliance/registry
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Uses Chef's recipe.json to first build dependencies in a layer that is cached
# before building the project thereby limiting unnecessary rebuilding if only
# source code is changed.
FROM chef AS builder
WORKDIR /usr/src/bytecodealliance/registry
COPY --from=planner /usr/src/bytecodealliance/registry/recipe.json ./
RUN cargo chef cook --release --workspace --features "postgres" --recipe-path recipe.json
COPY . .
RUN cargo build --release --workspace --features "postgres" 

# A minimal container with just the warg-server binary. It uses a slim base
# image instead of distroless for ease of installing the libpq5 library which
# is required for the postgres feature.
#
# TODO: Use distroless by copying in contents of libpq5 as a layer.
FROM debian:bullseye-slim AS warg-server
WORKDIR /app
RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        libpq5 \
    && rm -rf /var/cache/apt/archives /var/lib/apt/lists
COPY --from=builder "/usr/src/bytecodealliance/registry/target/release/warg-server" /usr/local/bin/
ENTRYPOINT [ "/usr/local/bin/warg-server" ]

# A warg-server container variant with some additional utilities installed for
# debugging.
FROM warg-server AS warg-server-debug
RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        ca-certificates curl netcat inetutils-ping postgresql-client jq \
    && rm -rf /var/cache/apt/archives /var/lib/apt/lists
RUN curl -sSL https://github.com/mikefarah/yq/releases/download/v4.33.3/yq_linux_amd64.tar.gz \
 | tar -O -zxf - ./yq_linux_amd64 \ 
 | tee /usr/local/bin/yq >/dev/null \
 && chmod +x /usr/local/bin/yq

# A container for the warg cli tool which can be used in one-off tasks or
# scheduled cron jobs for example.
FROM gcr.io/distroless/cc AS warg
COPY --from=builder "/usr/src/bytecodealliance/registry/target/release/warg" /usr/local/bin/
ENTRYPOINT [ "/usr/local/bin/warg" ]

# A base image for building database migration utiltiy containers based on
# diesel, the library used by warg for the postgres feature.
FROM rust:1.69-bullseye AS diesel
RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        libpq5 \
    && rm -rf /var/cache/apt/archives /var/lib/apt/lists
RUN cargo install diesel_cli --no-default-features --features postgres

# A container with an entrypoint configured to apply the warg postgres database
# migrations using the diesel utility. Add the database-url option as a command
# line argument or an environment variable to use.
FROM debian:bullseye-slim AS warg-postgres-migration
WORKDIR /app
RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        libpq5 \
    && rm -rf /var/cache/apt/archives /var/lib/apt/lists
COPY --from=diesel /usr/local/cargo/bin/diesel /usr/local/bin
COPY ./crates/server/diesel.toml ./
COPY ./crates/server/src/datastore/postgres ./src/datastore/postgres
ENTRYPOINT [ "/usr/local/bin/diesel", "migration", "run", "--migration-dir", "./src/datastore/postgres/migrations" ]
