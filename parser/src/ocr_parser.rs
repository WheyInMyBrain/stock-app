use std::fs;
use std::path::Path;
use regex::Regex;
use lazy_static::lazy_static;
use crate::ocr::utils::{OcrStatementExtractor, UnifiedOcrOutput};
use crate::ocr::balance_sheet::BalanceSheetExtractor;
use crate::ocr::revenue::RevenueStatementExtractor;
use crate::ocr::cash_flow::CashFlowStatementExtractor;

lazy_static! {
    // Fuzzy regex filters designed to survive messy OCR spaces (e.g., "c r o r e s", "In Crs")
    static ref GLOBAL_CRORE_REGEX: Regex = Regex::new(r"(?i)\bcr\b|\bc\s*r\s*o\s*r\s*e\s*s?\b").unwrap();
    static ref GLOBAL_LAKH_REGEX: Regex = Regex::new(r"(?i)\bl(?:a|i)k\s*h\s*s?\b|\bl\s*a\s*c\s*s?\b").unwrap();
    static ref GLOBAL_MILLION_REGEX: Regex = Regex::new(r"(?i)\bm\s*i\s*l\s*l\s*i\s*o\s*n\s*s?\b|\bmn\b").unwrap();
    static ref GLOBAL_THOUSAND_REGEX: Regex = Regex::new(r"(?i)\bt\s*h\s*o\s*u\s*s\s*a\s*n\s*d\s*s?\b").unwrap();
}

/// Scans the full document context to determine a single uniform multiplier.
fn identify_global_document_scale(full_text: &str) -> f64 {
    let mut crore_votes = 0;
    let mut lakh_votes = 0;
    let mut million_votes = 0;
    let mut thousand_votes = 0;

    // Scan the document to gather confirmation points across statements
    if GLOBAL_CRORE_REGEX.is_match(full_text) { crore_votes += 1; }
    if GLOBAL_LAKH_REGEX.is_match(full_text) { lakh_votes += 1; }
    if GLOBAL_MILLION_REGEX.is_match(full_text) { million_votes += 1; }
    if GLOBAL_THOUSAND_REGEX.is_match(full_text) { thousand_votes += 1; }

    // Resolve based on confirmation votes
    if crore_votes >= lakh_votes && crore_votes >= million_votes && crore_votes >= thousand_votes && crore_votes > 0 {
        10_000_000.0
    } else if lakh_votes >= crore_votes && lakh_votes >= million_votes && lakh_votes >= thousand_votes && lakh_votes > 0 {
        100_000.0
    } else if million_votes >= crore_votes && million_votes >= lakh_votes && million_votes >= thousand_votes && million_votes > 0 {
        1_000_000.0
    } else if thousand_votes >= crore_votes && thousand_votes >= lakh_votes && thousand_votes >= million_votes && thousand_votes > 0 {
        1_000.0
    } else {
        1.0 // Fallback to absolute unity
    }
}

pub fn extract_all_statements_from_file(file_path: &Path) -> Result<Vec<UnifiedOcrOutput>, String> {
    let file_name = file_path.file_name().unwrap().to_string_lossy().to_string();
    
    let text = fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read OCR file [{}]: {}", file_name, e))?;

    // 🎯 STEP 1: Compute a single uniform multiplier across the entire document context
    let global_multiplier = identify_global_document_scale(&text);

    let extractors: Vec<Box<dyn OcrStatementExtractor>> = vec![
        Box::new(BalanceSheetExtractor),
        Box::new(RevenueStatementExtractor),
        Box::new(CashFlowStatementExtractor),
    ];

    let mut combined_file_records = Vec::new();

    for extractor in extractors {
        let heading_regex = extractor.section_heading_regex();
        let matches: Vec<_> = heading_regex.find_iter(&text).collect();
        if matches.is_empty() { continue; }

        for i in 0..matches.len() {
            let matched_header = matches[i].as_str().replace('#', "").trim().to_string();
            let start_pos = matches[i].end();
            let end_pos = if i + 1 < matches.len() { matches[i + 1].start() } else { text.len() };
            
            let section_text = &text[start_pos..end_pos];
            
            let mut table_lines = Vec::new();
            for line in section_text.trim().lines() {
                let line_clean = line.trim();
                // Check if the row contains structural data layouts (handles both standard pipe and custom broken artifacts)
                if line_clean.starts_with('|') || line_clean.starts_with('┆') || line_clean.contains('|') || line_clean.contains('┆') {
                    table_lines.push(line_clean);
                } else if !table_lines.is_empty() {
                    // 🌟 FIX: Removed unused loop variable tracking allocations entirely to clear compiler warnings
                    for follow_line in section_text[section_text.find(line).unwrap()..].lines().take(5) {
                        let f_clean = follow_line.trim();
                        if !f_clean.is_empty() {
                            table_lines.push(f_clean);
                        }
                    }
                    break;
                }
            }

            if !table_lines.is_empty() {
                let table_str = table_lines.join("\n");
                let mut parsed_rows = extractor.parse_table(&file_name, &matched_header, &table_str);
                
                // 🎯 STEP 2: APPLY MATH NORMALIZATION AND DOCUMENT SCALE GLOBAL INJECTION
                for record in &mut parsed_rows {
                    record.curr_year = post_process_numeric_value(&record.particulars, &record.curr_year, global_multiplier);
                    record.prev_year = post_process_numeric_value(&record.particulars, &record.prev_year, global_multiplier);
                }

                combined_file_records.extend(parsed_rows);
            }
        }
    }

    Ok(combined_file_records)
}

/// Standardizes digits, refactors accounting parentheses to minus signage, and scales non-EPS values
fn post_process_numeric_value(particulars: &str, raw_val: &str, multiplier: f64) -> String {
    let trimmed = raw_val.trim();
    if trimmed == "-" || trimmed == "—" || trimmed.is_empty() {
        return "0".to_string();
    }

    let clean_numeric_regex = Regex::new(r"[(]?\d[\d,.]*[)]?").unwrap();
    if let Some(mat) = clean_numeric_regex.find(trimmed) {
        let core_str = mat.as_str();
        let is_negative = core_str.starts_with('(') && core_str.ends_with(')');
        
        let sanitized_digits = core_str
            .replace('(', "")
            .replace(')', "")
            .replace(',', "");

        if let Ok(parsed_num) = sanitized_digits.parse::<f64>() {
            let label = particulars.to_lowercase();
            
            // 🌟 CRITICAL SAFEGUARD: Do not scale Earnings Per Share (EPS) matrix metrics
            let final_multiplier = if label.contains("eps") || label.contains("earnings per") || label.contains("basic") || label.contains("diluted") {
                1.0
            } else {
                multiplier
            };

            let scaled_value = parsed_num * final_multiplier;
            if is_negative {
                return format!("-{}", scaled_value);
            } else {
                return scaled_value.to_string();
            }
        }
    }
    
    trimmed.to_string()
}