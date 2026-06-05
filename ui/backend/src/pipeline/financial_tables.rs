use std::collections::{BTreeSet};
use serde_json::{json, Value};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use parquet::file::reader::{FileReader, SerializedFileReader};
use parquet::record::RowAccessor;
use crate::pipeline::{WorkspaceModule, WorkspaceDataContext, CatalogItem};

pub struct FinancialTablesCard;

struct AnnualRow {
    file_name: String,
    particulars: String,
    curr_year: String,
    prev_year: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// 🚀 SELF-CONTAINED UTILITY FUNCTION (WITH DIAGNOSTIC ROUTING)
// ─────────────────────────────────────────────────────────────────────────────

/// Streams records from an annual report Parquet asset using a comprehensive path lookup grid
/// and hardcoded indices matching your Polars schema printout.
fn load_annual_parquet(
    data: &WorkspaceDataContext, 
    path: &str, 
    unique_years: &mut BTreeSet<String>
) -> Result<Vec<AnnualRow>, String> {
    // Read the exact path key directly from the workspace context
    let parquet_raw_payload = data.get_dataset(path);

    let b64_str = match parquet_raw_payload["bytes_base64"].as_str() {
        Some(s) => s,
        None => return Err(format!("Dataset path key not found in Workspace Context: {}", path)),
    };

    let vec_bytes = match STANDARD.decode(b64_str) {
        Ok(b) => b,
        Err(e) => return Err(format!("Base64 decode failed for path '{}': {}", path, e)),
    };

    let bytes_container = bytes::Bytes::from(vec_bytes);
    let file_reader = match SerializedFileReader::new(bytes_container) {
        Ok(r) => r,
        Err(e) => return Err(format!("Failed to create Parquet reader for path '{}': {}", path, e)),
    };

    let mut records = Vec::new();
    let num_groups = file_reader.metadata().num_row_groups();
    let mut row_group_idx = 0;
    
    while row_group_idx < num_groups {
        if let Ok(group) = file_reader.get_row_group(row_group_idx) {
            if let Ok(mut row_iter) = group.get_row_iter(None) {
                while let Some(Ok(row)) = row_iter.next() {
                    let file_name = row.get_string(0).map(|s| s.to_string()).unwrap_or_default();
                    if !file_name.is_empty() {
                        unique_years.insert(file_name.clone());
                    }

                    let particulars = row.get_string(3).map(|s| s.to_string()).unwrap_or_default();
                    let curr_year = row.get_string(5).map(|s| s.to_string()).unwrap_or_default();
                    let prev_year = row.get_string(6).map(|s| s.to_string()).unwrap_or_default();

                    records.push(AnnualRow {
                        file_name,
                        particulars,
                        curr_year,
                        prev_year,
                    });
                }
            }
        }
        row_group_idx += 1;
    }

    Ok(records)
}

// ─────────────────────────────────────────────────────────────────────────────
// 🚀 WORKSPACE TRAIT IMPLEMENTATION LAYER
// ─────────────────────────────────────────────────────────────────────────────

impl WorkspaceModule for FinancialTablesCard {
    fn catalog_definition(&self) -> CatalogItem {
        CatalogItem {
            id: "annual_financial_tables".to_string(),
            name: "Annual Financial Tables".to_string(),
            description: "Displays clean flat tables for the Income Statement, Balance Sheet, and Cash Flow Statement parsed from annual reports.".to_string(),
        }
    }

    fn compile(
        &self, 
        _ticker: &str, 
        timeframe: &str, 
        data: &WorkspaceDataContext
    ) -> Result<Value, String> {

        let mut unique_years = BTreeSet::new();

        // 📊 PASS 1: LOAD RECORDS DYNAMICALLY USING THE QUESTION MARK (?) OPERATOR TO PROPAGATE ERRORS
        let balance_records = load_annual_parquet(data, "parquets/annual_report/balance_sheet.parquet", &mut unique_years)?;
        let income_records = load_annual_parquet(data, "parquets/annual_report/income_statement.parquet", &mut unique_years)?;
        let cash_records = load_annual_parquet(data, "parquets/annual_report/cash_flow.parquet", &mut unique_years)?;

        if unique_years.is_empty() {
            return Err("Zero unique years parsed from annual report financial registries".to_string());
        }

        // 📊 STEP 2: REVERSE CHRONOLOGICAL SORT FOR DROPDOWN (NEWEST YEAR FIRST)
        let mut dropdown_options: Vec<String> = unique_years.iter().cloned().collect();
        dropdown_options.sort_by(|a, b| b.cmp(a));

        // Resolve active year choice matching selection parameters safely
        let mut active_year = timeframe.trim().to_string();
        if active_year.is_empty() || !unique_years.contains(&active_year) {
            active_year = dropdown_options[0].clone(); // Fallback to the latest available year
        }

        // 📊 STEP 3: CONSTRUCT GRID ROWS FOR THE ACTIVE SELECTED FINANCIAL YEAR
        let table_headers = vec!["Particulars".to_string(), "Current Year".to_string(), "Previous Year".to_string()];

        // 1. Compile Income Statement Table rows
        let mut income_rows = Vec::new();
        for row in &income_records {
            if row.file_name == active_year {
                income_rows.push(json!({
                    "type": "table_row",
                    "is_parent": false,
                    "is_child": false,
                    "parent_id": Option::<String>::None,
                    "align_right_values": true,
                    "cells": [
                        { "type": "text", "value": row.particulars.trim() },
                        { "type": "text", "value": row.curr_year.trim() },
                        { "type": "text", "value": row.prev_year.trim() }
                    ]
                }));
            }
        }

        // 2. Compile Balance Sheet Table rows
        let mut balance_rows = Vec::new();
        for row in &balance_records {
            if row.file_name == active_year {
                balance_rows.push(json!({
                    "type": "table_row",
                    "is_parent": false,
                    "is_child": false,
                    "parent_id": Option::<String>::None,
                    "align_right_values": true,
                    "cells": [
                        { "type": "text", "value": row.particulars.trim() },
                        { "type": "text", "value": row.curr_year.trim() },
                        { "type": "text", "value": row.prev_year.trim() }
                    ]
                }));
            }
        }

        // 3. Compile Cash Flow Table rows
        let mut cash_rows = Vec::new();
        for row in &cash_records {
            if row.file_name == active_year {
                cash_rows.push(json!({
                    "type": "table_row",
                    "is_parent": false,
                    "is_child": false,
                    "parent_id": Option::<String>::None,
                    "align_right_values": true,
                    "cells": [
                        { "type": "text", "value": row.particulars.trim() },
                        { "type": "text", "value": row.curr_year.trim() },
                        { "type": "text", "value": row.prev_year.trim() }
                    ]
                }));
            }
        }

        // 📊 STEP 4: OUTPUT CLEAN MULTI-TABLE DATA CARD SCHEMATICS
        Ok(json!({
            "type": "card",
            "title": format!("Annual Financial Statements ({})", active_year.replace(".md", "")),
            "subtitle": "// HISTORICAL ANNUAL REPORT REGISTRY DATA MATRIX // UNFILTERED FLAT VIEW //",
            "footer": format!("Displaying raw data tables for active year context: {}", active_year),
            "children": [
                // Dropdown control wrapper container
                {
                    "type": "container",
                    "className": "flex flex-row justify-between items-center w-full mt-1 mb-6 pointer-events-auto",
                    "style": { "display": "flex", "flexDirection": "row", "justifyContent": "between" },
                    "children": [
                        {
                            "type": "text",
                            "className": "text-xs font-semibold font-mono uppercase opacity-60 text-neutral-400", 
                            "value": "Select Reporting Period Target:"
                        },
                        {
                            "type": "select",
                            "action_target": "annual_financial_tables", // Frontend callback handler target hook
                            "default_value": active_year,
                            "options": dropdown_options
                        }
                    ]
                },

                // ─── 1. INCOME STATEMENT VIEW SEC ─────────────────────────────────────
                {
                    "type": "text",
                    "className": "text-sm font-bold text-neutral-200 mt-4 mb-2 tracking-wide uppercase opacity-90 border-b border-neutral-800 pb-1 w-full",
                    "value": "📈 Profit & Loss Account (Income Statement)"
                },
                {
                    "type": "container",
                    "className": "w-full overflow-x-auto overflow-y-visible mb-8",
                    "children": [
                        {
                            "type": "table",
                            "className": "w-full min-w-[750px] text-left border-collapse",
                            "headers": table_headers,
                            "align_right_columns": true,
                            "children": income_rows
                        }
                    ]
                },

                // ─── 2. BALANCE SHEET VIEW SEC ────────────────────────────────────────
                {
                    "type": "text",
                    "className": "text-sm font-bold text-neutral-200 mt-4 mb-2 tracking-wide uppercase opacity-90 border-b border-neutral-800 pb-1 w-full",
                    "value": "📦 Balance Sheet Snapshot Matrix"
                },
                {
                    "type": "container",
                    "className": "w-full overflow-x-auto overflow-y-visible mb-8",
                    "children": [
                        {
                            "type": "table",
                            "className": "w-full min-w-[750px] text-left border-collapse",
                            "headers": table_headers,
                            "align_right_columns": true,
                            "children": balance_rows
                        }
                    ]
                },

                // ─── 3. CASH FLOW STATEMENT VIEW SEC ──────────────────────────────────
                {
                    "type": "text",
                    "className": "text-sm font-bold text-neutral-200 mt-4 mb-2 tracking-wide uppercase opacity-90 border-b border-neutral-800 pb-1 w-full",
                    "value": "💸 Cash Flow Statement Summary"
                },
                {
                    "type": "container",
                    "className": "w-full overflow-x-auto overflow-y-visible mb-4",
                    "children": [
                        {
                            "type": "table",
                            "className": "w-full min-w-[750px] text-left border-collapse",
                            "headers": table_headers,
                            "align_right_columns": true,
                            "children": cash_rows
                        }
                    ]
                }
            ]
        }))
    }
}