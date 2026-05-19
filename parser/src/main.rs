use polars::prelude::PolarsResult;
use std::env;
use std::time::Instant;

fn main() -> PolarsResult<()> {
    // 1. Capture command line arguments passed via the terminal
    let args: Vec<String> = env::args().collect();

    // Verify a ticker was supplied. If not, print a clean usage hint.
    if args.len() < 2 {
        println!("❌ ERROR: Missing target ticker token.");
        println!("👉 Usage hint: cargo run --release <TICKER>");
        println!("👉 Example   : cargo run --release IMFA");
        return Ok(());
    }

    // Isolate the ticker name from the first argument space
    let current_ticker = &args[1].to_uppercase();

    println!("=================================================================================");
    println!("🦀 AUTOMATED DATA PIPELINE DETECTED TICKER TARGET: [{}]", current_ticker);
    println!("=================================================================================\n");

    let pipeline_start = Instant::now();
    let mut grand_total_rows = 0;

    // 2. Loop through every folder registered inside targets.rs automatically
    for folder in parser::targets::TARGET_REPORT_FOLDERS {
        // Build the precise dynamic path based on the user argument and loop folder
        let target_folder = format!("../data/{}/{}", current_ticker, folder);
        let target_glob = format!("{}/*.xml", target_folder);

        println!("🔄 Processing Group: {}", folder);

        let mut files_vec = Vec::new();
        let mut tags_vec = Vec::new();
        let mut contexts_vec = Vec::new();
        let mut dates_vec = Vec::new();
        let mut values_vec = Vec::new();

        let mut processed_files = 0;
        let mut skipped_files = 0;

        // Try reading the file path glob pattern
        let entries = match glob::glob(&target_glob) {
            Ok(e) => e,
            Err(_) => {
                println!("⚠️  Skipping: Folder directory track does not exist on disk at [{}]\n", target_folder);
                continue;
            }
        };

        for entry in entries {
            if let Ok(path) = entry {
                if let Ok(records) = parser::bse_parser::parse_bse_file(&path) {
                    for r in records {
                        files_vec.push(r.source_file);
                        tags_vec.push(r.tag_name);
                        contexts_vec.push(r.context_id);
                        dates_vec.push(r.date_bounds);
                        values_vec.push(r.raw_value);
                    }
                    processed_files += 1;
                } else {
                    skipped_files += 1;
                }
            }
        }

        // If no records were parsed from this specific folder, continue to the next folder in line
        if files_vec.is_empty() {
            println!("⚠️  No records found in: {}\n", folder);
            continue;
        }

        // Execute your dynamic Parquet writer module
        match parser::utils::save_to_parquet(
            &target_folder,
            &files_vec,
            &tags_vec,
            &contexts_vec,
            &dates_vec,
            &values_vec,
        ) {
            Ok(saved_path) => {
                println!("✅ Digested: {} files | Skipped: {}", processed_files, skipped_files);
                println!("📁 Saved As: {}\n", saved_path);
                
                // Track total written files across the entire pipeline execution frame
                grand_total_rows += files_vec.len();
            }
            Err(e) => println!("❌ Error saving dataset for {}: {}\n", folder, e),
        }
    }

    println!("=================================================================================");
    println!("🎉 PIPELINE CONCLUDED FOR TICKER SYSTEM [{}]", current_ticker);
    println!("=================================================================================");
    println!("📈 Grand Total Data Rows Synchronized: {}", grand_total_rows);
    println!("⏱️ Full Run Pipeline Execution Time   : {:?}", pipeline_start.elapsed());
    println!("=================================================================================");

    Ok(())
}