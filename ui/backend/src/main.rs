#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;
use std::path::PathBuf;

#[tauri::command]
fn get_history_tickers() -> Result<Vec<String>, String> {
    // 🎯 RESOLVE LOCAL ABSOLUTE WORKSPACE DATA FOLDER PATH BOUNDS:
    // Safely backs out of your flattened ui/backend/ execution layer straight to stock-app/data/
    let mut data_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    data_path.pop(); // Up to ui/
    data_path.pop(); // Up to stock-app/
    data_path.push("data");

    if !data_path.exists() {
        return Ok(Vec::new()); // Safely return an empty list if data tracking hasn't launched yet
    }

    let mut tickers = Vec::new();
    if let Ok(entries) = fs::read_dir(data_path) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_dir() {
                    let folder_name = entry.file_name().to_string_lossy().into_owned();
                    // Strip system junk metadata components safely
                    if !folder_name.starts_with('.') {
                        tickers.push(folder_name);
                    }
                }
            }
        }
    }

    // Sort alphabetically for clean indexing
    tickers.sort();
    Ok(tickers)
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_history_tickers])
        .run(tauri::generate_context!("tauri.conf.json"))
        .expect("error while running tauri application");
}