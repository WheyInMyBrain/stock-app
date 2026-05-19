// analysis/src/runner.rs

use polars::prelude::PolarsResult;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::time::Instant;

use crate::dcf;
use crate::monte_carlo;
use crate::epv;
use crate::multiples;

pub fn run_all_analysis_pipelines(
    ticker: &str,
    wacc: f64,
    terminal_g: f64,
    _growth_mult: f64,
    _margin_mult: f64,
) -> PolarsResult<()> {
    let global_timer_start = Instant::now();

    println!("=======================================================================================");
    println!("🚀 INITIALIZING PARALLEL ANALYTICS PIPELINE FOR TICKER: [{}]", ticker);
    println!("⚙️  CONFIGURATION OVERRIDES: WACC={} | TermG={}", wacc, terminal_g);
    println!("=======================================================================================");

    let target_dir = format!("../data/{}/analysis/", ticker);
    if let Err(e) = create_dir_all(&target_dir) {
        println!("❌ [SYSTEM ERROR]: Failed to establish output folder path hierarchy: {}", e);
        return Ok(());
    }

    let dir_ref = &target_dir;
    let ticker_ref = ticker;

    rayon::scope(|scope| {
        // Track A: High-Resolution Deterministic Matrix Grid (55,011 cells)
        scope.spawn(move |_| {
            let dcf_timer = Instant::now();
            match dcf::engine::execute_dual_dcf_pipeline(ticker_ref, wacc, terminal_g) {
                Ok(matrix_report) => {
                    let target_file_path = format!("{}/bse_dcf_projections.json", dir_ref);
                    if let Ok(json_str) = serde_json::to_string_pretty(&matrix_report) {
                        if let Ok(mut file) = File::create(&target_file_path) {
                            if file.write_all(json_str.as_bytes()).is_ok() {
                                println!("💾 [THREAD A SUCCESS]: High-Res DCF Matrix saved to disk. Runtime: {:?}", dcf_timer.elapsed());
                            }
                        }
                    }
                },
                Err(e) => println!("❌ [THREAD A ERROR]: DCF Matrix Pipeline crash: {}", e),
            }
        });

        // Track B: Stochastic Volatility Monte Carlo (183 Million Lifecycles calculated over 18,337 coordinate variations)
        scope.spawn(move |_| {
            let mc_timer = Instant::now();
            match monte_carlo::engine::execute_monte_carlo_matrix_pipeline(ticker_ref, wacc, terminal_g) {
                Ok(matrix_report) => {
                    let target_file_path = format!("{}/bse_monte_carlo_projections.json", dir_ref);
                    if let Ok(json_str) = serde_json::to_string_pretty(&matrix_report) {
                        if let Ok(mut file) = File::create(&target_file_path) {
                            if file.write_all(json_str.as_bytes()).is_ok() {
                                println!("💾 [THREAD B SUCCESS]: High-Res Monte Carlo Probability Matrix saved. Runtime: {:?}", mc_timer.elapsed());
                            }
                        }
                    }
                },
                Err(e) => println!("❌ [THREAD B ERROR]: Monte Carlo Grid Pipeline crash: {}", e),
            }
        });

        // Track C: Greenwald Earnings Power Value (High-Resolution Matrix)
        scope.spawn(move |_| {
            let epv_timer = Instant::now();
            match epv::engine::execute_rolling_epv_pipeline(ticker_ref, wacc) {
                Ok(matrix_report) => {
                    let target_file_path = format!("{}/bse_epv_projections.json", dir_ref);
                    if let Ok(json_str) = serde_json::to_string_pretty(&matrix_report) {
                        if let Ok(mut file) = File::create(&target_file_path) {
                            if file.write_all(json_str.as_bytes()).is_ok() {
                                println!("💾 [THREAD C SUCCESS]: High-Res EPV Matrix saved to disk. Runtime: {:?}", epv_timer.elapsed());
                            }
                        }
                    }
                },
                Err(e) => println!("❌ [THREAD C ERROR]: EPV Matrix Pipeline crash: {}", e),
            }
        });

        // Track D: Financial Multiples Analysis
        scope.spawn(move |_| {
            let multiples_timer = Instant::now();
            match multiples::engine::execute_multiples_analytical_pipeline(ticker_ref) {
                Ok(multiples_report) => {
                    let json_out_path = format!("{}/nse_corporate_financial_multiples.json", dir_ref);
                    if let Ok(json_string) = serde_json::to_string_pretty(&multiples_report) {
                        if let Ok(mut file_handle) = File::create(&json_out_path) {
                            if file_handle.write_all(json_string.as_bytes()).is_ok() {
                                println!("💾 [THREAD D SUCCESS]: High-Coverage Financial Multiples Timeline saved. Runtime: {:?}", multiples_timer.elapsed());
                            }
                        }
                    }
                },
                Err(err) => println!("❌ [THREAD D ERROR]: Multiples Pipeline Engine crashed: {}", err),
            }
        });
    });

    println!("=======================================================================================");
    println!("✅ TOTAL THREE-WAY CONCURRENT PIPELINE EXECUTION WINDOW: {:?}", global_timer_start.elapsed());
    println!("=======================================================================================");
    
    Ok(())
}