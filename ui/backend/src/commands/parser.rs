// stock-app/ui/backend/src/commands/parser.rs
use crate::commands::data_dir::resolve_data_directory_headless;

/// ⚡ PARQUET PIPELINE ENGINE CONNECTOR
pub fn run_pipeline_parser(
    symbol: &str,
    data_dir_override: Option<String>,
) -> Result<String, String> {
    let current_ticker = symbol.trim().to_uppercase();
    if current_ticker.is_empty() {
        return Err("❌ ERROR: Provided ticker symbol token cannot be blank.".to_string());
    }

    // A. Resolve base data repository coordinate path natively aligned with the downloader system
    let data_dir_base = match data_dir_override {
        Some(dir) => dir,
        None => resolve_data_directory_headless().to_string_lossy().to_string(),
    };

    // B. Dispatch the optimized high-speed library pipeline
    let summary = parser::run_ticker_parsing_pipeline(&data_dir_base, &current_ticker)
        .map_err(|e| format!("🚨 Ingestion Pipeline Failure: {}", e))?;

    // C. Reconstruct the high-contrast presentation text block to forward to the frontend
    let mut log_output = String::new();
    log_output.push_str("\n=================================================================================\n");
    log_output.push_str(&format!("🎉 PARSING CONCLUDED SUCCESSFULLY FOR TICKER [{}]\n", summary.ticker));
    log_output.push_str("=================================================================================\n");

    for group in summary.group_results {
        log_output.push_str(&format!(
            "📁 Group: {:<38} | Rows: {:<6} | Processed: {:<3} | Time: {}ms\n",
            group.folder_name,
            group.total_rows,
            group.processed_files,
            group.elapsed_ms
        ));
    }

    log_output.push_str("---------------------------------------------------------------------------------\n");
    log_output.push_str(&format!("📈 Grand Total Data Rows Synchronized: {}\n", summary.grand_total_rows));
    log_output.push_str(&format!("⏱️ Full Run Pipeline Execution Time   : {}ms\n", summary.total_elapsed_ms));
    log_output.push_str("=================================================================================\n");

    Ok(log_output)
}