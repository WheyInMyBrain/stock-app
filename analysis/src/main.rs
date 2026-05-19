// analysis/src/main.rs

use polars::prelude::PolarsResult;
use std::env;

mod dcf;
mod runner;
mod monte_carlo;
mod epv;

fn main() -> PolarsResult<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("❌ ERROR: Missing target ticker token.");
        println!("\n👉 Usage: cargo run --release <TICKER> [key=value] [key=value]...");
        println!("💡 Available Keys:");
        println!("   ├── wacc=<number>         (Default: 0.12)");
        println!("   ├── term_g=<number>       (Default: 0.04)");
        println!("   ├── growth_mult=<number>  (Default: 1.0)");
        println!("   └── margin_mult=<number>  (Default: 1.0)");
        println!("\n👉 Example: cargo run --release IMFA wacc=0.13 margin_mult=0.95");
        return Ok(());
    }

    // The first argument after the binary name is always our target ticker
    let ticker = &args[1].to_uppercase();

    // Core fallback defaults
    let mut wacc: f64 = 0.12;
    let mut terminal_g: f64 = 0.04;
    let mut growth_mult: f64 = 1.0;
    let mut margin_mult: f64 = 1.0;

    // Iterate through key-value overrides passed anywhere after the ticker parameter
    for arg in args.iter().skip(2) {
        if let Some((key, val_str)) = arg.split_once('=') {
            let key_cleaned = key.trim().to_lowercase();
            if let Ok(parsed_val) = val_str.trim().parse::<f64>() {
                match key_cleaned.as_str() {
                    "wacc" => wacc = parsed_val,
                    "term_g" | "terminal_g" => terminal_g = parsed_val,
                    "growth_mult" | "growth_multiplier" => growth_mult = parsed_val,
                    "margin_mult" | "margin_multiplier" => margin_mult = parsed_val,
                    unknown => {
                        println!("⚠️ WARNING: Ignored unrecognized parameter key: '{}'", unknown);
                    }
                }
            } else {
                println!("⚠️ WARNING: Value for '{}' could not be parsed as a float. Ignoring.", key);
            }
        } else {
            println!("⚠️ WARNING: Argument '{}' does not match key=value format. Ignoring.", arg);
        }
    }

    // Hand off processed configuration keys cleanly to the orchestrator runner
    runner::run_all_analysis_pipelines(ticker, wacc, terminal_g, growth_mult, margin_mult)?;

    Ok(())
}