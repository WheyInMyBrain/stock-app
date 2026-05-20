pub mod engine;

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkovRegimeCell {
    pub execution_snapshot_date: String,
    pub extracted_current_state: usize,
    pub margin_efficiency_growth_drift: f64,
    pub probability_matrix_transition_to_expansion: f64,
    pub probability_matrix_transition_to_stagnation: f64,
    pub probability_matrix_transition_to_crunch: f64,
}