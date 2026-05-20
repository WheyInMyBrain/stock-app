use rayon::prelude::*;
use crate::dcf::DcfMatrixCell;
use crate::data_loader::UnifiedCompanyMatrix;

struct DynamicYearData {
    date_bounds: String,
    revenue: f64,
    fcf_margin: f64,
    shares_outstanding: f64,
}

pub fn execute_dual_dcf_pipeline(
    matrix: &UnifiedCompanyMatrix, // 🎯 PICKER INTERCEPT: Takes pre-compiled cache directly
    base_wacc: f64,
    base_terminal_g: f64,
) -> Result<Vec<DcfMatrixCell>, &'static str> {
    
    // Internal picker closure maps timeline share tokens dynamically from memory cache
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
    // 📊 STEP 1: CALCULATE VALID CAPEX RATIOS FROM CACHED KEYS
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
    // 📊 STEP 2: RECONSTRUCT CHRONOLOGICAL TIMELINE CHUNKS
    // ==============================================================================
    let mut dynamic_timeline = Vec::new();
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
            let snapshot_date = matrix.file_to_date_map.get(file_key).unwrap().clone();
            let shares_outstanding = find_historical_shares(&snapshot_date);

            dynamic_timeline.push(DynamicYearData {
                date_bounds: snapshot_date,
                revenue: rev,
                fcf_margin,
                shares_outstanding,
            });
        }
    }

    if dynamic_timeline.len() < 2 {
        return Err("Insufficient sequential historical timeline chunks to populate matrix loops.");
    }

    let mut global_growth_sum = 0.0;
    let mut global_growth_count = 0.0;
    for j in 1..dynamic_timeline.len() {
        let prev_rev = dynamic_timeline[j - 1].revenue;
        if prev_rev > 0.0 {
            global_growth_sum += (dynamic_timeline[j].revenue - prev_rev) / prev_rev;
            global_growth_count += 1.0;
        }
    }
    let dynamic_base_omni_growth = if global_growth_count > 0.0 { global_growth_sum / global_growth_count } else { 0.03 };
    let dynamic_base_omni_margin = if !dynamic_timeline.is_empty() { dynamic_timeline.iter().map(|n| n.fcf_margin).sum::<f64>() / dynamic_timeline.len() as f64 } else { 0.10 };

    // ==============================================================================
    // 📊 STEP 3: CONCURRENT MATRIX MATHEMATICS COMPUTATION
    // ==============================================================================
    let mut wacc_scenarios = Vec::new();
    for step in -5..=5 { wacc_scenarios.push(base_wacc + (step as f64 * 0.005)); }

    let mut multiplier_scenarios = Vec::new();
    let mut current_mult = 0.75;
    while current_mult <= 1.25001 { multiplier_scenarios.push(current_mult); current_mult += 0.0003; }

    let mut calculation_tasks = Vec::new();
    for idx in 1..dynamic_timeline.len() {
        for &wacc in &wacc_scenarios {
            for &mult in &multiplier_scenarios { calculation_tasks.push((idx, wacc, mult)); }
        }
    }

    let matrix_grid_output: Vec<DcfMatrixCell> = calculation_tasks
        .par_iter()
        .map(|&(idx, wacc, mult)| {
            let current_node = &dynamic_timeline[idx];
            let cell_omni_growth = dynamic_base_omni_growth;
            let cell_omni_margin = dynamic_base_omni_margin;
            
            let mut total_g_sum = 0.0;
            for j in 1..=idx { total_g_sum += (dynamic_timeline[j].revenue - dynamic_timeline[j-1].revenue) / dynamic_timeline[j-1].revenue; }
            let rolling_growth = (total_g_sum / (idx as f64)) * mult;

            let mut total_m_sum = 0.0;
            for j in 0..=idx { total_m_sum += dynamic_timeline[j].fcf_margin; }
            let rolling_margin = (total_m_sum / ((idx + 1) as f64)) * mult;

            let mut roll_rev = current_node.revenue;
            let mut roll_pvs = 0.0;
            let mut last_roll_fcf = 0.0;
            for year in 1..=5 {
                roll_rev *= 1.0 + rolling_growth;
                let fcf = roll_rev * rolling_margin;
                roll_pvs += fcf / (1.0 + wacc).powi(year as i32);
                if year == 5 { last_roll_fcf = fcf; }
            }
            let roll_tv = (last_roll_fcf * (1.0 + base_terminal_g)) / (wacc - base_terminal_g);
            let roll_pv_tv = roll_tv / (1.0 + wacc).powi(5);
            let rolling_fair_value = (roll_pvs + roll_pv_tv) / current_node.shares_outstanding;

            let mut omni_rev = current_node.revenue;
            let mut omni_pvs = 0.0;
            let mut last_omni_fcf = 0.0;
            for year in 1..=5 {
                omni_rev *= 1.0 + cell_omni_growth;
                let fcf = omni_rev * cell_omni_margin;
                omni_pvs += fcf / (1.0 + wacc).powi(year as i32);
                if year == 5 { last_omni_fcf = fcf; }
            }
            let omni_tv = (last_omni_fcf * (1.0 + base_terminal_g)) / (wacc - base_terminal_g);
            let omni_pv_tv = omni_tv / (1.0 + wacc).powi(5);
            let omniscient_fair_value = (omni_pvs + omni_pv_tv) / current_node.shares_outstanding;

            DcfMatrixCell {
                year_end: current_node.date_bounds.clone(),
                base_revenue: current_node.revenue,
                wacc,
                terminal_g: base_terminal_g,
                growth_multiplier: mult,
                margin_multiplier: mult,
                rolling_fair_value,
                omniscient_fair_value,
            }
        })
        .collect();

    Ok(matrix_grid_output)
}