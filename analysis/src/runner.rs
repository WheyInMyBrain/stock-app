// analysis/src/runner.rs
use std::fs::create_dir_all;
use std::time::Instant;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use crate::data_loader::{CentralFinancialsDB, Exchange as LoaderExchange};

// ==============================================================================
// 🎯 GLOBAL ATOMIC TRACKERS
// ==============================================================================
pub static TOTAL_TASKS: AtomicU32 = AtomicU32::new(0);
pub static COMPLETED_TASKS: AtomicU32 = AtomicU32::new(0);

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
        println!("\x1b[33m[ANALYSIS] ❌ [SYSTEM ERROR]: Failed to establish output folder path hierarchy: {}\x1b[0m", e);
        return;
    }

    let dir_ref = &target_dir;
    let ticker_ref = ticker;
    
    println!("\x1b[33m[ANALYSIS] 🏛️  [DATA BROKER]: Pre-fetching unrestricted Parquet tables for [{ticker}]...\x1b[0m");
    
    let bse_data_matrix = CentralFinancialsDB::load_exchange_matrix(ticker, LoaderExchange::Bse, data_dir);
    let nse_data_matrix = CentralFinancialsDB::load_exchange_matrix(ticker, LoaderExchange::Nse, data_dir);

    match (&bse_data_matrix, &nse_data_matrix) {
        (Some(_), Some(_)) => println!("\x1b[33m[ANALYSIS] ⚖️  [LISTING DETECTED]: Dual-Exchange Asset. Filtering thread scope targets...\x1b[0m"),
        (Some(_), None)    => println!("\x1b[33m[ANALYSIS] 📢 [LISTING DETECTED]: Exclusive BSE Listing. Skipping NSE tracks cleanly...\x1b[0m"),
        (None, Some(_))    => println!("\x1b[33m[ANALYSIS] 📢 [LISTING DETECTED]: Exclusive NSE Listing. Skipping BSE tracks cleanly...\x1b[0m"),
        (None, None) => {
            println!("\x1b[33m[ANALYSIS] ❌ [DATA CRISIS]: No Parquet files found for [{ticker}] at location: {data_dir}. Aborting pipeline.\x1b[0m");
            TOTAL_TASKS.store(0, Ordering::SeqCst);
            COMPLETED_TASKS.store(0, Ordering::SeqCst);
            return;
        }
    }

    let has_bse = bse_data_matrix.is_some();
    let has_nse = nse_data_matrix.is_some();

    // 🎯 FIXED: Declared the accumulator variable here before the budget scanning loop
    let mut calculated_total = 0;
    let available_modules = ["dcf", "monte_carlo", "epv", "multiples", "merton_bates", "markov", "merton_kmv"];
    for m in &available_modules {
        if should_run(m, modules_selector) {
            if has_bse { calculated_total += 1; }
            if has_nse { calculated_total += 1; }
        }
    }

    TOTAL_TASKS.store(calculated_total, Ordering::SeqCst);
    COMPLETED_TASKS.store(0, Ordering::SeqCst);

    let shared_bse = Arc::new(bse_data_matrix);
    let shared_nse = Arc::new(nse_data_matrix);

    rayon::scope(|scope| {
        // ==============================================================================
        // 📊 Track A-1: High-Resolution Deterministic Matrix Grid for BSE
        // ==============================================================================
        if should_run("dcf", modules_selector) && has_bse {
            let bse_ref = Arc::clone(&shared_bse);
            scope.spawn(move |_| {
                let dcf_timer = Instant::now();
                if let Some(ref matrix) = *bse_ref {
                    if let Ok(matrix_report) = crate::dcf::engine::execute_dual_dcf_pipeline(matrix, wacc, terminal_g) {
                        crate::helper::dump_matrix_report_to_disk(&matrix_report, &format!("{}/bse_dcf_projections.json", dir_ref), "THREAD A-1 [DCF]", dcf_timer);
                    }
                }
                COMPLETED_TASKS.fetch_add(1, Ordering::SeqCst);
            });
        }

        // ==============================================================================
        // 📊 Track A-2: High-Resolution Deterministic Matrix Grid for NSE
        // ==============================================================================
        if should_run("dcf", modules_selector) && has_nse {
            let nse_ref = Arc::clone(&shared_nse);
            scope.spawn(move |_| {
                let dcf_timer = Instant::now();
                if let Some(ref matrix) = *nse_ref {
                    if let Ok(matrix_report) = crate::dcf::engine::execute_dual_dcf_pipeline(matrix, wacc, terminal_g) {
                        crate::helper::dump_matrix_report_to_disk(&matrix_report, &format!("{}/nse_dcf_projections.json", dir_ref), "THREAD A-2 [DCF]", dcf_timer);
                    }
                }
                COMPLETED_TASKS.fetch_add(1, Ordering::SeqCst);
            });
        }

        // ==============================================================================
        // 📊 Track B-1: Stochastic Monte Carlo Simulation Matrix for BSE
        // ==============================================================================
        if should_run("monte_carlo", modules_selector) && has_bse {
            let bse_mc_ref = Arc::clone(&shared_bse);
            scope.spawn(move |_| {
                let mc_timer = Instant::now();
                if let Some(ref matrix) = *bse_mc_ref {
                    if let Ok(matrix_report) = crate::monte_carlo::engine::execute_monte_carlo_matrix_pipeline(matrix, wacc, terminal_g) {
                        crate::helper::dump_matrix_report_to_disk(&matrix_report, &format!("{}/bse_monte_carlo_distributions.json", dir_ref), "THREAD B-1 [MC]", mc_timer);
                    }
                }
                COMPLETED_TASKS.fetch_add(1, Ordering::SeqCst);
            });
        }

        // ==============================================================================
        // 📊 Track B-2: Stochastic Monte Carlo Simulation Matrix for NSE
        // ==============================================================================
        if should_run("monte_carlo", modules_selector) && has_nse {
            let nse_mc_ref = Arc::clone(&shared_nse);
            scope.spawn(move |_| {
                let mc_timer = Instant::now();
                if let Some(ref matrix) = *nse_mc_ref {
                    if let Ok(matrix_report) = crate::monte_carlo::engine::execute_monte_carlo_matrix_pipeline(matrix, wacc, terminal_g) {
                        crate::helper::dump_matrix_report_to_disk(&matrix_report, &format!("{}/nse_monte_carlo_distributions.json", dir_ref), "THREAD B-2 [MC]", mc_timer);
                    }
                }
                COMPLETED_TASKS.fetch_add(1, Ordering::SeqCst);
            });
        }

        // ==============================================================================
        // 📊 Track C-1: High-Resolution Earnings Power Value (EPV) Matrix for BSE
        // ==============================================================================
        if should_run("epv", modules_selector) && has_bse {
            let bse_epv_ref = Arc::clone(&shared_bse);
            scope.spawn(move |_| {
                let epv_timer = Instant::now();
                if let Some(ref matrix) = *bse_epv_ref {
                    if let Ok(matrix_report) = crate::epv::engine::execute_rolling_epv_pipeline(matrix, wacc) {
                        crate::helper::dump_matrix_report_to_disk(&matrix_report, &format!("{}/bse_epv_projections.json", dir_ref), "THREAD C-1 [EPV]", epv_timer);
                    }
                }
                COMPLETED_TASKS.fetch_add(1, Ordering::SeqCst);
            });
        }

        // ==============================================================================
        // 📊 Track C-2: High-Resolution Earnings Power Value (EPV) Matrix for NSE
        // ==============================================================================
        if should_run("epv", modules_selector) && has_nse {
            let nse_epv_ref = Arc::clone(&shared_nse);
            scope.spawn(move |_| {
                let epv_timer = Instant::now();
                if let Some(ref matrix) = *nse_epv_ref {
                    if let Ok(matrix_report) = crate::epv::engine::execute_rolling_epv_pipeline(matrix, wacc) {
                        crate::helper::dump_matrix_report_to_disk(&matrix_report, &format!("{}/nse_epv_projections.json", dir_ref), "THREAD C-2 [EPV]", epv_timer);
                    }
                }
                COMPLETED_TASKS.fetch_add(1, Ordering::SeqCst);
            });
        }

        // ==============================================================================
        // 📊 TRACK D-1: BSE CORPORATE FINANCIAL MULTIPLES TIMELINE
        // ==============================================================================
        if should_run("multiples", modules_selector) && has_bse {
            let bse_mult_ref = Arc::clone(&shared_bse);
            scope.spawn(move |_| {
                let multiples_timer = Instant::now();
                if let Some(ref matrix) = *bse_mult_ref {
                    if let Ok(matrix_report) = crate::multiples::engine::execute_multiples_analytical_pipeline(matrix, ticker_ref, "bse") {
                        crate::helper::dump_matrix_report_to_disk(&matrix_report, &format!("{}/bse_corporate_financial_multiples.json", dir_ref), "THREAD D-1 [MULT]", multiples_timer);
                    }
                }
                COMPLETED_TASKS.fetch_add(1, Ordering::SeqCst);
            });
        }

        // ==============================================================================
        // 📊 TRACK D-2: NSE CORPORATE FINANCIAL MULTIPLES TIMELINE
        // ==============================================================================
        if should_run("multiples", modules_selector) && has_nse {
            let nse_mult_ref = Arc::clone(&shared_nse);
            scope.spawn(move |_| {
                let multiples_timer = Instant::now();
                if let Some(ref matrix) = *nse_mult_ref {
                    if let Ok(matrix_report) = crate::multiples::engine::execute_multiples_analytical_pipeline(matrix, ticker_ref, "nse") {
                        crate::helper::dump_matrix_report_to_disk(&matrix_report, &format!("{}/nse_corporate_financial_multiples.json", dir_ref), "THREAD D-2 [MULT]", multiples_timer);
                    }
                }
                COMPLETED_TASKS.fetch_add(1, Ordering::SeqCst);
            });
        }

        // ==============================================================================
        // 📊 TRACK E-1: BSE MERTON-BATES JUMP-DIFFUSION MATRIX
        // ==============================================================================
        if should_run("merton_bates", modules_selector) && has_bse {
            let bse_mb_ref = Arc::clone(&shared_bse);
            scope.spawn(move |_| {
                let merton_bates_timer = Instant::now();
                if let Some(ref matrix) = *bse_mb_ref {
                    let merton_bates_report = crate::merton_bates::engine::execute_merton_bates_pipeline(matrix);
                    crate::helper::dump_matrix_report_to_disk(&merton_bates_report, &format!("{}/bse_merton_bates_credit_risk.json", dir_ref), "THREAD E-1 [MB]", merton_bates_timer);
                }
                COMPLETED_TASKS.fetch_add(1, Ordering::SeqCst);
            });
        }

        // ==============================================================================
        // 📊 TRACK E-2: NSE MERTON-BATES JUMP-DIFFUSION MATRIX
        // ==============================================================================
        if should_run("merton_bates", modules_selector) && has_nse {
            let nse_mb_ref = Arc::clone(&shared_nse);
            scope.spawn(move |_| {
                let merton_bates_timer = Instant::now();
                if let Some(ref matrix) = *nse_mb_ref {
                    let merton_bates_report = crate::merton_bates::engine::execute_merton_bates_pipeline(matrix);
                    crate::helper::dump_matrix_report_to_disk(&merton_bates_report, &format!("{}/nse_merton_bates_credit_risk.json", dir_ref), "THREAD E-2 [MB]", merton_bates_timer);
                }
                COMPLETED_TASKS.fetch_add(1, Ordering::SeqCst);
            });
        }

        // ==============================================================================
        // 📊 TRACK F-1: BSE MARKOV SWITCHING STATE MATRIX REGIME
        // ==============================================================================
        if should_run("markov", modules_selector) && has_bse {
            let bse_mr_ref = Arc::clone(&shared_bse);
            scope.spawn(move |_| {
                let regime_timer = Instant::now();
                if let Some(ref matrix) = *bse_mr_ref {
                    let markov_report = crate::markov_regime::engine::execute_markov_regime_pipeline(matrix, ticker_ref);
                    crate::helper::dump_matrix_report_to_disk(&markov_report, &format!("{}/bse_markov_regime_transitions.json", dir_ref), "THREAD F-1 [MARKOV]", regime_timer);
                }
                COMPLETED_TASKS.fetch_add(1, Ordering::SeqCst);
            });
        }

        // ==============================================================================
        // 📊 TRACK F-2: NSE MARKOV SWITCHING STATE MATRIX REGIME
        // ==============================================================================
        if should_run("markov", modules_selector) && has_nse {
            let nse_mr_ref = Arc::clone(&shared_nse);
            scope.spawn(move |_| {
                let regime_timer = Instant::now();
                if let Some(ref matrix) = *nse_mr_ref {
                    let markov_report = crate::markov_regime::engine::execute_markov_regime_pipeline(matrix, ticker_ref);
                    crate::helper::dump_matrix_report_to_disk(&markov_report, &format!("{}/nse_markov_regime_transitions.json", dir_ref), "THREAD F-2 [MARKOV]", regime_timer);
                }
                COMPLETED_TASKS.fetch_add(1, Ordering::SeqCst);
            });
        }

        // ==============================================================================
        // 📊 TRACK G-1: BSE MERTON-KMV DISTANCE-TO-DEFAULT CREDIT MATRIX
        // ==============================================================================
        if should_run("merton_kmv", modules_selector) && has_bse {
            let bse_kmv_ref = Arc::clone(&shared_bse);
            scope.spawn(move |_| {
                let kmv_timer = Instant::now();
                if let Some(ref matrix) = *bse_kmv_ref {
                    let kmv_report = crate::merton_kmv::engine::execute_merton_kmv_pipeline(matrix, ticker_ref);
                    crate::helper::dump_matrix_report_to_disk(&kmv_report, &format!("{}/bse_merton_kmv_default_risk.json", dir_ref), "THREAD G-1 [KMV]", kmv_timer);
                }
                COMPLETED_TASKS.fetch_add(1, Ordering::SeqCst);
            });
        }

        // ==============================================================================
        // 📊 TRACK G-2: NSE MERTON-KMV DISTANCE-TO-DEFAULT CREDIT MATRIX
        // ==============================================================================
        if should_run("merton_kmv", modules_selector) && has_nse {
            let nse_kmv_ref = Arc::clone(&shared_nse);
            scope.spawn(move |_| {
                let kmv_timer = Instant::now();
                if let Some(ref matrix) = *nse_kmv_ref {
                    let kmv_report = crate::merton_kmv::engine::execute_merton_kmv_pipeline(matrix, ticker_ref);
                    crate::helper::dump_matrix_report_to_disk(&kmv_report, &format!("{}/nse_merton_kmv_default_risk.json", dir_ref), "THREAD G-2 [KMV]", kmv_timer);
                }
                COMPLETED_TASKS.fetch_add(1, Ordering::SeqCst);
            });
        }
    });
}