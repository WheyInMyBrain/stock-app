// stock-app/ui/backend/src/commands/parser.rs
use crate::commands::data_dir::resolve_data_directory_headless;

/// ⚡ PARQUET PIPELINE ENGINE CONNECTOR
/// Triggers the zero-copy, multi-threaded Parquet ingestion layer natively.
pub fn run_pipeline_parser(
    symbol: &str,
    data_dir_override: Option<String>,
) -> Result<String, String> {
    let current_ticker = symbol.trim().to_uppercase();
    if current_ticker.is_empty() {
        return Err("❌ ERROR: Provided ticker symbol token cannot be blank.".to_string());
    }

    // A. Resolve base data repository path aligned with the downloader system
    let data_dir_base = match data_dir_override {
        Some(dir) => dir,
        None => resolve_data_directory_headless().to_string_lossy().to_string(),
    };

    // B. Dispatch the optimized high-speed library pipeline 
    // (Live print logs now print directly from within the library function below)
    let _summary = parser::run_ticker_parsing_pipeline(&data_dir_base, &current_ticker)
        .map_err(|e| format!("🚨 Ingestion Pipeline Failure: {}", e))?;

    // 🎯 FIXED: Lean confirmation token returned cleanly to notify the frontend core
    Ok("Success".to_string())
}