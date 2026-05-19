// analysis/src/runner.rs

use polars::prelude::PolarsResult;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::time::Instant;

use crate::dcf;
use crate::monte_carlo;
use crate::epv; // Import our brand new EPV module namespace

pub fn run_all_analysis_pipelines(
    ticker: &str,
    wacc: f64,
    terminal_g: f64,
    growth_mult: f64,
    margin_mult: f64,
) -> PolarsResult<()> {
    let global_timer_start = Instant::now();

    println!("=======================================================================================");
    println!("🚀 INITIALIZING PARALLEL ANALYTICS PIPELINE FOR TICKER: [{}]", ticker);
    println!("⚙️  CONFIGURATION OVERRIDES: WACC={} | TermG={} | GrowthMult={} | MarginMult={}", wacc, terminal_g, growth_mult, margin_mult);
    println!("=======================================================================================");

    let target_dir = format!("../data/{}/analysis/", ticker);
    if let Err(e) = create_dir_all(&target_dir) {
        println!("❌ [SYSTEM ERROR]: Failed to establish output folder path hierarchy: {}", e);
        return Ok(());
    }

    let dir_ref = &target_dir;
    let ticker_ref = ticker;

    // 🎯 THREE-WAY WORK-STEALING RAYON CONCURRENCY POOL
    rayon::scope(|scope| {
        // Track A: Deterministic Growth DCF
        scope.spawn(move |_| {
            let dcf_timer = Instant::now();
            match dcf::engine::execute_dual_dcf_pipeline(ticker_ref, wacc, terminal_g, growth_mult, margin_mult) {
                Ok(report) => {
                    let target_file_path = format!("{}/bse_dcf_projections.json", dir_ref);
                    if let Ok(json_str) = serde_json::to_string_pretty(&report) {
                        if let Ok(mut file) = File::create(&target_file_path) {
                            if file.write_all(json_str.as_bytes()).is_ok() {
                                println!("💾 [THREAD A SUCCESS]: Deterministic DCF saved. Runtime: {:?}", dcf_timer.elapsed());
                            }
                        }
                    }
                },
                Err(e) => println!("❌ [THREAD A ERROR]: DCF Pipeline crash: {}", e),
            }
        });

        // Track B: Stochastic Volatility Monte Carlo (10,000 Trials)
        scope.spawn(move |_| {
            let mc_timer = Instant::now();
            match monte_carlo::engine::execute_monte_carlo_simulation(ticker_ref, wacc, terminal_g) {
                Ok(report) => {
                    let target_file_path = format!("{}/bse_monte_carlo_projections.json", dir_ref);
                    if let Ok(json_str) = serde_json::to_string_pretty(&report) {
                        if let Ok(mut file) = File::create(&target_file_path) {
                            if file.write_all(json_str.as_bytes()).is_ok() {
                                println!("💾 [THREAD B SUCCESS]: Stochastic Monte Carlo saved. Runtime: {:?}", mc_timer.elapsed());
                            }
                        }
                    }
                },
                Err(e) => println!("❌ [THREAD B ERROR]: Monte Carlo crash: {}", e),
            }
        });

        // Track C: Greenwald Earnings Power Value (EPV Timeline Trend)
        scope.spawn(move |_| {
            let epv_timer = Instant::now();
            match epv::engine::execute_rolling_epv_pipeline(ticker_ref, wacc) {
                Ok(report) => {
                    let target_file_path = format!("{}/bse_epv_projections.json", dir_ref);
                    if let Ok(json_str) = serde_json::to_string_pretty(&report) {
                        if let Ok(mut file) = File::create(&target_file_path) {
                            if file.write_all(json_str.as_bytes()).is_ok() {
                                println!("💾 [THREAD C SUCCESS]: Rolling EPV metrics saved. Runtime: {:?}", epv_timer.elapsed());
                            }
                        }
                    }
                },
                Err(e) => println!("❌ [THREAD C ERROR]: EPV Pipeline crash: {}", e),
            }
        });
    });

    println!("=======================================================================================");
    println!("✅ TOTAL THREE-WAY CONCURRENT PIPELINE EXECUTION WINDOW: {:?}", global_timer_start.elapsed());
    println!("=======================================================================================");
    
    Ok(())
}