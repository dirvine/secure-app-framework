#![allow(dead_code)]

// This module is compiled only when the `wasmtime-host` feature is enabled.
// It sketches the entrypoint for running a WASM component and wiring host
// implementations from the broker to the component-generated bindings.

#[cfg(feature = "wasmtime-host")]
mod impls {
    use super::*;
    use std::fs;
    use std::path::Path;
    use wasmtime::component::{Component, Linker};
    use wasmtime::{Config, Engine, Store};

    // Minimal loader to validate component parsing. Host bindings come next.
    pub fn run_component(component_path: &Path, _core: CoreCtx) -> Result<(), String> {
        let mut cfg = Config::new();
        cfg.wasm_component_model(true);

        let engine = Engine::new(&cfg).map_err(|e| e.to_string())?;
        if !component_path.exists() {
            return Err(format!("component not found: {}", component_path.display()));
        }

        // Basic read for clearer errors before Wasmtime parse
        let bytes = fs::read(component_path).map_err(|e| e.to_string())?;
        let _component = Component::from_binary(&engine, &bytes).map_err(|e| e.to_string())?;

        // Placeholder store and linker; no instantiation yet
        struct HostState;
        let mut store = Store::new(&engine, HostState);
        let _linker: Linker<HostState> = Linker::new(&engine);

        Err("component loaded; host bindings not implemented yet".to_string())
    }
}

#[derive(Clone)]
pub struct CoreCtx<'a> {
    pub ctx: saf_core::Context<'a>,
}

#[cfg(feature = "wasmtime-host")]
pub use impls::run_component;
