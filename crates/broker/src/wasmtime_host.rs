#![allow(dead_code)]

// This module is compiled only when the `wasmtime-host` feature is enabled.
// It implements the entrypoint for running a WASM component and wiring host
// implementations from the broker to the component-generated bindings.

#[cfg(feature = "wasmtime-host")]
mod bindings {
    wasmtime::component::bindgen!({
        path: "../wit",
        world: "app",
        trappable_imports: true,
        async: false,
    });
}

#[cfg(feature = "wasmtime-host")]
mod impls {
    use super::*;
    use crate::wasmtime_host::bindings;
    use anyhow::Result;
    use std::fs;
    use std::path::Path;
    use wasmtime::component::{Component, Linker};
    use wasmtime::{Config, Engine, Store};
    use wasmtime_wasi::{WasiCtx, WasiCtxBuilder};

    // Host adapter implementing imported interfaces, delegating to core hosts.
    struct Host<'a> {
        core: CoreCtx<'a>,
    }

    // fs
    impl<'a> bindings::saf::app::fs::Host for Host<'a> {
        fn list_dir(&mut self, path: String) -> Result<Vec<String>> {
            self.core
                .ctx
                .fs
                .list_dir(&path)
                .map_err(|e| anyhow::anyhow!(e))
        }
        fn read_text(&mut self, path: String) -> Result<String> {
            self.core
                .ctx
                .fs
                .read_text(&path)
                .map_err(|e| anyhow::anyhow!(e))
        }
        fn write_text(&mut self, path: String, content: String) -> Result<()> {
            self.core
                .ctx
                .fs
                .write_text(&path, &content)
                .map_err(|e| anyhow::anyhow!(e))
        }
    }

    // net
    impl<'a> bindings::saf::app::net::Host for Host<'a> {
        fn get_text(&mut self, url: String) -> Result<String> {
            self.core
                .ctx
                .net
                .get_text(&url)
                .map_err(|e| anyhow::anyhow!(e))
        }
    }

    // log
    impl<'a> bindings::saf::app::log::Host for Host<'a> {
        fn event(&mut self, message: String) -> Result<()> {
            self.core.ctx.log.event(&message);
            Ok(())
        }
    }

    // time (stub: use system time seconds)
    impl<'a> bindings::saf::app::time::Host for Host<'a> {
        fn now_unix_seconds(&mut self) -> Result<u64> {
            Ok(std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs())
        }
    }

    // rand (deterministic stub for testing; production should use OS RNG)
    impl<'a> bindings::saf::app::rand::Host for Host<'a> {
        fn fill(&mut self, len: u32) -> Result<Vec<u8>> {
            // Use deterministic RNG for reproducible testing
            use rand::{rngs::StdRng, RngCore, SeedableRng};
            let mut rng = StdRng::from_entropy();
            let mut buf = vec![0u8; len as usize];
            rng.fill_bytes(&mut buf);
            Ok(buf)
        }
    }

    pub fn run_component(component_path: &Path, core: CoreCtx) -> Result<(), String> {
        // Engine with component model enabled
        let mut cfg = Config::new();
        cfg.wasm_component_model(true);
        let engine = Engine::new(&cfg).map_err(|e| e.to_string())?;

        if !component_path.exists() {
            return Err(format!("component not found: {}", component_path.display()));
        }

        // Load component
        let bytes = fs::read(component_path).map_err(|e| e.to_string())?;
        let component = Component::from_binary(&engine, &bytes).map_err(|e| e.to_string())?;

        // Store + linker with host stored in state
        struct State<'a> {
            host: Host<'a>,
        }
        let mut store: Store<State> = Store::new(
            &engine,
            State {
                host: Host { core },
            },
        );
        let mut linker: Linker<State> = Linker::new(&engine);

        // Instantiate bindings and provide host implementations
        bindings::saf::app::fs::add_to_linker(&mut linker, |s: &mut State| &mut s.host)
            .map_err(|e| e.to_string())?;
        bindings::saf::app::net::add_to_linker(&mut linker, |s: &mut State| &mut s.host)
            .map_err(|e| e.to_string())?;
        bindings::saf::app::log::add_to_linker(&mut linker, |s: &mut State| &mut s.host)
            .map_err(|e| e.to_string())?;
        bindings::saf::app::time::add_to_linker(&mut linker, |s: &mut State| &mut s.host)
            .map_err(|e| e.to_string())?;
        bindings::saf::app::rand::add_to_linker(&mut linker, |s: &mut State| &mut s.host)
            .map_err(|e| e.to_string())?;

        // Instantiate component
        let (exports, _instance) = bindings::App::instantiate(&mut store, &component, &linker)
            .map_err(|e| e.to_string())?;

        // Call exported start function
        match exports.call_start(&mut store) {
            Ok(s) => {
                // Print or log the returned string for demo
                println!("component.start: {}", s);
                Ok(())
            }
            Err(e) => Err(format!("Component execution failed: {}", e)),
        }
    }
}

#[derive(Clone)]
pub struct CoreCtx<'a> {
    pub ctx: saf_core::Context<'a>,
}

#[cfg(feature = "wasmtime-host")]
pub use impls::run_component;

#[cfg(not(feature = "wasmtime-host"))]
pub fn run_component(_component_path: &std::path::Path, _core: CoreCtx) -> Result<(), String> {
    Err("Component execution requires the 'wasmtime-host' feature".to_string())
}
