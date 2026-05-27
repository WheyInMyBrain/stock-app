use polars::prelude::PolarsResult;
use rayon::prelude::*;
use std::env;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;

// Create a local, unified structural container for the parallel collection loop
#[derive(Debug, Clone)]
struct UnifiedRecord {
    source_file: String,
    tag_name: String,
    context_id: String,
    date_bounds: String,
    raw_value: String,
}

fn main() -> PolarsResult<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("❌ ERROR: Missing target ticker token.");
        println!("👉 Usage hint: cargo run --release <TICKER>");
        return Ok(());
    }

    let current_ticker = &args[1].to_uppercase();

    println!("=================================================================================");
    println!("🦀 AUTOMATED PARSING PIPELINE TARGETING SECTOR: [{}]", current_ticker);
    println!("=================================================================================\n");

    let pipeline_start = Instant::now();
    let mut grand_total_rows = 0;

    for folder in parser::targets::TARGET_REPORT_FOLDERS {
        let group_start = Instant::now();
        let target_folder = format!("../data/{}/{}", current_ticker, folder);
        let file_extension = if folder.contains("ocr") { "*.md" } else { "*.xml" };
        let target_glob = format!("{}/{}", target_folder, file_extension);

        println!("🔄 Processing Group: {}", folder);

        let processed_files = AtomicU32::new(0);
        let skipped_files = AtomicU32::new(0);

        let entries = match glob::glob(&target_glob) {
            Ok(e) => e,
            Err(_) => {
                println!("⚠️  Skipping: Folder directory tract does not exist on disk at [{}]\n", target_folder);
                continue;
            }
        };

        // SINGLE-PASS INGESTION FOR OCR DATA
        if *folder == "ocr/annual-reports" {
            let stream_results: Vec<_> = entries
                .into_iter()
                .flatten()
                .par_bridge()
                .filter_map(|path| {
                    match parser::ocr_parser::extract_all_statements_from_file(&path) {
                        Ok(records) => {
                            processed_files.fetch_add(1, Ordering::Relaxed);
                            Some(records)
                        }
                        Err(_) => {
                            skipped_files.fetch_add(1, Ordering::Relaxed);
                            None
                        }
                    }
                })
                .flatten()
                .collect();

            let mut statement_groups: std::collections::HashMap<String, Vec<parser::ocr::utils::UnifiedOcrOutput>> = std::collections::HashMap::new();
            for record in stream_results {
                statement_groups.entry(record.statement_type.clone()).or_default().push(record);
            }

            for (statement_type, records) in statement_groups {
                if records.is_empty() { continue; }

                let mut files_vec = Vec::with_capacity(records.len());
                let mut tags_vec = Vec::with_capacity(records.len());
                let mut contexts_vec = Vec::with_capacity(records.len());
                let mut dates_vec = Vec::with_capacity(records.len());
                let mut values_vec = Vec::with_capacity(records.len());

                for r in records {
                    files_vec.push(r.source_file); 
                    contexts_vec.push(r.context);  
                    tags_vec.push(r.tag);
                    dates_vec.push(r.date_bounds);
                    values_vec.push(r.payload_value);
                }

                // 🎯 FIXED: No balance sheet folder layer anymore. Drops right into annual_report/
                let dynamic_hierarchy = vec!["annual_report"];

                // 🎯 CRITICAL PASS: Change input_folder_path string parameter dynamically 
                // to explicitly carry the statement type token name down into utils.rs
                let route_with_statement_metadata = format!("{}/{}", target_folder, statement_type);

                match parser::utils::save_to_parquet(
                    &route_with_statement_metadata,
                    &dynamic_hierarchy,
                    &files_vec,
                    &tags_vec,
                    &contexts_vec,
                    &dates_vec,
                    &values_vec,
                ) {
                    Ok(_saved_path) => {
                        grand_total_rows += files_vec.len();
                    }
                    Err(e) => println!("❌ Error saving merged OCR Parquet track: {}\n", e),
                }
            }

            println!("✅ Digested: {} files | Skipped: {}", processed_files.load(Ordering::Relaxed), skipped_files.load(Ordering::Relaxed));
            println!("⏱️  Duration : {:?}", group_start.elapsed());
            println!();
            continue; 
        }

        // Collect all parallel streams into our uniform local UnifiedRecord container layout
        let all_records: Vec<UnifiedRecord> = entries
            .into_iter()
            .flatten()
            .par_bridge() 
            .filter_map(|path| {
                if folder.starts_with("bse_") {
                    match parser::bse_parser::parse_bse_file(&path, folder) {
                        Ok(records) => {
                            processed_files.fetch_add(1, Ordering::Relaxed);
                            // 🚀 Map the distinct BseRecord vectors to your local standard layout
                            let mapped = records.into_iter().map(|r| UnifiedRecord {
                                source_file: r.source_file,
                                tag_name: r.tag_name,
                                context_id: r.context_id,
                                date_bounds: r.date_bounds,
                                raw_value: r.raw_value,
                            }).collect::<Vec<_>>();
                            Some(mapped)
                        }
                        Err(_) => {
                            skipped_files.fetch_add(1, Ordering::Relaxed);
                            None
                        }
                    }
                } else if folder.starts_with("nse_") {
                    match parser::nse_parser::parse_nse_file(&path, folder) {
                        Ok(records) => {
                            processed_files.fetch_add(1, Ordering::Relaxed);
                            // 🚀 Map the distinct NseRecord vectors to the exact same local standard layout
                            let mapped = records.into_iter().map(|r| UnifiedRecord {
                                source_file: r.source_file,
                                tag_name: r.tag_name,
                                context_id: r.context_id,
                                date_bounds: r.date_bounds,
                                raw_value: r.raw_value,
                            }).collect::<Vec<_>>();
                            Some(mapped)
                        }
                        Err(_) => {
                            skipped_files.fetch_add(1, Ordering::Relaxed);
                            None
                        }
                    }
                } else {
                    None
                }
            })
            .flatten() 
            .collect(); 

        if all_records.is_empty() {
            println!("⚠️  No records found inside: {}\n", folder);
            continue;
        }

        let mut files_vec = Vec::with_capacity(all_records.len());
        let mut tags_vec = Vec::with_capacity(all_records.len());
        let mut contexts_vec = Vec::with_capacity(all_records.len());
        let mut dates_vec = Vec::with_capacity(all_records.len());
        let mut values_vec = Vec::with_capacity(all_records.len());

        for r in all_records {
            files_vec.push(r.source_file);
            tags_vec.push(r.tag_name);
            contexts_vec.push(r.context_id);
            dates_vec.push(r.date_bounds);
            values_vec.push(r.raw_value);
        }

        match parser::utils::save_to_parquet(
            &target_folder,
            &[],
            &files_vec,
            &tags_vec,
            &contexts_vec,
            &dates_vec,
            &values_vec,
        ) {
            Ok(saved_path) => {
                println!(
                    "✅ Digested: {} files | Skipped: {}", 
                    processed_files.load(Ordering::Relaxed), 
                    skipped_files.load(Ordering::Relaxed)
                );
                println!("📁 Saved As: {}", saved_path);
                println!("⏱️  Duration : {:?}", group_start.elapsed());
                println!();
                grand_total_rows += files_vec.len();
            }
            Err(e) => println!("❌ Error saving dataset for {}: {}\n", folder, e),
        }
    }

    println!("=================================================================================");
    println!("🎉 PARSING CONCLUDED FOR TICKER SYSTEM [{}]", current_ticker);
    println!("=================================================================================");
    println!("📈 Grand Total Data Rows Synchronized: {}", grand_total_rows);
    println!("⏱️ Full Run Pipeline Execution Time   : {:?}", pipeline_start.elapsed());
    println!("=================================================================================");

    Ok(())
}