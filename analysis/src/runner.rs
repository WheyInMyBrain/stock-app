use std::fs::create_dir_all;
use std::time::Instant;
use std::sync::Arc;
use crate::data_loader::{CentralFinancialsDB, Exchange as LoaderExchange};

/// 🎯 SELECTIVE EXTENSION FILTER
/// Evaluates if a given track matches the requested operational pipeline selection flags.
fn should_run(module_name: &str, selector: &str) -> bool {
    let clean_selector = selector.trim().to_lowercase();
    if clean_selector.is_empty() || clean_selector == "all" {
        return true;
    }
    
    // Split input values by commas and support convenient shorthand syntax overrides
    clean_selector.split(',').map(|s| s.trim()).any(|token| {
        token == module_name || match (module_name, token) {
            ("monte_carlo", "mc") => true,
            ("merton_bates", "mb") => true,
            ("markov", "regime") => true,
            ("merton_kmv", "kmv") => true,
            _ => false,
        }
    })
}

// Updated signature to accept the target modules selector string slice reference
pub fn run_global_analysis_pipeline(ticker: &str, wacc: f64, terminal_g: f64, data_dir: &str, modules_selector: &str) {
    let target_dir = format!("{}/{}/analysis", data_dir, ticker);
    if let Err(e) = create_dir_all(&target_dir) {
        println!("❌ [SYSTEM ERROR]: Failed to establish output folder path hierarchy: {}", e);
        return;
    }

    let dir_ref = &target_dir;
    let ticker_ref = ticker;
    
    println!("🏛️  [DATA BROKER]: Pre-fetching unrestricted Parquet tables for [{ticker}]...");
    
    let bse_data_matrix = CentralFinancialsDB::load_exchange_matrix(ticker, LoaderExchange::Bse, data_dir);
    let nse_data_matrix = CentralFinancialsDB::load_exchange_matrix(ticker, LoaderExchange::Nse, data_dir);

    match (&bse_data_matrix, &nse_data_matrix) {
        (Some(_), Some(_)) => println!("⚖️  [LISTING DETECTED]: Dual-Exchange Asset. Filtering thread scope targets..."),
        (Some(_), None)    => println!("📢 [LISTING DETECTED]: Exclusive BSE Listing. Skipping NSE tracks cleanly..."),
        (None, Some(_))    => println!("📢 [LISTING DETECTED]: Exclusive NSE Listing. Skipping BSE tracks cleanly..."),
        (None, None) => {
            println!("❌ [DATA CRISIS]: No Parquet files found for [{ticker}] at location: {data_dir}. Aborting pipeline.");
            return;
        }
    }

    let shared_bse = Arc::new(bse_data_matrix);
    let shared_nse = Arc::new(nse_data_matrix);

    rayon::scope(|scope| {

        // ==============================================================================
        // 📊 Track A-1: High-Resolution Deterministic Matrix Grid for BSE
        // ==============================================================================
        if should_run("dcf", modules_selector) {
            let bse_ref = Arc::clone(&shared_bse);
            scope.spawn(move |_| {
                let dcf_timer = Instant::now();
                if let Some(ref matrix) = *bse_ref {
                    if let Ok(matrix_report) = crate::dcf::engine::execute_dual_dcf_pipeline(matrix, wacc, terminal_g) {
                        crate::helper::dump_matrix_report_to_disk(
                            &matrix_report,
                            &format!("{}/bse_dcf_projections.json", dir_ref),
                            "THREAD A-1 [DCF]",
                            dcf_timer,
                        );
                    }
                }
            });
        }

        // ==============================================================================
        // 📊 Track A-2: High-Resolution Deterministic Matrix Grid for NSE
        // ==============================================================================
        if should_run("dcf", modules_selector) {
            let nse_ref = Arc::clone(&shared_nse);
            scope.spawn(move |_| {
                let dcf_timer = Instant::now();
                if let Some(ref matrix) = *nse_ref {
                    if let Ok(matrix_report) = crate::dcf::engine::execute_dual_dcf_pipeline(matrix, wacc, terminal_g) {
                        crate::helper::dump_matrix_report_to_disk(
                            &matrix_report,
                            &format!("{}/nse_dcf_projections.json", dir_ref),
                            "THREAD A-2 [DCF]",
                            dcf_timer,
                        );
                    }
                }
            });
        }

        // ==============================================================================
        // 📊 Track B-1: Stochastic Monte Carlo Simulation Matrix for BSE
        // ==============================================================================
        if should_run("monte_carlo", modules_selector) {
            let bse_mc_ref = Arc::clone(&shared_bse);
            scope.spawn(move |_| {
                let mc_timer = Instant::now();
                if let Some(ref matrix) = *bse_mc_ref {
                    if let Ok(matrix_report) = crate::monte_carlo::engine::execute_monte_carlo_matrix_pipeline(matrix, wacc, terminal_g) {
                        crate::helper::dump_matrix_report_to_disk(
                            &matrix_report,
                            &format!("{}/bse_monte_carlo_distributions.json", dir_ref),
                            "THREAD B-1 [MC]",
                            mc_timer,
                        );
                    }
                }
            });
        }

        // ==============================================================================
        // 📊 Track B-2: Stochastic Monte Carlo Simulation Matrix for NSE
        // ==============================================================================
        if should_run("monte_carlo", modules_selector) {
            let nse_mc_ref = Arc::clone(&shared_nse);
            scope.spawn(move |_| {
                let mc_timer = Instant::now();
                if let Some(ref matrix) = *nse_mc_ref {
                    if let Ok(matrix_report) = crate::monte_carlo::engine::execute_monte_carlo_matrix_pipeline(matrix, wacc, terminal_g) {
                        crate::helper::dump_matrix_report_to_disk(
                            &matrix_report,
                            &format!("{}/nse_monte_carlo_distributions.json", dir_ref),
                            "THREAD B-2 [MC]",
                            mc_timer,
                        );
                    }
                }
            });
        }

        // ==============================================================================
        // 📊 Track C-1: High-Resolution Earnings Power Value (EPV) Matrix for BSE
        // ==============================================================================
        if should_run("epv", modules_selector) {
            let bse_epv_ref = Arc::clone(&shared_bse);
            scope.spawn(move |_| {
                let epv_timer = Instant::now();
                if let Some(ref matrix) = *bse_epv_ref {
                    if let Ok(matrix_report) = crate::epv::engine::execute_rolling_epv_pipeline(matrix, wacc) {
                        crate::helper::dump_matrix_report_to_disk(
                            &matrix_report,
                            &format!("{}/bse_epv_projections.json", dir_ref),
                            "THREAD C-1 [EPV]",
                            epv_timer,
                        );
                    }
                }
            });
        }

        // ==============================================================================
        // 📊 Track C-2: High-Resolution Earnings Power Value (EPV) Matrix for NSE
        // ==============================================================================
        if should_run("epv", modules_selector) {
            let nse_epv_ref = Arc::clone(&shared_nse);
            scope.spawn(move |_| {
                let epv_timer = Instant::now();
                if let Some(ref matrix) = *nse_epv_ref {
                    if let Ok(matrix_report) = crate::epv::engine::execute_rolling_epv_pipeline(matrix, wacc) {
                        crate::helper::dump_matrix_report_to_disk(
                            &matrix_report,
                            &format!("{}/nse_epv_projections.json", dir_ref),
                            "THREAD C-2 [EPV]",
                            epv_timer,
                        );
                    }
                }
            });
        }

        // ==============================================================================
        // 📊 TRACK D-1: BSE CORPORATE FINANCIAL MULTIPLES TIMELINE
        // ==============================================================================
        if should_run("multiples", modules_selector) {
            let bse_mult_ref = Arc::clone(&shared_bse);
            scope.spawn(move |_| {
                let multiples_timer = Instant::now();
                if let Some(ref matrix) = *bse_mult_ref {
                    if let Ok(matrix_report) = crate::multiples::engine::execute_multiples_analytical_pipeline(matrix, ticker_ref, "bse") {
                        crate::helper::dump_matrix_report_to_disk(
                            &matrix_report,
                            &format!("{}/bse_corporate_financial_multiples.json", dir_ref),
                            "THREAD D-1 [MULT]",
                            multiples_timer,
                        );
                    }
                }
            });
        }

        // ==============================================================================
        // 📊 TRACK D-2: NSE CORPORATE FINANCIAL MULTIPLES TIMELINE
        // ==============================================================================
        if should_run("multiples", modules_selector) {
            let nse_mult_ref = Arc::clone(&shared_nse);
            scope.spawn(move |_| {
                let multiples_timer = Instant::now();
                if let Some(ref matrix) = *nse_mult_ref {
                    if let Ok(matrix_report) = crate::multiples::engine::execute_multiples_analytical_pipeline(matrix, ticker_ref, "nse") {
                        crate::helper::dump_matrix_report_to_disk(
                            &matrix_report,
                            &format!("{}/nse_corporate_financial_multiples.json", dir_ref),
                            "THREAD D-2 [MULT]",
                            multiples_timer,
                        );
                    }
                }
            });
        }

        // ==============================================================================
        // 📊 TRACK E-1: BSE MERTON-BATES JUMP-DIFFUSION MATRIX
        // ==============================================================================
        if should_run("merton_bates", modules_selector) {
            let bse_mb_ref = Arc::clone(&shared_bse);
            scope.spawn(move |_| {
                let merton_bates_timer = Instant::now();
                if let Some(ref matrix) = *bse_mb_ref {
                    let merton_bates_report = crate::merton_bates::engine::execute_merton_bates_pipeline(matrix);
                    crate::helper::dump_matrix_report_to_disk(
                        &merton_bates_report,
                        &format!("{}/bse_merton_bates_credit_risk.json", dir_ref),
                        "THREAD E-1 [MB]",
                        merton_bates_timer,
                    );
                }
            });
        }

        // ==============================================================================
        // 📊 TRACK E-2: NSE MERTON-BATES JUMP-DIFFUSION MATRIX
        // ==============================================================================
        if should_run("merton_bates", modules_selector) {
            let nse_mb_ref = Arc::clone(&shared_nse);
            scope.spawn(move |_| {
                let merton_bates_timer = Instant::now();
                if let Some(ref matrix) = *nse_mb_ref {
                    let merton_bates_report = crate::merton_bates::engine::execute_merton_bates_pipeline(matrix);
                    crate::helper::dump_matrix_report_to_disk(
                        &merton_bates_report,
                        &format!("{}/nse_merton_bates_credit_risk.json", dir_ref),
                        "THREAD E-2 [MB]",
                        merton_bates_timer,
                    );
                }
            });
        }

        // ==============================================================================
        // 📊 TRACK F-1: BSE MARKOV SWITCHING STATE MATRIX REGIME
        // ==============================================================================
        if should_run("markov", modules_selector) {
            let bse_mr_ref = Arc::clone(&shared_bse);
            scope.spawn(move |_| {
                let regime_timer = Instant::now();
                if let Some(ref matrix) = *bse_mr_ref {
                    let markov_report = crate::markov_regime::engine::execute_markov_regime_pipeline(matrix, ticker_ref);
                    crate::helper::dump_matrix_report_to_disk(
                        &markov_report,
                        &format!("{}/bse_markov_regime_transitions.json", dir_ref),
                        "THREAD F-1 [MARKOV]",
                        regime_timer,
                    );
                }
            });
        }

        // ==============================================================================
        // 📊 TRACK F-2: NSE MARKOV SWITCHING STATE MATRIX REGIME
        // ==============================================================================
        if should_run("markov", modules_selector) {
            let nse_mr_ref = Arc::clone(&shared_nse);
            scope.spawn(move |_| {
                let regime_timer = Instant::now();
                if let Some(ref matrix) = *nse_mr_ref {
                    let markov_report = crate::markov_regime::engine::execute_markov_regime_pipeline(matrix, ticker_ref);
                    crate::helper::dump_matrix_report_to_disk(
                        &markov_report,
                        &format!("{}/nse_markov_regime_transitions.json", dir_ref),
                        "THREAD F-2 [MARKOV]",
                        regime_timer,
                    );
                }
            });
        }

        // ==============================================================================
        // 📊 TRACK G-1: BSE MERTON-KMV DISTANCE-TO-DEFAULT CREDIT MATRIX
        // ==============================================================================
        if should_run("merton_kmv", modules_selector) {
            let bse_kmv_ref = Arc::clone(&shared_bse);
            scope.spawn(move |_| {
                let kmv_timer = Instant::now();
                if let Some(ref matrix) = *bse_kmv_ref {
                    let kmv_report = crate::merton_kmv::engine::execute_merton_kmv_pipeline(matrix, ticker_ref);
                    crate::helper::dump_matrix_report_to_disk(
                        &kmv_report,
                        &format!("{}/bse_merton_kmv_default_risk.json", dir_ref),
                        "THREAD G-1 [KMV]",
                        kmv_timer,
                    );
                }
            });
        }

        // ==============================================================================
        // 📊 TRACK G-2: NSE MERTON-KMV DISTANCE-TO-DEFAULT CREDIT MATRIX
        // ==============================================================================
        if should_run("merton_kmv", modules_selector) {
            let nse_kmv_ref = Arc::clone(&shared_nse);
            scope.spawn(move |_| {
                let kmv_timer = Instant::now();
                if let Some(ref matrix) = *nse_kmv_ref {
                    let kmv_report = crate::merton_kmv::engine::execute_merton_kmv_pipeline(matrix, ticker_ref);
                    crate::helper::dump_matrix_report_to_disk(
                        &kmv_report,
                        &format!("{}/nse_merton_kmv_default_risk.json", dir_ref),
                        "THREAD G-2 [KMV]",
                        kmv_timer,
                    );
                }
            });
        }

    });
}