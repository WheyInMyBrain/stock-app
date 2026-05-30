// analysis/src/data_loader.rs

use std::collections::HashMap;
use std::path::Path;
use std::fs::File;
use std::io::BufReader;
use polars::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Exchange { Bse, Nse }

/// The pure financial warehouse matrix delivered directly to picking engines
#[derive(Debug, Clone)]
pub struct UnifiedCompanyMatrix {
    pub sorted_file_keys: Vec<String>,
    pub file_to_date_map: HashMap<String, String>,
    pub document_matrix: HashMap<String, HashMap<String, f64>>, // Stores ALL tags present in the parquets!
    pub share_history_timeline: HashMap<String, f64>,
    
    // High-Fidelity Centralized Market Price and Volatility Channels
    pub chronological_dates: Vec<String>,
    pub price_timeline: HashMap<String, f64>,
    pub volatility_timeline: HashMap<String, f64>,
}

pub struct CentralFinancialsDB;

impl CentralFinancialsDB {
    /// Autonomous Data Broker: Structures and caches every single data tag and price timeline in memory
    // 🎯 FIXED: Added data_dir string slice mapping to separate system layout variables cleanly
    pub fn load_exchange_matrix(ticker: &str, exchange: Exchange, data_dir: &str) -> Option<UnifiedCompanyMatrix> {
        let (fin_path, shp_path, exchange_prefix) = match exchange {
            Exchange::Bse => (
                format!("{}/{}/parquets/bse_financial-results-docs.parquet", data_dir, ticker),
                format!("{}/{}/parquets/bse_shareholding-pattern-docs.parquet", data_dir, ticker),
                "bse",
            ),
            Exchange::Nse => (
                format!("{}/{}/parquets/nse_corporates-financial-results.parquet", data_dir, ticker),
                format!("{}/{}/parquets/nse_corporate-shareholding-master.parquet", data_dir, ticker),
                "nse",
            ),
        };

        if !Path::new(&fin_path).exists() || !Path::new(&shp_path).exists() {
            return None;
        }

        // Single-pass I/O block collection
        let df_fin = LazyFrame::scan_parquet(&fin_path, Default::default()).ok()?.collect().ok()?;
        let df_shp = LazyFrame::scan_parquet(&shp_path, Default::default()).ok()?.collect().ok()?;

        // ==============================================================================
        // 📊 CORE SHARES OUTSTANDING LOADER
        // ==============================================================================
        let mut share_history_timeline = HashMap::new();
        let shp_tag = df_shp.column("tag_name").ok()?.str().ok()?;
        let shp_ctx = df_shp.column("context_id").ok()?.str().ok()?;
        let shp_bounds = df_shp.column("date_bounds").ok()?.str().ok()?;
        let shp_val = df_shp.column("raw_value").ok()?.str().ok()?;

        for idx in 0..df_shp.shape().0 {
            if shp_tag.get(idx).unwrap_or("") == "NumberOfShares" {
                let context = shp_ctx.get(idx).unwrap_or("");
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

        // ==============================================================================
        // 📊 UNRESTRICTED DYNAMIC ALIGNMENT MACHINE (No Tag Lists!)
        // ==============================================================================
        let date_bounds_col = df_fin.column("date_bounds").ok()?.str().ok()?;
        let source_file_col = df_fin.column("source_file").ok()?.str().ok()?;
        let tag_name_col = df_fin.column("tag_name").ok()?.str().ok()?;
        let raw_value_col = df_fin.column("raw_value").ok()?.str().ok()?;

        let mut sorted_file_keys = Vec::new();
        let mut document_matrix = HashMap::new();
        let mut file_to_date_map = HashMap::new();

        for idx in 0..df_fin.shape().0 {
            let file = source_file_col.get(idx).unwrap_or("").to_string();
            let tag = tag_name_col.get(idx).unwrap_or("").to_string();
            let raw_val = raw_value_col.get(idx).unwrap_or("");

            // Enforce basic core file layout criteria
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

                // March-Annual Gatekeeper protects your data array dimensions from quarterly files
                if exchange == Exchange::Nse && !parsed_date.ends_with("-03-31") {
                    is_candidate = false;
                }

                if is_candidate {
                    if !document_matrix.contains_key(&file) {
                        sorted_file_keys.push(file.clone());
                        document_matrix.insert(file.clone(), HashMap::new());
                        file_to_date_map.insert(file.clone(), parsed_date);
                    }
                    
                    let cleaned_val: f64 = raw_val.replace(",", "").replace(" ", "").trim().parse().unwrap_or(0.0);
                    
                    // Natively pick and insert whatever accounting tag is present in this row!
                    if let Some(metrics) = document_matrix.get_mut(&file) {
                        metrics.insert(tag, cleaned_val);
                    }
                }
            }
        }

        // Organize unique files chronologically
        let file_to_date_ref = &file_to_date_map;
        sorted_file_keys.sort_by(|a, b| {
            let d_a = file_to_date_ref.get(a).map(|s| s.as_str()).unwrap_or("");
            let d_b = file_to_date_ref.get(b).map(|s| s.as_str()).unwrap_or("");
            d_a.cmp(d_b)
        });

        // ==============================================================================
        // 📊 🎯 FIXED: DYNAMIC PATH RESOLUTION FOR HISTORICAL CHART DATA
        // ==============================================================================
        let mut chronological_dates = Vec::new();
        let mut price_timeline = HashMap::new();
        let mut volatility_timeline = HashMap::new();

        let chart_path_str = format!("{}/{}/{}_historical-chart-data/10Y.json", data_dir, ticker, exchange_prefix);
        let chart_path = Path::new(&chart_path_str);

        if chart_path.exists() {
            if let Ok(file) = File::open(chart_path) {
                let reader = BufReader::new(file);
                if let Ok(chart_json) = serde_json::from_reader::<_, serde_json::Value>(reader) {
                    if let Some(graph_data) = chart_json.get("grapthData").and_then(|v| v.as_array()) {
                        let mut sequential_records: Vec<(String, f64)> = Vec::with_capacity(graph_data.len());

                        for tuple in graph_data {
                            if let (Some(ms_val), Some(price_val)) = (tuple.get(0).and_then(|v| v.as_i64()), tuple.get(1).and_then(|v| v.as_f64())) {
                                if ms_val > 0 && price_val > 0.0 {
                                    let seconds = ms_val / 1000;
                                    let day_raw = seconds / 86400;
                                    let r_year = 1970 + (day_raw / 365);
                                    let r_month = ((day_raw % 365) / 30) + 1;
                                    let r_day = (day_raw % 30) + 1;
                                    let clean_date_key = format!("{:04}-{:02}-{:02}", r_year, r_month, r_day);

                                    sequential_records.push((clean_date_key, price_val));
                                }
                            }
                        }

                        if sequential_records.len() >= 22 {
                            for i in 21..sequential_records.len() {
                                let (ref target_date, target_price) = sequential_records[i];
                                
                                let mut log_returns = Vec::with_capacity(21);
                                for j in (i - 20)..=i {
                                    let p_curr = sequential_records[j].1;
                                    let p_prev = sequential_records[j - 1].1;
                                    if p_prev > 0.0 && p_curr > 0.0 {
                                        log_returns.push((p_curr / p_prev).ln());
                                    }
                                }

                                if !log_returns.is_empty() {
                                    let mean = log_returns.iter().sum::<f64>() / log_returns.len() as f64;
                                    let variance = log_returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (log_returns.len() - 1) as f64;
                                    let annualized_volatility = variance.sqrt() * (252.0_f64).sqrt();
                                    
                                    let safe_vol = if annualized_volatility.is_nan() || annualized_volatility <= 0.0 { 0.25 } else { annualized_volatility };

                                    chronological_dates.push(target_date.clone());
                                    price_timeline.insert(target_date.clone(), target_price);
                                    volatility_timeline.insert(target_date.clone(), safe_vol);
                                }
                            }
                        }
                    }
                }
            }
        }

        Some(UnifiedCompanyMatrix {
            sorted_file_keys,
            file_to_date_map,
            document_matrix,
            share_history_timeline,
            chronological_dates,
            price_timeline,
            volatility_timeline,
        })
    }
}