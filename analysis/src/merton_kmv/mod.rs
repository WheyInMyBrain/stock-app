pub mod engine;

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MertonKmvCell {
    pub snapshot_date: String,
    pub equity_value_market_cap: f64,
    pub structural_default_barrier: f64,
    pub inferred_asset_value: f64,
    pub inferred_asset_volatility: f64,
    pub distance_to_default_dd: f64,
    pub expected_default_frequency_edf: f64,
}