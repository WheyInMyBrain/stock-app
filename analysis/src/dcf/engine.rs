// analysis/src/dcf/engine.rs

use polars::prelude::*;
use std::path::Path;
use rayon::prelude::*;

use crate::dcf::DcfMatrixCell;

struct DynamicYearData {
    date_bounds: String,
    revenue: f64,
    fcf_margin: f64,
    shares_outstanding: f64,
}

pub fn execute_dual_dcf_pipeline(
    ticker: &str,
    base_wacc: f64,
    base_terminal_g: f64,
) -> PolarsResult<Vec<DcfMatrixCell>> {
    let bse_path = format!("../data/{}/parquets/bse_financial-results-docs.parquet", ticker);
    let shp_path = format!("../data/{}/parquets/bse_shareholding-pattern-docs.parquet", ticker);

    if !Path::new(&bse_path).exists() || !Path::new(&shp_path).exists() {
        return Err(PolarsError::ComputeError(
            format!("Required Parquet source tables missing for ticker {}.", ticker).into()
        ));
    }

    let df_fin = LazyFrame::scan_parquet(&bse_path, Default::default())?.collect()?;
    let df_shp = LazyFrame::scan_parquet(&shp_path, Default::default())?.collect()?;

    let target_tags = [
        "RevenueFromOperations",
        "CashFlowsFromUsedInOperatingActivities",
        "PurchaseOfPropertyPlantAndEquipmentClassifiedAsInvestingActivities",
        "ProfitBeforeTax",
        "DepreciationDepletionAndAmortisationExpense",
        "TaxExpense",
        "FinanceCosts"
    ];
    
    let tag_col = df_fin.column("tag_name")?.str()?;
    let mask: BooleanChunked = tag_col.into_iter().map(|opt_val| {
        match opt_val {
            Some(val) => target_tags.contains(&val),
            None => false,
        }
    }).collect();
    
    let df_filtered = df_fin.filter(&mask)?;

    let date_bounds_col = df_filtered.column("date_bounds")?.str()?;
    let source_file_col = df_filtered.column("source_file")?.str()?;
    let tag_name_col = df_filtered.column("tag_name")?.str()?;
    let raw_value_col = df_filtered.column("raw_value")?.str()?;

    let mut unique_groups = Vec::new();
    let mut grouped_rows = std::collections::HashMap::new();

    for idx in 0..df_filtered.shape().0 {
        let bounds = date_bounds_col.get(idx).unwrap_or("").to_string();
        let source = source_file_col.get(idx).unwrap_or("").to_string();
        let tag = tag_name_col.get(idx).unwrap_or("").to_string();
        let raw_val = raw_value_col.get(idx).unwrap_or("").to_string();

        if bounds.contains("-04-01 to ") && bounds.contains("-03-31") 
           && source.contains("Consolidated") && source.contains("_MC") 
        {
            let key = (bounds.clone(), source.clone());
            if !grouped_rows.contains_key(&key) {
                unique_groups.push(key.clone());
                grouped_rows.insert(key.clone(), std::collections::HashMap::new());
            }
            
            let cleaned_val: f64 = raw_val.replace(",", "").replace(" ", "").trim().parse().unwrap_or(0.0);
            if let Some(map) = grouped_rows.get_mut(&key) {
                map.insert(tag, cleaned_val);
            }
        }
    }

    unique_groups.sort_by(|a, b| a.0.cmp(&b.0));

    let mut capex_ratios = Vec::new();
    for (bounds, source) in &unique_groups {
        if let Some(metrics) = grouped_rows.get(&(bounds.clone(), source.clone())) {
            let rev = *metrics.get("RevenueFromOperations").unwrap_or(&0.0);
            let capex = metrics.get("PurchaseOfPropertyPlantAndEquipmentClassifiedAsInvestingActivities").cloned().unwrap_or(0.0).abs();
            let cfo = *metrics.get("CashFlowsFromUsedInOperatingActivities").unwrap_or(&0.0);
            
            if cfo != 0.0 && capex != 0.0 && rev > 0.0 {
                capex_ratios.push(capex / rev);
            }
        }
    }
    let valid_capex_ratio = if !capex_ratios.is_empty() {
        capex_ratios.iter().sum::<f64>() / capex_ratios.len() as f64
    } else {
        0.05 
    };

    let mut dynamic_timeline = Vec::new();

    for (bounds, source) in unique_groups {
        if let Some(metrics) = grouped_rows.get(&(bounds.clone(), source.clone())) {
            let rev = *metrics.get("RevenueFromOperations").unwrap_or(&0.0);
            let cfo = *metrics.get("CashFlowsFromUsedInOperatingActivities").unwrap_or(&0.0);
            let capex = metrics.get("PurchaseOfPropertyPlantAndEquipmentClassifiedAsInvestingActivities").cloned().unwrap_or(0.0).abs();

            let calculated_fcf = if cfo == 0.0 || capex == 0.0 {
                let pbt = *metrics.get("ProfitBeforeTax").unwrap_or(&0.0);
                let depr = *metrics.get("DepreciationDepletionAndAmortisationExpense").unwrap_or(&0.0);
                let tax = *metrics.get("TaxExpense").unwrap_or(&0.0);
                let finance = *metrics.get("FinanceCosts").unwrap_or(&0.0);
                
                let estimated_cfo = pbt + depr - tax - finance;
                let estimated_capex = rev * valid_capex_ratio;
                estimated_cfo - estimated_capex
            } else {
                cfo - capex
            };

            let fcf_margin = if rev > 0.0 { calculated_fcf / rev } else { 0.0 };

            let year_end_date = bounds.split(" to ").collect::<Vec<&str>>()[1];
            let shp_mask = df_shp.column("date_bounds")?.str()?.equal(year_end_date);
            let df_shares = df_shp.filter(&shp_mask)?;
            
            let mut extracted_shares: Option<f64> = None;
            let shp_tag_col = df_shares.column("tag_name")?.str()?;
            let shp_ctx_col = df_shares.column("context_id")?.str()?;
            let shp_val_col = df_shares.column("raw_value")?.str()?;

            for idx in 0..df_shares.shape().0 {
                if let (Some(tag), Some(ctx)) = (shp_tag_col.get(idx), shp_ctx_col.get(idx)) {
                    if tag == "NumberOfShares" && ctx == "ShareholdingPattern_ContextI" {
                        if let Some(val_str) = shp_val_col.get(idx) {
                            if let Ok(parsed_shares) = val_str.replace(",", "").trim().parse::<f64>() {
                                if parsed_shares > 1_000_000.0 {
                                    extracted_shares = Some(parsed_shares);
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            let shares_outstanding = match extracted_shares {
                Some(shares) => shares,
                None => {
                    return Err(PolarsError::ComputeError(
                        format!("CRITICAL EXTRACTION GAP: Missing total outstanding shares matching date window ending {}.", year_end_date).into()
                    ));
                }
            };

            dynamic_timeline.push(DynamicYearData {
                date_bounds: bounds,
                revenue: rev,
                fcf_margin,
                shares_outstanding,
            });
        }
    }

    let mut wacc_scenarios = Vec::new();
    for step in -5..=5 {
        wacc_scenarios.push(base_wacc + (step as f64 * 0.005));
    }

    let mut multiplier_scenarios = Vec::new();
    let mut current_mult = 0.75;
    while current_mult <= 1.25001 {
        multiplier_scenarios.push(current_mult);
        current_mult += 0.0003;
    }

    let mut calculation_tasks = Vec::new();
    for idx in 2..dynamic_timeline.len() {
        for &wacc in &wacc_scenarios {
            for &mult in &multiplier_scenarios {
                calculation_tasks.push((idx, wacc, mult));
            }
        }
    }

    println!("⚡ Computing {} high-precision valuation cells concurrently in memory...", calculation_tasks.len());

    let matrix_grid_output: Vec<DcfMatrixCell> = calculation_tasks
        .par_iter()
        .map(|&(idx, wacc, mult)| {
            let current_node = &dynamic_timeline[idx];
            let year_label = current_node.date_bounds.split(" to ").collect::<Vec<&str>>()[1].to_string();

            let cell_omni_growth = -0.0035 * mult;
            let cell_omni_margin = 0.1033 * mult;
            
            let mut total_g_sum = 0.0;
            for j in 1..=idx {
                total_g_sum += (dynamic_timeline[j].revenue - dynamic_timeline[j-1].revenue) / dynamic_timeline[j-1].revenue;
            }
            let rolling_growth = (total_g_sum / (idx as f64)) * mult;

            let mut total_m_sum = 0.0;
            for j in 0..=idx {
                total_m_sum += dynamic_timeline[j].fcf_margin;
            }
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
                year_end: year_label,
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