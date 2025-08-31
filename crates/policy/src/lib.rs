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

    pub fn with_allowed_domains(mut self, domains: Vec<String>) -> Self {
        self.allowed_domains = domains;
        self
    }

    pub fn is_url_allowed(&self, url: &str) -> bool {
        for d in &self.allowed_domains {
            if url.starts_with(&format!("https://{d}/")) || url == format!("https://{d}") {
                return true;
            }
        }
        false
    }
}
