use regex::Regex;
use lazy_static::lazy_static;
use crate::ocr::utils::{OcrStatementExtractor, UnifiedOcrOutput};

lazy_static! {
    // 🎯 High-fidelity anchor matching primary Cash Flow headings exactly like before
    static ref CASH_FLOW_HEADING_REGEX: Regex = Regex::new(
        r"(?i)(##\s*(?:[A-Za-z]+\s+){0,3}Cash\s+Flow\s*(?:Statement|Flows)?(?:\s+for\s+the.*)?)"
    ).unwrap();
    
    static ref FINANCIAL_REGEX: Regex = Regex::new(r"\d+\.\d+|^\d[\d,]{2,}|^-$").unwrap();
    static ref SPLIT_HEADER_REGEX: Regex = Regex::new(r"(?i)particulars|for\s+the\s+year|year\s+ended").unwrap();
}

pub struct CashFlowStatementExtractor;

impl OcrStatementExtractor for CashFlowStatementExtractor {
    fn statement_type(&self) -> &'static str {
        "cash_flow"
    }

    fn section_heading_regex(&self) -> &Regex {
        &CASH_FLOW_HEADING_REGEX
    }

    fn parse_table(&self, file_name: &str, header_text: &str, table_str: &str) -> Vec<UnifiedOcrOutput> {
        let report_type = if header_text.to_lowercase().contains("consolidated") {
            "Consolidated".to_string()
        } else {
            "Standalone".to_string()
        };

        let mut raw_grid: Vec<Vec<String>> = Vec::new();
        let mut max_cols = 0;

        // 🎯 STAGE 1: RAW MANUAL PIPE EXPANSION (Ensures no empty-header columns get dropped!)
        for line in table_str.lines() {
            let line_clean = line.trim();
            if line_clean.is_empty() || line_clean.starts_with("|---") || line_clean.starts_with("| :---") {
                continue;
            }

            let mut cells: Vec<String> = line_clean.split('|').map(|c| c.trim().to_string()).collect();
            if !cells.is_empty() && cells[0].is_empty() { cells.remove(0); }
            if !cells.is_empty() && cells[cells.len() - 1].is_empty() { cells.pop(); }

            if cells.is_empty() || cells.iter().all(|c| c.is_empty()) {
                continue;
            }

            if cells.len() > max_cols {
                max_cols = cells.len();
            }
            raw_grid.push(cells);
        }

        if raw_grid.is_empty() { return Vec::new(); }

        // 🎯 STAGE 2: DYNAMIC FINANCIAL BARRIER EVALUATION
        let mut barrier_idx = max_cols.saturating_sub(2).max(1);

        for idx in 0..max_cols {
            let mut total_valid = 0;
            let mut financial_matches = 0;

            for row in &raw_grid {
                if idx < row.len() && !row[idx].is_empty() {
                    total_valid += 1;
                    if FINANCIAL_REGEX.is_match(&row[idx]) {
                        financial_matches += 1;
                    }
                }
            }

            if total_valid > 0 && (financial_matches as f64 / total_valid as f64) > 0.4 {
                barrier_idx = idx;
                break;
            }
        }

        if barrier_idx == 0 {
            barrier_idx = 1;
        } else if barrier_idx > max_cols.saturating_sub(1) {
            barrier_idx = max_cols.saturating_sub(2).max(1);
        }

        // 🎯 STAGE 3: LEFT-SIDE MERGE & ANTI-STUTTER DEDUPLICATION
        let mut processed_records = Vec::with_capacity(raw_grid.len());

        for row in raw_grid {
            let mut cells = row;
            // Pad out shorter rows dynamically to maintain metric safezone indexes
            while cells.len() < max_cols {
                cells.push(String::new());
            }

            let left_side_tokens: Vec<String> = cells[0..barrier_idx]
                .iter()
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect();

            let mut deduplicated_tokens: Vec<String> = Vec::new();
            for token in left_side_tokens {
                if deduplicated_tokens.is_empty() {
                    deduplicated_tokens.push(token);
                } else {
                    let prev_token = deduplicated_tokens.last().unwrap().to_lowercase();
                    let current_token = token.to_lowercase();

                    if current_token == prev_token || prev_token.contains(&current_token) {
                        continue; // Skip perfect duplicates or sub-string crumbs
                    } else if current_token.contains(&prev_token) {
                        if let Some(last) = deduplicated_tokens.last_mut() {
                            *last = token; // Overwrite step-up versions
                        }
                    } else {
                        deduplicated_tokens.push(token);
                    }
                }
            }

            let particulars = deduplicated_tokens.join(" ").trim().to_string();

            // Right side metrics resolution (Since notes are hardcoded to skip, grab the two numeric spaces)
            let right_side_data = cells[barrier_idx..].to_vec();
            let active_candidates: Vec<String> = right_side_data
                .iter()
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect();

            let mut curr_year_val = String::new();
            let mut prev_year_val = String::new();

            if active_candidates.len() == 1 {
                curr_year_val = active_candidates[0].clone();
            } else if active_candidates.len() >= 2 {
                curr_year_val = active_candidates[0].clone();
                prev_year_val = active_candidates[1].clone();
            }

            // Drop structural row artifacts that are entirely blank strings
            if particulars.is_empty() && curr_year_val.is_empty() && prev_year_val.is_empty() {
                continue;
            }

            processed_records.push(UnifiedOcrOutput {
                source_file: file_name.to_string(),
                statement_type: "cash_flow".to_string(),
                particulars,
                context: report_type.clone(),
                notes: String::new(), // Explicitly blank as confirmed
                curr_year: curr_year_val,
                prev_year: prev_year_val,
            });
        }

        // 🎯 STAGE 4: TOP METADATA HEADER SLICER
        if processed_records.is_empty() { return Vec::new(); }

        let mut slice_start_idx = 0;
        let mut check_header_str = String::new();
        for i in 0..std::cmp::min(3, processed_records.len()) {
            check_header_str.push_str(&processed_records[i].particulars);
        }

        if SPLIT_HEADER_REGEX.is_match(&check_header_str) && processed_records.len() > 3 {
            slice_start_idx = 3;
        }

        processed_records[slice_start_idx..].to_vec()
    }
}