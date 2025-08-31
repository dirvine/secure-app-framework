#![allow(dead_code)]

// This module is compiled only when the `wasmtime-host` feature is enabled.
// It sketches the entrypoint for running a WASM component and wiring host
// implementations from the broker to the component-generated bindings.

#[cfg(feature = "wasmtime-host")]
mod impls {
    // When enabling this feature, add real dependencies:
    // wasmtime = { version = "*", features = ["component-model"] }
    // wasmtime-wasi = "*"
    // wit-bindgen = "*" (or cargo-component generated bindings)

    use super::*;
    use std::path::Path;

    pub fn run_component(_component_path: &Path, _ctx: CoreCtx) -> Result<(), String> {
        // TODO: load component, instantiate with host shims for fs/net/log/time/rand,
        // and drive the test app world entrypoints.
        Err("wasmtime integration not implemented".to_string())
    }
}

#[derive(Clone)]
pub struct CoreCtx<'a> {
    pub ctx: saf_core::Context<'a>,
}

#[cfg(feature = "wasmtime-host")]
pub use impls::run_component;
