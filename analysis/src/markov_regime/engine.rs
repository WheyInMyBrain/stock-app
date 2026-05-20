use std::collections::HashMap;
use std::path::Path;
use polars::prelude::*;
use rayon::prelude::*;

use crate::markov_regime::MarkovRegimeCell;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Exchange { Bse, Nse }

pub fn execute_markov_regime_pipeline(
    ticker: &str,
    exchange: Exchange,
) -> Vec<MarkovRegimeCell> {
    let mut grid_results = Vec::new();

    // 🎯 STAGE 1: Resolve explicit data paths from your system parquets
    let path_str = match exchange {
        Exchange::Bse => format!("../data/{}/parquets/bse_financial-results-docs.parquet", ticker),
        Exchange::Nse => format!("../data/{}/parquets/nse_corporates-financial-results.parquet", ticker),
    };

    if !Path::new(&path_str).exists() {
        println!("⚠️  [MARKOV_REGIME]: Target parquet source missing at {}. Aborting cleanly.", path_str);
        return grid_results;
    }

    // 🎯 STAGE 2: Read Parquet data via Polars LazyFrame lazy loading
    let df_raw = match LazyFrame::scan_parquet(&path_str, ScanArgsParquet::default()) {
        Ok(lf) => match lf.collect() {
            Ok(df) => df,
            Err(_) => return grid_results,
        },
        Err(_) => return grid_results,
    };

    // Extract target tags required to evaluate EBIT margins
    let target_tags = ["RevenueFromOperations", "ProfitBeforeTax", "FinanceCosts"];
    
    let tag_col = match df_raw.column("tag_name").and_then(|c| c.str()) {
        Ok(c) => c,
        Err(_) => return grid_results,
    };

    let mask: BooleanChunked = tag_col.into_iter().map(|opt| opt.map_or(false, |v| target_tags.contains(&v))).collect();
    let df_filtered = match df_raw.filter(&mask) {
        Ok(df) => df,
        Err(_) => return grid_results,
    };

    let date_bounds_col = df_filtered.column("date_bounds").unwrap().str().unwrap();
    let source_file_col = df_filtered.column("source_file").unwrap().str().unwrap();
    let tag_name_col = df_filtered.column("tag_name").unwrap().str().unwrap();
    let raw_value_col = df_filtered.column("raw_value").unwrap().str().unwrap();

    let mut unique_groups = Vec::new();
    let mut document_matrix: HashMap<String, HashMap<String, f64>> = HashMap::new();
    let mut file_to_date_map: HashMap<String, String> = HashMap::new();

    // 🎯 STAGE 3: THE GATEKEEPER - Parse dates upfront and intercept quarterly noise rows
    for idx in 0..df_filtered.shape().0 {
        let file = source_file_col.get(idx).unwrap_or("").to_string();
        let tag = tag_name_col.get(idx).unwrap_or("").to_string();
        let raw_val = raw_value_col.get(idx).unwrap_or("");

        let mut is_candidate = match exchange {
            Exchange::Bse => file.contains("Consolidated") && file.contains("_MC") && date_bounds_col.get(idx).unwrap_or("").contains("-04-01 to "),
            Exchange::Nse => file.contains("Consolidated"),
        };

        if is_candidate {
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
                            "jan" => "01", "feb" => "02", "mar" => "03", "apr" => "04", "may" => "05", "jun" => "06", 
                            "jul" => "07", "aug" => "08", "sep" => "09", "oct" => "10", "nov" => "11", "dec" => "12", _ => "03"
                        };
                        format!("{}-{}-{}", comps[2], m_num, comps[0])
                    } else { "2024-03-31".to_string() }
                }
            };

            // Drop structural quarterly records safely
            if exchange == Exchange::Nse && !parsed_date.ends_with("-03-31") {
                is_candidate = false;
            }

            if is_candidate {
                if !document_matrix.contains_key(&file) {
                    unique_groups.push(file.clone());
                    document_matrix.insert(file.clone(), HashMap::new());
                    file_to_date_map.insert(file.clone(), parsed_date);
                }
                let cleaned_val: f64 = raw_val.replace(",", "").replace(" ", "").trim().parse().unwrap_or(0.0);
                if let Some(metrics) = document_matrix.get_mut(&file) {
                    metrics.insert(tag, cleaned_val);
                }
            }
        }
    }

    // Chronologically sort unique groups safely using string slice references
    unique_groups.sort_by(|a, b| {
        let d_a = file_to_date_map.get(a).map(|s| s.as_str()).unwrap_or("");
        let d_b = file_to_date_map.get(b).map(|s| s.as_str()).unwrap_or("");
        d_a.cmp(d_b)
    });

    // 🎯 STAGE 4: Process observations sequentially for tracking shifts
    let mut observation_stream = Vec::new();
    let mut matching_dates = Vec::new();

    for file_key in &unique_groups {
        if let Some(metrics) = document_matrix.get(file_key) {
            let rev = *metrics.get("RevenueFromOperations").unwrap_or(&0.0);
            let pbt = *metrics.get("ProfitBeforeTax").unwrap_or(&0.0);
            let interest = *metrics.get("FinanceCosts").unwrap_or(&0.0);
            let ebit = pbt + interest;
            let margin = if rev > 0.0 { ebit / rev } else { 0.0 };
            let date_str = file_to_date_map.get(file_key).cloned().unwrap_or("2024-03-31".to_string());

            if rev > 0.0 {
                observation_stream.push(margin);
                matching_dates.push(date_str);
            }
        }
    }

    if observation_stream.is_empty() {
        return grid_results;
    }

    // Setup Fixed transition matrix parameters
    let transition_matrix = vec![
        vec![0.70, 0.20, 0.10], 
        vec![0.15, 0.70, 0.15], 
        vec![0.10, 0.30, 0.60], 
    ];

    // 🎯 STAGE 5: Parallel matrix compute block via Rayon
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

    println!("🎯 [markov_regime] Natively ingested parquets. Extracted {} transition states for [{}]", grid_results.len(), ticker);
    grid_results
}