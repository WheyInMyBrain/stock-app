use rayon::prelude::*;
use crate::epv::EpvMatrixCell;
use crate::data_loader::UnifiedCompanyMatrix;

struct InternalLedgerRow {
    date_bounds: String,
    revenue: f64,
    ebit_margin: f64,
    depr: f64,
    tax: f64,
    capex: f64,
    ebit: f64,
}

pub fn execute_rolling_epv_pipeline(
    matrix: &UnifiedCompanyMatrix, // 🎯 PICKER INTERCEPT: Reads directly from shared RAM cache
    base_wacc: f64,
) -> Result<Vec<EpvMatrixCell>, &'static str> {
    
    // Internal picker closure resolves historical shares outstanding dynamically from pre-loaded memory structures
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
    // 📊 STEP 1: MAPPED LEDGER GENERATION FROM CACHED DATA ENTRIES
    // ==============================================================================
    let mut ledger: Vec<InternalLedgerRow> = Vec::new();
    for file_key in &matrix.sorted_file_keys {
        if let Some(metrics) = matrix.document_matrix.get(file_key) {
            let rev = *metrics.get("RevenueFromOperations").unwrap_or(&0.0);
            let pbt = *metrics.get("ProfitBeforeTax").unwrap_or(&0.0);
            let interest = *metrics.get("FinanceCosts").unwrap_or(&0.0);
            let depr = *metrics.get("DepreciationDepletionAndAmortisationExpense").unwrap_or(&0.0);
            let tax = *metrics.get("TaxExpense").unwrap_or(&0.0);
            let capex = metrics.get("PurchaseOfPropertyPlantAndEquipmentClassifiedAsInvestingActivities").cloned().unwrap_or(0.0).abs();
            
            let ebit = pbt + interest;
            let ebit_margin = if rev > 0.0 { ebit / rev } else { 0.0 };
            let snapshot_date = matrix.file_to_date_map.get(file_key).unwrap().clone();

            if rev > 0.0 {
                ledger.push(InternalLedgerRow {
                    date_bounds: snapshot_date,
                    revenue: rev,
                    ebit_margin,
                    depr,
                    tax,
                    capex,
                    ebit,
                });
            }
        }
    }

    if ledger.is_empty() {
        return Err("Insufficient analytical segments compiled to initialize EPV valuation matrix loops.");
    }

    // Pre-calculate outstanding shares to bypass lookups during execution loops
    let mut shares_timeline_mapping = Vec::with_capacity(ledger.len());
    for idx in 0..ledger.len() {
        let shares_outstanding = find_historical_shares(&ledger[idx].date_bounds);
        shares_timeline_mapping.push(shares_outstanding);
    }

    // ==============================================================================
    // 📊 STEP 2: CONCURRENT MATRIX MATHEMATICS COMPUTATION
    // ==============================================================================
    let mut wacc_scenarios = Vec::new();
    for step in -5..=5 { wacc_scenarios.push(base_wacc + (step as f64 * 0.005)); }

    let mut multiplier_scenarios = Vec::new();
    let mut current_mult = 0.75;
    while current_mult <= 1.25001 { multiplier_scenarios.push(current_mult); current_mult += 0.0003; }

    let mut grid_tasks = Vec::new();
    for idx in 0..ledger.len() {
        for &wacc in &wacc_scenarios {
            for &mult in &multiplier_scenarios { grid_tasks.push((idx, wacc, mult)); }
        }
    }

    println!("⚡ [EPV Picker]: Computing {} high-precision rolling EPV matrix cells concurrently...", grid_tasks.len());

    let final_matrix_output: Vec<EpvMatrixCell> = grid_tasks
        .par_iter()
        .map(|&(idx, wacc, mult)| {
            let current_node = &ledger[idx];
            let shares_used = shares_timeline_mapping[idx];

            let mut margin_sum = 0.0;
            let mut capex_sum = 0.0;
            let mut depr_sum = 0.0;
            for j in 0..=idx {
                margin_sum += ledger[j].ebit_margin;
                capex_sum += ledger[j].capex;
                depr_sum += ledger[j].depr;
            }
            
            let rolling_mean_ebit_margin = (margin_sum / ((idx + 1) as f64)) * mult;
            let avg_historical_capex = capex_sum / ((idx + 1) as f64);
            let avg_historical_depr = depr_sum / ((idx + 1) as f64);

            let normalized_ebit = current_node.revenue * rolling_mean_ebit_margin;

            let mut effective_tax_rate = current_node.tax / (if current_node.ebit > 0.0 { current_node.ebit } else { 1.0 });
            if effective_tax_rate < 0.0 || effective_tax_rate > 0.40 { effective_tax_rate = 0.25; }

            let maintenance_capex_ratio = (avg_historical_depr / (if avg_historical_capex > 0.0 { avg_historical_capex } else { 1.0 })).min(1.0);
            let normalized_capex = current_node.capex * maintenance_capex_ratio;

            let normalized_earnings_power = (normalized_ebit * (1.0 - effective_tax_rate)) + current_node.depr - normalized_capex;
            let enterprise_value_epv = normalized_earnings_power / wacc;
            let epv_fair_value = enterprise_value_epv / shares_used;

            EpvMatrixCell {
                year_end: current_node.date_bounds.clone(),
                base_revenue: current_node.revenue,
                wacc,
                operational_multiplier: mult,
                historical_ebit_margin: rolling_mean_ebit_margin,
                normalized_fcf: normalized_earnings_power,
                epv_fair_value,
            }
        })
        .collect();

    Ok(final_matrix_output)
}