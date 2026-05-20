// analysis/src/epv/engine.rs

use polars::prelude::*;
use std::path::Path;
use std::collections::HashMap;
use rayon::prelude::*;

use crate::dcf::Exchange; // 🎯 Centralized data-routing discriminator enum
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
    exchange: Exchange, // 🎯 ROUTE INGESTION STREAMS DYNAMICALLY
    base_wacc: f64,
) -> PolarsResult<Vec<EpvMatrixCell>> {
    
    // Resolve dynamic paths based on selected exchange parameter
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

    // 🛡️ NATIVE LISTING BYPASS GUARD (PREVENTS CRITICAL FAILURES ON EXCLUSIVE LISTINGS)
    if !Path::new(&fin_path).exists() || !Path::new(&shp_path).exists() {
        println!(
            "⚠️  [EPV ENGINE BYPASS]: Parquet tables missing for [{}] on {:?}. Skipping track cleanly.",
            ticker, exchange
        );
        return Ok(Vec::new());
    }

    // Ingest Data Tables into memory
    let df_fin = LazyFrame::scan_parquet(&fin_path, Default::default())?.collect()?;
    let df_shp = LazyFrame::scan_parquet(&shp_path, Default::default())?.collect()?;

    // ==============================================================================
    // 📊 STEP 1: PARSE SHARES OUTSTANDING (UNIFIED SEMANTIC TIMELINE LOOKUP)
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
        "RevenueFromOperations", "ProfitBeforeTax", "FinanceCosts", 
        "DepreciationDepletionAndAmortisationExpense", "TaxExpense",
        "PurchaseOfPropertyPlantAndEquipmentClassifiedAsInvestingActivities"
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
    // 📊 STEP 3: MAPPED LEDGER GENERATION & TIMELINE CACHING
    // ==============================================================================
    let mut ledger: Vec<InternalLedgerRow> = Vec::new();
    for file_key in unique_groups {
        if let Some(metrics) = document_matrix.get(&file_key) {
            let rev = *metrics.get("RevenueFromOperations").unwrap_or(&0.0);
            let pbt = *metrics.get("ProfitBeforeTax").unwrap_or(&0.0);
            let interest = *metrics.get("FinanceCosts").unwrap_or(&0.0);
            let depr = *metrics.get("DepreciationDepletionAndAmortisationExpense").unwrap_or(&0.0);
            let tax = *metrics.get("TaxExpense").unwrap_or(&0.0);
            let capex = metrics.get("PurchaseOfPropertyPlantAndEquipmentClassifiedAsInvestingActivities").cloned().unwrap_or(0.0).abs();
            
            let ebit = pbt + interest;
            let ebit_margin = if rev > 0.0 { ebit / rev } else { 0.0 };
            let snapshot_date = file_to_date_map.get(&file_key).unwrap().clone();

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
        return Err(PolarsError::ComputeError("Insufficient analytical segments compiled to initialize EPV valuation matrix loops.".into()));
    }

    // Pre-calculate outstanding shares to bypass thread lookups
    let mut shares_timeline_mapping = Vec::with_capacity(ledger.len());
    for idx in 0..ledger.len() {
        let shares_outstanding = find_historical_shares(&ledger[idx].date_bounds);
        shares_timeline_mapping.push(shares_outstanding);
    }

    // Generate high-resolution matrix ranges in RAM
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

    println!("⚡ Computing {} high-precision rolling EPV matrix cells concurrently for {}...", grid_tasks.len(), ticker);

    // ==============================================================================
    // 📊 STEP 4: RUN RAYON PARALLEL PERPETUITY MAPPING
    // ==============================================================================
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