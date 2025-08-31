#![forbid(unsafe_code)]

use std::fs::{create_dir_all, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

/// Placeholder, non-cryptographic hash chain for audit events.
/// Replace with BLAKE3 in a future milestone.
#[derive(Debug, Clone, Copy)]
struct ChainHash(u64);

impl ChainHash {
    fn new() -> Self {
        Self(0)
    }
    fn update(&mut self, msg: &str) {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.0.hash(&mut h);
        msg.hash(&mut h);
        self.0 = h.finish();
    }
}

pub struct AuditLog {
    file: BufWriter<std::fs::File>,
    state: ChainHash,
    _path: PathBuf,
}

impl AuditLog {
    pub fn new(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| e.to_string())?;
        Ok(Self {
            file: BufWriter::new(file),
            state: ChainHash::new(),
            _path: path.to_path_buf(),
        })
    }

    pub fn append(&mut self, message: &str) -> Result<(), String> {
        self.state.update(message);
        let line = format!("{}|{}\n", self.state.0, message);
        self.file
            .write_all(line.as_bytes())
            .map_err(|e| e.to_string())?;
        self.file.flush().map_err(|e| e.to_string())
    }
}
