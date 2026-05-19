// analysis/src/dcf/mod.rs

pub mod engine;

use serde::Serialize;

/// New high-resolution multidimensional grid container for persistent storage.
#[derive(Debug, Clone, Serialize)]
pub struct DcfMatrixCell {
    pub year_end: String,
    pub base_revenue: f64,
    pub wacc: f64,
    pub terminal_g: f64,
    pub growth_multiplier: f64,
    pub margin_multiplier: f64,
    pub rolling_fair_value: f64,
    pub omniscient_fair_value: f64,
}