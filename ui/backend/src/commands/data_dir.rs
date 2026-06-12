// stock-app/ui/backend/src/commands/data_dir.rs

use std::path::{PathBuf, Path};
use std::fs::{read_to_string};

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