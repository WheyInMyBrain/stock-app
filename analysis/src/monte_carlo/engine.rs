// analysis/src/monte_carlo/engine.rs

use polars::prelude::*;
use std::path::Path;
use rand::rng; // Updated from thread_rng to satisfy the modern API standard
use rand_distr::{Distribution, Normal};

use crate::monte_carlo::MonteCarloReport;

struct HistoryLedgerData {
    revenue: f64,
    fcf_margin: f64,
}

pub fn execute_monte_carlo_simulation(
    ticker: &str,
    wacc: f64,
    terminal_g: f64,
) -> PolarsResult<MonteCarloReport> {
    let bse_path = format!("../data/{}/parquets/bse_financial-results-docs.parquet", ticker);
    let shp_path = format!("../data/{}/parquets/bse_shareholding-pattern-docs.parquet", ticker);

    if !Path::new(&bse_path).exists() || !Path::new(&shp_path).exists() {
        return Err(PolarsError::ComputeError(
            format!("Required Parquet source tables missing for Monte Carlo ticker {}.", ticker).into()
        ));
    }

    // 1. Load data tables lazily
    let df_fin = LazyFrame::scan_parquet(&bse_path, Default::default())?.collect()?;
    let df_shp = LazyFrame::scan_parquet(&shp_path, Default::default())?.collect()?;

    // 2. Fetch the absolute latest chronological outstanding share count row
    let mut extracted_shares: Option<f64> = None;
    let shp_tag_col = df_shp.column("tag_name")?.str()?;
    let shp_ctx_col = df_shp.column("context_id")?.str()?;
    let shp_val_col = df_shp.column("raw_value")?.str()?;

    for idx in (0..df_shp.shape().0).rev() { 
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
                "CRITICAL ARTIFACT ERROR: Could not locate a valid outstanding share base to clear the MC simulation.".into()
            ));
        }
    };

    // 3. Reconstruct historical arrays via manual dynamic pivot loop
    let target_tags = [
        "RevenueFromOperations", "CashFlowsFromUsedInOperatingActivities",
        "PurchaseOfPropertyPlantAndEquipmentClassifiedAsInvestingActivities",
        "ProfitBeforeTax", "DepreciationDepletionAndAmortisationExpense",
        "TaxExpense", "FinanceCosts"
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

    // Calculate baseline CapEx intensity
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

    // Compile vector matrices
    let mut history_ledger = Vec::new();
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
                pbt + depr - tax - finance - (rev * valid_capex_ratio)
            } else {
                cfo - capex
            };

            let fcf_margin = if rev > 0.0 { calculated_fcf / rev } else { 0.0 };
            if rev > 0.0 {
                history_ledger.push(HistoryLedgerData { revenue: rev, fcf_margin });
            }
        }
    }

    if history_ledger.len() < 2 {
        return Err(PolarsError::ComputeError(
            "Insufficient chronological historical ledger rows found to calculate variance metrics.".into()
        ));
    }

    // 4. Derive statistical Means and Volatilities (Standard Deviations) natively
    let mut growth_rates = Vec::new();
    for idx in 1..history_ledger.len() {
        let prev = history_ledger[idx - 1].revenue;
        let curr = history_ledger[idx].revenue;
        growth_rates.push((curr - prev) / prev);
    }
    let margins: Vec<f64> = history_ledger.iter().map(|h| h.fcf_margin).collect();

    let mean_growth = growth_rates.iter().sum::<f64>() / growth_rates.len() as f64;
    let var_growth = growth_rates.iter().map(|g| (g - mean_growth).powi(2)).sum::<f64>() / growth_rates.len() as f64;
    let std_growth = var_growth.sqrt();

    let mean_margin = margins.iter().sum::<f64>() / margins.len() as f64;
    let var_margin = margins.iter().map(|m| (m - mean_margin).powi(2)).sum::<f64>() / margins.len() as f64;
    let std_margin = var_margin.sqrt();

    let base_revenue = history_ledger.last().unwrap().revenue; 
    let total_trials = 10000;
    let mut simulation_results = Vec::with_capacity(total_trials);

    // 5. Initialize normal distribution curves mapping corporate operational volatility
    let dist_growth = Normal::new(mean_growth, std_growth).map_err(|e| PolarsError::ComputeError(e.to_string().into()))?;
    let dist_margin = Normal::new(mean_margin, std_margin).map_err(|e| PolarsError::ComputeError(e.to_string().into()))?;
    let mut my_rng = rng(); // Fixed warning: using modernized .rng() instantiation pattern

    // Run the 10,000 trial engine loop
    for _ in 0..total_trials {
        let sampled_growth = dist_growth.sample(&mut my_rng);
        let sampled_margin = dist_margin.sample(&mut my_rng);

        let mut current_rev = base_revenue;
        let mut present_values_sum = 0.0;
        let mut final_fcf = 0.0;

        for year in 1..=5 {
            current_rev *= 1.0 + sampled_growth;
            let fcf = current_rev * sampled_margin;
            present_values_sum += fcf / ((1.0 + wacc) as f64).powi(year);
            if year == 5 {
                final_fcf = fcf;
            }
        }

        let terminal_value = (final_fcf * (1.0 + terminal_g)) / (wacc - terminal_g);
        let pv_terminal_value = terminal_value / ((1.0 + wacc) as f64).powi(5);
        let enterprise_value = present_values_sum + pv_terminal_value;
        let share_price = enterprise_value / shares_outstanding;

        if share_price > 0.0 && !share_price.is_nan() && !share_price.is_infinite() {
            simulation_results.push(share_price);
        }
    }

    // Sort ascending to pull quantile boundaries
    simulation_results.sort_by(|a, b| a.partial_cmp(b).unwrap());

    if simulation_results.is_empty() {
        return Err(PolarsError::ComputeError("Simulation generated zero non-negative valuations.".into()));
    }

    let length = simulation_results.len() as f64;
    let get_quantile = |q: f64| -> f64 {
        let idx = ((length * q) as usize).min(simulation_results.len() - 1);
        simulation_results[idx]
    };

    let mean_expected = simulation_results.iter().sum::<f64>() / simulation_results.len() as f64;

    Ok(MonteCarloReport {
        ticker: ticker.to_string(),
        trials_executed: simulation_results.len(),
        p10_bear: get_quantile(0.10),
        p30_conservative: get_quantile(0.30),
        p50_median: get_quantile(0.50),
        p70_optimistic: get_quantile(0.70),
        p90_bull: get_quantile(0.90),
        mean_expected,
    })
}