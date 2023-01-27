use std::path::Path;

use anyhow::{Context, Result};
use wasmtime::{Engine, Linker, Module, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder};

pub(crate) fn run_wasm(path: impl AsRef<Path>) -> Result<()> {
    let engine = Engine::default();

    let path = path.as_ref();
    let module = Module::from_file(&engine, path)
        .with_context(|| format!("Failed to load Wasm from {path:?}"))?;

    let wasi_ctx = WasiCtxBuilder::new()
        .inherit_stdout()
        .args(&[])?
        .envs(&[])?
        .build();
    let mut store = Store::new(&engine, wasi_ctx);

    let mut linker: Linker<WasiCtx> = Linker::new(&engine);
    wasmtime_wasi::add_to_linker(&mut linker, |cx| cx)?;
    linker.module(&mut store, "", &module)?;

    let func = linker.get_default(&mut store, "")?;
    func.call(&mut store, &[], &mut [])?;

    Ok(())
}
