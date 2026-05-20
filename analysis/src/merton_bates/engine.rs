use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use chrono::{TimeZone, Utc};
use rayon::prelude::*;
use rand_distr::{Normal, Distribution, Poisson};
use serde::Deserialize;
use crate::merton_bates::MertonBatesCell;
use crate::data_loader::Exchange; // 🎯 Unified discriminator enum imported from central core

#[derive(Debug, Deserialize)]
struct ChartDataWrapper {
    #[serde(rename = "grapthData")]
    pub graph_data: Vec<Vec<serde_json::Value>>,
}

/// Ingests your 10Y timeline chart and computes rolling real-world prices and volatilities
fn extract_price_and_vol_vectors(
    ticker: &str,
    exchange: Exchange,
) -> (Vec<String>, HashMap<String, f64>, HashMap<String, f64>) {
    let mut dates = Vec::new();
    let mut prices = HashMap::new();
    let mut volatilities = HashMap::new();

    let path_str = match exchange {
        Exchange::Bse => format!("../data/{}/bse_historical-chart-data/10Y.json", ticker),
        Exchange::Nse => format!("../data/{}/nse_historical-chart-data/10Y.json", ticker),
    };

    let file_path = Path::new(&path_str);
    if !file_path.exists() {
        println!("⚠️  [MERTON_BATES]: Target 10Y.json trace missing at {:?}. Aborting track safely.", file_path);
        return (dates, prices, volatilities);
    }

    let file = match File::open(file_path) {
        Ok(f) => f,
        Err(_) => return (dates, prices, volatilities),
    };
    let reader = BufReader::new(file);

    let parsed_chart: ChartDataWrapper = match serde_json::from_reader(reader) {
        Ok(data) => data,
        Err(_) => return (dates, prices, volatilities),
    };

    let mut sequential_records: Vec<(String, f64)> = Vec::new();

    for tuple in parsed_chart.graph_data {
        if tuple.len() < 2 { continue; }
        
        let ms_timestamp = tuple[0].as_i64().unwrap_or(0);
        let raw_price = tuple[1].as_f64().unwrap_or(0.0);

        if ms_timestamp == 0 || raw_price <= 0.0 { continue; }

        let datetime = Utc.timestamp_millis_opt(ms_timestamp).unwrap();
        let formatted_date = datetime.format("%Y-%m-%d").to_string();
        
        sequential_records.push((formatted_date, raw_price));
    }

    if sequential_records.len() < 22 {
        return (dates, prices, volatilities);
    }

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
        let safe_vol = if annualized_volatility.is_nan() || annualized_volatility <= 0.0 { 0.20 } else { annualized_volatility };

        dates.push(target_date.clone());
        prices.insert(target_date.clone(), target_price);
        volatilities.insert(target_date.clone(), safe_vol);
    }

    (dates, prices, volatilities)
}

/// Runs Euler-Maruyama path-stepping simulations across the Rayon thread pool
fn simulate_jump_diffusion_paths(
    s_0: f64, mu: f64, sigma: f64, lambda: f64, mu_j: f64, sigma_j: f64,
    days_to_forecast: usize, num_paths: usize,
) -> Vec<f64> {
    let dt: f64 = 1.0 / 252.0;
    let sqrt_dt = dt.sqrt();
    let mut final_prices = vec![0.0; num_paths];

    final_prices.par_iter_mut().for_each(|price_slot| {
        let mut rng = rand::rng(); 
        let normal_dist = Normal::new(0.0, 1.0).unwrap();
        let jump_size_dist = Normal::new(mu_j, sigma_j).unwrap();
        let poisson_dist = Poisson::new(lambda * dt).unwrap();
        
        let mut s_t = s_0;

        for _ in 0..days_to_forecast {
            if s_t <= 0.0 { break; }

            let z_1 = normal_dist.sample(&mut rng);
            let num_jumps = poisson_dist.sample(&mut rng) as usize;
            let mut jump_compounded_factor = 0.0;
            
            for _ in 0..num_jumps {
                jump_compounded_factor += jump_size_dist.sample(&mut rng); 
            }

            let continuous_drift = mu * s_t * dt;
            let continuous_diffusion = sigma * s_t * z_1 * sqrt_dt;
            let discontinuous_jump = s_t * jump_compounded_factor;

            s_t += continuous_drift + continuous_diffusion + discontinuous_jump;
        }
        *price_slot = s_t;
    });

    final_prices.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    final_prices
}

pub fn execute_merton_bates_pipeline(
    ticker: &str,
    exchange: Exchange,
) -> Vec<MertonBatesCell> {
    let (timeline_dates, historical_prices, historical_volatilities) = extract_price_and_vol_vectors(ticker, exchange);

    if timeline_dates.is_empty() {
        return Vec::new();
    }

    // Baseline continuous parameters configuration setup
    let lambda_scenarios = [1.0, 3.0];       
    let jump_size_scenarios = [-0.10, 0.05];  
    let mu_drift = 0.07;                      
    let sigma_j_vol = 0.12;                   
    let trading_days = 63;                    
    let total_paths = 10_000;                 

    // ==============================================================================
    // 📊 STEP 1: TASK GRID GENERATION MATRIX
    // ==============================================================================
    let mut evaluation_tasks = Vec::new();
    for date_key in &timeline_dates {
        let current_price = *historical_prices.get(date_key).unwrap_or(&0.0);
        let current_vol = *historical_volatilities.get(date_key).unwrap_or(&0.25);

        if current_price <= 0.0 { continue; }

        for &lambda in &lambda_scenarios {
            for &mu_j in &jump_size_scenarios {
                evaluation_tasks.push((date_key.clone(), current_price, current_vol, lambda, mu_j));
            }
        }
    }

    // ==============================================================================
    // 📊 STEP 2: CONCURRENT JUMP-DIFFUSION SIMULATIONS via RAYON
    // ==============================================================================
    println!("🎯 [Merton-Bates Picker]: Simulating trajectory distributions for [{}]...", ticker);

    let grid_results: Vec<MertonBatesCell> = evaluation_tasks
        .par_iter()
        .map(|(date_key, current_price, current_vol, lambda, mu_j)| {
            let simulated_distribution = simulate_jump_diffusion_paths(
                *current_price, mu_drift, *current_vol, *lambda, *mu_j, sigma_j_vol,
                trading_days, total_paths,
            );

            let idx_95 = (total_paths as f64 * 0.05) as usize;
            let idx_99 = (total_paths as f64 * 0.01) as usize;
            
            let var_95 = simulated_distribution[idx_95];
            let var_99 = simulated_distribution[idx_99];
            let expected_mean: f64 = simulated_distribution.iter().sum::<f64>() / total_paths as f64;

            MertonBatesCell {
                snapshot_date: date_key.clone(),
                base_stock_price: *current_price,
                implied_annual_volatility: *current_vol,
                jump_intensity_lambda: *lambda,
                expected_jump_size_mu_j: *mu_j,
                value_at_risk_95: var_95,
                value_at_risk_99: var_99,
                simulated_expected_value: expected_mean,
            }
        })
        .collect();

    grid_results
}