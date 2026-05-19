// analysis/src/epv/mod.rs

pub mod engine;

use serde::Serialize;

/// New high-resolution multidimensional grid container for persistent storage.
#[derive(Debug, Clone, Serialize)]
pub struct EpvMatrixCell {
    pub year_end: String,
    pub base_revenue: f64,
    pub wacc: f64,
    pub operational_multiplier: f64,
    pub historical_ebit_margin: f64,
    pub normalized_fcf: f64,
    pub epv_fair_value: f64,
}