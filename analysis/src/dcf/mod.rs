pub mod engine;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct DcfResultRow {
    pub year_end: String,
    pub baseline_revenue: f64,
    pub rolling_fair_value: f64,
    pub omniscient_fair_value: f64,
    pub shares_used: f64,
}