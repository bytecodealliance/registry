FROM rust:1.69-bullseye AS chef
RUN rustup update stable --no-self-update \
    && rustup default stable \
    && rustup target add wasm32-wasi \
    && rustup target add wasm32-unknown-unknown
RUN mkdir -p /usr/local/protobuf \
    && wget https://github.com/protocolbuffers/protobuf/releases/download/v22.3/protoc-22.3-linux-x86_64.zip \
    && unzip protoc-22.3-linux-x86_64.zip -d /usr/local/protobuf \
    && rm protoc-22.3-linux-x86_64.zip
ENV PATH="${PATH}:/usr/local/protobuf/bin"
RUN cargo install cargo-chef --locked

FROM chef AS planner
WORKDIR /usr/src/bytecodealliance/registry
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
WORKDIR /usr/src/bytecodealliance/registry
COPY --from=planner /usr/src/bytecodealliance/registry/recipe.json ./
RUN cargo chef cook --release --workspace --features "postgres" --recipe-path recipe.json
COPY . .
RUN cargo build --release --workspace --features "postgres" 

FROM debian:bullseye-slim AS warg-server
WORKDIR /app
RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        libpq5 \
    && rm -rf /var/cache/apt/archives /var/lib/apt/lists
COPY --from=builder "/usr/src/bytecodealliance/registry/target/release/warg-server" /usr/local/bin/
ENTRYPOINT [ "/usr/local/bin/warg-server" ]

FROM warg-server AS warg-server-debug
RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        ca-certificates curl netcat inetutils-ping postgresql-client jq \
    && rm -rf /var/cache/apt/archives /var/lib/apt/lists
RUN curl -sSL https://github.com/mikefarah/yq/releases/download/v4.33.3/yq_linux_amd64.tar.gz \
 | tar -O -zxf - ./yq_linux_amd64 \ 
 | tee /usr/local/bin/yq >/dev/null \
 && chmod +x /usr/local/bin/yq

FROM gcr.io/distroless/cc AS warg
COPY --from=builder "/usr/src/bytecodealliance/registry/target/release/warg" /usr/local/bin/
ENTRYPOINT [ "/usr/local/bin/warg" ]

FROM rust:1.69-bullseye AS diesel
RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        libpq5 \
    && rm -rf /var/cache/apt/archives /var/lib/apt/lists
RUN cargo install diesel_cli --no-default-features --features postgres

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
