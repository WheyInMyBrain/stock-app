use serde_json::{json, Value};
use std::collections::HashMap;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use parquet::file::reader::FileReader;
use parquet::file::serialized_reader::SerializedFileReader;
use parquet::record::RowAccessor;
use crate::commands::pipeline::CatalogItem;
use crate::pipeline::{WorkspaceModule, WorkspaceDataContext};

pub struct QuarterlyFinancialsCard;

fn transform_camel_case(input: &str) -> String {
    let mut result = String::new();
    for (i, ch) in input.chars().enumerate() {
        if i > 0 && ch.is_uppercase() {
            result.push(' ');
        }
        result.push(ch);
    }
    result
}

fn map_filing_to_sortable_score(filename: &str) -> u32 {
    let clean_name = filename.replace(".xml", "");
    let segments: Vec<&str> = clean_name.split('-').collect();
    if segments.len() != 3 { return 0; }

    let day = segments[0].parse::<u32>().unwrap_or(0);
    let year = segments[2].parse::<u32>().unwrap_or(0);
    
    let month = match segments[1].to_uppercase().as_str() {
        "JAN" => 1, "FEB" => 2, "MAR" => 3, "APR" => 4, "MAY" => 5, "JUN" => 6,
        "JUL" => 7, "AUG" => 8, "SEP" => 9, "OCT" => 10, "NOV" => 11, "DEC" => 12,
        _ => 0
    };

    (year * 10000) + (month * 100) + day
}

fn format_financial_number(raw_val: &str) -> String {
    let trimmed = raw_val.trim();
    if trimmed.is_empty() || trimmed == "NaN" || trimmed == "NA" {
        return "-".to_string();
    }
    if let Ok(value_float) = trimmed.parse::<f64>() {
        if value_float == 0.0 {
            return "0.00".to_string();
        }
        if value_float.abs() >= 10_000_000.0 {
            return format!("{:.2} Cr", value_float / 10_000_000.0);
        } else if value_float.abs() >= 100_000.0 {
            return format!("{:.2} Lk", value_float / 100_000.0);
        }
        return format!("{:.2}", value_float);
    }
    trimmed.to_string()
}

struct HierarchyRowConfig {
    tag_name: &'static str,
    is_parent: bool,
    parent_id: &'static str,
}

impl WorkspaceModule for QuarterlyFinancialsCard {
    fn catalog_definition(&self) -> CatalogItem {
        CatalogItem {
            id: "quarterly_financials".to_string(),
            name: "Quarterly Financial Tree".to_string(),
            description: "Aggregates corporate earnings statements into a date-wise matrix with collapsible arithmetic dropdown groupings.".to_string(),
        }
    }

    fn compile(&self, _ticker: &str, _timeframe: &str, data: &WorkspaceDataContext) -> Result<Value, String> {
        let parquet_raw_payload = data.get_dataset("parquets/nse_corporates-financial-results.parquet");
        let mut raw_records: Vec<(String, String, String)> = Vec::new();

        if let Some(b64_str) = parquet_raw_payload["bytes_base64"].as_str() {
            if let Ok(vec_bytes) = STANDARD.decode(b64_str) {
                let bytes_container = bytes::Bytes::from(vec_bytes);
                if let Ok(file_reader) = SerializedFileReader::new(bytes_container) {
                    let num_groups = file_reader.metadata().num_row_groups();
                    let mut row_group_idx = 0;
                    while row_group_idx < num_groups {
                        if let Ok(group) = file_reader.get_row_group(row_group_idx) {
                            if let Ok(mut row_iter) = group.get_row_iter(None) {
                                while let Some(Ok(row)) = row_iter.next() {
                                    let source_file = row.get_string(0).map(|s| s.to_string()).unwrap_or_default();
                                    let tag_name = row.get_string(1).map(|s| s.to_string()).unwrap_or_default();
                                    let raw_value = row.get_string(4).map(|s| s.to_string()).unwrap_or_default();
                                    raw_records.push((source_file, tag_name, raw_value));
                                }
                            }
                        }
                        row_group_idx += 1;
                    }
                }
            }
        }

        if raw_records.is_empty() {
            return Err("Zero data records parsed from financial registry Parquet".to_string());
        }

        let mut unique_filing_dates: Vec<String> = Vec::new();
        let mut matrix_data_map: HashMap<String, String> = HashMap::new();

        for (source_file, tag_name, raw_value) in &raw_records {
            let clean_file = source_file.replace(".xml", "");
            let parts: Vec<&str> = clean_file.split('_').collect();
            if parts.len() != 2 { continue; }

            let filing_date = parts[0].to_string();
            let report_type = parts[1].to_uppercase();

            if report_type == "CONSOLIDATED" {
                if !unique_filing_dates.contains(&filing_date) {
                    unique_filing_dates.push(filing_date.clone());
                }
                let data_lookup_key = format!("{}__{}", filing_date, tag_name);
                matrix_data_map.entry(data_lookup_key).or_insert_with(|| raw_value.clone());
            }
        }

        unique_filing_dates.sort_by(|a, b| map_filing_to_sortable_score(b).cmp(&map_filing_to_sortable_score(a)));

        // Unaltered accounting sequence matching our master complete checklist
        let structured_accounting_tree = vec![
            HierarchyRowConfig { tag_name: "Income", is_parent: true, parent_id: "income_group" },
            HierarchyRowConfig { tag_name: "RevenueFromOperations", is_parent: false, parent_id: "income_group" },
            HierarchyRowConfig { tag_name: "OtherIncome", is_parent: false, parent_id: "income_group" },

            HierarchyRowConfig { tag_name: "Expenses", is_parent: true, parent_id: "expense_group" },
            HierarchyRowConfig { tag_name: "CostOfMaterialsConsumed", is_parent: false, parent_id: "expense_group" },
            HierarchyRowConfig { tag_name: "PurchasesOfStockInTrade", is_parent: false, parent_id: "expense_group" },
            HierarchyRowConfig { tag_name: "ChangesInInventoriesOfFinishedGoodsWorkInProgressAndStockInTrade", is_parent: false, parent_id: "expense_group" },
            HierarchyRowConfig { tag_name: "EmployeeBenefitExpense", is_parent: false, parent_id: "expense_group" },
            HierarchyRowConfig { tag_name: "FinanceCosts", is_parent: false, parent_id: "expense_group" },
            HierarchyRowConfig { tag_name: "DepreciationDepletionAndAmortisationExpense", is_parent: false, parent_id: "expense_group" },
            HierarchyRowConfig { tag_name: "OtherExpenses", is_parent: false, parent_id: "expense_group" },

            HierarchyRowConfig { tag_name: "ProfitBeforeExceptionalItemsAndTax", is_parent: false, parent_id: "" },
            HierarchyRowConfig { tag_name: "ExceptionalItemsBeforeTax", is_parent: false, parent_id: "" },
            HierarchyRowConfig { tag_name: "ProfitBeforeTax", is_parent: false, parent_id: "" },

            HierarchyRowConfig { tag_name: "TaxExpense", is_parent: true, parent_id: "tax_group" },
            HierarchyRowConfig { tag_name: "CurrentTax", is_parent: false, parent_id: "tax_group" },
            HierarchyRowConfig { tag_name: "DeferredTax", is_parent: false, parent_id: "tax_group" },

            HierarchyRowConfig { tag_name: "ProfitLossForPeriod", is_parent: true, parent_id: "profit_group" },
            HierarchyRowConfig { tag_name: "ProfitLossForPeriodFromContinuingOperations", is_parent: false, parent_id: "profit_group" },
            HierarchyRowConfig { tag_name: "NetMovementInRegulatoryDeferralAccountBalancesRelatedToProfitOrLossAndTheRelatedDeferredTaxMovement", is_parent: false, parent_id: "profit_group" },
            HierarchyRowConfig { tag_name: "ShareOfProfitLossOfAssociatesAndJointVenturesAccountedForUsingEquityMethod", is_parent: false, parent_id: "profit_group" },

            HierarchyRowConfig { tag_name: "ProfitLossFromDiscontinuedOperationsBeforeTax", is_parent: true, parent_id: "discontinued_group" },
            HierarchyRowConfig { tag_name: "TaxExpenseOfDiscontinuedOperations", is_parent: false, parent_id: "discontinued_group" },
            HierarchyRowConfig { tag_name: "ProfitLossFromDiscontinuedOperationsAfterTax", is_parent: false, parent_id: "discontinued_group" },

            HierarchyRowConfig { tag_name: "ComprehensiveIncomeForThePeriod", is_parent: true, parent_id: "comprehensive_group" },
            HierarchyRowConfig { tag_name: "OtherComprehensiveIncomeNetOfTaxes", is_parent: false, parent_id: "comprehensive_group" },
            HierarchyRowConfig { tag_name: "ComprehensiveIncomeForThePeriodAttributableToOwnersOfParent", is_parent: false, parent_id: "comprehensive_group" },
            HierarchyRowConfig { tag_name: "ComprehensiveIncomeForThePeriodAttributableToOwnersOfParentNonControllingInterests", is_parent: false, parent_id: "comprehensive_group" },

            HierarchyRowConfig { tag_name: "PaidUpValueOfEquityShareCapital", is_parent: true, parent_id: "capital_group" },
            HierarchyRowConfig { tag_name: "FaceValueOfEquityShareCapital", is_parent: false, parent_id: "capital_group" },

            HierarchyRowConfig { tag_name: "BasicEarningsLossPerShareFromContinuingAndDiscontinuedOperations", is_parent: true, parent_id: "eps_group" },
            HierarchyRowConfig { tag_name: "BasicEarningsLossPerShareFromContinuingOperations", is_parent: false, parent_id: "eps_group" },
            HierarchyRowConfig { tag_name: "DilutedEarningsLossPerShareFromContinuingOperations", is_parent: false, parent_id: "eps_group" },
            HierarchyRowConfig { tag_name: "BasicEarningsLossPerShareFromDiscontinuedOperations", is_parent: false, parent_id: "eps_group" },
            HierarchyRowConfig { tag_name: "DilutedEarningsLossPerShareFromDiscontinuedOperations", is_parent: false, parent_id: "eps_group" },
            HierarchyRowConfig { tag_name: "DilutedEarningsLossPerShareFromContinuingAndDiscontinuedOperations", is_parent: false, parent_id: "eps_group" },
        ];

        let mut compiled_rows = Vec::new();

        for config in structured_accounting_tree {
            let mut row_cells = Vec::new();
            let clean_row_header = transform_camel_case(config.tag_name);

            row_cells.push(json!({
                "type": "text",
                "value": clean_row_header
            }));

            for date in &unique_filing_dates {
                let lookup_key = format!("{}__{}", date, config.tag_name);
                let raw_amount = matrix_data_map.get(&lookup_key).map(|s| s.as_str()).unwrap_or("");
                let formatted_amount = format_financial_number(raw_amount);

                row_cells.push(json!({
                    "type": "text",
                    "value": formatted_amount
                }));
            }

            let has_parent_group = !config.parent_id.is_empty();
            let is_child_row = has_parent_group && !config.is_parent;

            compiled_rows.push(json!({
                "type": "table_row",
                "is_parent": config.is_parent,
                "is_child": is_child_row,
                "parent_id": if has_parent_group { Some(config.parent_id.to_string()) } else { None },
                "align_right_values": true,
                "cells": row_cells
            }));
        }

        let mut table_headers = vec!["Financial Line Item".to_string()];
        table_headers.extend(unique_filing_dates.clone());

        Ok(json!({
            "type": "card",
            "title": "Consolidated Financial Performance Tree",
            "subtitle": "// INTERACTIVE DROPDOWN ACCOUNTING MATRIX // CONSOLIDATED TIME LOG",
            "footer": format!("Total active tracked accounting metrics: 36 tags spanning {} quarters", unique_filing_dates.len()),
            "children": [
                {
                    "type": "container",
                    "className": "w-full overflow-x-auto overflow-y-visible",
                    "children": [
                        {
                            "type": "table",
                            "className": "w-full min-w-[750px] text-left border-collapse",
                            "headers": table_headers,
                            "align_right_columns": true,
                            "children": compiled_rows
                        }
                    ]
                }
            ]
        }))
    }
}