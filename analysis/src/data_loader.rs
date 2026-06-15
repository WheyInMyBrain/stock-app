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
    
    // Retains only raw, sequential timeline vectors and price mappings
    pub raw_chart_records: Vec<(String, f64)>,
    pub price_timeline: HashMap<String, f64>,
}

pub struct CentralFinancialsDB;

impl CentralFinancialsDB {
    /// Autonomous Data Broker: Structures and caches every single data tag and price timeline in memory
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
        // 📊 OPTIMIZED SHARES OUTSTANDING LOADER (Zipped Iterators)
        // ==============================================================================
        let mut share_history_timeline = HashMap::new();
        let shp_tag = df_shp.column("tag_name").ok()?.str().ok()?;
        let shp_ctx = df_shp.column("context_id").ok()?.str().ok()?;
        let shp_bounds = df_shp.column("date_bounds").ok()?.str().ok()?;
        let shp_val = df_shp.column("raw_value").ok()?.str().ok()?;

        // Using zipped iterators avoids the slow chunk-traversal overhead of .get(idx)
        for (((tag, ctx), bounds), val) in shp_tag.iter()
            .zip(shp_ctx.iter())
            .zip(shp_bounds.iter())
            .zip(shp_val.iter()) 
        {
            if tag.unwrap_or("") == "NumberOfShares" {
                let context = ctx.unwrap_or("");
                if context == "ShareholdingPatternI" || context == "ShareholdingPattern_ContextI" {
                    let date_key = bounds.unwrap_or("");
                    if !date_key.is_empty() {
                        let raw_str = val.unwrap_or("0").replace(",", "").replace(" ", "");
                        if let Ok(parsed_shares) = raw_str.parse::<f64>() {
                            if parsed_shares > 1_000_000.0 {
                                share_history_timeline.insert(date_key.to_string(), parsed_shares);
                            }
                        }
                    }
                }
            }
        }

        // ==============================================================================
        // 📊 SINGLE-PASS COLLAPSED FINANCIALS ENGINE (Zero Repeated Scans)
        // ==============================================================================
        let date_bounds_col = df_fin.column("date_bounds").ok()?.str().ok()?;
        let source_file_col = df_fin.column("source_file").ok()?.str().ok()?;
        let tag_name_col = df_fin.column("tag_name").ok()?.str().ok()?;
        let raw_value_col = df_fin.column("raw_value").ok()?.str().ok()?;

        #[derive(Default)]
        struct FileCacheEntry {
            date: String,
            nature: String,
            has_bse_annual_bounds: bool,
            metrics: HashMap<String, f64>,
        }

        let mut file_processing_map: HashMap<String, FileCacheEntry> = HashMap::new();

        // One single pass through the dataframe extracts metadata and fills metrics simultaneously
        for (((bounds, file), tag), val) in date_bounds_col.iter()
            .zip(source_file_col.iter())
            .zip(tag_name_col.iter())
            .zip(raw_value_col.iter()) 
        {
            let file_name = match file {
                Some(f) if !f.is_empty() => f,
                _ => continue,
            };

            let tag_str = tag.unwrap_or("");
            let val_str = val.unwrap_or("");
            let bounds_str = bounds.unwrap_or("");

            let entry = file_processing_map.entry(file_name.to_string()).or_default();

            // Intercept taxonomy metadata tokens on the fly
            if tag_str == "DateOfEndOfReportingPeriod" && !val_str.is_empty() && val_str != "NA" {
                entry.date = val_str.trim().to_string();
            } else if tag_str == "NatureOfReportStandaloneConsolidated" && !val_str.is_empty() && val_str != "NA" {
                entry.nature = val_str.trim().to_uppercase();
            }

            if exchange == Exchange::Bse && bounds_str.contains("-04-01 to ") {
                entry.has_bse_annual_bounds = true;
            }

            // Parse and cleanly store the numeric observation inside the mapped bucket
            let cleaned_val: f64 = val_str.replace(",", "").replace(" ", "").trim().parse().unwrap_or(0.0);
            entry.metrics.insert(tag_str.to_string(), cleaned_val);
        }

        // ==============================================================================
        // 📊 TAXONOMY SELECTION FILTER (Post-Pass Matrix Construction)
        // ==============================================================================
        let mut sorted_file_keys = Vec::new();
        let mut document_matrix = HashMap::new();
        let mut file_to_date_map = HashMap::new();

        // Loop over unique files map instead of thousands of rows
        for (file_key, cache) in file_processing_map {
            let is_consolidated = cache.nature.contains("CONSOLIDATED");
            let mut is_candidate = is_consolidated && !cache.date.is_empty() && cache.date.ends_with("-03-31");

            if is_candidate && exchange == Exchange::Bse {
                is_candidate = cache.has_bse_annual_bounds;
            }

            if is_candidate {
                sorted_file_keys.push(file_key.clone());
                file_to_date_map.insert(file_key.clone(), cache.date);
                document_matrix.insert(file_key, cache.metrics);
            }
        }

        // Organize reporting entries chronologically based on statement endpoint dates
        let file_to_date_ref = &file_to_date_map;
        sorted_file_keys.sort_by(|a, b| {
            let d_a = file_to_date_ref.get(a).map(|s| s.as_str()).unwrap_or("");
            let d_b = file_to_date_ref.get(b).map(|s| s.as_str()).unwrap_or("");
            d_a.cmp(d_b)
        });

        // ==============================================================================
        // 📊 INGESTION: HISTORICAL CHART RECORDS BROKER
        // ==============================================================================
        let mut raw_chart_records = Vec::new();
        let mut price_timeline = HashMap::new();

        let chart_path_str = format!("{}/{}/{}_historical-chart-data/10Y.json", data_dir, ticker, exchange_prefix);
        let chart_path = Path::new(&chart_path_str);

        if chart_path.exists() {
            if let Ok(file) = File::open(chart_path) {
                let reader = BufReader::new(file);
                if let Ok(chart_json) = serde_json::from_reader::<_, serde_json::Value>(reader) {
                    if let Some(graph_data) = chart_json.get("grapthData").and_then(|v| v.as_array()) {
                        raw_chart_records.reserve(graph_data.len());
                        price_timeline.reserve(graph_data.len());

                        for tuple in graph_data {
                            if let (Some(ms_val), Some(price_val)) = (tuple.get(0).and_then(|v| v.as_i64()), tuple.get(1).and_then(|v| v.as_f64())) {
                                if ms_val > 0 && price_val > 0.0 {
                                    let seconds = ms_val / 1000;
                                    let day_raw = seconds / 86400;
                                    let r_year = 1970 + (day_raw / 365);
                                    let r_month = ((day_raw % 365) / 30) + 1;
                                    let r_day = (day_raw % 30) + 1;
                                    let clean_date_key = format!("{:04}-{:02}-{:02}", r_year, r_month, r_day);

                                    raw_chart_records.push((clean_date_key.clone(), price_val));
                                    price_timeline.insert(clean_date_key, price_val);
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
            raw_chart_records,
            price_timeline,
        })
    }
}