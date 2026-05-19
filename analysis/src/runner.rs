// analysis/src/runner.rs

use polars::prelude::PolarsResult;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::time::Instant; // Standard high-resolution monotonic timer primitive

use crate::dcf;

pub fn run_all_analysis_pipelines(
    ticker: &str,
    wacc: f64,
    terminal_g: f64,
    growth_mult: f64,
    margin_mult: f64,
) -> PolarsResult<()> {
    // Start tracking our runtime execution horizon immediately
    let timer_start = Instant::now();

    println!("=======================================================================================");
    println!("🚀 INITIALIZING ANALYTICS PIPELINE FOR TICKER: [{}]", ticker);
    println!("⚙️ CONFIGURATION OVERRIDES: WACC={} | TermG={} | GrowthMult={} | MarginMult={}", wacc, terminal_g, growth_mult, margin_mult);
    println!("=======================================================================================");

    // Fire processing layers down into engine execution
    match dcf::engine::execute_dual_dcf_pipeline(ticker, wacc, terminal_g, growth_mult, margin_mult) {
        Ok(report) => {
            // 🎯 FIXED PATH LAYOUT: Saves artifact into stock-app/data/IMFA/analysis
            let target_dir = format!("../data/{}/analysis/", ticker);
            let target_file_path = format!("{}/bse_dcf_projections.json", target_dir);

            if create_dir_all(&target_dir).is_ok() {
                if let Ok(json_str) = serde_json::to_string_pretty(&report) {
                    if let Ok(mut file) = File::create(&target_file_path) {
                        if file.write_all(json_str.as_bytes()).is_ok() {
                            let duration = timer_start.elapsed();
                            println!("💾 SUCCESS: Analytics matrix saved to: {}", target_file_path);
                            println!("⏱️ Total pipeline calculation time: {:?}", duration);
                        }
                    }
                }
            }
        },
        Err(e) => {
            println!("❌ [DCF MODULE ERROR]: Failed to execute metrics derivation sequence: {}", e);
        }
    }

    println!("=======================================================================================");
    Ok(())
}