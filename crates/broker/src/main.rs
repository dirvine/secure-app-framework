use std::env;
use std::fs::{create_dir_all, read_dir, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

use saf_audit::AuditLog;
use saf_core::{fetch_json, list_dir as core_list_dir, Context, FsHost, LogHost, NetHost};
use saf_policy::Policy;
mod wasmtime_host;
mod workspace_picker;

#[cfg(feature = "ui")]
use tauri::{AppHandle, Manager};

fn sanitize_rel_path(path: &str) -> Option<String> {
    let p = Path::new(path);
    if p.is_absolute() {
        return None;
    }
    let mut parts = Vec::new();
    for comp in p.components() {
        match comp {
            Component::Normal(seg) => {
                let s = seg.to_string_lossy();
                if s.is_empty() {
                    return None;
                }
                parts.push(s.into_owned());
            }
            Component::CurDir => {}
            Component::ParentDir => return None,
            _ => return None,
        }
    }
    Some(parts.join("/"))
}

struct StdFsHost {
    root: PathBuf,
}
impl FsHost for StdFsHost {
    fn list_dir(&self, path: &str) -> Result<Vec<String>, String> {
        let rel = sanitize_rel_path(path).ok_or_else(|| "invalid path".to_string())?;
        let dir = self.root.join(rel);
        let mut out = Vec::new();
        let entries = read_dir(&dir).map_err(|e| e.to_string())?;
        for ent in entries {
            let ent = ent.map_err(|e| e.to_string())?;
            if let Some(name) = ent.file_name().to_str() {
                out.push(name.to_string());
            }
        }
        Ok(out)
    }
    fn read_text(&self, path: &str) -> Result<String, String> {
        let rel = sanitize_rel_path(path).ok_or_else(|| "invalid path".to_string())?;
        let p = self.root.join(rel);
        let mut f = File::open(&p).map_err(|e| e.to_string())?;
        let mut s = String::new();
        f.read_to_string(&mut s).map_err(|e| e.to_string())?;
        Ok(s)
    }
    fn write_text(&self, path: &str, content: &str) -> Result<(), String> {
        let rel = sanitize_rel_path(path).ok_or_else(|| "invalid path".to_string())?;
        let p = self.root.join(&rel);
        if let Some(parent) = p.parent() {
            create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut f = File::create(&p).map_err(|e| e.to_string())?;
        f.write_all(content.as_bytes()).map_err(|e| e.to_string())
    }
}

struct StdLogHost {
    inner: std::sync::Mutex<AuditLog>,
}
impl LogHost for StdLogHost {
    fn event(&self, message: &str) {
        if let Ok(mut g) = self.inner.lock() {
            let _ = g.append(message);
        }
    }
}

struct StubNetHost {
    policy: Policy,
}
impl NetHost for StubNetHost {
    fn get_text(&self, url: &str) -> Result<String, String> {
        if !self.policy.is_url_allowed(url) {
            return Err("blocked by policy".to_string());
        }
        if url == "https://example.org/data.json" {
            return Ok("{\"example\":true}".to_string());
        }
        Err("network not implemented".to_string())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let mut workspace_id = None;
    let mut run_component = None;
    let mut interactive = true;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--workspace-id" => {
                if i + 1 < args.len() {
                    workspace_id = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("--workspace-id requires an argument");
                    std::process::exit(1);
                }
            }
            "--run-component" => {
                if i + 1 < args.len() {
                    run_component = Some(PathBuf::from(&args[i + 1]));
                    i += 2;
                } else {
                    eprintln!("--run-component requires an argument");
                    std::process::exit(1);
                }
            }
            "--headless" => {
                interactive = false;
                i += 1;
            }
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                print_help();
                std::process::exit(1);
            }
        }
    }

    // Initialize workspace store
    let workspace_store = workspace_picker::WorkspaceStore::new()
        .map_err(|e| format!("Failed to initialize workspace store: {}", e))?;

    // Determine workspace
    let workspace = if let Some(id) = workspace_id {
        // Restore existing workspace
        let picker = workspace_picker::create_picker();
        let (path, _token) = workspace_store
            .load_workspace(&id)
            .map_err(|e| format!("Failed to load workspace {}: {}", id, e))?;

        let restored_path = picker
            .restore_workspace(&_token)
            .map_err(|e| format!("Failed to restore workspace access: {}", e))?;

        if restored_path != path {
            return Err(format!(
                "Workspace path mismatch: expected {}, got {}",
                path.display(),
                restored_path.display()
            )
            .into());
        }

        println!("Restored workspace: {}", path.display());
        path
    } else if interactive {
        // Pick new workspace interactively
        let picker = workspace_picker::create_picker();
        let (path, token) = picker
            .pick_workspace()
            .map_err(|e| format!("Failed to pick workspace: {}", e))?;

        let id = format!("workspace_{}", uuid::Uuid::new_v4().simple());
        workspace_store
            .save_workspace(&id, &path, &token)
            .map_err(|e| format!("Failed to save workspace: {}", e))?;

        println!("Selected workspace: {} (ID: {})", path.display(), id);
        path
    } else {
        // Use current directory in headless mode
        env::current_dir().unwrap_or(PathBuf::from("."))
    };

    // Initialize audit log
    let audit_path = workspace.join(".saf").join("audit.log");
    let audit_log =
        AuditLog::new(&audit_path).map_err(|e| format!("Failed to initialize audit log: {}", e))?;

    let log = StdLogHost {
        inner: std::sync::Mutex::new(audit_log),
    };

    let fs = StdFsHost {
        root: workspace.clone(),
    };

    let policy = Policy::new()
        .with_allowed_domains(vec!["example.org".to_string(), "httpbin.org".to_string()]);

    let net = StubNetHost { policy };

    let ctx = Context {
        fs: &fs,
        net: &net,
        log: &log,
    };

    log.event("broker.start");

    // Handle component execution
    if let Some(comp_path) = run_component {
        #[cfg(feature = "wasmtime-host")]
        {
            let core_ctx = wasmtime_host::CoreCtx { ctx };
            wasmtime_host::run_component(&comp_path, core_ctx)
                .map_err(|e| format!("Component execution failed: {}", e))?;
            return Ok(());
        }
        #[cfg(not(feature = "wasmtime-host"))]
        {
            return Err(
                "--run-component requires building with the 'wasmtime-host' feature".into(),
            );
        }
    }

    // Launch UI or run demo
    if interactive {
        #[cfg(feature = "ui")]
        {
            launch_ui(workspace, ctx).await?;
        }
        #[cfg(not(feature = "ui"))]
        {
            run_demo(workspace, ctx).await?;
        }
    } else {
        run_demo(workspace, ctx).await?;
    }

    Ok(())
}

fn print_help() {
    println!("Secure App Framework Broker");
    println!();
    println!("USAGE:");
    println!("    broker [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    --workspace-id <ID>    Restore a previously saved workspace");
    println!("    --run-component <PATH> Execute a WASM component");
    println!("    --headless             Run without UI");
    println!("    --help, -h             Show this help message");
    println!();
    println!("Without arguments, launches the interactive workspace picker.");
}

#[cfg(feature = "ui")]
async fn launch_ui(workspace: PathBuf, ctx: Context<'_>) -> Result<(), Box<dyn std::error::Error>> {
    use saf_ui::launch;

    // For now, just run the demo - UI integration would go here
    println!("UI mode selected but not yet implemented - running demo instead");
    run_demo(workspace, ctx).await?;
    Ok(())
}

async fn run_demo(workspace: PathBuf, ctx: Context<'_>) -> Result<(), Box<dyn std::error::Error>> {
    // Demo: list workspace root
    match core_list_dir(&ctx, "") {
        Ok(entries) => {
            println!(
                "workspace: {} ({} entries)",
                workspace.display(),
                entries.len()
            );
            for entry in entries {
                println!("  {}", entry);
            }
        }
        Err(e) => eprintln!("list_dir error: {}", e),
    }

    // Demo: try a fetch to allowed example URL
    match fetch_json(&ctx, "https://httpbin.org/json") {
        Ok(body) => println!("fetched httpbin.org: {} bytes", body.len()),
        Err(e) => eprintln!("fetch error: {}", e),
    }

    Ok(())
}
