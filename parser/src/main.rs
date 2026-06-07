// parser/src/main.rs
use std::env;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize simple terminal logging layer context
    tracing_subscriber::fmt::init();

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("❌ ERROR: Missing target ticker token.");
        println!("👉 Usage: cargo run --release <TICKER> [--data-dir /path/to/data]");
        return Ok(());
    }

    let current_ticker = args[1].to_uppercase();
    let mut data_dir_base = "../data".to_string();

    for i in 2..args.len() {
        if args[i].starts_with("--data-dir=") {
            if let Some(val) = args[i].split('=').nth(1) {
                data_dir_base = val.trim().to_string();
            }
        } else if args[i] == "--data-dir" && i + 1 < args.len() {
            data_dir_base = args[i + 1].trim().to_string();
        }
    }

    let run_start = Instant::now();
    
    // Invoke the library pipeline cleanly
    match parser::run_ticker_parsing_pipeline(&data_dir_base, &current_ticker) {
        Ok(summary) => {
            println!("\n=================================================================================");
            println!("🎉 PARSING CONCLUDED SUCCESSFULLY FOR TICKER [{}]", summary.ticker);
            println!("=================================================================================");
            for folder in summary.group_results {
                println!(
                    "📁 Group: {:<36} | Rows: {:<6} | Processed: {:<3} | Time: {}ms", 
                    folder.folder_name, folder.total_rows, folder.processed_files, folder.elapsed_ms
                );
            }
            println!("---------------------------------------------------------------------------------");
            println!("📈 Grand Total Data Rows Synchronized: {}", summary.grand_total_rows);
            println!("⏱️ Full Run Pipeline Execution Time   : {}ms", run_start.elapsed().as_millis());
            println!("=================================================================================\n");
        }
        Err(e) => {
            println!("🚨 Pipeline Ingestion failed: {}", e);
        }
    }

    Ok(())
}