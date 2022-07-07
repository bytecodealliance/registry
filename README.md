# Wasm Registry Prototype

```bash
$ cargo run -p wasm-registry-prototype-server &
$ cargo run -p wasm-registry-test-client -- publish my-package 1.2.3 README.md  # README.md could be any smallish file
$ cargo run -p wasm-registry-test-client -- fetch my-package 1.2.3
```
