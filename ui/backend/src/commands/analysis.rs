// stock-app/ui/backend/src/commands/analysis.rs
use crate::commands::data_dir::resolve_data_directory_headless;

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

    println!("🚀 [ANALYSIS ENGINE]: Invoked valuation matrix compilation for ticker [{}]", current_ticker);

    // C. Execute the library pipeline orchestration sequence directly
    analysis::runner::run_global_analysis_pipeline(
        &current_ticker,
        final_wacc,
        final_g,
        &final_dir,
        &final_modules,
    );

    Ok(format!(
        "✨ VALUATION MATRIX ANALYSIS CONCLUDED SUCCESSFULLY FOR TICKER [{}]\n\
         =================================================================================\n\
         📈 Active Analytical Scope Targets: {}\n\
         📐 Discounting Rate Context (WACC) : {:.2}%\n\
         🔮 Long-Term Growth Rate (Term G) : {:.2}%\n\
         📂 Target Output Directory Trace   : {}/{}/analysis\n\
         =================================================================================\n\
         All valuation matrices, DCF models, and Monte Carlo grids compiled to JSON frames.",
        current_ticker, final_modules, final_wacc * 100.0, final_g * 100.0, final_dir, current_ticker
    ))
}