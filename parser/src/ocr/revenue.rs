use std::cmp;
use regex::Regex;
use lazy_static::lazy_static;
use crate::ocr::utils::{OcrStatementExtractor, UnifiedOcrOutput};

lazy_static! {
    // Matches Statement of Profit & Loss, Income Statement, Profit and Loss Account variations
    static ref REVENUE_HEADING_REGEX: Regex = Regex::new(
        r"(?i)(##\s*(?:[A-Za-z]+\s+){0,3}(?:Profit\s+(?:&\s+|and\s+)Loss|Income|Revenue)\s+(?:Statement|Account|Operations)?(?:\s+for\s+the.*)?)"
    ).unwrap();
    
    static ref NOTES_REGEX: Regex = Regex::new(r"^\d{1,2}$|^\([a-zA-Z0-9]{1,3}\)$|^[IVXLC]{1,4}$").unwrap();
    static ref HEADER_NOTES_PATTERN: Regex = Regex::new(r"(?i)\bnotes\b|\bno\b\s*\.\s*|\bsch\b").unwrap();
    static ref FINANCIAL_REGEX: Regex = Regex::new(r"\d+\.\d+|^\d[\d,]{2,}|^-$").unwrap();
    static ref SPLIT_HEADER_REGEX: Regex = Regex::new(r"(?i)for\s+the\s+year|particulars|as\s+at").unwrap();
}

pub struct RevenueStatementExtractor;

impl OcrStatementExtractor for RevenueStatementExtractor {
    fn statement_type(&self) -> &'static str {
        "income_statement"
    }

    fn section_heading_regex(&self) -> &Regex {
        &REVENUE_HEADING_REGEX
    }

    fn parse_table(&self, file_name: &str, header_text: &str, table_str: &str) -> Vec<UnifiedOcrOutput> {
        let report_type = if header_text.to_lowercase().contains("consolidated") {
            "Consolidated".to_string()
        } else {
            "Standalone".to_string()
        };

        // Parse lines into a raw matrix grid
        let mut raw_grid: Vec<Vec<String>> = Vec::new();
        let mut header_row: Vec<String> = Vec::new();
        let mut is_first = true;

        for line in table_str.lines() {
            let line_clean = line.trim();
            if line_clean.is_empty() || line_clean.starts_with("|---") || line_clean.starts_with("| :---") {
                continue;
            }

            let delimiter = if line_clean.contains('┆') { '┆' } else { '|' };
            if !line_clean.contains(delimiter) {
                continue; // Cleanly pass over footnotes or text under the table structure
            }

            let mut cells: Vec<String> = line_clean.split(delimiter).map(|c| c.trim().to_string()).collect();
            if !cells.is_empty() && cells[0].is_empty() { cells.remove(0); }
            if !cells.is_empty() && cells[cells.len() - 1].is_empty() { cells.pop(); }

            if cells.is_empty() || cells.iter().all(|c| c.is_empty()) {
                continue;
            }

            if is_first {
                header_row = cells.clone();
                is_first = false;
            }
            raw_grid.push(cells);
        }

        if raw_grid.is_empty() { return Vec::new(); }
        let num_cols = raw_grid[0].len();

        // 🎯 STRUCTURAL FILTER ADAPTATION (Enforce standard column layout width)
        if num_cols < 3 || num_cols > 6 {
            return Vec::new();
        }

        // 🎯 EXPLICIT HEADER TRACKING FOR NOTES
        let mut detected_notes_idx: Option<usize> = None;
        for idx in 1..(num_cols.saturating_sub(1)) {
            if idx < header_row.len() && HEADER_NOTES_PATTERN.is_match(&header_row[idx]) {
                detected_notes_idx = Some(idx);
                break;
            }
        }

        // FALLBACK CONTENT PATTERN MATCHING (50% Threshold)
        if detected_notes_idx.is_none() && num_cols > 2 {
            let mut highest_notes_score = 0;
            for idx in 1..(num_cols.saturating_sub(1)) {
                let mut total_valid = 0;
                let mut notes_pattern_matches = 0;

                for row in &raw_grid {
                    if idx < row.len() && !row[idx].is_empty() {
                        total_valid += 1;
                        if NOTES_REGEX.is_match(&row[idx]) {
                            notes_pattern_matches += 1;
                        }
                    }
                }

                if total_valid > 0 {
                    let match_rate = notes_pattern_matches as f64 / total_valid as f64;
                    if match_rate >= 0.50 && notes_pattern_matches > highest_notes_score {
                        highest_notes_score = notes_pattern_matches;
                        detected_notes_idx = Some(idx);
                    }
                }
            }
        }

        // 🎯 DETERMINE STRUCTURAL TEXT MERGING BARRIER INDEX
        let barrier_idx = match detected_notes_idx {
            Some(idx) => idx,
            None => {
                let mut detected_barrier = None;
                for idx in 0..num_cols {
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
                        detected_barrier = Some(idx);
                        break;
                    }
                }
                detected_barrier.unwrap_or(cmp::min(3, num_cols.saturating_sub(1)))
            }
        };

        // 🎯 PROCESS LEFT-SIDE MERGE WITH STRING DE-DUPLICATION
        let mut repaired_rows = Vec::with_capacity(raw_grid.len());
        for row in raw_grid {
            let mut cells = row;
            while cells.len() < num_cols {
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
                        continue;
                    } else if current_token.contains(&prev_token) {
                        if let Some(last) = deduplicated_tokens.last_mut() {
                            *last = token;
                        }
                    } else {
                        deduplicated_tokens.push(token);
                    }
                }
            }

            let particulars = deduplicated_tokens.join(" ").trim().to_string();

            let notes_val = match detected_notes_idx {
                Some(idx) => cells[idx].clone(),
                None => String::new(),
            };

            let remaining_data = if detected_notes_idx.is_some() {
                cells[(barrier_idx + 1)..].to_vec()
            } else {
                cells[barrier_idx..].to_vec()
            };

            repaired_rows.push((particulars, notes_val, remaining_data));
        }

        // 🎯 VALUE SHIFT PROCESSING (NUMERICS RIGHT OF THE NOTES BARRIER)
        let mut final_processed_rows = Vec::with_capacity(repaired_rows.len());
        for (particulars, notes_val, right_side_data) in repaired_rows {
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

            final_processed_rows.push(UnifiedOcrOutput {
                source_file: file_name.to_string(),
                statement_type: "income_statement".to_string(),
                particulars,
                context: report_type.clone(),
                notes: notes_val,
                curr_year: curr_year_val,
                prev_year: prev_year_val,
            });
        }

        // 🎯 FILTER REMNANT HEADERS & LOWER THRESHOLD SHIELD
        if final_processed_rows.len() < 10 {
            return Vec::new();
        }

        let mut slice_start_idx = 0;
        let mut check_header_str = String::new();
        for i in 0..cmp::min(3, final_processed_rows.len()) {
            check_header_str.push_str(&final_processed_rows[i].particulars);
        }
        
        if SPLIT_HEADER_REGEX.is_match(&check_header_str) && final_processed_rows.len() > 3 {
            slice_start_idx = 3;
        }

        final_processed_rows[slice_start_idx..].to_vec()
    }
}