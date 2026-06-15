use polars::prelude::PolarsResult;
use std::env;

mod data_loader;
mod dcf;
mod helper;
mod runner;
mod monte_carlo;
mod epv;
mod multiples;
mod merton_bates;
mod markov_regime;
mod merton_kmv;

fn main() -> PolarsResult<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("❌ ERROR: Missing target ticker token.");
        println!("\n👉 Usage: cargo run --release <TICKER> [key=value] [key=value]...");
        println!("💡 Available Keys:");
        println!("   ├── wacc=<number>         (Default: 0.12)");
        println!("   ├── term_g=<number>       (Default: 0.04)");
        println!("   ├── data-dir=<path>       (Default: ../data)");
        println!("   └── run=<modules>         (Default: all, e.g., multiples,dcf,mc)");
        println!("\n👉 Example: cargo run --release IMFA wacc=0.13 run=multiples,dcf data-dir=/absolute/path/to/data");
        return Ok(());
    }

    // The first argument after the binary name is always our target ticker
    let ticker = &args[1].to_uppercase();

    // Core fallback defaults
    let mut wacc: f64 = 0.12;
    let mut terminal_g: f64 = 0.04;
    let mut data_dir: String = "../data".to_string(); 
    let mut modules_selector: String = "all".to_string(); // 🎯 NEW: Dynamic selective execution track default

    // Iterate through key-value overrides passed anywhere after the ticker parameter
    for arg in args.iter().skip(2) {
        if let Some((key, val_str)) = arg.split_once('=') {
            let key_cleaned = key.trim().to_lowercase();
            match key_cleaned.as_str() {
                "wacc" => {
                    if let Ok(parsed_val) = val_str.trim().parse::<f64>() {
                        wacc = parsed_val;
                    }
                }
                "term_g" | "terminal_g" => {
                    if let Ok(parsed_val) = val_str.trim().parse::<f64>() {
                        terminal_g = parsed_val;
                    }
                }
                "data_dir" | "data-dir" | "--data-dir" => {
                    data_dir = val_str.trim().to_string();
                }
                // 🎯 NEW: Parse custom module selection triggers or shorthand tokens from console inputs
                "modules" | "run" | "track" => {
                    modules_selector = val_str.trim().to_string();
                }
                unknown => {
                    println!("⚠️ WARNING: Ignored unrecognized parameter key: '{}'", unknown);
                }
            }
        } else {
            println!("⚠️ WARNING: Argument '{}' does not match key=value format. Ignoring.", arg);
        }
    }

    println!("=================================================================================");
    println!("📈 RUNNING ANALYSIS WORKSPACE CORE MATRIX FOR TICKER: {}", ticker);
    println!("📍 Target Unified Data Storage Path Coordinate: {}", data_dir);
    println!("🎯 Active Analytical Scope Target Tracker      : {}", modules_selector);
    println!("=================================================================================");

    // 🎯 Execute the updated pipeline orchestration sequence passing down the selective modules choice
    runner::run_global_analysis_pipeline(ticker, wacc, terminal_g, &data_dir, &modules_selector);

    Ok(())
}