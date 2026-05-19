// analysis/src/monte_carlo/mod.rs

pub mod engine;

use serde::Serialize;

/// Container holding the aggregated percentile outcomes 
/// of our randomized simulation matrix.
#[derive(Debug, Clone, Serialize)]
pub struct MonteCarloReport {
    pub ticker: String,
    pub trials_executed: usize,
    pub p10_bear: f64,
    pub p30_conservative: f64,
    pub p50_median: f64,
    pub p70_optimistic: f64,
    pub p90_bull: f64,
    pub mean_expected: f64,
}