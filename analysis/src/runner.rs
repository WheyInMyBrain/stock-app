// analysis/src/runner.rs

use std::fs::create_dir_all;
use std::time::Instant;
use std::sync::Arc;
use crate::data_loader::{CentralFinancialsDB, Exchange as LoaderExchange};

// Updated signature to accept the dynamic data_dir string reference from main.rs
pub fn run_global_analysis_pipeline(ticker: &str, wacc: f64, terminal_g: f64, data_dir: &str) {
    // 🎯 FIXED: Construct analysis outputs relative to our assigned data directory path anchor
    let target_dir = format!("{}/{}/analysis", data_dir, ticker);
    if let Err(e) = create_dir_all(&target_dir) {
        println!("❌ [SYSTEM ERROR]: Failed to establish output folder path hierarchy: {}", e);
        return;
    }

    let dir_ref = &target_dir;
    let ticker_ref = ticker;
    
    println!("🏛️  [DATA BROKER]: Pre-fetching unrestricted Parquet tables for [{ticker}]...");
    
    // Forwarded data_dir down to your zero-copy asset matrix data brokers!
    let bse_data_matrix = CentralFinancialsDB::load_exchange_matrix(ticker, LoaderExchange::Bse, data_dir);
    let nse_data_matrix = CentralFinancialsDB::load_exchange_matrix(ticker, LoaderExchange::Nse, data_dir);

    // 🎯 MONITOR LISTING COVERAGE UPFRONT
    match (&bse_data_matrix, &nse_data_matrix) {
        (Some(_), Some(_)) => println!("⚖️  [LISTING DETECTED]: Dual-Exchange Asset. Spawning all 14 analytics tracks..."),
        (Some(_), None)    => println!("📢 [LISTING DETECTED]: Exclusive BSE Listing. Skipping NSE tracks cleanly..."),
        (None, Some(_))    => println!("📢 [LISTING DETECTED]: Exclusive NSE Listing. Skipping BSE tracks cleanly..."),
        (None, None) => {
            println!("❌ [DATA CRISIS]: No Parquet files found for [{ticker}] on either BSE or NSE at location: {data_dir}. Aborting pipeline.");
            return;
        }
    }

    // Wrap the data in read-only thread-safe atomic references
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

        // ==============================================================================
        // 📊 Track B-1: Stochastic Monte Carlo Simulation Matrix for BSE
        // ==============================================================================
        let bse_mc_ref = Arc::clone(&shared_bse);
        scope.spawn(move |_| {
            let mc_timer = Instant::now();
            if let Some(ref matrix) = *bse_mc_ref {
                if let Ok(matrix_report) = crate::monte_carlo::engine::execute_monte_carlo_matrix_pipeline(matrix, wacc, terminal_g) {
                    crate::helper::dump_matrix_report_to_disk(
                        &matrix_report,
                        &format!("{}/bse_monte_carlo_distributions.json", dir_ref),
                        "THREAD B-1",
                        mc_timer,
                    );
                }
            }
        });

        // ==============================================================================
        // 📊 Track B-2: Stochastic Monte Carlo Simulation Matrix for NSE
        // ==============================================================================
        let nse_mc_ref = Arc::clone(&shared_nse);
        scope.spawn(move |_| {
            let mc_timer = Instant::now();
            if let Some(ref matrix) = *nse_mc_ref {
                if let Ok(matrix_report) = crate::monte_carlo::engine::execute_monte_carlo_matrix_pipeline(matrix, wacc, terminal_g) {
                    crate::helper::dump_matrix_report_to_disk(
                        &matrix_report,
                        &format!("{}/nse_monte_carlo_distributions.json", dir_ref),
                        "THREAD B-2",
                        mc_timer,
                    );
                }
            }
        });

        // ==============================================================================
        // 📊 Track C-1: High-Resolution Earnings Power Value (EPV) Matrix for BSE
        // ==============================================================================
        let bse_epv_ref = Arc::clone(&shared_bse);
        scope.spawn(move |_| {
            let epv_timer = Instant::now();
            if let Some(ref matrix) = *bse_epv_ref {
                if let Ok(matrix_report) = crate::epv::engine::execute_rolling_epv_pipeline(matrix, wacc) {
                    crate::helper::dump_matrix_report_to_disk(
                        &matrix_report,
                        &format!("{}/bse_epv_projections.json", dir_ref),
                        "THREAD C-1",
                        epv_timer,
                    );
                }
            }
        });

        // ==============================================================================
        // 📊 Track C-2: High-Resolution Earnings Power Value (EPV) Matrix for NSE
        // ==============================================================================
        let nse_epv_ref = Arc::clone(&shared_nse);
        scope.spawn(move |_| {
            let epv_timer = Instant::now();
            if let Some(ref matrix) = *nse_epv_ref {
                if let Ok(matrix_report) = crate::epv::engine::execute_rolling_epv_pipeline(matrix, wacc) {
                    crate::helper::dump_matrix_report_to_disk(
                        &matrix_report,
                        &format!("{}/nse_epv_projections.json", dir_ref),
                        "THREAD C-2",
                        epv_timer,
                    );
                }
            }
        });

        // ==============================================================================
        // 📊 TRACK D-1: BSE CORPORATE FINANCIAL MULTIPLES TIMELINE
        // ==============================================================================
        let bse_mult_ref = Arc::clone(&shared_bse);
        scope.spawn(move |_| {
            let multiples_timer = Instant::now();
            if let Some(ref matrix) = *bse_mult_ref {
                // Modified parameters reflect our newly optimized memory cache design
                if let Ok(matrix_report) = crate::multiples::engine::execute_multiples_analytical_pipeline(matrix, ticker_ref, "bse") {
                    crate::helper::dump_matrix_report_to_disk(
                        &matrix_report,
                        &format!("{}/bse_corporate_financial_multiples.json", dir_ref),
                        "THREAD D-1",
                        multiples_timer,
                    );
                }
            }
        });

        // ==============================================================================
        // 📊 TRACK D-2: NSE CORPORATE FINANCIAL MULTIPLES TIMELINE
        // ==============================================================================
        let nse_mult_ref = Arc::clone(&shared_nse);
        scope.spawn(move |_| {
            let multiples_timer = Instant::now();
            if let Some(ref matrix) = *nse_mult_ref {
                // Modified parameters reflect our newly optimized memory cache design
                if let Ok(matrix_report) = crate::multiples::engine::execute_multiples_analytical_pipeline(matrix, ticker_ref, "nse") {
                    crate::helper::dump_matrix_report_to_disk(
                        &matrix_report,
                        &format!("{}/nse_corporate_financial_multiples.json", dir_ref),
                        "THREAD D-2",
                        multiples_timer,
                    );
                }
            }
        });

        // ==============================================================================
        // 📊 TRACK E-1: BSE MERTON-BATES JUMP-DIFFUSION MATRIX
        // ==============================================================================
        let bse_mb_ref = Arc::clone(&shared_bse);
        scope.spawn(move |_| {
            let merton_bates_timer = Instant::now();
            if let Some(ref matrix) = *bse_mb_ref {
                // 🎯 FIXED: Takes ONLY the matrix reference pointer now!
                let merton_bates_report = crate::merton_bates::engine::execute_merton_bates_pipeline(matrix);
                crate::helper::dump_matrix_report_to_disk(
                    &merton_bates_report,
                    &format!("{}/bse_merton_bates_credit_risk.json", dir_ref),
                    "THREAD E-1",
                    merton_bates_timer,
                );
            }
        });

        // ==============================================================================
        // 📊 TRACK E-2: NSE MERTON-BATES JUMP-DIFFUSION MATRIX
        // ==============================================================================
        let nse_mb_ref = Arc::clone(&shared_nse);
        scope.spawn(move |_| {
            let merton_bates_timer = Instant::now();
            if let Some(ref matrix) = *nse_mb_ref {
                // 🎯 FIXED: Takes ONLY the matrix reference pointer now!
                let merton_bates_report = crate::merton_bates::engine::execute_merton_bates_pipeline(matrix);
                crate::helper::dump_matrix_report_to_disk(
                    &merton_bates_report,
                    &format!("{}/nse_merton_bates_credit_risk.json", dir_ref),
                    "THREAD E-2",
                    merton_bates_timer,
                );
            }
        });

        // ==============================================================================
        // 📊 TRACK F-1: BSE MARKOV SWITCHING STATE MATRIX REGIME
        // ==============================================================================
        let bse_mr_ref = Arc::clone(&shared_bse);
        scope.spawn(move |_| {
            let regime_timer = Instant::now();
            if let Some(ref matrix) = *bse_mr_ref {
                let markov_report = crate::markov_regime::engine::execute_markov_regime_pipeline(matrix, ticker_ref);
                crate::helper::dump_matrix_report_to_disk(
                    &markov_report,
                    &format!("{}/bse_markov_regime_transitions.json", dir_ref),
                    "THREAD F-1",
                    regime_timer,
                );
            }
        });

        // ==============================================================================
        // 📊 TRACK F-2: NSE MARKOV SWITCHING STATE MATRIX REGIME
        // ==============================================================================
        let nse_mr_ref = Arc::clone(&shared_nse);
        scope.spawn(move |_| {
            let regime_timer = Instant::now();
            if let Some(ref matrix) = *nse_mr_ref {
                let markov_report = crate::markov_regime::engine::execute_markov_regime_pipeline(matrix, ticker_ref);
                crate::helper::dump_matrix_report_to_disk(
                    &markov_report,
                    &format!("{}/nse_markov_regime_transitions.json", dir_ref),
                    "THREAD F-2",
                    regime_timer,
                );
            }
        });

        // ==============================================================================
        // 📊 TRACK G-1: BSE MERTON-KMV DISTANCE-TO-DEFAULT CREDIT MATRIX
        // ==============================================================================
        let bse_kmv_ref = Arc::clone(&shared_bse);
        scope.spawn(move |_| {
            let kmv_timer = Instant::now();
            if let Some(ref matrix) = *bse_kmv_ref {
                // 🎯 FIXED: Dropped trailing string argument to match your new 2-param layout signature
                let kmv_report = crate::merton_kmv::engine::execute_merton_kmv_pipeline(matrix, ticker_ref);
                crate::helper::dump_matrix_report_to_disk(
                    &kmv_report,
                    &format!("{}/bse_merton_kmv_default_risk.json", dir_ref),
                    "THREAD G-1",
                    kmv_timer,
                );
            }
        });

        // ==============================================================================
        // 📊 TRACK G-2: NSE MERTON-KMV DISTANCE-TO-DEFAULT CREDIT MATRIX
        // ==============================================================================
        let nse_kmv_ref = Arc::clone(&shared_nse);
        scope.spawn(move |_| {
            let kmv_timer = Instant::now();
            if let Some(ref matrix) = *nse_kmv_ref {
                // 🎯 FIXED: Dropped trailing string argument to match your new 2-param layout signature
                let kmv_report = crate::merton_kmv::engine::execute_merton_kmv_pipeline(matrix, ticker_ref);
                crate::helper::dump_matrix_report_to_disk(
                    &kmv_report,
                    &format!("{}/nse_merton_kmv_default_risk.json", dir_ref),
                    "THREAD G-2",
                    kmv_timer,
                );
            }
        });

    });
}