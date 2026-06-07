// stock-app/ui/frontend_native/src/core/parser.rs

/// 📑 Configuration parameters for triggering the multi-threaded dataset parser pipeline natively.
#[derive(Debug, Clone, Default)]
pub struct ParserConfig {
    pub ticker: String,
    pub data_dir_override: Option<String>,
}

/// ⚡ CORE PARSER DISPATCHER
/// Forwards your dataset parsing configurations directly to the zero-copy backend extraction library layer.
pub fn dispatch_parse(config: ParserConfig) -> Result<String, String> {
    backend::commands::parser::run_pipeline_parser(
        &config.ticker,
        config.data_dir_override,
    )
}