use std::path::Path;
use crate::search::search::FinalStockMetadata;

pub fn load_stock_metadata(symbol: &str, global_data_dir: &Path) -> Result<FinalStockMetadata, String> {
    let target_symbol = symbol.trim().to_uppercase();
    let file_path = global_data_dir.join(&target_symbol).join("metadata.json");

    let content = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read stock metadata file: {}", e))?;

    let meta = serde_json::from_str::<FinalStockMetadata>(&content)
        .map_err(|e| format!("Failed to parse stock metadata JSON: {}", e))?;

    Ok(meta)
}