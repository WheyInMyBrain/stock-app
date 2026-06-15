use regex::Regex;
use lazy_static::lazy_static;
use crate::ocr::utils::{OcrStatementExtractor, UnifiedOcrOutput};

lazy_static! {
    static ref BS_HEADING_REGEX: Regex = Regex::new(r"(?i)(##\s*(?:[A-Za-z]+\s+){0,2}Balance\s+Sheet(?:\s+as\s+at.*)?)").unwrap();
}

pub struct BalanceSheetExtractor;

impl OcrStatementExtractor for BalanceSheetExtractor {
    fn statement_type(&self) -> &'static str {
        "balance_sheet"
    }

    fn section_heading_regex(&self) -> &Regex {
        &BS_HEADING_REGEX
    }

    fn parse_table(&self, file_name: &str, header_text: &str, table_str: &str) -> Vec<UnifiedOcrOutput> {
        process_markdown_table(file_name, header_text, table_str)
    }
}

pub fn process_markdown_table(file_name: &str, header_text: &str, table_str: &str) -> Vec<UnifiedOcrOutput> {
    let report_type = if header_text.to_lowercase().contains("consolidated") {
        "Consolidated".to_string()
    } else {
        "Standalone".to_string()
    };

    let notes_regex = Regex::new(r"^\d{1,2}$|^\([a-zA-Z0-9]{1,3}\)$|^[IVXLC]{1,4}$").unwrap();
    let financial_regex = Regex::new(r"\d+\.\d+|^\d[\d,]{2,}|^-$").unwrap();
    let split_header_regex = Regex::new(r"(?i)as\s+at").unwrap();

    let mut grid: Vec<Vec<String>> = Vec::new();
    for line in table_str.lines() {
        let line_clean = line.trim();
        if line_clean.is_empty() || line_clean.starts_with("|---") || line_clean.starts_with("| :---") {
            continue;
        }
        
        let delimiter = if line_clean.contains('┆') { '┆' } else { '|' };
        if !line_clean.contains(delimiter) {
            continue;
        }
        
        let mut cells: Vec<String> = line_clean.split(delimiter).map(|c| c.trim().to_string()).collect();
        if !cells.is_empty() && cells[0].is_empty() { cells.remove(0); }
        if !cells.is_empty() && cells[cells.len() - 1].is_empty() { cells.pop(); }
        
        if !cells.is_empty() && cells.iter().any(|c| !c.is_empty()) {
            grid.push(cells);
        }
    }

    if grid.is_empty() { return Vec::new(); }
    let num_cols = grid[0].len();

    let mut detected_notes_idx: Option<usize> = None;
    let mut highest_notes_score = 0;

    if num_cols > 2 {
        for idx in 1..(num_cols - 1) {
            let mut total_valid = 0;
            let mut notes_pattern_matches = 0;
            
            for row in &grid {
                if idx < row.len() && !row[idx].is_empty() {
                    total_valid += 1;
                    if notes_regex.is_match(&row[idx]) {
                        notes_pattern_matches += 1;
                    }
                }
            }
            if total_valid > 0 {
                let match_rate = notes_pattern_matches as f64 / total_valid as f64;
                if match_rate > 0.60 && notes_pattern_matches > highest_notes_score {
                    highest_notes_score = notes_pattern_matches;
                    detected_notes_idx = Some(idx);
                }
            }
        }
    }

    let barrier_idx = match detected_notes_idx {
        Some(idx) => idx,
        None => {
            let mut detected_barrier = None;
            for idx in 0..num_cols {
                let mut total_valid = 0;
                let mut financial_matches = 0;
                for row in &grid {
                    if idx < row.len() && !row[idx].is_empty() {
                        total_valid += 1;
                        if financial_regex.is_match(&row[idx]) {
                            financial_matches += 1;
                        }
                    }
                }
                if total_valid > 0 && (financial_matches as f64 / total_valid as f64) > 0.4 {
                    detected_barrier = Some(idx);
                    break;
                }
            }
            detected_barrier.unwrap_or(std::cmp::min(3, num_cols.saturating_sub(1)))
        }
    };

    let mut repaired_rows = Vec::with_capacity(grid.len());
    for row in grid {
        let mut cells = row;
        while cells.len() < num_cols {
            cells.push(String::new());
        }

        let left_side: Vec<String> = cells[0..barrier_idx]
            .iter()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        let particulars = left_side.join(" ").trim().to_string();

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
            statement_type: "balance_sheet".to_string(),
            particulars,
            context: report_type.clone(),
            notes: notes_val,
            curr_year: curr_year_val,
            prev_year: prev_year_val,
        });
    }

    if final_processed_rows.len() < 25 {
        return Vec::new();
    }

    let mut slice_start_idx = 0;
    let mut check_header_str = String::new();
    for i in 0..std::cmp::min(3, final_processed_rows.len()) {
        check_header_str.push_str(&final_processed_rows[i].particulars);
    }
    if split_header_regex.is_match(&check_header_str) && final_processed_rows.len() > 3 {
        slice_start_idx = 3;
    }

    final_processed_rows[slice_start_idx..].to_vec()
}