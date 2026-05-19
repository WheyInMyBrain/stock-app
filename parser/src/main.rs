use polars::prelude::PolarsResult;
use std::time::Instant;

fn main() -> PolarsResult<()> {
    // 🎯 Simply point to the input folder inside data/
    let target_folder = "../data/IMFA/bse_financial-results-docs";
    let target_glob = format!("{}/*.xml", target_folder);

    println!("=================================================================================");
    println!("🦀 AUTOMATED XBRL PIPELINE RUNNING FOR DIRECTORY: [{}]", target_folder);
    println!("=================================================================================\n");

    let total_start = Instant::now();

    let mut files_vec = Vec::new();
    let mut tags_vec = Vec::new();
    let mut contexts_vec = Vec::new();
    let mut dates_vec = Vec::new();
    let mut values_vec = Vec::new();

    let mut processed_files = 0;
    let mut skipped_files = 0;

    let mut entries = glob::glob(&target_glob).expect("Failed to read search glob pattern");
    while let Some(Ok(path)) = entries.next() {
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

    if files_vec.is_empty() {
        println!("⚠️ No records discovered inside directory grid. Skipping Parquet write lifecycle.");
        return Ok(());
    }

    println!("📦 Memory processing finished. Offloading vectors to automated Parquet engine...");

    // Pass the raw path string to the writer
    match parser::utils::save_to_parquet(
        target_folder,
        &files_vec,
        &tags_vec,
        &contexts_vec,
        &dates_vec,
        &values_vec,
    ) {
        Ok(saved_path) => {
            println!("\n=================================================================================");
            println!("🎉 PROCESS COMPLETE: DIRECTORY MATRIX SYNCHRONIZED SUCCESSFULLY");
            println!("=================================================================================");
            println!("📊 Valid Files Fully Digested     : {}", processed_files);
            println!("⚠️ Empty Files Safely Skipped     : {}", skipped_files);
            println!("⏱️ Full Run Pipeline Duration     : {:?}", total_start.elapsed());
            println!("📁 Dynamic Parquet Output Saved As: {}", saved_path);
            println!("=================================================================================");
        }
        Err(e) => println!("❌ Error saving dataset: {}", e),
    }

    Ok(())
}