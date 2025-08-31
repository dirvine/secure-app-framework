use std::path::{Path, PathBuf};

/// Cross-platform workspace picker interface
pub trait WorkspacePicker {
    /// Pick a workspace directory, returning the path and a persistent token
    fn pick_workspace(&self) -> Result<(PathBuf, String), String>;

    /// Restore a workspace from a persistent token
    fn restore_workspace(&self, token: &str) -> Result<PathBuf, String>;
}

/// Persistent workspace storage
pub struct WorkspaceStore {
    store_path: PathBuf,
}

impl WorkspaceStore {
    pub fn new() -> Result<Self, String> {
        let store_path = dirs::data_dir()
            .ok_or("No data directory available")?
            .join("secure-app-framework")
            .join("workspaces.json");

        if let Some(parent) = store_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        Ok(Self { store_path })
    }

    pub fn save_workspace(&self, id: &str, path: &Path, token: &str) -> Result<(), String> {
        let mut workspaces: std::collections::HashMap<String, serde_json::Value> =
            if self.store_path.exists() {
                let content =
                    std::fs::read_to_string(&self.store_path).map_err(|e| e.to_string())?;
                serde_json::from_str(&content).unwrap_or_default()
            } else {
                std::collections::HashMap::new()
            };

        workspaces.insert(
            id.to_string(),
            serde_json::json!({
                "path": path.to_string_lossy(),
                "token": token,
                "created": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            }),
        );

        let content = serde_json::to_string_pretty(&workspaces).map_err(|e| e.to_string())?;
        std::fs::write(&self.store_path, content).map_err(|e| e.to_string())
    }

    pub fn load_workspace(&self, id: &str) -> Result<(PathBuf, String), String> {
        let content = std::fs::read_to_string(&self.store_path).map_err(|e| e.to_string())?;
        let workspaces: std::collections::HashMap<String, serde_json::Value> =
            serde_json::from_str(&content).map_err(|e| e.to_string())?;

        let entry = workspaces
            .get(id)
            .ok_or_else(|| format!("Workspace {} not found", id))?;

        let path = entry
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or("Invalid workspace entry")?;

        let token = entry
            .get("token")
            .and_then(|v| v.as_str())
            .ok_or("Invalid workspace entry")?;

        Ok((PathBuf::from(path), token.to_string()))
    }

    pub fn list_workspaces(&self) -> Result<Vec<String>, String> {
        if !self.store_path.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(&self.store_path).map_err(|e| e.to_string())?;
        let workspaces: std::collections::HashMap<String, serde_json::Value> =
            serde_json::from_str(&content).map_err(|e| e.to_string())?;

        Ok(workspaces.keys().cloned().collect())
    }
}

#[cfg(target_os = "linux")]
pub struct LinuxPicker;

#[cfg(target_os = "linux")]
impl LinuxPicker {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(target_os = "linux")]
impl WorkspacePicker for LinuxPicker {
    fn pick_workspace(&self) -> Result<(PathBuf, String), String> {
        // For now, use current directory as fallback
        let path = std::env::current_dir().map_err(|e| e.to_string())?;
        let token = path.to_string_lossy().to_string();
        Ok((path, token))
    }

    fn restore_workspace(&self, token: &str) -> Result<PathBuf, String> {
        let path = PathBuf::from(token);
        if path.exists() && path.is_dir() {
            Ok(path)
        } else {
            Err("Workspace directory no longer exists".to_string())
        }
    }
}

#[cfg(target_os = "windows")]
pub struct WindowsPicker;

#[cfg(target_os = "windows")]
impl WindowsPicker {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(target_os = "windows")]
impl WorkspacePicker for WindowsPicker {
    fn pick_workspace(&self) -> Result<(PathBuf, String), String> {
        // For now, use current directory as fallback
        let path = std::env::current_dir().map_err(|e| e.to_string())?;
        let token = path.to_string_lossy().to_string();
        Ok((path, token))
    }

    fn restore_workspace(&self, token: &str) -> Result<PathBuf, String> {
        let path = PathBuf::from(token);
        if path.exists() && path.is_dir() {
            Ok(path)
        } else {
            Err("Workspace directory no longer exists".to_string())
        }
    }
}

#[cfg(target_os = "macos")]
pub struct MacPicker;

#[cfg(target_os = "macos")]
impl MacPicker {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(target_os = "macos")]
impl WorkspacePicker for MacPicker {
    fn pick_workspace(&self) -> Result<(PathBuf, String), String> {
        // For now, use current directory as fallback
        let path = std::env::current_dir().map_err(|e| e.to_string())?;
        let token = path.to_string_lossy().to_string();
        Ok((path, token))
    }

    fn restore_workspace(&self, token: &str) -> Result<PathBuf, String> {
        let path = PathBuf::from(token);
        if path.exists() && path.is_dir() {
            Ok(path)
        } else {
            Err("Workspace directory no longer exists".to_string())
        }
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
pub struct FallbackPicker;

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
impl FallbackPicker {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
impl WorkspacePicker for FallbackPicker {
    fn pick_workspace(&self) -> Result<(PathBuf, String), String> {
        let path = std::env::current_dir().map_err(|e| e.to_string())?;
        let token = path.to_string_lossy().to_string();
        Ok((path, token))
    }

    fn restore_workspace(&self, token: &str) -> Result<PathBuf, String> {
        let path = PathBuf::from(token);
        if path.exists() && path.is_dir() {
            Ok(path)
        } else {
            Err("Workspace directory no longer exists".to_string())
        }
    }
}

/// Create a platform-specific workspace picker
pub fn create_picker() -> Box<dyn WorkspacePicker + Send + Sync> {
    #[cfg(target_os = "linux")]
    {
        Box::new(LinuxPicker::new())
    }
    #[cfg(target_os = "windows")]
    {
        Box::new(WindowsPicker::new())
    }
    #[cfg(target_os = "macos")]
    {
        Box::new(MacPicker::new())
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        Box::new(FallbackPicker::new())
    }
}
