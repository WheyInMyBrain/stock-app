use std::fs;
use std::path::Path;
use crate::ocr::utils::{OcrStatementExtractor, UnifiedOcrOutput};
use crate::ocr::balance_sheet::BalanceSheetExtractor;
use crate::ocr::revenue::RevenueStatementExtractor;
use crate::ocr::cash_flow::CashFlowStatementExtractor;

pub fn extract_all_statements_from_file(file_path: &Path) -> Result<Vec<UnifiedOcrOutput>, String> {
    let file_name = file_path.file_name().unwrap().to_string_lossy().to_string();
    
    let text = fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read OCR file [{}]: {}", file_name, e))?;

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
                if line_clean.starts_with('|') {
                    table_lines.push(line_clean);
                } else if !table_lines.is_empty() {
                    break;
                }
            }

            if !table_lines.is_empty() {
                let table_str = table_lines.join("\n");
                let parsed_rows = extractor.parse_table(&file_name, &matched_header, &table_str);
                combined_file_records.extend(parsed_rows);
            }
        }
    }

    Ok(combined_file_records)
}