use rayon::prelude::*;
use crate::markov_regime::MarkovRegimeCell;
use crate::data_loader::UnifiedCompanyMatrix;

pub fn execute_markov_regime_pipeline(
    matrix: &UnifiedCompanyMatrix, // 🎯 PICKER INTERCEPT: Ingests structured shared memory context
    ticker: &str,
) -> Vec<MarkovRegimeCell> {
    let mut grid_results = Vec::new();

    // ==============================================================================
    // 📊 STEP 1: EXTRACT OPERATIONAL MARGINS FROM SHARED IN-MEMORY DATA WAREHOUSE
    // ==============================================================================
    let mut observation_stream = Vec::new();
    let mut matching_dates = Vec::new();

    for file_key in &matrix.sorted_file_keys {
        if let Some(metrics) = matrix.document_matrix.get(file_key) {
            let rev = *metrics.get("RevenueFromOperations").unwrap_or(&0.0);
            let pbt = *metrics.get("ProfitBeforeTax").unwrap_or(&0.0);
            let interest = *metrics.get("FinanceCosts").unwrap_or(&0.0);
            
            let ebit = pbt + interest;
            let margin = if rev > 0.0 { ebit / rev } else { 0.0 };
            let date_str = matrix.file_to_date_map.get(file_key).cloned().unwrap_or("2024-03-31".to_string());

            if rev > 0.0 {
                observation_stream.push(margin);
                matching_dates.push(date_str);
            }
        }
    }

    if observation_stream.is_empty() {
        return grid_results;
    }

    // Fixed hidden-state baseline transition matrix parameters
    // State 0 = Expansion, State 1 = Stagnation, State 2 = Structural Crunch
    let transition_matrix = vec![
        vec![0.70, 0.20, 0.10], 
        vec![0.15, 0.70, 0.15], 
        vec![0.10, 0.30, 0.60], 
    ];

    // ==============================================================================
    // 📊 STEP 2: CONCURRENT MARKOV PROBABILITY MATRIX STATE MAPPING via RAYON
    // ==============================================================================
    println!("🎭 [Markov Picker]: Inferring multi-regime transitional pathways for [{}]...", ticker);

    grid_results = (0..observation_stream.len())
        .into_par_iter()
        .map(|idx| {
            let current_date = &matching_dates[idx];
            let margin = observation_stream[idx];

            let inferred_state = if margin > 0.18 {
                0 // Expansion State Mode
            } else if margin >= 0.08 && margin <= 0.18 {
                1 // Stagnation State Mode
            } else {
                2 // Crunch State Mode
            };

            MarkovRegimeCell {
                execution_snapshot_date: current_date.clone(),
                extracted_current_state: inferred_state,
                margin_efficiency_growth_drift: margin,
                probability_matrix_transition_to_expansion: transition_matrix[inferred_state][0],
                probability_matrix_transition_to_stagnation: transition_matrix[inferred_state][1],
                probability_matrix_transition_to_crunch: transition_matrix[inferred_state][2],
            }
        })
        .collect();

    grid_results
}