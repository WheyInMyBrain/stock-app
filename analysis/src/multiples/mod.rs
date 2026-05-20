// analysis/src/multiples/mod.rs

pub mod engine;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CorporateMultiplesReport {
    pub source_file: String,
    pub snapshot_date: String,
    
    // Tier 1: Continuous Operational & Flow Multipliers
    pub revenue: f64,
    pub ebit_margin: f64,
    pub net_margin: f64,
    pub fcf_margin: f64,
    pub interest_coverage: f64,
    pub accruals_to_sales_intensity: f64,
    pub degree_of_operating_leverage: f64,
    pub breakeven_operating_revenue: f64,
    pub capex_to_depreciation_coverage: f64,
    pub estimated_infrastructure_nbv_age_years: f64,
    
    // Tier 2: Structural & Trading Multipliers (Optional Snapshots)
    pub stock_price: f64,
    pub total_shares: f64,
    pub roic: Option<f64>,
    pub roe: Option<f64>,
    pub roa: Option<f64>,
    pub debt_to_equity: Option<f64>,
    pub current_ratio: Option<f64>,
    pub quick_ratio: Option<f64>,
    pub inventory_turnover: Option<f64>,
    pub cash_conversion_cycle_days: Option<f64>,
    pub enterprise_value: Option<f64>,
    pub ev_to_ebitda: Option<f64>,
    pub piotroski_f_score: Option<u8>,
    pub beneish_m_score: Option<f64>,
    pub altman_z_score: Option<f64>,

    // 🎯 NEW TIER 2 ADDITIONS: Stress Testing & Dissolution Value
    pub defensive_cash_burn_months: Option<f64>,
    pub net_liquidating_dissolution_cash: Option<f64>,
    pub simulated_assets_post_10_percent_slump: Option<f64>,
    pub simulated_assets_post_20_percent_slump: Option<f64>,
    pub simulated_assets_post_30_percent_slump: Option<f64>,
    pub simulated_assets_post_40_percent_slump: Option<f64>,
    pub simulated_assets_post_50_percent_slump: Option<f64>,
}