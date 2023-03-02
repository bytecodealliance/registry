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

1. The latest [stable Rust](https://www.rust-lang.org/tools/install).
2. A [protobuf compiler](http://google.github.io/proto-lens/installing-protoc.html).

## Installation

To install `warg-cli`, first you'll want to install
[the latest stable Rust](https://www.rust-lang.org/tools/install) and then
you'll execute to  install the subcommand:

```
cargo install --git https://github.com/bytecodealliance/registry
```

The [currently published crate](https://crates.io/crates/warg-cli)
on crates.io is a nonfunctional placeholder and these instructions will be
updated to install the crates.io package once a proper release is made.

## Getting Started

TODO

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
