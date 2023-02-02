#!/usr/bin/env -S bash -e

rustup target list --installed | grep -q wasm32-wasi \
    || (echo 'Please `rustup target add wasm32-wasi`' && exit 1)

set -x

cargo build --target wasm32-wasi --release
cp target/wasm32-wasi/release/simple-grep.wasm simple-grep-1.0.0.wasm

cargo build --target wasm32-wasi --release --features evil
cp target/wasm32-wasi/release/simple-grep.wasm simple-grep-1.0.1.wasm
