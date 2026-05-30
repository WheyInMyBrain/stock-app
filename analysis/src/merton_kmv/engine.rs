use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use chrono::{TimeZone, Utc};
use serde::Deserialize;
use rayon::prelude::*;
use crate::merton_kmv::MertonKmvCell;
use crate::data_loader::UnifiedCompanyMatrix;

#[derive(Debug, Deserialize)]
struct ChartDataWrapper {
    #[serde(rename = "grapthData")]
    pub graph_data: Vec<Vec<serde_json::Value>>,
}

fn standard_normal_cdf(x: f64) -> f64 {
    let t = 1.0 / (1.0 + 0.2316419 * x.abs());
    let d = 0.3989422804014327 * (-x * x * 0.5).exp();
    let prob = d * t * (0.31938153 + t * (-0.356563782 + t * (1.781477937 + t * (-1.821255978 + t * 1.330274429))));
    if x >= 0.0 { 1.0 - prob } else { prob }
}

fn standard_normal_pdf(x: f64) -> f64 {
    0.3989422804014327 * (-x * x * 0.5).exp()
}

/// Ingests historical charts and extracts ordered records to allow for chronological bisection searching
/// Extracts historical charts and structural volatility vectors directly from the pre-loaded warehouse matrix cache
fn fetch_market_vol_structures(
    matrix: &crate::data_loader::UnifiedCompanyMatrix, // 🎯 PICKER INTERCEPT: Zero I/O overhead
) -> Vec<(String, f64, f64)> {
    let mut comprehensive_timeline = Vec::with_capacity(matrix.chronological_dates.len());

    // Reconstruct the ordered trajectory tuples natively using the zero-copy centralized hash records
    for target_date in &matrix.chronological_dates {
        let target_price = *matrix.price_timeline.get(target_date).unwrap_or(&0.0);
        let target_vol = *matrix.volatility_timeline.get(target_date).unwrap_or(&0.35); // Native default fallback

        if target_price > 0.0 {
            comprehensive_timeline.push((target_date.clone(), target_price, target_vol));
        }
    }

    comprehensive_timeline
}

pub fn execute_merton_kmv_pipeline(
    matrix: &UnifiedCompanyMatrix,
    ticker: &str,
    exchange_lowercase: &str,
) -> Vec<MertonKmvCell> {
    // Fetch trading timeline tracking logs chronologically
    let trading_timeline = fetch_market_vol_structures(ticker, exchange_lowercase);
    if trading_timeline.is_empty() {
        return Vec::new();
    }

    let r_risk_free = 0.065; 
    let t_time = 1.0;        

    // 🎯 BISECTION WINDOW SEARCH: Find the closest historical trading price matching the filing date
    let find_closest_trading_node = |target_date: &str| -> (f64, f64) {
        match trading_timeline.binary_search_by(|(date, _, _)| date.as_str().cmp(target_date)) {
            Ok(exact_idx) => (trading_timeline[exact_idx].1, trading_timeline[exact_idx].2),
            Err(inserted_idx) => {
                if inserted_idx > 0 {
                    let closest_idx = (inserted_idx - 1).min(trading_timeline.len() - 1);
                    (trading_timeline[closest_idx].1, trading_timeline[closest_idx].2)
                } else {
                    (trading_timeline[0].1, trading_timeline[0].2)
                }
            }
        }
    };

    let mut task_grid = Vec::new();

    for file_key in &matrix.sorted_file_keys {
        if let Some(metrics) = matrix.document_matrix.get(file_key) {
            let date_str = matrix.file_to_date_map.get(file_key).cloned().unwrap_or("2024-03-31".to_string());
            
            let ca = *metrics.get("CurrentAssets").unwrap_or(&0.0);
            let assets = *metrics.get("Assets").unwrap_or(&0.0);
            let total_assets = if assets > 0.0 { assets } else { ca + metrics.get("NoncurrentAssets").unwrap_or(&0.0) };
            
            if total_assets <= 0.0 { continue; }

            let current_liab = *metrics.get("CurrentLiabilities").unwrap_or(&0.0);
            let total_liab = *metrics.get("Liabilities").unwrap_or(&0.0);

            let debt_barrier = if total_liab > 0.0 {
                if current_liab > 0.0 { current_liab + 0.5 * (total_liab - current_liab) } else { total_liab * 0.75 }
            } else {
                current_liab + 5_000_000.0
            };

            if debt_barrier <= 0.0 { continue; }

            let shares_outstanding = *matrix.share_history_timeline.get(&date_str).unwrap_or(&53_954_106.0);
            
            // Extract the closest price metrics via our bisection finder closure
            let (share_price, equity_vol) = find_closest_trading_node(&date_str);
            let calculated_equity_market_cap = share_price * shares_outstanding;

            if calculated_equity_market_cap > 0.0 {
                task_grid.push((date_str, calculated_equity_market_cap, debt_barrier, equity_vol));
            }
        }
    }

    // Solve simultaneous equations over the task matrix array concurrently
    let results: Vec<MertonKmvCell> = task_grid
        .par_iter()
        .map(|(date_str, equity_val, debt_barrier, equity_vol)| {
            let e = *equity_val;
            let d = *debt_barrier;
            let sigma_e = *equity_vol;

            let mut v_a = e + d; 
            let mut sigma_a = sigma_e * (e / v_a);

            for _iter in 0..200 {
                let d1 = (v_a / d).ln() + (r_risk_free + 0.5 * sigma_a * sigma_a) * t_time / (sigma_a * t_time.sqrt());
                let d2 = d1 - sigma_a * t_time.sqrt();

                let n_d1 = standard_normal_cdf(d1);
                let n_d2 = standard_normal_cdf(d2);
                let pdf_d1 = standard_normal_pdf(d1);

                let f1 = v_a * n_d1 - (-r_risk_free * t_time).exp() * d * n_d2 - e;
                let f2 = v_a * n_d1 * sigma_a - e * sigma_e;

                if f1.abs() < 1e-5 && f2.abs() < 1e-5 { break; }

                let df1_dv = n_d1;
                let df1_dsigma = v_a * t_time.sqrt() * pdf_d1;
                let df2_dv = n_d1 * sigma_a;
                let df2_dsigma = v_a * n_d1;

                let det = df1_dv * df2_dsigma - df1_dsigma * df2_dv;
                if det.abs() < 1e-8 { break; }

                let delta_v = (f2 * df1_dsigma - f1 * df2_dsigma) / det;
                let delta_sigma = (f1 * df2_dv - f2 * df1_dv) / det;

                v_a += delta_v;
                sigma_a += delta_sigma;

                if v_a <= 0.0 { v_a = e + d; }
                if sigma_a <= 0.0 { sigma_a = 0.01; }
            }

            let final_d1 = (v_a / d).ln() + (r_risk_free + 0.5 * sigma_a * sigma_a) * t_time / (sigma_a * t_time.sqrt());
            let distance_to_default = final_d1;
            let expected_default_frequency = standard_normal_cdf(-distance_to_default);

            MertonKmvCell {
                snapshot_date: date_str.clone(),
                equity_value_market_cap: e,
                structural_default_barrier: d,
                inferred_asset_value: v_a,
                inferred_asset_volatility: sigma_a,
                distance_to_default_dd: distance_to_default,
                expected_default_frequency_edf: expected_default_frequency,
            }
        })
        .collect();

    results
}