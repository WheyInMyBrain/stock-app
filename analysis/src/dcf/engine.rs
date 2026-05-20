// analysis/src/dcf/engine.rs

use polars::prelude::*;
use std::path::Path;
use std::collections::HashMap;
use rayon::prelude::*;

use crate::dcf::{DcfMatrixCell, Exchange};

struct DynamicYearData {
    date_bounds: String,
    revenue: f64,
    fcf_margin: f64,
    shares_outstanding: f64,
}

pub fn execute_dual_dcf_pipeline(
    ticker: &str,
    exchange: Exchange, // 🎯 CHOOSE INGESTION ROUTE DYNAMICALLY
    base_wacc: f64,
    base_terminal_g: f64,
) -> PolarsResult<Vec<DcfMatrixCell>> {
    
    // Resolve dynamic paths based on selected exchange enum parameter
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

    if !Path::new(&fin_path).exists() || !Path::new(&shp_path).exists() {
        println!(
            "⚠️  [INGESTION BYPASS]: Parquet tables missing for [{}] on {:?}. Skipping track cleanly.",
            ticker, exchange
        );
        // Return an empty vector instead of a critical error. This keeps the parallel engine alive!
        return Ok(Vec::new());
    }

    let df_fin = LazyFrame::scan_parquet(&fin_path, Default::default())?.collect()?;
    let df_shp = LazyFrame::scan_parquet(&shp_path, Default::default())?.collect()?;

    // ==============================================================================
    // 📊 STEP 1: PARSE SHARES OUTSTANDING (FLEXIBLE CONTEXT ANALYZER)
    // ==============================================================================
    let mut share_history_timeline: HashMap<String, f64> = HashMap::new();
    let shp_tag = df_shp.column("tag_name")?.str()?;
    let shp_ctx = df_shp.column("context_id")?.str()?;
    let shp_bounds = df_shp.column("date_bounds")?.str()?;
    let shp_val = df_shp.column("raw_value")?.str()?;

    for idx in 0..df_shp.shape().0 {
        let tag = shp_tag.get(idx).unwrap_or("");
        
        if tag == "NumberOfShares" {
            let context = shp_ctx.get(idx).unwrap_or("");
            // Flexible match strategy: catches standard BSE fields and varying NSE contexts
            if context == "ShareholdingPatternI" || context == "ShareholdingPattern_ContextI" {
                let date_key = shp_bounds.get(idx).unwrap_or("").to_string();
                let raw_str = shp_val.get(idx).unwrap_or("0").replace(",", "").replace(" ", "");
                let parsed_shares: f64 = raw_str.parse().unwrap_or(0.0);
                
                if parsed_shares > 1_000_000.0 && !date_key.is_empty() {
                    share_history_timeline.insert(date_key, parsed_shares);
                }
            }
        }
    }

    let find_historical_shares = |target_date: &str| -> f64 {
        if let Some(&shares) = share_history_timeline.get(target_date) {
            return shares;
        }
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
    // 📊 STEP 3: TIMELINE RECONSTRUCTION & FORENSIC SEED CONTEXT
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

    let mut dynamic_timeline = Vec::new();
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
            let snapshot_date = file_to_date_map.get(&file_key).unwrap().clone();
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
        return Err(PolarsError::ComputeError("Insufficient sequential historical timeline chunks to populate matrix loops.".into()));
    }

    // Determine lifecycle variables dynamically from historical records
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
    // 📊 STEP 4: GRID VALUE CALCULATOR TASK MATRICES
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

    println!("⚡ Computing {} high-precision valuation cells concurrently in memory...", calculation_tasks.len());

    let matrix_grid_output: Vec<DcfMatrixCell> = calculation_tasks
        .par_iter()
        .map(|&(idx, wacc, mult)| {
            let current_node = &dynamic_timeline[idx];
            
            // Unshifted base control anchors prevent line-node collision overlap
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