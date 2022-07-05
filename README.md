# Wasm Registry Prototype

```bash
$ cargo run --bin wargd &
$ cargo run --bin warg-test-client publish my-package 1.2.3 README.md  # README.md could be any smallish file
$ cargo run --bin warg-test-client fetch my-package 1.2.3
```
