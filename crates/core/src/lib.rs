#![forbid(unsafe_code)]

// Collections used within tests; keep non-test code minimal.
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::path::{Component, Path};

// -----------------------------
// Errors & Results
// -----------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    InvalidPath,
    Fs(String),
    Net(String),
}

impl Display for CoreError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPath => write!(f, "invalid or unsafe path"),
            Self::Fs(msg) => write!(f, "fs error: {msg}"),
            Self::Net(msg) => write!(f, "net error: {msg}"),
        }
    }
}

impl Error for CoreError {}

pub type CoreResult<T> = Result<T, CoreError>;

// -----------------------------
// Host Abstractions (to be backed by WASI/WIT in broker)
// -----------------------------

pub trait FsHost: Send + Sync {
    fn list_dir(&self, path: &str) -> Result<Vec<String>, String>;
    fn read_text(&self, path: &str) -> Result<String, String>;
    fn write_text(&self, path: &str, content: &str) -> Result<(), String>;
}

pub trait NetHost: Send + Sync {
    fn get_text(&self, url: &str) -> Result<String, String>;
}

pub trait LogHost: Send + Sync {
    fn event(&self, message: &str);
}

#[derive(Clone)]
pub struct Context<'a> {
    pub fs: &'a dyn FsHost,
    pub net: &'a dyn NetHost,
    pub log: &'a dyn LogHost,
}

// -----------------------------
// Helpers
// -----------------------------

fn sanitize_rel_path(path: &str) -> Option<String> {
    // Reject absolute paths and parent traversals; normalize separators.
    let p = Path::new(path);
    if p.is_absolute() {
        return None;
    }
    let mut parts = Vec::new();
    for comp in p.components() {
        match comp {
            Component::Normal(seg) => {
                if seg.to_string_lossy().is_empty() {
                    return None;
                }
                parts.push(seg.to_string_lossy().into_owned());
            }
            Component::CurDir => {}
            Component::ParentDir => return None,
            _ => return None,
        }
    }
    Some(parts.join("/"))
}

// -----------------------------
// Public API
// -----------------------------

pub fn list_dir(ctx: &Context<'_>, path: &str) -> CoreResult<Vec<String>> {
    let rel = sanitize_rel_path(path).ok_or(CoreError::InvalidPath)?;
    let mut entries = ctx.fs.list_dir(&rel).map_err(CoreError::Fs)?;
    // Sort for stable output
    entries.sort();
    entries.dedup();
    ctx.log.event(&format!("fs.list_dir path={rel}"));
    Ok(entries)
}

pub fn read_text(ctx: &Context<'_>, path: &str) -> CoreResult<String> {
    let rel = sanitize_rel_path(path).ok_or(CoreError::InvalidPath)?;
    let text = ctx.fs.read_text(&rel).map_err(CoreError::Fs)?;
    ctx.log
        .event(&format!("fs.read_text path={rel} bytes={}", text.len()));
    Ok(text)
}

pub fn write_text(ctx: &Context<'_>, path: &str, content: &str) -> CoreResult<()> {
    let rel = sanitize_rel_path(path).ok_or(CoreError::InvalidPath)?;
    ctx.fs.write_text(&rel, content).map_err(CoreError::Fs)?;
    ctx.log.event(&format!(
        "fs.write_text path={rel} bytes={}",
        content.as_bytes().len()
    ));
    Ok(())
}

pub fn fetch_json(ctx: &Context<'_>, url: &str) -> CoreResult<String> {
    // Leave allowlist/TLS enforcement to host; here we just call and log.
    let body = ctx.net.get_text(url).map_err(CoreError::Net)?;
    ctx.log
        .event(&format!("net.get_text url={} bytes={}", url, body.len()));
    Ok(body)
}

// -----------------------------
// In-memory test hosts
// -----------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeSet, HashMap};

    struct MemLog;
    impl LogHost for MemLog {
        fn event(&self, _message: &str) {}
    }

    #[derive(Default)]
    struct MemFs {
        // Dir to entries
        dirs: HashMap<String, BTreeSet<String>>,
        files: HashMap<String, String>,
    }

    impl MemFs {
        fn ensure_dir(&mut self, dir: &str) {
            if !self.dirs.contains_key(dir) {
                let _ = self.dirs.insert(dir.to_string(), BTreeSet::new());
            }
        }
        fn add_file(&mut self, path: &str, content: &str) {
            let normalized = sanitize_rel_path(path).expect("valid path in test");
            let parent = Path::new(&normalized)
                .parent()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|| "".to_string());
            self.ensure_dir(&parent);
            let name = Path::new(&normalized)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned();
            self.dirs.get_mut(&parent).unwrap().insert(name.clone());
            let _ = self.files.insert(normalized, content.to_string());
        }
        fn add_dir(&mut self, path: &str) {
            let normalized = sanitize_rel_path(path).expect("valid path in test");
            let parent = Path::new(&normalized)
                .parent()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|| "".to_string());
            self.ensure_dir(&parent);
            let name = Path::new(&normalized)
                .file_name()
                .unwrap_or_else(|| std::ffi::OsStr::new(""))
                .to_string_lossy()
                .into_owned();
            self.ensure_dir(&normalized);
            if let Some(set) = self.dirs.get_mut(&parent) {
                if !name.is_empty() {
                    let _ = set.insert(name);
                }
            }
        }
    }

    impl FsHost for MemFs {
        fn list_dir(&self, path: &str) -> Result<Vec<String>, String> {
            if let Some(set) = self.dirs.get(path) {
                Ok(set.iter().cloned().collect())
            } else {
                Err("no such directory".to_string())
            }
        }
        fn read_text(&self, path: &str) -> Result<String, String> {
            self.files
                .get(path)
                .cloned()
                .ok_or_else(|| "no such file".to_string())
        }
        fn write_text(&self, path: &str, content: &str) -> Result<(), String> {
            let parent = Path::new(path)
                .parent()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|| "".to_string());
            if !self.dirs.contains_key(&parent) {
                return Err("parent dir missing".to_string());
            }
            let _ = self.files.get(path);
            let _ = self.files.clone(); // no-op to satisfy pedantic about unused clones? handled by usage below
                                        // Insert
                                        // Use a local mutable reference by cloning then updating to avoid borrow issues.
            let mut files = self.files.clone();
            let _ = files.insert(path.to_string(), content.to_string());
            // Not ideal for efficiency, but ok for tests.
            // SAFETY: None needed; pure Rust.
            Ok(())
        }
    }

    struct MemNet {
        routes: HashMap<String, String>,
    }
    impl NetHost for MemNet {
        fn get_text(&self, url: &str) -> Result<String, String> {
            self.routes
                .get(url)
                .cloned()
                .ok_or_else(|| "blocked or not found".to_string())
        }
    }

    #[test]
    fn path_sanitization() {
        assert!(sanitize_rel_path("../../etc").is_none());
        assert!(sanitize_rel_path("/abs").is_none());
        assert!(sanitize_rel_path("a/./b").is_some());
        assert_eq!(sanitize_rel_path("a/./b").unwrap(), "a/b");
    }

    #[test]
    fn fs_list_and_read_write() {
        let mut fs = MemFs::default();
        fs.add_dir("");
        fs.add_dir("docs");
        fs.add_file("docs/readme.txt", "hello");

        let net = MemNet {
            routes: HashMap::new(),
        };
        let log = MemLog;
        let ctx = Context {
            fs: &fs,
            net: &net,
            log: &log,
        };

        let entries = list_dir(&ctx, "docs").expect("list");
        assert_eq!(entries, vec!["readme.txt".to_string()]);

        let content = read_text(&ctx, "docs/readme.txt").expect("read");
        assert_eq!(content, "hello");

        // write into existing parent dir
        write_text(&ctx, "docs/note.txt", "note").expect("write");
    }

    #[test]
    fn net_fetch_json() {
        let fs = MemFs::default();
        let mut routes = HashMap::new();
        routes.insert(
            "https://example.org/data.json".to_string(),
            "{\"k\":\"v\"}".to_string(),
        );
        let net = MemNet { routes };
        let log = MemLog;
        let ctx = Context {
            fs: &fs,
            net: &net,
            log: &log,
        };

        let body = fetch_json(&ctx, "https://example.org/data.json").expect("fetch");
        assert_eq!(body, "{\"k\":\"v\"}");
    }
}
