pub mod data_loader;
pub mod company_profile;
pub mod stock_stats;

use serde_json::Value;
use crate::commands::pipeline::CatalogItem;
pub use data_loader::WorkspaceDataContext;

/// 🪐 The clean framework rule that every standalone backend card must implement
pub trait WorkspaceModule {
    /// Return the static catalog item details for the picker panel
    fn catalog_definition(&self) -> CatalogItem;

    /// Compile and build the dynamic layout tree using ticker and active timeframe metrics
    fn compile(&self, ticker: &str, timeframe: &str, data: &WorkspaceDataContext) -> Result<Value, String>;
}