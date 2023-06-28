<div align="center">
  <h1><code>WebAssembly Component Registry</code></h1>

<strong>A <a href="https://bytecodealliance.org/">Bytecode Alliance</a> project</strong>

  <p>
    <strong>An implementation of the Warg protocol, client, and server for distributing <a href="https://github.com/WebAssembly/component-model/">WebAssembly components</a>.</strong>
  </p>

  <p>
    <a href="https://github.com/bytecodealliance/registry/actions?query=workflow%3ACI"><img src="https://github.com/bytecodealliance/registry/workflows/Rust/badge.svg" alt="build status" /></a>
    <a href="https://crates.io/crates/warg-cli"><img src="https://img.shields.io/crates/v/warg-cli.svg?style=flat-square" alt="Crates.io version" /></a>
    <a href="https://crates.io/crates/warg-cli"><img src="https://img.shields.io/crates/d/warg-cli.svg?style=flat-square" alt="Download" /></a>
    <a href="https://bytecodealliance.github.io/warg-cli/"><img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square" alt="docs.rs docs" /></a>
  </p>
</div>

## Overview

This repository contains an implementation of the Warg protocol, a client,
server, and CLI.

A Warg client and server can be used to distribute WebAssembly components to
various component tooling.

## Prerequisites

- The latest [stable Rust](https://www.rust-lang.org/tools/install).

## Installation

To install `warg`, first you'll want to install
[the latest stable Rust](https://www.rust-lang.org/tools/install) and then
you'll execute to  install the subcommand:

```
cargo install --git https://github.com/bytecodealliance/registry
```

The [currently published crate](https://crates.io/crates/warg-cli)
on crates.io is a nonfunctional placeholder and these instructions will be
updated to install the crates.io package once a proper release is made.

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
the configuration file will specify the default registry URL to use so that the
`--registry` option does not need to be specified for every command.

Data downloaded by the client is stored in [`$CACHE_DIR/warg`][cache_dir] by 
default.

Next, create a new signing key to publish packages with:

```
warg key new 127.0.0.1
```

The new signing key will be stored in your operating system's key store and
used to sign package log entries when publishing to the registry.

[config_dir]: https://docs.rs/dirs/5.0.0/dirs/fn.config_dir.html
[cache_dir]: https://docs.rs/dirs/5.0.0/dirs/fn.cache_dir.html

### Publishing a package

A new package can be initialized by running:

```
warg publish init hello
```

This creates a new package named `hello` in the registry.

A version of the package can be published by running:

```
warg publish release --name hello --version 0.1.0 hello.wasm
```

This publishes a package named `hello` with version `0.1.0` and content from 
`hello.wasm`.

Alternatively, the above can be batched into a single publish operation:

```
warg publish start hello
warg publish init hello
warg publish release --name hello --version 0.1.0 hello.wasm
warg publish submit
```

Here the records created from initializing the package and releasing version
0.1.0 are made as part of the same transaction.

Use `warg publish abort` to abort a pending publish operation.

### Running a package

For demonstration purposes, the `run` command in `warg` will download and 
run a package using [Wasmtime](https://wasmtime.dev/).

The package is expected to be a Wasm module implementing a WASI command.

A demo module that implements a simple "grep" tool is available in `demo/simple-grep-1.0.0.wasm`.

To publish the demo module:

```
warg publish start simple-grep
warg publish init simple-grep
warg publish release --name simple-grep --version 1.0.0 demo/simple-grep-1.0.0.wasm
warg publish submit
```

To run the demo package:

```
echo 'hello world' | warg run simple-grep hello
```

This should download and run the package, and print out the line `hello world` as it matches the pattern `hello`.

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

Ideally, there should be tests written for all changes. Test can be run via:

```
cargo test --all
```

### Testing with Containers

See the [local infra documentation](infra/local/README.md) on how to develop and test with locally running containers.

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
