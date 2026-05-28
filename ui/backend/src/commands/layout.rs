use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;

fn get_data_directory_path() -> PathBuf {
    let mut data_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    data_path.pop(); // Up to ui/
    data_path.pop(); // Up to stock-app/
    data_path.push("data");
    data_path
}

#[tauri::command]
pub fn save_workspace_layout(layout_json: String) -> Result<(), String> {
    let mut config_path = get_data_directory_path();
    fs::create_dir_all(&config_path).map_err(|e| e.to_string())?;
    config_path.push("layout_config.json");

    let mut file = fs::File::create(config_path).map_err(|e| e.to_string())?;
    file.write_all(layout_json.as_bytes()).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn load_workspace_layout() -> Result<String, String> {
    let mut config_path = get_data_directory_path();
    config_path.push("layout_config.json");

    if !config_path.exists() {
        return Ok("{}".to_string());
    }

    let mut file = fs::File::open(config_path).map_err(|e| e.to_string())?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).map_err(|e| e.to_string())?;
    Ok(contents)
}