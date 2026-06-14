// stock-app/ui/backend/src/database/financial.rs

use std::collections::BTreeSet;
use polars::prelude::*;
use crate::database::WorkspaceDataLoader;
use crate::commands::memory_pool::store_parsed_table;

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct FinancialStatementLineItem {
    pub particulars: String,
    pub context_id: String,
    pub current_year_value: String,
    pub previous_year_value: String,
    pub file_name: String,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct FinancialStatementCollection {
    pub income_statement: Vec<FinancialStatementLineItem>,
    pub balance_sheet: Vec<FinancialStatementLineItem>,
    pub cash_flow: Vec<FinancialStatementLineItem>,
    pub available_files: Vec<String>,
}

/// Robust formatter that handles floating-point anomalies and builds 3-2-2 Indian digit groupings
fn clean_and_format_indian_currency(raw_val: &str) -> String {
    let trimmed = raw_val.trim();
    if trimmed.is_empty() || trimmed == "0" || trimmed == "0.0" { return "0".to_string(); }

    // Parse to f64 first to dissolve nasty .9999999 or .0000001 floating point artifacts
    let parsed_num = match trimmed.parse::<f64>() {
        Ok(n) => n,
        Err(_) => return trimmed.to_string(), // Fallback to raw string if it's purely textual
    };

    // Round cleanly to 2 decimal places to catch precision drift
    let rounded_val = (parsed_num * 100.0).round() / 100.0;
    let val_str = format!("{:.2}", rounded_val);

    let is_negative = rounded_val < 0.0;
    
    // Isolate integer and decimal parts
    let clean_int: String = val_str.split('.').next().unwrap_or("").chars().filter(|c| c.is_ascii_digit()).collect();
    let clean_dec: String = val_str.split('.').nth(1).unwrap_or("00").chars().filter(|c| c.is_ascii_digit()).collect();

    if clean_int.is_empty() || clean_int == "0" {
        let final_str = format!("0.{}", clean_dec);
        return if is_negative { format!("-{}", final_str) } else { final_str };
    }

    let mut result = String::new();
    let len = clean_int.len();

    if len <= 3 {
        result.push_str(&clean_int);
    } else {
        let last_three = &clean_int[len - 3..];
        let remaining = &clean_int[..len - 3];
        
        let mut groups = Vec::new();
        let mut chars: Vec<char> = remaining.chars().collect();
        
        while !chars.is_empty() {
            let split_pos = chars.len().saturating_sub(2);
            let group: String = chars.drain(split_pos..).collect();
            groups.push(group);
        }
        groups.reverse();
        
        result.push_str(&groups.join(","));
        result.push(',');
        result.push_str(last_three);
    }

    // Append the clean, rounded decimals
    if clean_dec != "00" && !clean_dec.is_empty() {
        result = format!("{}.{}", result, clean_dec);
    }

    if is_negative { format!("-{}", result) } else { result }
}

fn clean_file_label(raw_name: &str) -> String {
    let base = raw_name.replace(".parquet", "").replace(".json", "").to_uppercase();
    if let Some(first_year) = base.split('-').next() {
        first_year.trim().to_string()
    } else {
        base
    }
}

fn parse_statement_parquet(bytes: Vec<u8>) -> Result<Vec<FinancialStatementLineItem>, PolarsError> {
    let mut items = Vec::new();
    let df = ParquetReader::new(std::io::Cursor::new(bytes)).finish()?;

    let p_ca = df.column("particulars")?.str()?;
    let ctx_ca = df.column("context_id")?.str()?;
    let f_ca = df.column("file_name")?.str()?;
    let curr_ca = df.column("curr_year")?.str()?;
    let prev_ca = df.column("prev_year")?.str()?;

    for i in 0..df.height() {
        let context = ctx_ca.get(i).unwrap_or("Consolidated").to_string();
        if context.to_lowercase() != "consolidated" { continue; }

        let raw_file = match f_ca.get(i) { Some(f) => f.to_string(), None => continue };
        let clean_label = clean_file_label(&raw_file);

        let curr_raw = curr_ca.get(i).unwrap_or("0");
        let prev_raw = prev_ca.get(i).unwrap_or("0");

        items.push(FinancialStatementLineItem {
            particulars: p_ca.get(i).unwrap_or_default().to_string(),
            context_id: context,
            // Pre-format items right here during ingestion to save CPU cycles on repaint loops
            current_year_value: clean_and_format_indian_currency(curr_raw),
            previous_year_value: clean_and_format_indian_currency(prev_raw),
            file_name: clean_label,
        });
    }

    Ok(items)
}

pub fn hydrate_raw_financial_statements(ticker: &str) -> Result<(), String> {
    let loader = WorkspaceDataLoader::bind(ticker);
    let mut collection = FinancialStatementCollection::default();
    let mut file_set = BTreeSet::new();

    if let Ok(bytes) = loader.load_raw_bytes("parquets/annual_report/income_statement.parquet") {
        if let Ok(lines) = parse_statement_parquet(bytes) {
            for line in &lines { file_set.insert(line.file_name.clone()); }
            collection.income_statement = lines;
        }
    }

    if let Ok(bytes) = loader.load_raw_bytes("parquets/annual_report/balance_sheet.parquet") {
        if let Ok(lines) = parse_statement_parquet(bytes) {
            for line in &lines { file_set.insert(line.file_name.clone()); }
            collection.balance_sheet = lines;
        }
    }

    if let Ok(bytes) = loader.load_raw_bytes("parquets/annual_report/cash_flow.parquet") {
        if let Ok(lines) = parse_statement_parquet(bytes) {
            for line in &lines { file_set.insert(line.file_name.clone()); }
            collection.cash_flow = lines;
        }
    }

    collection.available_files = file_set.into_iter().collect();
    store_parsed_table("financial_metadata", collection);
    Ok(())
}