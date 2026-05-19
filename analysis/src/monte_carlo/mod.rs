// analysis/src/dcf/mod.rs

pub mod engine;

use serde::Serialize;

/// High-resolution multidimensional grid container for persistent storage.
#[derive(Debug, Clone, Serialize)]
pub struct MonteCarloMatrixCell {
    pub wacc: f64,
    pub terminal_g: f64,
    pub operational_multiplier: f64,
    pub trials_executed: usize,
    pub p10_bear: f64,
    pub p30_conservative: f64,
    pub p50_median: f64,
    pub p70_optimistic: f64,
    pub p90_bull: f64,
    pub mean_expected: f64,
}