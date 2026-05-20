// analysis/src/runner.rs

use polars::prelude::PolarsResult;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::time::Instant;
use std::sync::Arc;
use crate::data_loader::{CentralFinancialsDB, Exchange as LoaderExchange};

use crate::dcf::Exchange;
use crate::monte_carlo;
use crate::epv;
use crate::multiples;
use crate::merton_bates::engine::execute_merton_bates_pipeline;

pub fn run_global_analysis_pipeline(ticker: &str, wacc: f64, terminal_g: f64) {
    // 🎯 Establish output folder path hierarchy natively inside the runner logic
    let target_dir = format!("../data/{}/analysis", ticker);
    if let Err(e) = create_dir_all(&target_dir) {
        println!("❌ [SYSTEM ERROR]: Failed to establish output folder path hierarchy: {}", e);
        return;
    }

    let dir_ref = &target_dir;
    let _ticker_ref = ticker; // Clears the unused variable compiler warning
    
    println!("🏛️  [DATA BROKER]: Pre-fetching unrestricted Parquet tables for [{}]...", ticker);
    
    // Ingest the raw parquet data blocks exactly once on the main execution thread
    let bse_data_matrix = CentralFinancialsDB::load_exchange_matrix(ticker, LoaderExchange::Bse);
    let nse_data_matrix = CentralFinancialsDB::load_exchange_matrix(ticker, LoaderExchange::Nse);

    // Wrap the structured data blocks in read-only thread-safe atomic references
    let shared_bse = Arc::new(bse_data_matrix);
    let shared_nse = Arc::new(nse_data_matrix);

    rayon::scope(|scope| {

        // ==============================================================================
        // 📊 Track A-1: High-Resolution Deterministic Matrix Grid for BSE
        // ==============================================================================
        let bse_ref = Arc::clone(&shared_bse);
        scope.spawn(move |_| {
            let dcf_timer = Instant::now();
            if let Some(ref matrix) = *bse_ref {
                if let Ok(matrix_report) = crate::dcf::engine::execute_dual_dcf_pipeline(matrix, wacc, terminal_g) {
                    crate::helper::dump_matrix_report_to_disk(
                        &matrix_report,
                        &format!("{}/bse_dcf_projections.json", dir_ref),
                        "THREAD A-1",
                        dcf_timer,
                    );
                }
            }
        });

        // ==============================================================================
        // 📊 Track A-2: High-Resolution Deterministic Matrix Grid for NSE
        // ==============================================================================
        let nse_ref = Arc::clone(&shared_nse);
        scope.spawn(move |_| {
            let dcf_timer = Instant::now();
            if let Some(ref matrix) = *nse_ref {
                if let Ok(matrix_report) = crate::dcf::engine::execute_dual_dcf_pipeline(matrix, wacc, terminal_g) {
                    crate::helper::dump_matrix_report_to_disk(
                        &matrix_report,
                        &format!("{}/nse_dcf_projections.json", dir_ref),
                        "THREAD A-2",
                        dcf_timer,
                    );
                }
            }
        });

    });
}

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

        // ==============================================================================
        // 📊 TRACK E-1: BSE MERTON-BATES JUMP-DIFFUSION MATRIX
        // ==============================================================================
        scope.spawn(move |_| {
            let merton_bates_timer = Instant::now();
            let local_mb_exchange = crate::merton_bates::engine::Exchange::Bse;
            
            let merton_bates_report = execute_merton_bates_pipeline(ticker_ref, local_mb_exchange);
            
            if !merton_bates_report.is_empty() {
                let json_out_path = format!("{}/bse_merton_bates_credit_risk.json", dir_ref);
                if let Ok(json_string) = serde_json::to_string_pretty(&merton_bates_report) {
                    let _ = File::create(&json_out_path).map(|mut f| f.write_all(json_string.as_bytes()));
                    println!("💾 [THREAD E-1 SUCCESS]: High-Coverage BSE Merton-Bates Credit Risk saved. Runtime: {:?}", merton_bates_timer.elapsed());
                }
            } else {
                println!("⚠️  [THREAD E-1 BYPASS]: Empty dataset or 10Y chart trace missing for [{}] on BSE.", ticker_ref);
            }
        });

        // ==============================================================================
        // 📊 TRACK E-2: NSE MERTON-BATES JUMP-DIFFUSION MATRIX
        // ==============================================================================
        scope.spawn(move |_| {
            let merton_bates_timer = Instant::now();
            let local_mb_exchange = crate::merton_bates::engine::Exchange::Nse;
            
            let merton_bates_report = execute_merton_bates_pipeline(ticker_ref, local_mb_exchange);
            
            if !merton_bates_report.is_empty() {
                let json_out_path = format!("{}/nse_merton_bates_credit_risk.json", dir_ref);
                if let Ok(json_string) = serde_json::to_string_pretty(&merton_bates_report) {
                    let _ = File::create(&json_out_path).map(|mut f| f.write_all(json_string.as_bytes()));
                    println!("💾 [THREAD E-2 SUCCESS]: High-Coverage NSE Merton-Bates Credit Risk saved. Runtime: {:?}", merton_bates_timer.elapsed());
                }
            } else {
                println!("⚠️  [THREAD E-2 BYPASS]: Empty dataset or 10Y chart trace missing for [{}] on NSE.", ticker_ref);
            }
        });

        // ==============================================================================
        // 📊 TRACK F-1: BSE MARKOV SWITCHING STATE MATRIX REGIME
        // ==============================================================================
        scope.spawn(move |_| {
            let regime_timer = Instant::now();
            let local_mr_exchange = crate::markov_regime::engine::Exchange::Bse;

            let markov_report = crate::markov_regime::engine::execute_markov_regime_pipeline(ticker_ref, local_mr_exchange);

            if !markov_report.is_empty() {
                let json_out_path = format!("{}/bse_markov_regime_transitions.json", dir_ref);
                if let Ok(json_string) = serde_json::to_string_pretty(&markov_report) {
                    let _ = File::create(&json_out_path).map(|mut f| f.write_all(json_string.as_bytes()));
                    println!("💾 [THREAD F-1 SUCCESS]: High-Coverage BSE Markov Transition Grid saved. Runtime: {:?}", regime_timer.elapsed());
                }
            }
        });

        // ==============================================================================
        // 📊 TRACK F-2: NSE MARKOV SWITCHING STATE MATRIX REGIME
        // ==============================================================================
        scope.spawn(move |_| {
            let regime_timer = Instant::now();
            let local_mr_exchange = crate::markov_regime::engine::Exchange::Nse;

            let markov_report = crate::markov_regime::engine::execute_markov_regime_pipeline(ticker_ref, local_mr_exchange);

            if !markov_report.is_empty() {
                // 🎯 FIXED: Old duplicate json_out_path assignment line completely removed
                let json_out_path = format!("{}/nse_markov_regime_transitions.json", dir_ref);
                if let Ok(json_string) = serde_json::to_string_pretty(&markov_report) {
                    let _ = File::create(&json_out_path).map(|mut f| f.write_all(json_string.as_bytes()));
                    println!("💾 [THREAD F-2 SUCCESS]: High-Coverage NSE Markov Transition Grid saved. Runtime: {:?}", regime_timer.elapsed());
                }
            }
        });

    });

    println!("=======================================================================================");
    println!("✅ TOTAL FIVE-WAY CONCURRENT PIPELINE EXECUTION WINDOW: {:?}", global_timer_start.elapsed());
    println!("=======================================================================================");
    
    Ok(())
}