// downloader-rust/src/misc/download.rs
use std::fs;
use std::path::{Path, PathBuf};

/// Generates the directory hierarchy structure: global_data_dir/{symbol}/bse_{api_name}
pub fn build_save_directory(global_data_dir: &Path, symbol: &str, api_name: &str) -> Result<PathBuf, String> {
    let exchange_folder = format!("misc_{}", api_name);
    let target_path = global_data_dir.join(symbol).join(exchange_folder);
    
    fs::create_dir_all(&target_path)
        .map_err(|e| format!("Failed creating directory trees: {}", e))?;
        
    Ok(target_path)
}