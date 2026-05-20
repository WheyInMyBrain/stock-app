// analysis/src/runner.rs

use polars::prelude::PolarsResult;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::time::Instant;

use crate::dcf;
use crate::dcf::Exchange;
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
        // ==============================================================================
        // 📊 Track A-1: High-Resolution Deterministic Matrix Grid for BSE
        // ==============================================================================
        scope.spawn(move |_| {
            let dcf_timer = Instant::now();
            match dcf::engine::execute_dual_dcf_pipeline(ticker_ref, dcf::Exchange::Bse, wacc, terminal_g) {
                Ok(matrix_report) => {
                    // Only write to disk if the exchange data actually exists!
                    if !matrix_report.is_empty() {
                        let target_file_path = format!("{}/bse_dcf_projections.json", dir_ref);
                        if let Ok(json_str) = serde_json::to_string_pretty(&matrix_report) {
                            if let Ok(mut file) = File::create(&target_file_path) {
                                if file.write_all(json_str.as_bytes()).is_ok() {
                                    println!("💾 [THREAD A-1 SUCCESS]: High-Res BSE DCF Matrix saved. Runtime: {:?}", dcf_timer.elapsed());
                                }
                            }
                        }
                    }
                },
                Err(e) => println!("❌ [THREAD A-1 ERROR]: BSE DCF Matrix Pipeline crash: {}", e),
            }
        });

        // ==============================================================================
        // 📊 Track A-2: High-Resolution Deterministic Matrix Grid for NSE
        // ==============================================================================
        scope.spawn(move |_| {
            let dcf_timer = Instant::now();
            match dcf::engine::execute_dual_dcf_pipeline(ticker_ref, dcf::Exchange::Nse, wacc, terminal_g) {
                Ok(matrix_report) => {
                    // Only write to disk if the exchange data actually exists!
                    if !matrix_report.is_empty() {
                        let target_file_path = format!("{}/nse_dcf_projections.json", dir_ref);
                        if let Ok(json_str) = serde_json::to_string_pretty(&matrix_report) {
                            if let Ok(mut file) = File::create(&target_file_path) {
                                if file.write_all(json_str.as_bytes()).is_ok() {
                                    println!("💾 [THREAD A-2 SUCCESS]: High-Res NSE DCF Matrix saved. Runtime: {:?}", dcf_timer.elapsed());
                                }
                            }
                        }
                    }
                },
                Err(e) => println!("❌ [THREAD A-2 ERROR]: NSE DCF Matrix Pipeline crash: {}", e),
            }
        });

        // ==============================================================================
        // 🎲 TRACK B-1: BSE STOCHASTIC MONTE CARLO PROBABILITIES
        // ==============================================================================
        scope.spawn(move |_| {
            let mc_timer = Instant::now();
            match monte_carlo::engine::execute_monte_carlo_matrix_pipeline(ticker_ref, Exchange::Bse, wacc, terminal_g) {
                Ok(matrix_report) => {
                    if !matrix_report.is_empty() {
                        let target_file_path = format!("{}/bse_monte_carlo_projections.json", dir_ref);
                        if let Ok(json_str) = serde_json::to_string_pretty(&matrix_report) {
                            if let Ok(mut file) = File::create(&target_file_path) {
                                if file.write_all(json_str.as_bytes()).is_ok() {
                                    println!("💾 [THREAD B-1 SUCCESS]: High-Res BSE Monte Carlo saved. Runtime: {:?}", mc_timer.elapsed());
                                }
                            }
                        }
                    }
                },
                Err(e) => println!("❌ [THREAD B-1 ERROR]: BSE Monte Carlo Grid Pipeline crash: {}", e),
            }
        });

        // ==============================================================================
        // 🎲 TRACK B-2: NSE STOCHASTIC MONTE CARLO PROBABILITIES
        // ==============================================================================
        scope.spawn(move |_| {
            let mc_timer = Instant::now();
            match monte_carlo::engine::execute_monte_carlo_matrix_pipeline(ticker_ref, Exchange::Nse, wacc, terminal_g) {
                Ok(matrix_report) => {
                    if !matrix_report.is_empty() {
                        let target_file_path = format!("{}/nse_monte_carlo_projections.json", dir_ref);
                        if let Ok(json_str) = serde_json::to_string_pretty(&matrix_report) {
                            if let Ok(mut file) = File::create(&target_file_path) {
                                if file.write_all(json_str.as_bytes()).is_ok() {
                                    println!("💾 [THREAD B-2 SUCCESS]: High-Res NSE Monte Carlo saved. Runtime: {:?}", mc_timer.elapsed());
                                }
                            }
                        }
                    }
                },
                Err(e) => println!("❌ [THREAD B-2 ERROR]: NSE Monte Carlo Grid Pipeline crash: {}", e),
            }
        });

        // ==============================================================================
        // 📊 TRACK C-1: BSE GREENWALD EARNINGS POWER VALUE (EPV) MATRIX
        // ==============================================================================
        scope.spawn(move |_| {
            let epv_timer = Instant::now();
            match epv::engine::execute_rolling_epv_pipeline(ticker_ref, Exchange::Bse, wacc) {
                Ok(matrix_report) => {
                    if !matrix_report.is_empty() {
                        let target_file_path = format!("{}/bse_epv_projections.json", dir_ref);
                        if let Ok(json_str) = serde_json::to_string_pretty(&matrix_report) {
                            if let Ok(mut file) = File::create(&target_file_path) {
                                if file.write_all(json_str.as_bytes()).is_ok() {
                                    println!("💾 [THREAD C-1 SUCCESS]: High-Res BSE EPV Matrix saved. Runtime: {:?}", epv_timer.elapsed());
                                }
                            }
                        }
                    }
                },
                Err(e) => println!("❌ [THREAD C-1 ERROR]: BSE EPV Matrix Pipeline crash: {}", e),
            }
        });

        // ==============================================================================
        // 📊 TRACK C-2: NSE GREENWALD EARNINGS POWER VALUE (EPV) MATRIX
        // ==============================================================================
        scope.spawn(move |_| {
            let epv_timer = Instant::now();
            match epv::engine::execute_rolling_epv_pipeline(ticker_ref, Exchange::Nse, wacc) {
                Ok(matrix_report) => {
                    if !matrix_report.is_empty() {
                        let target_file_path = format!("{}/nse_epv_projections.json", dir_ref);
                        if let Ok(json_str) = serde_json::to_string_pretty(&matrix_report) {
                            if let Ok(mut file) = File::create(&target_file_path) {
                                if file.write_all(json_str.as_bytes()).is_ok() {
                                    println!("💾 [THREAD C-2 SUCCESS]: High-Res NSE EPV Matrix saved. Runtime: {:?}", epv_timer.elapsed());
                                }
                            }
                        }
                    }
                },
                Err(e) => println!("❌ [THREAD C-2 ERROR]: NSE EPV Matrix Pipeline crash: {}", e),
            }
        });

        // Track D: Financial Multiples Analysis
        // ==============================================================================
        // 📊 TRACK D-1: BSE CORPORATE FINANCIAL MULTIPLES TIMELINE
        // ==============================================================================
        scope.spawn(move |_| {
            let multiples_timer = Instant::now();
            match multiples::engine::execute_multiples_analytical_pipeline(ticker_ref, Exchange::Bse) {
                Ok(multiples_report) => {
                    if !multiples_report.is_empty() {
                        let json_out_path = format!("{}/bse_corporate_financial_multiples.json", dir_ref);
                        if let Ok(json_string) = serde_json::to_string_pretty(&multiples_report) {
                            let _ = File::create(&json_out_path).map(|mut f| f.write_all(json_string.as_bytes()));
                            println!("💾 [THREAD D-1 SUCCESS]: High-Coverage BSE Multiples saved. Runtime: {:?}", multiples_timer.elapsed());
                        }
                    }
                },
                Err(err) => println!("❌ [THREAD D-1 ERROR]: BSE Multiples Pipeline Engine crashed: {}", err),
            }
        });

        // ==============================================================================
        // 📊 TRACK D-2: NSE CORPORATE FINANCIAL MULTIPLES TIMELINE
        // ==============================================================================
        scope.spawn(move |_| {
            let multiples_timer = Instant::now();
            match multiples::engine::execute_multiples_analytical_pipeline(ticker_ref, Exchange::Nse) {
                Ok(multiples_report) => {
                    if !multiples_report.is_empty() {
                        let json_out_path = format!("{}/nse_corporate_financial_multiples.json", dir_ref);
                        if let Ok(json_string) = serde_json::to_string_pretty(&multiples_report) {
                            let _ = File::create(&json_out_path).map(|mut f| f.write_all(json_string.as_bytes()));
                            println!("💾 [THREAD D-2 SUCCESS]: High-Coverage NSE Multiples saved. Runtime: {:?}", multiples_timer.elapsed());
                        }
                    }
                },
                Err(err) => println!("❌ [THREAD D-2 ERROR]: NSE Multiples Pipeline Engine crashed: {}", err),
            }
        });
    });

    println!("=======================================================================================");
    println!("✅ TOTAL FIVE-WAY CONCURRENT PIPELINE EXECUTION WINDOW: {:?}", global_timer_start.elapsed());
    println!("=======================================================================================");
    
    Ok(())
}