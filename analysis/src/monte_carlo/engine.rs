use rand::rng;
use rand_distr::{Distribution, Normal};
use rayon::prelude::*;
use crate::monte_carlo::MonteCarloMatrixCell;
use crate::data_loader::UnifiedCompanyMatrix;

struct HistoryLedgerData {
    revenue: f64,
    fcf_margin: f64,
}

/// The high-resolution stochastic matrix pipeline generator
pub fn execute_monte_carlo_matrix_pipeline(
    matrix: &UnifiedCompanyMatrix, // 🎯 PICKER INTERCEPT: Ingests structured shared memory context
    base_wacc: f64,
    base_terminal_g: f64,
) -> Result<Vec<MonteCarloMatrixCell>, &'static str> {
    
    // Internal helper closure maps historical shares out of memory structures
    let find_historical_shares = |target_date: &str| -> f64 {
        if let Some(&shares) = matrix.share_history_timeline.get(target_date) {
            return shares;
        }
        let target_year = target_date.split('-').next().unwrap_or("2024");
        let mut sorted_dates: Vec<&String> = matrix.share_history_timeline.keys().collect();
        sorted_dates.sort();
        
        for date_key in sorted_dates.iter().rev() {
            if date_key.starts_with(target_year) {
                return *matrix.share_history_timeline.get(*date_key).unwrap_or(&53_954_106.0);
            }
        }
        *matrix.share_history_timeline.values().next().unwrap_or(&53_954_106.0)
    };

    // ==============================================================================
    // 📊 STEP 1: PARSE VALID CAPEX RATIOS FROM DATA WAREHOUSE MATRIX
    // ==============================================================================
    let mut capex_ratios = Vec::new();
    for file_key in &matrix.sorted_file_keys {
        if let Some(metrics) = matrix.document_matrix.get(file_key) {
            let rev = *metrics.get("RevenueFromOperations").unwrap_or(&0.0);
            let capex = metrics.get("PurchaseOfPropertyPlantAndEquipmentClassifiedAsInvestingActivities").cloned().unwrap_or(0.0).abs();
            if rev > 0.0 && capex > 0.0 { capex_ratios.push(capex / rev); }
        }
    }
    let valid_capex_ratio = if !capex_ratios.is_empty() { capex_ratios.iter().sum::<f64>() / capex_ratios.len() as f64 } else { 0.05 };

    // ==============================================================================
    // 📊 STEP 2: RECONSTRUCT TIMELINE STATISTICS & RISK MOMENTS
    // ==============================================================================
    let mut history_ledger = Vec::new();
    let mut last_snapshot_date = "2024-03-31".to_string();

    for file_key in &matrix.sorted_file_keys {
        if let Some(metrics) = matrix.document_matrix.get(file_key) {
            let rev = *metrics.get("RevenueFromOperations").unwrap_or(&0.0);
            let cfo = *metrics.get("CashFlowsFromUsedInOperatingActivities").unwrap_or(&0.0);
            let capex = metrics.get("PurchaseOfPropertyPlantAndEquipmentClassifiedAsInvestingActivities").cloned().unwrap_or(0.0).abs();

            if rev <= 0.0 { continue; }

            let calculated_fcf = if cfo == 0.0 || capex == 0.0 {
                let pbt = *metrics.get("ProfitBeforeTax").unwrap_or(&0.0);
                let depr = *metrics.get("DepreciationDepletionAndAmortisationExpense").unwrap_or(&0.0);
                let tax = *metrics.get("TaxExpense").unwrap_or(&0.0);
                let finance = *metrics.get("FinanceCosts").unwrap_or(&0.0);
                (pbt + depr - tax - finance) - (rev * valid_capex_ratio)
            } else { cfo - capex };

            let fcf_margin = calculated_fcf / rev;
            last_snapshot_date = matrix.file_to_date_map.get(file_key).unwrap().clone();
            history_ledger.push(HistoryLedgerData { revenue: rev, fcf_margin });
        }
    }

    if history_ledger.len() < 2 {
        return Err("Insufficient sequential historical timeline chunks to populate Monte Carlo variance distributions.");
    }

    let mut growth_rates = Vec::new();
    for idx in 1..history_ledger.len() {
        let prev = history_ledger[idx - 1].revenue;
        let curr = history_ledger[idx].revenue;
        if prev > 0.0 { growth_rates.push((curr - prev) / prev); }
    }
    let margins: Vec<f64> = history_ledger.iter().map(|h| h.fcf_margin).collect();

    let base_mean_growth = growth_rates.iter().sum::<f64>() / growth_rates.len() as f64;
    let var_growth = growth_rates.iter().map(|g| (g - base_mean_growth).powi(2)).sum::<f64>() / growth_rates.len() as f64;
    let std_growth = var_growth.sqrt().max(0.01); 

    let base_mean_margin = margins.iter().sum::<f64>() / margins.len() as f64;
    let var_margin = margins.iter().map(|m| (m - base_mean_margin).powi(2)).sum::<f64>() / margins.len() as f64;
    let std_margin = var_margin.sqrt().max(0.01);

    let base_revenue = history_ledger.last().unwrap().revenue; 
    let shares_outstanding = find_historical_shares(&last_snapshot_date);

    // ==============================================================================
    // 📊 STEP 3: CONCURRENT STOCHASTIC SCENARIO EXECUTION
    // ==============================================================================
    let mut wacc_scenarios = Vec::new();
    for step in -5..=5 { wacc_scenarios.push(base_wacc + (step as f64 * 0.005)); }

    let mut multiplier_scenarios = Vec::new();
    let mut current_mult = 0.75;
    while current_mult <= 1.25001 { multiplier_scenarios.push(current_mult); current_mult += 0.0003; }

    let mut grid_tasks = Vec::new();
    for &wacc in &wacc_scenarios {
        for &mult in &multiplier_scenarios { grid_tasks.push((wacc, mult)); }
    }

    println!("🎲 [Monte Carlo Picker]: Launching parallel processing for {} high-res stochastic scenarios...", grid_tasks.len());

    let final_matrix_output: Vec<MonteCarloMatrixCell> = grid_tasks
        .par_iter()
        .map(|&(wacc, mult)| {
            let shifted_mean_growth = base_mean_growth * mult;
            let shifted_mean_margin = base_mean_margin * mult;

            let dist_growth = Normal::new(shifted_mean_growth, std_growth).unwrap();
            let dist_margin = Normal::new(shifted_mean_margin, std_margin).unwrap();
            let mut local_rng = rng();

            let total_trials = 10000;
            let mut trial_prices = Vec::with_capacity(total_trials);

            for _ in 0..total_trials {
                let sampled_growth = dist_growth.sample(&mut local_rng);
                let sampled_margin = dist_margin.sample(&mut local_rng);

                let mut current_rev = base_revenue;
                let mut pv_discrete_sum = 0.0;
                let mut final_year_fcf = 0.0;

                for year in 1..=5 {
                    current_rev *= 1.0 + sampled_growth;
                    let fcf = current_rev * sampled_margin;
                    pv_discrete_sum += fcf / (1.0 + wacc).powi(year);
                    if year == 5 { final_year_fcf = fcf; }
                }

                let terminal_value = (final_year_fcf * (1.0 + base_terminal_g)) / (wacc - base_terminal_g);
                let pv_terminal_value = terminal_value / (1.0 + wacc).powi(5);
                let share_price = (pv_discrete_sum + pv_terminal_value) / shares_outstanding;

                if share_price > 0.0 && !share_price.is_nan() && !share_price.is_infinite() {
                    trial_prices.push(share_price);
                }
            }

            trial_prices.sort_by(|a, b| a.partial_cmp(b).unwrap());

            if trial_prices.is_empty() {
                return MonteCarloMatrixCell {
                    wacc, terminal_g: base_terminal_g, operational_multiplier: mult, trials_executed: 0,
                    p10_bear: 0.0, p30_conservative: 0.0, p50_median: 0.0, p70_optimistic: 0.0, p90_bull: 0.0, mean_expected: 0.0
                };
            }

            let len = trial_prices.len() as f64;
            let get_quantile = |q: f64| -> f64 {
                let idx = ((len * q) as usize).min(trial_prices.len() - 1);
                trial_prices[idx]
            };

            let mean_expected = trial_prices.iter().sum::<f64>() / trial_prices.len() as f64;

            MonteCarloMatrixCell {
                wacc,
                terminal_g: base_terminal_g,
                operational_multiplier: mult,
                trials_executed: trial_prices.len(),
                p10_bear: get_quantile(0.10),
                p30_conservative: get_quantile(0.30),
                p50_median: get_quantile(0.50),
                p70_optimistic: get_quantile(0.70),
                p90_bull: get_quantile(0.90),
                mean_expected,
            }
        })
        .collect();

    Ok(final_matrix_output)
}