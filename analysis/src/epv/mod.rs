// analysis/src/epv/mod.rs

pub mod engine;

use serde::Serialize;

/// Container holding the historical trend rows for our no-growth EPV pipeline
#[derive(Debug, Clone, Serialize)]
pub struct EpvResultRow {
    pub year_end: String,
    pub base_revenue: f64,
    pub historical_ebit_margin: f64,
    pub normalized_fcf: f64,
    pub epv_fair_value: f64,
}