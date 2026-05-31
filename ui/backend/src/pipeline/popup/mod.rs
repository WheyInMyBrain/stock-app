// stock-app/ui/backend/src/pipeline/popup/mod.rs

pub mod stock_chart;

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

/// The uniform trait configuration blueprint all windows must fulfill
pub trait WorkspacePopup: Send + Sync {
    fn window_title(&self, ticker: &str) -> String;
    fn initial_size(&self) -> (f64, f64);
    fn compile(&self, ticker: &str) -> Result<serde_json::Value, String>;
}

/// 🎯 THE THREAD-SAFE GLOBAL REGISTRY BOX
static POPUP_REGISTRY: OnceLock<HashMap<&'static str, Arc<dyn WorkspacePopup>>> = OnceLock::new();

/// Returns the central registry map. If it hasn't been initialized yet, it dynamically builds it.
pub fn get_popup_registry() -> &'static HashMap<&'static str, Arc<dyn WorkspacePopup>> {
    POPUP_REGISTRY.get_or_init(|| {
        let mut registry: HashMap<&'static str, Arc<dyn WorkspacePopup>> = HashMap::new();

        // =================================================================
        // REGISTER YOUR MODULE Blueprints HERE
        // =================================================================
        registry.insert("stock_chart", Arc::new(stock_chart::StockChartPopup));
        
        // =================================================================

        registry
    })
}