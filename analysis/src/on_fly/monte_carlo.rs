// stock-app/analysis/src/on_fly/monte_carlo.rs

use rand_distr::{Normal, Distribution};
use rayon::prelude::*; 

/// Input configurations passed down explicitly by the engine orchestrator
pub struct MonteCarloInputMetrics {
    pub forecast_days: usize,
    pub num_simulations: usize,
    pub confidence_level: f64, 
    pub visual_paths_to_return: usize,
    pub historical_lookback: usize,
}

/// Consolidated output packet containing raw structural analytics
pub struct MonteCarloCalculationOutput {
    pub expected_value: f64,
    pub upper_bound: f64,
    pub lower_bound: f64,
    pub visual_paths: Vec<Vec<f64>>, 
    pub status_ok: bool,
    pub error_msg: String,
}

/// Executes a native, highly parallelized Geometric Brownian Motion (GBM) simulation over RAM
pub fn calculate_monte_carlo_on_fly(
    prices: &[f64],
    metrics: &MonteCarloInputMetrics,
) -> MonteCarloCalculationOutput {
    if prices.len() < 2 {
        return MonteCarloCalculationOutput {
            expected_value: 0.0,
            upper_bound: 0.0,
            lower_bound: 0.0,
            visual_paths: Vec::new(),
            status_ok: false,
            error_msg: "❌ ERROR: Insufficient price history data to generate baseline metrics.".to_string(),
        };
    }

    // Determine your exact historical lookback window slice bounds natively
    let slice_start = if prices.len() > metrics.historical_lookback {
        prices.len() - metrics.historical_lookback - 1
    } else {
        0
    };

    // 1. Calculate daily logarithmic returns strictly within the lookback window
    let mut log_returns = Vec::new();
    for i in (slice_start + 1)..prices.len() {
        if prices[i - 1] <= 0.0 || prices[i] <= 0.0 { continue; }
        log_returns.push((prices[i] / prices[i - 1]).ln());
    }

    let count = log_returns.len() as f64;
    if count < 1.0 {
        return MonteCarloCalculationOutput {
            expected_value: 0.0,
            upper_bound: 0.0,
            lower_bound: 0.0,
            visual_paths: Vec::new(),
            status_ok: false,
            error_msg: "🚨 [MATH FAULT]: Zero valid positive price coordinates matched lookback limits.".to_string(),
        };
    }

    // 2. Compute Mean and Variance of daily log returns over your lookback window
    let mean_daily = log_returns.iter().sum::<f64>() / count;
    let variance_daily = log_returns.iter()
        .map(|r| (r - mean_daily).powi(2))
        .sum::<f64>() / if count > 1.0 { count - 1.0 } else { 1.0 };
    
    let std_dev_daily = variance_daily.sqrt();

    // 3. Annualize historical components 
    const TRADING_DAYS: f64 = 252.0;
    let drift = mean_daily * TRADING_DAYS;
    let volatility = std_dev_daily * TRADING_DAYS.sqrt();

    if volatility == 0.0 {
        return MonteCarloCalculationOutput {
            expected_value: 0.0,
            upper_bound: 0.0,
            lower_bound: 0.0,
            visual_paths: Vec::new(),
            status_ok: false,
            error_msg: "🚨 [MATH FAULT]: Flat baseline detected inside lookback window.".to_string(),
        };
    }

    let last_known_price = prices[prices.len() - 1];
    let dt = 1.0 / TRADING_DAYS;

    let drift_exp_component = (drift - (volatility.powi(2) / 2.0)) * dt;
    let vol_exp_component = volatility * dt.sqrt();

    // =========================================================================
    // PARALLEL ENGINE SIMULATION LOOP BLOCK (RAYON MAPPING)
    // =========================================================================
    let simulation_results: Vec<(f64, Option<Vec<f64>>)> = (0..metrics.num_simulations)
        .into_par_iter()
        .map(|sim_idx| {
            let mut thread_rng = rand::rng(); 
            let normal_dist = Normal::new(0.0, 1.0).unwrap();
            
            let mut current_price = last_known_price;
            let mut single_path = if sim_idx < metrics.visual_paths_to_return {
                let mut p = Vec::with_capacity(metrics.forecast_days + 1);
                p.push(last_known_price);
                Some(p)
            } else {
                None
            };

            for _day in 0..metrics.forecast_days {
                let epsilon: f64 = normal_dist.sample(&mut thread_rng);
                let exponent = drift_exp_component + (vol_exp_component * epsilon);
                current_price *= exponent.exp();

                if let Some(ref mut p) = single_path {
                    p.push(current_price);
                }
            }

            (current_price, single_path)
        })
        .collect();

    let mut final_prices = Vec::with_capacity(metrics.num_simulations);
    let mut visual_paths = Vec::with_capacity(metrics.visual_paths_to_return);

    for (final_price, path_opt) in simulation_results {
        final_prices.push(final_price);
        if let Some(path) = path_opt {
            visual_paths.push(path);
        }
    }

    final_prices.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    
    let total_f = metrics.num_simulations as f64;
    let expected_val = final_prices.iter().sum::<f64>() / total_f;

    let alpha = (100.0 - metrics.confidence_level) / 2.0 / 100.0; 
    let lower_idx = (total_f * alpha) as usize;
    let upper_idx = (total_f * (1.0 - alpha)) as usize;

    let lower_bound = final_prices[lower_idx.min(final_prices.len() - 1)];
    let upper_bound = final_prices[upper_idx.min(final_prices.len() - 1)];

    MonteCarloCalculationOutput {
        expected_value: expected_val,
        upper_bound,
        lower_bound,
        visual_paths,
        status_ok: true,
        error_msg: "".to_string(),
    }
}