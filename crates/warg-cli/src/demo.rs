use std::path::Path;

use anyhow::{Context, Result};
use wasmtime::{Engine, Linker, Module, Store};
use wasmtime_wasi::{I32Exit, WasiCtx, WasiCtxBuilder};

pub(crate) fn run_wasm(path: impl AsRef<Path>, args: &[String]) -> Result<()> {
    let engine = Engine::default();

    let path = path.as_ref();
    let module = Module::from_file(&engine, path)
        .with_context(|| format!("Failed to load Wasm from {path:?}"))?;

    let wasi_ctx = WasiCtxBuilder::new()
        .inherit_stdio()
        .arg(&path.file_name().unwrap().to_string_lossy())?
        .args(args)?
        .envs(&[])?
        .build();
    let mut store = Store::new(&engine, wasi_ctx);

    let mut linker: Linker<WasiCtx> = Linker::new(&engine);
    wasmtime_wasi::add_to_linker(&mut linker, |cx| cx)?;
    linker.module(&mut store, "", &module)?;

    let func = linker.get_default(&mut store, "")?;
    let res = func.call(&mut store, &[], &mut []);

    // Handle exit()
    res.or_else(|err| {
        if let Some(I32Exit(status)) = err.downcast_ref() {
            eprintln!("module exited with status {status}");
            Ok(())
        } else {
            Err(err)
        }
    })?;

    Ok(())
}
