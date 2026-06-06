// stock-app/ui/backend/src/commands/data_dir.rs

use tauri::{command, AppHandle, Manager};
use std::path::{PathBuf, Path};
use std::fs::{create_dir_all, write, read_to_string};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
struct AppConfig {
    custom_data_dir: Option<String>,
}

fn get_config_path(app_handle: &AppHandle) -> PathBuf {
    let mut path = app_handle.path().app_config_dir().unwrap_or_else(|_| PathBuf::from("."));
    let _ = create_dir_all(&path);
    path.push("config.json");
    path
}

/// 🎯 HEADLESS RESOLVER: Safe to call anywhere without a Tauri AppHandle context
pub fn resolve_data_directory_headless() -> PathBuf {
    // 1. Check if the native frontend's configuration anchor file exists
    if let Ok(path_str) = read_to_string("terminal_config.txt") {
        let trimmed = path_str.trim();
        if !trimmed.is_empty() && Path::new(trimmed).exists() {
            return PathBuf::from(trimmed);
        }
    }

    // 2. Default hardcoded fallback for development environment
    let mut default_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    default_path.pop(); // Back out to ui/
    default_path.pop(); // Back out to stock-app/
    default_path.push("data");
    
    default_path
}

/// Dynamic State Resolver for Tauri
#[command]
pub fn get_active_data_directory(app_handle: AppHandle) -> String {
    // Check our universal local anchor file first
    if let Ok(path_str) = read_to_string("terminal_config.txt") {
        let trimmed = path_str.trim();
        if !trimmed.is_empty() && Path::new(trimmed).exists() {
            return trimmed.to_string();
        }
    }

    let config_file = get_config_path(&app_handle);
    if config_file.exists() {
        if let Ok(content) = read_to_string(config_file) {
            if let Ok(config) = serde_json::from_str::<AppConfig>(&content) {
                if let Some(dir) = config.custom_data_dir {
                    if Path::new(&dir).exists() {
                        return dir;
                    }
                }
            }
        }
    }

    resolve_data_directory_headless().to_string_lossy().to_string()
}

/// Dialog Setter for Tauri
#[command]
pub fn set_custom_data_directory(app_handle: AppHandle, target_path: String) -> Result<String, String> {
    let target = Path::new(&target_path);
    if !target.exists() {
        return Err("The provided system folder target path does not exist on disk.".to_string());
    }

    // Maintain alignment by updating our local terminal config tracker file
    let _ = write("terminal_config.txt", target_path.trim());

    let config_file = get_config_path(&app_handle);
    let new_config = AppConfig {
        custom_data_dir: Some(target_path.clone()),
    };

    let serialized = serde_json::to_string_pretty(&new_config)
        .map_err(|e| format!("Serialization failure: {}", e))?;
        
    write(config_file, serialized)
        .map_err(|e| format!("Failed to preserve configuration matrix state to disk: {}", e))?;

    println!("🎯 [SYSTEM CONFIG]: Central data directory re-anchored to: {}", target_path);
    Ok(target_path)
}