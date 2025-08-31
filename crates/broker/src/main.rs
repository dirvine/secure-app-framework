use std::env;
use std::fs::{create_dir_all, read_dir, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

use saf_audit::AuditLog;
use saf_core::{fetch_json, list_dir as core_list_dir, Context, FsHost, LogHost, NetHost};
use saf_policy::Policy;

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

fn main() {
    // Minimal CLI: broker [--workspace <path>]
    let mut args = env::args().skip(1);
    let mut workspace = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    while let Some(a) = args.next() {
        if a == "--workspace" {
            if let Some(p) = args.next() {
                workspace = PathBuf::from(p);
            }
        }
    }

    let audit_path = workspace.join(".saf").join("audit.log");
    let audit_log = match AuditLog::new(&audit_path) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("failed to init audit log: {e}");
            return;
        }
    };
    let log = StdLogHost {
        inner: std::sync::Mutex::new(audit_log),
    };
    let fs = StdFsHost {
        root: workspace.clone(),
    };
    let policy = Policy::new().with_allowed_domains(vec!["example.org".to_string()]);
    let net = StubNetHost { policy };

    let ctx = Context {
        fs: &fs,
        net: &net,
        log: &log,
    };
    log.event("broker.start");

    // Demo: list workspace root
    match core_list_dir(&ctx, "") {
        Ok(entries) => {
            println!(
                "workspace: {} ({} entries)",
                workspace.display(),
                entries.len()
            );
        }
        Err(e) => eprintln!("list_dir error: {e}"),
    }

    // Demo: try a fetch to allowed example URL
    match fetch_json(&ctx, "https://example.org/data.json") {
        Ok(body) => println!("fetched example.org: {} bytes", body.len()),
        Err(e) => eprintln!("fetch error: {e}"),
    }
}
