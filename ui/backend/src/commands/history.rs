use std::fs;
use std::path::PathBuf;

fn get_data_directory_path() -> PathBuf {
    let mut data_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    data_path.pop(); // Up to ui/
    data_path.pop(); // Up to stock-app/
    data_path.push("data");
    data_path
}

#[tauri::command]
pub fn get_history_tickers() -> Result<Vec<String>, String> {
    let data_path = get_data_directory_path();

    if !data_path.exists() {
        return Ok(Vec::new());
    }

    let mut tickers = Vec::new();
    if let Ok(entries) = fs::read_dir(data_path) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_dir() {
                    let folder_name = entry.file_name().to_string_lossy().into_owned();
                    if !folder_name.starts_with('.') {
                        tickers.push(folder_name);
                    }
                }
            }
        }
    }

    tickers.sort();
    Ok(tickers)
}