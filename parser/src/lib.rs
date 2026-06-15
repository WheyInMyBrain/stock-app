// stock-app/parser/src/lib.rs
pub mod bse;
pub mod nse;
pub mod ocr;

pub mod bse_parser;
pub mod nse_parser;
pub mod ocr_parser;
pub mod targets;
pub mod utils;

use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;
use rayon::prelude::*;

// A local, unified container layout to harmonize parallel chunks into a single uniform type
#[derive(Debug, Clone)]
struct UnifiedRecord {
    source_file: String,
    tag_name: String,
    context_id: String,
    date_bounds: String,
    raw_value: String,
}

// Columnar Struct-of-Arrays optimization container to accumulate vector chunks smoothly
#[derive(Default)]
struct ColumnarAccumulator {
    files: Vec<String>,
    tags: Vec<String>,
    contexts: Vec<String>,
    dates: Vec<String>,
    values: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct GroupMetrics {
    pub folder_name: String,
    pub processed_files: u32,
    pub skipped_files: u32,
    pub total_rows: usize,
    pub elapsed_ms: u128,
}

#[derive(Debug, Clone, Default)]
pub struct PipelineResult {
    pub ticker: String,
    pub grand_total_rows: usize,
    pub group_results: Vec<GroupMetrics>,
    pub total_elapsed_ms: u128,
}

/// 🚀 THE HIGH-SPEED ENGINE ENTRY POINT: Fully asynchronous/thread-safe execution loop
pub fn run_ticker_parsing_pipeline<P: AsRef<Path>>(
    data_dir: P,
    ticker: &str,
) -> Result<PipelineResult, String> {
    let pipeline_start = Instant::now();
    let current_ticker = ticker.to_uppercase();
    let base_path = data_dir.as_ref();

    let mut pipeline_summary = PipelineResult {
        ticker: current_ticker.clone(),
        grand_total_rows: 0,
        group_results: Vec::new(),
        total_elapsed_ms: 0,
    };

    println!("\x1b[93m[PARSER] 🚀 Initiating parallel dataset parsing routines for ticker [{}]\x1b[0m", current_ticker);

    for folder in targets::TARGET_REPORT_FOLDERS {
        let group_start = Instant::now();
        let target_folder = base_path.join(&current_ticker).join(folder);
        
        // Dynamically compute target glob patterns depending on the folder schema type
        let glob_patterns = if folder.contains("ocr") {
            vec![target_folder.join("*.md")]
        } else if folder.contains("integrated-finance") {
            vec![target_folder.join("*.html"), target_folder.join("*.xhtml"), target_folder.join("*.xml")]
        } else {
            vec![target_folder.join("*.xml")]
        };

        let mut entries = Vec::new();
        for pattern in glob_patterns {
            if let Some(g_str) = pattern.to_str() {
                if let Ok(paths) = glob::glob(g_str) {
                    entries.extend(paths.flatten());
                }
            }
        }

        if entries.is_empty() {
            continue;
        }

        let processed_files = AtomicU32::new(0);
        let skipped_files = AtomicU32::new(0);

        // ============================================================================
        // 📑 ROUTE A: INGESTION FOR ANNUAL OCR DATA (MARKDOWN)
        // ============================================================================
        if *folder == "ocr/annual-reports" {
            let stream_results: Vec<_> = entries
                .into_par_iter() 
                .filter_map(|path| {
                    match ocr_parser::extract_all_statements_from_file(&path) {
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

            let mut statement_groups: std::collections::HashMap<String, Vec<crate::ocr::utils::UnifiedOcrOutput>> = std::collections::HashMap::new();
            for record in stream_results {
                statement_groups.entry(record.statement_type.clone()).or_default().push(record);
            }

            let mut group_total_rows = 0;
            for (statement_type, records) in statement_groups {
                if records.is_empty() { continue; }

                let mut files_vec = Vec::with_capacity(records.len());
                let mut statement_vec = Vec::with_capacity(records.len());
                let mut contexts_vec = Vec::with_capacity(records.len());
                let mut particulars_vec = Vec::with_capacity(records.len());
                let mut notes_vec = Vec::with_capacity(records.len());
                let mut curr_vec = Vec::with_capacity(records.len());
                let mut prev_vec = Vec::with_capacity(records.len());

                for r in records {
                    files_vec.push(r.source_file);
                    statement_vec.push(r.statement_type.clone());
                    contexts_vec.push(r.context);
                    particulars_vec.push(r.particulars);
                    notes_vec.push(r.notes);
                    curr_vec.push(r.curr_year);
                    prev_vec.push(r.prev_year);
                }

                group_total_rows += particulars_vec.len();

                if let Err(_) = utils::save_ocr_to_parquet(
                    base_path,
                    &current_ticker,
                    &statement_type,
                    files_vec,
                    statement_vec,
                    contexts_vec,
                    particulars_vec,
                    notes_vec,
                    curr_vec,
                    prev_vec,
                ) {}
            }

            let metrics = GroupMetrics {
                folder_name: folder.to_string(),
                processed_files: processed_files.load(Ordering::Relaxed),
                skipped_files: skipped_files.load(Ordering::Relaxed),
                total_rows: group_total_rows,
                elapsed_ms: group_start.elapsed().as_millis(),
            };

            println!(
                "\x1b[93m[PARSER] 📁 Group: {:<38} | Rows: {:<6} | Processed: {:<3} | Time: {}ms\x1b[0m",
                metrics.folder_name, metrics.total_rows, metrics.processed_files, metrics.elapsed_ms
            );

            pipeline_summary.grand_total_rows += group_total_rows;
            pipeline_summary.group_results.push(metrics);
            continue; 
        }

        // ============================================================================
        // 📁 ROUTE B: INGESTION FOR STRUCTURAL REPORT CHUNKS (XML / HTML Layouts)
        // ============================================================================
        let chunks_vector: Vec<Vec<UnifiedRecord>> = entries
            .into_par_iter()
            .filter_map(|path| {
                if folder.starts_with("bse_") {
                    match bse_parser::parse_bse_file(&path, folder) {
                        Ok(records) => {
                            processed_files.fetch_add(1, Ordering::Relaxed);
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
                    match nse_parser::parse_nse_file(&path, folder) {
                        Ok(records) => {
                            processed_files.fetch_add(1, Ordering::Relaxed);
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
            .collect();

        let combined_capacity: usize = chunks_vector.iter().map(|c| c.len()).sum();
        if combined_capacity == 0 {
            continue;
        }

        let mut accumulator = ColumnarAccumulator {
            files: Vec::with_capacity(combined_capacity),
            tags: Vec::with_capacity(combined_capacity),
            contexts: Vec::with_capacity(combined_capacity),
            dates: Vec::with_capacity(combined_capacity),
            values: Vec::with_capacity(combined_capacity),
        };

        for chunk in chunks_vector {
            for record in chunk {
                accumulator.files.push(record.source_file);
                accumulator.tags.push(record.tag_name);
                accumulator.contexts.push(record.context_id);
                accumulator.dates.push(record.date_bounds);
                accumulator.values.push(record.raw_value);
            }
        }

        match utils::save_to_parquet(
            base_path,
            &current_ticker,
            folder,
            accumulator.files,
            accumulator.tags,
            accumulator.contexts,
            accumulator.dates,
            accumulator.values,
        ) {
            Ok(_) => {
                let metrics = GroupMetrics {
                    folder_name: folder.to_string(),
                    processed_files: processed_files.load(Ordering::Relaxed),
                    skipped_files: skipped_files.load(Ordering::Relaxed),
                    total_rows: combined_capacity,
                    elapsed_ms: group_start.elapsed().as_millis(),
                };

                println!(
                    "\x1b[93m[PARSER] 📁 Group: {:<38} | Rows: {:<6} | Processed: {:<3} | Time: {}ms\x1b[0m",
                    metrics.folder_name, metrics.total_rows, metrics.processed_files, metrics.elapsed_ms
                );

                pipeline_summary.grand_total_rows += combined_capacity;
                pipeline_summary.group_results.push(metrics);
            }
            Err(_) => {}
        }
    }

    pipeline_summary.total_elapsed_ms = pipeline_start.elapsed().as_millis();

    println!("\x1b[93m[PARSER] ---------------------------------------------------------------------------------\x1b[0m");
    println!("\x1b[93m[PARSER] 📈 Grand Total Data Rows Synchronized: {}\x1b[0m", pipeline_summary.grand_total_rows);
    println!("\x1b[93m[PARSER] ⏱️ Full Run Pipeline Execution Time   : {}ms\x1b[0m", pipeline_summary.total_elapsed_ms);
    println!("\x1b[93m[PARSER] =================================================================================\x1b[0m");

    Ok(pipeline_summary)
}