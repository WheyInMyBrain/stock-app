// stock-app/ui/frontend_native/src/core/analysis.rs

/// 📐 Configuration specifications for triggering analytical matrix tracks.
/// Allows passing custom overrides or specific modular segments to execute.
#[derive(Debug, Clone, Default)]
pub struct AnalysisConfig {
    pub ticker: String,
    pub wacc: Option<f64>,
    pub terminal_g: Option<f64>,
    pub data_dir_override: Option<String>,
    pub modules: Option<String>,
}

/// ⚡ CORE ANALYSIS DISPATCHER
/// Forwards your analytical runtime parameters directly to the multi-threaded backend core engine.
pub fn dispatch_analysis(config: AnalysisConfig) -> Result<String, String> {
    backend::commands::analysis::trigger_core_analysis(
        &config.ticker,
        config.wacc,
        config.terminal_g,
        config.data_dir_override,
        config.modules,
    )
}