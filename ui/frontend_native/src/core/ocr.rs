// stock-app/ui/frontend_native/src/core/ocr.rs

/// 🎯 Parameter Configuration Snapshot for the OCR Sidecar Engine
#[derive(Clone, Debug)]
pub struct OcrConfig {
    pub ticker: String,
    pub data_dir_override: Option<String>,
}

/// ⚡ CORE FRONTEND OCR DISPATCHER
/// Bridges frontend thread worker pipelines directly into the synchronous backend sidecar process execution channel.
pub fn dispatch_ocr(config: OcrConfig) -> Result<String, String> {
    backend::commands::ocr::run_ocr_pipeline_command(
        &config.ticker,
        config.data_dir_override,
    )
}