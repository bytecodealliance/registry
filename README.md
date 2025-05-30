> [!WARNING]  
> This repository is no longer being actively developed by Bytecode Alliance members.
> Work on an OCI-based registry system continues in the
> [bytecodealliance/wasm-pkg-tools repository](https://github.com/bytecodealliance/wasm-pkg-tools).
> Any questions about this repository can be discussed in that repository or the
> [Bytecode Alliance Zulip #SIG-Packaging channel](https://bytecodealliance.zulipchat.com/#narrow/channel/441851-SIG-Packaging).

<div align="center">
  <h1><code>WebAssembly Registry (Warg)</code></h1>

<strong>A <a href="https://bytecodealliance.org/">Bytecode Alliance</a> project</strong>

  <p>
    <strong>The reference implementation of the Warg protocol, client, and server for distributing <a href="https://github.com/WebAssembly/component-model/">WebAssembly components and interfaces</a> as well as core modules.</strong>
  </p>

  <p>
    <a href="https://github.com/bytecodealliance/registry/actions/workflows/main.yml"><img src="https://github.com/bytecodealliance/registry/actions/workflows/main.yml/badge.svg" alt="build status" /></a>
    <a href="https://crates.io/crates/warg-cli"><img src="https://img.shields.io/crates/v/warg-cli.svg?style=flat-square" alt="Crates.io version" /></a>
    <a href="https://crates.io/crates/warg-cli"><img src="https://img.shields.io/crates/d/warg-cli.svg?style=flat-square" alt="Download" /></a>
    <a href="https://docs.rs/warg-client/latest/warg_client/"><img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square" alt="docs.rs docs" /></a>
  </p>
</div>

## Overview

This repository contains the reference implementation of the Warg protocol, a client library,
server, and CLI.

A Warg client and server can be used to distribute WebAssembly components to
various component tooling.

See the [introduction](docs/README.md) for the design decisions and scope.

## Prerequisites

- Install the latest [stable Rust](https://www.rust-lang.org/tools/install).

## Installation

To install or upgrade the `warg` CLI:
```
cargo install warg-cli
```

To install or upgrade the reference implementation server:
```
cargo install warg-server
```


## Getting Started

### Running the server

Before running the server, set the `WARG_OPERATOR_KEY` environment
variable:

```
export WARG_OPERATOR_KEY="ecdsa-p256:I+UlDo0HxyBBFeelhPPWmD+LnklOpqZDkrFP5VduASk="
```

`WARG_OPERATOR_KEY` is the private key of the server operator.

Currently this is sourced through an environment variable, but soon this will 
be sourced via command line arguments or integration with system key rings.

Use `cargo` to run the server:

```
mkdir content
cargo run -p warg-server -- --content-dir content
```

The `content` directory created here is where the server will store package 
contents.

**Note: currently the server stores its state only in memory, so it will be 
lost when the server is restarted. A persistence layer will be added in the 
near future.**

### Setting up the client

Start by configuring the client to use the local server's URL:

```
warg config --registry http://127.0.0.1:8090
```

This creates a [`$CONFIG_DIR/warg/config.json`][config_dir] configuration file; 
the configuration file will specify the home registry URL to use so that the
`--registry` option does not need to be specified for every command.

Data downloaded by the client is stored in [`$CACHE_DIR/warg`][cache_dir] by 
default.

Next, create a new signing key to publish packages with:

```
warg key new --registry 127.0.0.1:8090
```

The new signing key will be stored in your operating system's key store and
used to sign package log entries when publishing to the registry.

[config_dir]: https://docs.rs/dirs/5.0.0/dirs/fn.config_dir.html
[cache_dir]: https://docs.rs/dirs/5.0.0/dirs/fn.cache_dir.html

### Publishing a package

A new package can be initialized by running:

```
warg publish init example:hello
```

This creates a new package in the `example` namespace with the name `hello`.

A version of the package can be published by running:

```
warg publish release --name example:hello --version 0.1.0 hello.wasm
```

This publishes a package named `example:hello` with version `0.1.0` and content from 
`hello.wasm`.

Alternatively, the above can be batched into a single publish operation:

```
warg publish start example:hello
warg publish init example:hello
warg publish release --name example:hello --version 0.1.0 hello.wasm
warg publish submit
```

Here the records created from initializing the package and releasing version
0.1.0 are made as part of the same transaction.

Use `warg publish abort` to abort a pending publish operation.

### Managing package permissions

> Note: The package permissions system is a work in progress.

You can grant permissions to another public key with the `warg publish grant` subcommand:

```
warg publish grant --name example:hello ecdsa-p256:ABC...
```

> You can get your own public key with the `warg key info` subcommand.

By default, both `publish` and `yank` permissions are granted. This can be modified with the `--permission` flag.

Similarly, permissions may be revoked via `warg publish revoke`. Note that
keys are identified by ID (fingerprint) for revocation:

```
warg publish revoke --name example:hello sha256:abc...
```

### Resetting and clearing local data

To reset local package log data for registries:
```
warg reset
```

To clear downloaded package content for all registries:
```
warg clear
```


## Contributing

This is a [Bytecode Alliance](https://bytecodealliance.org/) project, and
follows the Bytecode Alliance's [Code of Conduct](CODE_OF_CONDUCT.md) and
[Organizational Code of Conduct](ORG_CODE_OF_CONDUCT.md).

### Getting the Code

You'll clone the code via `git`:

```
git clone https://github.com/bytecodealliance/registry
```

### Testing Changes

Ideally, there should be tests written for all changes.

Run the tests of the in-memory implementation of the `warg-server`:

```
cargo test --workspace
```

Run the tests of the Postgres implementation of the `warg-server`:

```
docker run -d --name postgres-test -e POSTGRES_PASSWORD=password -p 5433:5432 postgres
diesel database setup --database-url postgres://postgres:password@localhost:5433/test-registry --migration-dir crates/server/src/datastore/postgres/migrations
WARG_DATABASE_URL=postgres://postgres:password@localhost:5433/test-registry cargo test --features postgres -- --nocapture
```

You may need to install [Docker](https://www.docker.com/get-started/) and the
[Diesel CLI](https://diesel.rs/guides/getting-started) first with the Postgres feature.

```
cargo install diesel_cli --no-default-features --features postgres
```


### Submitting Changes

Changes to this repository are managed through pull requests (PRs). Everyone
is welcome to submit a pull request! We'll try to get to reviewing it or
responding to it in at most a few days.

### Code Formatting

Code is required to be formatted with the current Rust stable's `cargo fmt`
command. This is checked on CI.

### Continuous Integration

The CI for the repository is relatively significant. It tests changes on
Windows, macOS, and Linux.
