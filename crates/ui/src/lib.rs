#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};

// Shared state between Tauri commands and the broker
pub struct AppState {
    pub workspace: Mutex<Option<PathBuf>>,
    pub audit_log_path: Mutex<Option<PathBuf>>,
}

// UI event types for communication
#[derive(Serialize, Deserialize, Clone)]
pub enum UiEvent {
    WorkspaceSelected { path: String, id: String },
    FilesListed { entries: Vec<String> },
    FileRead { path: String, content: String },
    NetworkFetched { url: String, response: String },
    AuditEvent { message: String },
    Error { message: String },
}

// Tauri commands for broker interaction
#[tauri::command]
async fn select_workspace(app: AppHandle) -> Result<String, String> {
    // Trigger workspace picker through broker
    // For now, return a placeholder
    app.emit_all(
        "workspace-selected",
        UiEvent::WorkspaceSelected {
            path: "/tmp/workspace".to_string(),
            id: "demo_workspace".to_string(),
        },
    )
    .map_err(|e| e.to_string())?;

    Ok("Workspace selection initiated".to_string())
}

#[tauri::command]
async fn list_directory(app: AppHandle, path: String) -> Result<Vec<String>, String> {
    // Call broker's list_dir function
    // For demo, return mock data
    let entries = vec![
        "documents".to_string(),
        "images".to_string(),
        "config.json".to_string(),
        "readme.txt".to_string(),
    ];

    app.emit_all(
        "files-listed",
        UiEvent::FilesListed {
            entries: entries.clone(),
        },
    )
    .map_err(|e| e.to_string())?;

    Ok(entries)
}

#[tauri::command]
async fn read_file(app: AppHandle, path: String) -> Result<String, String> {
    // Call broker's read_text function
    // For demo, return mock content
    let content = match path.as_str() {
        "readme.txt" => "# Secure App Framework\n\nThis is a demo workspace.",
        "config.json" => "{\n  \"app\": \"secure-app-framework\",\n  \"version\": \"0.1.0\"\n}",
        _ => "File content not available in demo mode.",
    };

    app.emit_all(
        "file-read",
        UiEvent::FileRead {
            path: path.clone(),
            content: content.to_string(),
        },
    )
    .map_err(|e| e.to_string())?;

    Ok(content.to_string())
}

#[tauri::command]
async fn fetch_url(app: AppHandle, url: String) -> Result<String, String> {
    // Call broker's fetch_json function
    // For demo, return mock response
    let response = if url.contains("example.org") {
        "{\"status\": \"success\", \"data\": \"Demo response from example.org\"}"
    } else if url.contains("httpbin.org") {
        "{\"url\": \"https://httpbin.org/json\", \"json\": {\"demo\": true}}"
    } else {
        "{\"error\": \"URL not allowed by policy\"}"
    };

    app.emit_all(
        "network-fetched",
        UiEvent::NetworkFetched {
            url: url.clone(),
            response: response.to_string(),
        },
    )
    .map_err(|e| e.to_string())?;

    Ok(response.to_string())
}

#[tauri::command]
async fn get_audit_log(app: AppHandle) -> Result<Vec<String>, String> {
    // Read audit log from broker
    // For demo, return mock entries
    let entries = vec![
        "2024-01-01 10:00:00 | broker.start".to_string(),
        "2024-01-01 10:00:01 | fs.list_dir path=".to_string(),
        "2024-01-01 10:00:02 | net.get_text url=https://httpbin.org/json".to_string(),
    ];

    Ok(entries)
}

pub fn launch() -> Result<(), String> {
    tauri::Builder::default()
        .manage(AppState {
            workspace: Mutex::new(None),
            audit_log_path: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            select_workspace,
            list_directory,
            read_file,
            fetch_url,
            get_audit_log
        ])
        .run(tauri::generate_context!())
        .map_err(|e| format!("Failed to launch Tauri app: {}", e))?;

    Ok(())
}
