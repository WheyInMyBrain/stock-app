// analysis/src/monte_carlo/engine.rs

use polars::prelude::*;
use std::path::Path;
use std::collections::HashMap;
use rand::rng;
use rand_distr::{Distribution, Normal};
use rayon::prelude::*;

use crate::dcf::Exchange; // 🎯 Centralized data-routing discriminator enum
use crate::monte_carlo::MonteCarloMatrixCell;

struct HistoryLedgerData {
    revenue: f64,
    fcf_margin: f64,
}

/// The high-resolution stochastic matrix pipeline generator
pub fn execute_monte_carlo_matrix_pipeline(
    ticker: &str,
    exchange: Exchange, // 🎯 CHOOSE DATA STREAM ON THE FLY
    base_wacc: f64,
    base_terminal_g: f64,
) -> PolarsResult<Vec<MonteCarloMatrixCell>> {
    
    // Resolve paths dynamically based on exchange context
    let (fin_path, shp_path) = match exchange {
        Exchange::Bse => (
            format!("../data/{}/parquets/bse_financial-results-docs.parquet", ticker),
            format!("../data/{}/parquets/bse_shareholding-pattern-docs.parquet", ticker),
        ),
        Exchange::Nse => (
            format!("../data/{}/parquets/nse_corporates-financial-results.parquet", ticker),
            format!("../data/{}/parquets/nse_corporate-shareholding-master.parquet", ticker),
        ),
    };

    // 🛡️ NATIVE LISTING BYPASS GUARD (HANDLES EXCLUSIVE LISTINGS GRACEFULLY)
    if !Path::new(&fin_path).exists() || !Path::new(&shp_path).exists() {
        println!(
            "⚠️  [MONTE CARLO BYPASS]: Parquet tables missing for [{}] on {:?}. Skipping track cleanly.",
            ticker, exchange
        );
        return Ok(Vec::new());
    }

    // Ingest data into memory
    let df_fin = LazyFrame::scan_parquet(&fin_path, Default::default())?.collect()?;
    let df_shp = LazyFrame::scan_parquet(&shp_path, Default::default())?.collect()?;

    // ==============================================================================
    // 📊 STEP 1: PARSE SHARES OUTSTANDING (UNIFIED TIMELINE LOOKUP)
    // ==============================================================================
    let mut share_history_timeline: HashMap<String, f64> = HashMap::new();
    let shp_tag = df_shp.column("tag_name")?.str()?;
    let shp_ctx = df_shp.column("context_id")?.str()?;
    let shp_bounds = df_shp.column("date_bounds")?.str()?;
    let shp_val = df_shp.column("raw_value")?.str()?;

    for idx in 0..df_shp.shape().0 {
        let tag = shp_tag.get(idx).unwrap_or("");
        let context = shp_ctx.get(idx).unwrap_or("");
        
        if tag == "NumberOfShares" && (context == "ShareholdingPatternI" || context == "ShareholdingPattern_ContextI") {
            let date_key = shp_bounds.get(idx).unwrap_or("").to_string();
            let raw_str = shp_val.get(idx).unwrap_or("0").replace(",", "").replace(" ", "");
            let parsed_shares: f64 = raw_str.parse().unwrap_or(0.0);
            if parsed_shares > 1_000_000.0 && !date_key.is_empty() {
                share_history_timeline.insert(date_key, parsed_shares);
            }
        }
    }

    let find_historical_shares = |target_date: &str| -> f64 {
        if let Some(&shares) = share_history_timeline.get(target_date) { return shares; }
        let target_year = target_date.split('-').next().unwrap_or("2024");
        let mut sorted_dates: Vec<&String> = share_history_timeline.keys().collect();
        sorted_dates.sort();
        for date_key in sorted_dates.iter().rev() {
            if date_key.starts_with(target_year) {
                return *share_history_timeline.get(*date_key).unwrap_or(&53_954_106.0);
            }
        }
        *share_history_timeline.values().next().unwrap_or(&53_954_106.0)
    };

    // ==============================================================================
    // 📊 STEP 2: IN-MEMORY PIVOT MATCHING & DATE STANDARDIZATION
    // ==============================================================================
    let target_tags = [
        "RevenueFromOperations", "CashFlowsFromUsedInOperatingActivities",
        "PurchaseOfPropertyPlantAndEquipmentClassifiedAsInvestingActivities", "ProfitBeforeTax",
        "DepreciationDepletionAndAmortisationExpense", "TaxExpense", "FinanceCosts"
    ];

    let tag_col = df_fin.column("tag_name")?.str()?;
    let mask: BooleanChunked = tag_col.into_iter().map(|opt| opt.map_or(false, |v| target_tags.contains(&v))).collect();
    let df_filtered = df_fin.filter(&mask)?;
    
    let date_bounds_col = df_filtered.column("date_bounds")?.str()?;
    let source_file_col = df_filtered.column("source_file")?.str()?;
    let tag_name_col = df_filtered.column("tag_name")?.str()?;
    let raw_value_col = df_filtered.column("raw_value")?.str()?;

    let mut unique_groups = Vec::new();
    let mut document_matrix: HashMap<String, HashMap<String, f64>> = HashMap::new();
    let mut file_to_date_map: HashMap<String, String> = HashMap::new();

    for idx in 0..df_filtered.shape().0 {
        let file = source_file_col.get(idx).unwrap_or("").to_string();
        let tag = tag_name_col.get(idx).unwrap_or("").to_string();
        let raw_val = raw_value_col.get(idx).unwrap_or("");

        // 🎯 STAGE 1: Broad candidate collection based on foundational traits
        let mut is_candidate = match exchange {
            Exchange::Bse => {
                file.contains("Consolidated") 
                    && file.contains("_MC") 
                    && date_bounds_col.get(idx).unwrap_or("").contains("-04-01 to ")
            },
            Exchange::Nse => file.contains("Consolidated"), // Grab all candidates to evaluate dates next
        };

        if is_candidate {
            // 🎯 STAGE 2: Standardize the tracking date token down to ISO "YYYY-MM-DD"
            let parsed_date = match exchange {
                Exchange::Bse => {
                    let bounds_str = date_bounds_col.get(idx).unwrap_or("");
                    bounds_str.split(" to ").collect::<Vec<&str>>().get(1).unwrap_or(&"2024-03-31").to_string()
                },
                Exchange::Nse => {
                    let prefix = file.split('_').next().unwrap_or("31-Mar-2024");
                    let comps: Vec<&str> = prefix.split('-').collect();
                    if comps.len() >= 3 {
                        let m_num = match comps[1].to_lowercase().as_str() {
                            "jan" => "01", "feb" => "02", "mar" => "03", "apr" => "04", 
                            "may" => "05", "jun" => "06", "jul" => "07", "aug" => "08", 
                            "sep" => "09", "oct" => "10", "nov" => "11", "dec" => "12", 
                            _ => "03"
                        };
                        format!("{}-{}-{}", comps[2], m_num, comps[0])
                    } else { 
                        "2024-03-31".to_string() 
                    }
                }
            };

            // 🎯 STAGE 3: THE GATEKEEPER - If it is an NSE filing, it MUST resolve to a March Annual node!
            // This drops all structural quarterly/interim files completely regardless of filename syntax.
            if exchange == Exchange::Nse && !parsed_date.ends_with("-03-31") {
                is_candidate = false;
            }

            // 🎯 STAGE 4: WRITE METRICS MAPPED TO VERIFIED MATRIX ENTRIES
            if is_candidate {
                if !document_matrix.contains_key(&file) {
                    unique_groups.push(file.clone());
                    document_matrix.insert(file.clone(), HashMap::new());
                    file_to_date_map.insert(file.clone(), parsed_date);
                }

                let cleaned_val: f64 = raw_val
                    .replace(",", "")
                    .replace(" ", "")
                    .trim()
                    .parse()
                    .unwrap_or(0.0);

                if let Some(metrics) = document_matrix.get_mut(&file) {
                    metrics.insert(tag, cleaned_val);
                }
            }
        }
    }
    unique_groups.sort();

    // ==============================================================================
    // 📊 STEP 3: RECONSTRUCT TIMELINE LEDGER & STATISTICAL MOMENTS
    // ==============================================================================
    let mut capex_ratios = Vec::new();
    for file_key in &unique_groups {
        if let Some(metrics) = document_matrix.get(file_key) {
            let rev = *metrics.get("RevenueFromOperations").unwrap_or(&0.0);
            let capex = metrics.get("PurchaseOfPropertyPlantAndEquipmentClassifiedAsInvestingActivities").cloned().unwrap_or(0.0).abs();
            if rev > 0.0 && capex > 0.0 { capex_ratios.push(capex / rev); }
        }
    }
    let valid_capex_ratio = if !capex_ratios.is_empty() { capex_ratios.iter().sum::<f64>() / capex_ratios.len() as f64 } else { 0.05 };

    let mut history_ledger = Vec::new();
    let mut last_snapshot_date = "2024-03-31".to_string();

    for file_key in unique_groups {
        if let Some(metrics) = document_matrix.get(&file_key) {
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
            last_snapshot_date = file_to_date_map.get(&file_key).unwrap().clone();
            history_ledger.push(HistoryLedgerData { revenue: rev, fcf_margin });
        }
    }

    if history_ledger.len() < 2 {
        return Err(PolarsError::ComputeError("Insufficient sequential historical timeline chunks to populate Monte Carlo variance distributions.".into()));
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
    let std_growth = var_growth.sqrt().max(0.01); // Standard deviation sanity floor

    let base_mean_margin = margins.iter().sum::<f64>() / margins.len() as f64;
    let var_margin = margins.iter().map(|m| (m - base_mean_margin).powi(2)).sum::<f64>() / margins.len() as f64;
    let std_margin = var_margin.sqrt().max(0.01);

    let base_revenue = history_ledger.last().unwrap().revenue; 
    let shares_outstanding = find_historical_shares(&last_snapshot_date);

    // ==============================================================================
    // 📊 STEP 4: GRID GENERATION & STOCHASTIC MONTE CARLO SIMULATION
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

    println!("🎲 Launching parallel processing for {} high-res Monte Carlo scenarios...", grid_tasks.len());

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