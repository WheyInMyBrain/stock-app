// analysis/src/epv/engine.rs

use polars::prelude::*;
use std::path::Path;
use rayon::prelude::*;

use crate::epv::EpvMatrixCell;

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
    ticker: &str, 
    base_wacc: f64,
) -> PolarsResult<Vec<EpvMatrixCell>> {
    let bse_path = format!("../data/{}/parquets/bse_financial-results-docs.parquet", ticker);
    let shp_path = format!("../data/{}/parquets/bse_shareholding-pattern-docs.parquet", ticker);

    if !Path::new(&bse_path).exists() || !Path::new(&shp_path).exists() {
        return Err(PolarsError::ComputeError(
            format!("Required Parquet source tables missing for EPV ticker {}.", ticker).into()
        ));
    }

    // 1. Ingest Data Tables into memory EXACTLY ONCE
    let df_fin = LazyFrame::scan_parquet(&bse_path, Default::default())?.collect()?;
    let df_shp = LazyFrame::scan_parquet(&shp_path, Default::default())?.collect()?;

    let target_tags = [
        "RevenueFromOperations", 
        "ProfitBeforeTax", 
        "FinanceCosts", 
        "DepreciationDepletionAndAmortisationExpense",
        "TaxExpense",
        "PurchaseOfPropertyPlantAndEquipmentClassifiedAsInvestingActivities"
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

    // 2. Map structures into a clean history ledger vector
    let mut ledger: Vec<InternalLedgerRow> = Vec::new();
    for (bounds, source) in unique_groups {
        if let Some(metrics) = grouped_rows.get(&(bounds.clone(), source.clone())) {
            let rev = *metrics.get("RevenueFromOperations").unwrap_or(&0.0);
            let pbt = *metrics.get("ProfitBeforeTax").unwrap_or(&0.0);
            let interest = *metrics.get("FinanceCosts").unwrap_or(&0.0);
            let depr = *metrics.get("DepreciationDepletionAndAmortisationExpense").unwrap_or(&0.0);
            let tax = *metrics.get("TaxExpense").unwrap_or(&0.0);
            let capex = metrics.get("PurchaseOfPropertyPlantAndEquipmentClassifiedAsInvestingActivities").cloned().unwrap_or(0.0).abs();
            
            let ebit = pbt + interest;
            let ebit_margin = if rev > 0.0 { ebit / rev } else { 0.0 };

            if rev > 0.0 {
                ledger.push(InternalLedgerRow {
                    date_bounds: bounds,
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

    let shp_bounds_col = df_shp.column("date_bounds")?.str()?;
    let shp_tag_col = df_shp.column("tag_name")?.str()?;
    let shp_ctx_col = df_shp.column("context_id")?.str()?;
    let shp_val_col = df_shp.column("raw_value")?.str()?;

    // 3. Generate high-resolution matrix ranges in RAM
    let mut wacc_scenarios = Vec::new();
    for step in -5..=5 {
        wacc_scenarios.push(base_wacc + (step as f64 * 0.005)); // +/- 2.5% in 0.5% steps
    }

    let mut multiplier_scenarios = Vec::new();
    let mut current_mult = 0.75;
    while current_mult <= 1.25001 {
        multiplier_scenarios.push(current_mult);
        current_mult += 0.0003; // Exact 0.03% increments
    }

    // Pre-calculate baseline outstanding shares matching each node timeline index to avoid looping inside threads
    let mut shares_timeline_mapping = Vec::with_capacity(ledger.len());
    for idx in 0..ledger.len() {
        let year_label = ledger[idx].date_bounds.split(" to ").collect::<Vec<&str>>()[1];
        let mut extracted_shares: Option<f64> = None;
        for s_idx in 0..df_shp.shape().0 {
            if shp_bounds_col.get(s_idx) == Some(year_label) {
                if let (Some(tag), Some(ctx)) = (shp_tag_col.get(s_idx), shp_ctx_col.get(s_idx)) {
                    if tag == "NumberOfShares" && ctx == "ShareholdingPattern_ContextI" {
                        if let Some(val_str) = shp_val_col.get(s_idx) {
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
        }

        let shares_outstanding = match extracted_shares {
            Some(shares) => shares,
            None => {
                return Err(PolarsError::ComputeError(
                    format!("CRITICAL EPV ARTIFACT ERROR: Missing outstanding shares matching context keys for year end date {}.", year_label).into()
                ));
            }
        };
        shares_timeline_mapping.push(shares_outstanding);
    }

    // Flatten cell coordinates for parallel mapping execution
    let mut grid_tasks = Vec::new();
    for idx in 0..ledger.len() {
        for &wacc in &wacc_scenarios {
            for &mult in &multiplier_scenarios {
                grid_tasks.push((idx, wacc, mult));
            }
        }
    }

    println!("⚡ Computing {} high-precision rolling EPV matrix cells concurrently in memory...", grid_tasks.len());

    // 4. Run Rayon parallel mapping across tasks
    let final_matrix_output: Vec<EpvMatrixCell> = grid_tasks
        .par_iter()
        .map(|&(idx, wacc, mult)| {
            let current_node = &ledger[idx];
            let year_label = current_node.date_bounds.split(" to ").collect::<Vec<&str>>()[1].to_string();
            let shares_used = shares_timeline_mapping[idx];

            // Compute historical baseline parameters up to this specific point in time
            let mut margin_sum = 0.0;
            let mut capex_sum = 0.0;
            let mut depr_sum = 0.0;
            for j in 0..=idx {
                margin_sum += ledger[j].ebit_margin;
                capex_sum += ledger[j].capex;
                depr_sum += ledger[j].depr;
            }
            
            // Distort margins dynamically matching the active multiplier axis
            let rolling_mean_ebit_margin = (margin_sum / ((idx + 1) as f64)) * mult;
            let avg_historical_capex = capex_sum / ((idx + 1) as f64);
            let avg_historical_depr = depr_sum / ((idx + 1) as f64);

            // Normalize EBIT for this matrix cell configuration
            let normalized_ebit = current_node.revenue * rolling_mean_ebit_margin;

            // Audit operational tax bounds
            let mut effective_tax_rate = current_node.tax / (if current_node.ebit > 0.0 { current_node.ebit } else { 1.0 });
            if effective_tax_rate < 0.0 || effective_tax_rate > 0.40 {
                effective_tax_rate = 0.25;
            }

            // Apply maintenance CapEx extraction constraints
            let maintenance_capex_ratio = (avg_historical_depr / (if avg_historical_capex > 0.0 { avg_historical_capex } else { 1.0 })).min(1.0);
            let normalized_capex = current_node.capex * maintenance_capex_ratio;

            // Calculate sustainable earnings power cash flow perpetuity
            let normalized_earnings_power = (normalized_ebit * (1.0 - effective_tax_rate)) + current_node.depr - normalized_capex;
            let enterprise_value_epv = normalized_earnings_power / wacc;
            let epv_fair_value = enterprise_value_epv / shares_used;

            EpvMatrixCell {
                year_end: year_label,
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