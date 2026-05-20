pub mod engine;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MertonBatesCell {
    pub snapshot_date: String,
    pub base_stock_price: f64,
    pub implied_annual_volatility: f64,
    pub jump_intensity_lambda: f64,
    pub expected_jump_size_mu_j: f64,
    pub value_at_risk_95: f64,        // 95% worst-case downside threshold
    pub value_at_risk_99: f64,        // 99% systemic shock threshold
    pub simulated_expected_value: f64, // True risk-adjusted path mean price
}