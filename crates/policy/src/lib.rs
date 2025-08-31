#![forbid(unsafe_code)]

#[derive(Debug, Default, Clone)]
pub struct Policy {
    pub allowed_domains: Vec<String>,
    pub max_bytes: u64,
}

impl Policy {
    pub fn new() -> Self {
        Self {
            allowed_domains: Vec::new(),
            max_bytes: 10 * 1024 * 1024,
        }
    }
}
