// stock-app/ui/backend/src/commands/history.rs

use std::fs;
use crate::commands::data_dir::resolve_data_directory_headless;

/// 🎯 HEADLESS TICKER LIST FETCH: Reads folders cleanly using our active base directory path
pub fn get_history_tickers_headless() -> Vec<String> {
    let data_path = resolve_data_directory_headless();

    if !data_path.exists() {
        return Vec::new();
    }

    let mut tickers = Vec::new();
    if let Ok(entries) = fs::read_dir(data_path) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_dir() {
                    let folder_name = entry.file_name().to_string_lossy().into_owned();
                    // Ignore system metadata files and hidden dot directories
                    if !folder_name.starts_with('.') {
                        tickers.push(folder_name);
                    }
                }
            }
        }
    }

    tickers.sort();
    tickers
}

#[tauri::command]
pub fn get_history_tickers() -> Result<Vec<String>, String> {
    Ok(get_history_tickers_headless())
}