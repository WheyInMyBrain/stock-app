pub mod company_profile;
pub mod stock_stats;
pub mod stock_chart;
pub mod board_directory;
pub mod investors_complain;
pub mod board_of_director;
pub mod quarterly_financial;
pub mod investors;
pub mod financial_tables;
pub mod balance_sheet;
pub mod cashflow;
pub mod financial_multiples;
pub mod dcf_projections;
pub mod epv_projections;
pub mod merton_bates;
pub mod merton_kmv;
pub mod monte_carlo;
pub mod popup;

use serde_json::Value;
use crate::commands::pipeline::CatalogItem;
use crate::commands::data_loader::WorkspaceDataContext;

/// 🪐 The clean framework rule that every standalone backend card must implement
pub trait WorkspaceModule {
    /// Return the static catalog item details for the picker panel
    fn catalog_definition(&self) -> CatalogItem;

    /// Compile and build the dynamic layout tree using ticker and active timeframe metrics
    fn compile(&self, ticker: &str, timeframe: &str, data: &WorkspaceDataContext) -> Result<Value, String>;
}