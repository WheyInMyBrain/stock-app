// stock-app/ui/backend/src/commands/analysis.rs
use crate::commands::data_dir::resolve_data_directory_headless;
use std::sync::atomic::Ordering;

/// 📊 Live Progress Inspector for the Frontend Viewport
/// Returns a tuple containing the exact atomic task metrics: (completed_tasks, total_tasks)
pub fn get_analysis_progress() -> (u32, u32) {
    let completed = analysis::runner::COMPLETED_TASKS.load(Ordering::Relaxed);
    let total = analysis::runner::TOTAL_TASKS.load(Ordering::Relaxed);
    (completed, total)
}

/// ⚡ VALUATION MATRIX ANALYSIS DISPATCHER
/// Triggers the parallel deterministic metrics engine and projection workflows natively.
pub fn trigger_core_analysis(
    symbol: &str, 
    wacc: Option<f64>, 
    terminal_g: Option<f64>, 
    data_dir_override: Option<String>, 
    modules: Option<String>
) -> Result<String, String> {
    let current_ticker = symbol.trim().to_uppercase();
    if current_ticker.is_empty() {
        return Err("❌ ERROR: Target stock ticker token cannot be blank.".to_string());
    }

    // A. Resolve parameter configurations with default CLI value fallbacks
    let final_wacc = wacc.unwrap_or(0.12);
    let final_g = terminal_g.unwrap_or(0.04);
    let final_modules = modules.unwrap_or_else(|| "all".to_string());

    // B. Resolve unified file system storage directories natively 
    let final_dir = match data_dir_override {
        Some(dir) => dir,
        None => resolve_data_directory_headless().to_string_lossy().to_string(),
    };

    // C. Execute the library pipeline orchestration sequence directly
    analysis::runner::run_global_analysis_pipeline(
        &current_ticker,
        final_wacc,
        final_g,
        &final_dir,
        &final_modules,
    );

    // 🎯 FIXED: Lean confirmation token returned cleanly
    Ok("Success".to_string())
}