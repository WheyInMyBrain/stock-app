use rayon::prelude::*;
use crate::merton_kmv::MertonKmvCell;
use crate::data_loader::UnifiedCompanyMatrix;

fn standard_normal_cdf(x: f64) -> f64 {
    let t = 1.0 / (1.0 + 0.2316419 * x.abs());
    let d = 0.3989422804014327 * (-x * x * 0.5).exp();
    let prob = d * t * (0.31938153 + t * (-0.356563782 + t * (1.781477937 + t * (-1.821255978 + t * 1.330274429))));
    if x >= 0.0 { 1.0 - prob } else { prob }
}

fn standard_normal_pdf(x: f64) -> f64 {
    0.3989422804014327 * (-x * x * 0.5).exp()
}

/// Extracts historical charts and processes dynamic structural volatility vectors straight from raw memory chart records
fn fetch_market_vol_structures(
    matrix: &UnifiedCompanyMatrix,
) -> Vec<(String, f64, f64)> {
    let mut comprehensive_timeline = Vec::new();
    let sequential_records = &matrix.raw_chart_records;

    if sequential_records.len() < 22 {
        return comprehensive_timeline;
    }

    comprehensive_timeline.reserve(sequential_records.len() - 21);

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
        if log_returns.is_empty() { continue; }

        let mean = log_returns.iter().sum::<f64>() / log_returns.len() as f64;
        let variance = log_returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (log_returns.len() - 1) as f64;
        let annualized_volatility = variance.sqrt() * (252.0_f64).sqrt();
        
        // KMV SPECIFIC DEFAULTS: Preserve your model unique fallback floor volatility constraint
        let safe_vol = if annualized_volatility.is_nan() || annualized_volatility <= 0.0 { 0.35 } else { annualized_volatility };

        comprehensive_timeline.push((target_date.clone(), target_price, safe_vol));
    }

    comprehensive_timeline
}

pub fn execute_merton_kmv_pipeline(
    matrix: &UnifiedCompanyMatrix,
    _ticker: &str,
) -> Vec<MertonKmvCell> {
    // Fetch trading timeline tracking logs chronologically with inline volatility processing passes
    let trading_timeline = fetch_market_vol_structures(matrix);
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