use std::collections::{HashMap, BTreeSet};
use serde_json::{json, Value};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use parquet::file::reader::{FileReader, SerializedFileReader};
use parquet::record::RowAccessor;
use crate::pipeline::{WorkspaceModule, WorkspaceDataContext, CatalogItem};

pub struct BalanceSheetCard;

struct HierarchyRowConfig {
    tag_name: &'static str,
    is_parent: bool,
    parent_id: &'static str,
}

// ─────────────────────────────────────────────────────────────────────────────
// 🚀 SELF-CONTAINED UTILITY FUNCTIONS
// ─────────────────────────────────────────────────────────────────────────────

fn transform_camel_case(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && ch.is_uppercase() {
            result.push(' ');
        }
        if i == 0 {
            result.push(ch.to_ascii_uppercase());
        } else {
            result.push(ch);
        }
    }
    result
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

// ─────────────────────────────────────────────────────────────────────────────
// 🚀 WORKSPACE TRAIT IMPLEMENTATION LAYER
// ─────────────────────────────────────────────────────────────────────────────
impl WorkspaceModule for BalanceSheetCard {
    fn catalog_definition(&self) -> CatalogItem {
        CatalogItem {
            id: "balance_sheet_financials".to_string(),
            name: "Balance Sheet".to_string(),
            description: "Interactive Ind-As Schedule III Balance Sheet snapshot matrix timeline.".to_string(),
        }
    }

    fn compile(
        &self, 
        _ticker: &str, 
        timeframe: &str, 
        data: &WorkspaceDataContext
    ) -> Result<Value, String> {

        // 🛠️ DYNAMIC LOADING LAYER: Direct extraction from the centralized base64 parquet registry
        let parquet_raw_payload = data.get_dataset("parquets/nse_corporates-financial-results.parquet");
        let mut raw_records: Vec<(String, String, String, String)> = Vec::new();

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
                                    let context_id = row.get_string(2).map(|s| s.to_string()).unwrap_or_default();
                                    let raw_value = row.get_string(4).map(|s| s.to_string()).unwrap_or_default();
                                    raw_records.push((source_file, tag_name, context_id, raw_value));
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

        // 🛠️ METADATA EXTRACTION LAYER
        let mut available_types_set = BTreeSet::new();
        let mut file_to_date: HashMap<String, String> = HashMap::new();
        let mut file_to_type: HashMap<String, String> = HashMap::new();
        let mut file_context_to_date: HashMap<(String, String), String> = HashMap::new();

        for (source_file, tag_name, context_id, raw_value) in &raw_records {
            let val = raw_value.trim().to_string();
            if val.is_empty() || val == "NA" { continue; }

            if tag_name == "DateOfEndOfReportingPeriod" {
                file_to_date.insert(source_file.clone(), val.clone());
                file_context_to_date.insert((source_file.clone(), context_id.clone()), val);
            } else if tag_name == "NatureOfReportStandaloneConsolidated" {
                let report_type_upper = val.to_uppercase();
                file_to_type.insert(source_file.clone(), report_type_upper.clone());
                available_types_set.insert(report_type_upper);
            }
        }

        // 📊 STEP 1: DYNAMIC PERSPECTIVE & FREQUENCY DROPDOWN DECODER
        let mut available_types: Vec<String> = available_types_set.into_iter().collect();
        if available_types.is_empty() {
            available_types = vec!["CONSOLIDATED".to_string(), "STANDALONE".to_string()];
        }

        let mut active_report_type = "CONSOLIDATED".to_string();
        let mut active_period_type = "QUARTERLY".to_string();

        let raw_timeframe = timeframe.trim().to_uppercase();
        if !raw_timeframe.is_empty() {
            if raw_timeframe.contains("STANDALONE") {
                active_report_type = "STANDALONE".to_string();
            } else if raw_timeframe.contains("CONSOLIDATED") {
                active_report_type = "CONSOLIDATED".to_string();
            }

            if raw_timeframe.contains("ANNUAL") || raw_timeframe.contains("YEAR") {
                active_period_type = "ANNUALLY".to_string();
            } else if raw_timeframe.contains("QUARTER") {
                active_period_type = "QUARTERLY".to_string();
            }
        }

        if !available_types.contains(&active_report_type) {
            active_report_type = available_types[0].clone();
        }

        // 🎯 SNAPSHOT ROUTING CONTEXT: Targets instant snapshot parameters (OneI / FourI)
        let target_context_ids = if active_period_type == "ANNUALLY" {
            vec!["FourI".to_string(), "OneI".to_string()]
        } else {
            vec!["OneI".to_string()]
        };

        let current_active_select_label = format!(
            "{} - {}", 
            transform_camel_case(&active_report_type.to_lowercase()), 
            if active_period_type == "ANNUALLY" { "Annually" } else { "Quarterly" }
        );

        let mut unified_dropdown_options = Vec::new();
        for t in &available_types {
            let t_label = transform_camel_case(&t.to_lowercase());
            unified_dropdown_options.push(format!("{} - Quarterly", t_label));
            unified_dropdown_options.push(format!("{} - Annually", t_label));
        }

        // 📊 STEP 2: MATRIX TIME AXIS COORDINATE POPULATION
        let mut unique_filing_dates: Vec<String> = Vec::new();
        let mut matrix_data_map: HashMap<String, String> = HashMap::new();

        for (source_file, tag_name, context_id, raw_value) in &raw_records {
            let true_date = file_context_to_date.get(&(source_file.clone(), context_id.clone()))
                .cloned()
                .unwrap_or_else(|| file_to_date.get(source_file).cloned().unwrap_or_default());
                
            let report_type = file_to_type.get(source_file).cloned().unwrap_or_default();

            if true_date.is_empty() { continue; }

            if report_type.contains(&active_report_type) && target_context_ids.contains(context_id) {
                // MARCH SNAPSHOT THRESHOLD FOR ANNUAL VIEWS
                if active_period_type == "ANNUALLY" && !true_date.contains("-03-") {
                    continue;
                }

                if !unique_filing_dates.contains(&true_date) {
                    unique_filing_dates.push(true_date.clone());
                }
                
                let data_lookup_key = format!("{}__{}", true_date, tag_name);
                matrix_data_map.entry(data_lookup_key).or_insert_with(|| raw_value.clone());
            }
        }

        // Sort timeline columns descending (newest filings on the left)
        unique_filing_dates.sort_by(|a, b| b.cmp(a));

        // 📊 STEP 3: IND-AS SCHEDULE III SEQUENTIAL ACCOUNTING TREE STRUCTURE
        let structured_balance_sheet_tree = vec![
            // ─── NON-CURRENT ASSETS ACCORDION SUB-TIERS
            HierarchyRowConfig { tag_name: "NoncurrentAssets", is_parent: true, parent_id: "non_current_assets_group" },
            HierarchyRowConfig { tag_name: "PropertyPlantAndEquipment", is_parent: false, parent_id: "non_current_assets_group" },
            HierarchyRowConfig { tag_name: "CapitalWorkInProgress", is_parent: false, parent_id: "non_current_assets_group" },
            HierarchyRowConfig { tag_name: "InvestmentProperty", is_parent: false, parent_id: "non_current_assets_group" },
            HierarchyRowConfig { tag_name: "Goodwill", is_parent: false, parent_id: "non_current_assets_group" },
            HierarchyRowConfig { tag_name: "OtherIntangibleAssets", is_parent: false, parent_id: "non_current_assets_group" },
            HierarchyRowConfig { tag_name: "IntangibleAssetsUnderDevelopment", is_parent: false, parent_id: "non_current_assets_group" },
            HierarchyRowConfig { tag_name: "BiologicalAssetsOtherThanBearerPlants", is_parent: false, parent_id: "non_current_assets_group" },
            HierarchyRowConfig { tag_name: "InvestmentsAccountedForUsingEquityMethod", is_parent: false, parent_id: "non_current_assets_group" },
            HierarchyRowConfig { tag_name: "NoncurrentInvestments", is_parent: false, parent_id: "non_current_assets_group" },
            HierarchyRowConfig { tag_name: "TradeReceivablesNoncurrent", is_parent: false, parent_id: "non_current_assets_group" },
            HierarchyRowConfig { tag_name: "LoansNoncurrent", is_parent: false, parent_id: "non_current_assets_group" },
            HierarchyRowConfig { tag_name: "OtherNoncurrentFinancialAssets", is_parent: false, parent_id: "non_current_assets_group" },
            HierarchyRowConfig { tag_name: "NoncurrentFinancialAssets", is_parent: false, parent_id: "non_current_assets_group" },
            HierarchyRowConfig { tag_name: "DeferredTaxAssetsNet", is_parent: false, parent_id: "non_current_assets_group" },
            HierarchyRowConfig { tag_name: "OtherNoncurrentAssets", is_parent: false, parent_id: "non_current_assets_group" },

            // ─── CURRENT ASSETS ACCORDION SUB-TIERS
            HierarchyRowConfig { tag_name: "CurrentAssets", is_parent: true, parent_id: "current_assets_group" },
            HierarchyRowConfig { tag_name: "Inventories", is_parent: false, parent_id: "current_assets_group" },
            HierarchyRowConfig { tag_name: "CurrentInvestments", is_parent: false, parent_id: "current_assets_group" },
            HierarchyRowConfig { tag_name: "TradeReceivablesCurrent", is_parent: false, parent_id: "current_assets_group" },
            HierarchyRowConfig { tag_name: "CashAndCashEquivalents", is_parent: false, parent_id: "current_assets_group" },
            HierarchyRowConfig { tag_name: "BankBalanceOtherThanCashAndCashEquivalents", is_parent: false, parent_id: "current_assets_group" },
            HierarchyRowConfig { tag_name: "LoansCurrent", is_parent: false, parent_id: "current_assets_group" },
            HierarchyRowConfig { tag_name: "OtherCurrentFinancialAssets", is_parent: false, parent_id: "current_assets_group" },
            HierarchyRowConfig { tag_name: "CurrentFinancialAssets", is_parent: false, parent_id: "current_assets_group" },
            HierarchyRowConfig { tag_name: "CurrentTaxAssets", is_parent: false, parent_id: "current_assets_group" },
            HierarchyRowConfig { tag_name: "OtherCurrentAssets", is_parent: false, parent_id: "current_assets_group" },
            HierarchyRowConfig { tag_name: "NoncurrentAssetsClassifiedAsHeldForSale", is_parent: false, parent_id: "current_assets_group" },
            HierarchyRowConfig { tag_name: "RegulatoryDeferralAccountDebitBalancesAndRelatedDeferredTaxAssets", is_parent: false, parent_id: "current_assets_group" },

            // ─── GRAND TOTAL ASSETS (ROOT ANCHOR LINE)
            HierarchyRowConfig { tag_name: "Assets", is_parent: false, parent_id: "" },

            // ─── EQUITY ACCORDION CLUSTER
            HierarchyRowConfig { tag_name: "Equity", is_parent: true, parent_id: "equity_group" },
            HierarchyRowConfig { tag_name: "EquityShareCapital", is_parent: false, parent_id: "equity_group" },
            HierarchyRowConfig { tag_name: "OtherEquity", is_parent: false, parent_id: "equity_group" },
            HierarchyRowConfig { tag_name: "EquityAttributableToOwnersOfParent", is_parent: false, parent_id: "equity_group" },
            HierarchyRowConfig { tag_name: "NonControllingInterest", is_parent: false, parent_id: "equity_group" },

            // ─── NON-CURRENT LIABILITIES ACCORDION SUB-TIERS
            HierarchyRowConfig { tag_name: "NoncurrentLiabilities", is_parent: true, parent_id: "non_current_liab_group" },
            HierarchyRowConfig { tag_name: "BorrowingsNoncurrent", is_parent: false, parent_id: "non_current_liab_group" },
            HierarchyRowConfig { tag_name: "TradePayablesNoncurrent", is_parent: false, parent_id: "non_current_liab_group" },
            HierarchyRowConfig { tag_name: "OtherNoncurrentFinancialLiabilities", is_parent: false, parent_id: "non_current_liab_group" },
            HierarchyRowConfig { tag_name: "NoncurrentFinancialLiabilities", is_parent: false, parent_id: "non_current_liab_group" },
            HierarchyRowConfig { tag_name: "ProvisionsNoncurrent", is_parent: false, parent_id: "non_current_liab_group" },
            HierarchyRowConfig { tag_name: "DeferredTaxLiabilitiesNet", is_parent: false, parent_id: "non_current_liab_group" },
            HierarchyRowConfig { tag_name: "DeferredGovernmentGrantsNoncurrent", is_parent: false, parent_id: "non_current_liab_group" },
            HierarchyRowConfig { tag_name: "OtherNoncurrentLiabilities", is_parent: false, parent_id: "non_current_liab_group" },

            // ─── CURRENT LIABILITIES ACCORDION SUB-TIERS
            HierarchyRowConfig { tag_name: "CurrentLiabilities", is_parent: true, parent_id: "current_liab_group" },
            HierarchyRowConfig { tag_name: "BorrowingsCurrent", is_parent: false, parent_id: "current_liab_group" },
            HierarchyRowConfig { tag_name: "TradePayablesCurrent", is_parent: false, parent_id: "current_liab_group" },
            HierarchyRowConfig { tag_name: "OtherCurrentFinancialLiabilities", is_parent: false, parent_id: "current_liab_group" },
            HierarchyRowConfig { tag_name: "CurrentFinancialLiabilities", is_parent: false, parent_id: "current_liab_group" },
            HierarchyRowConfig { tag_name: "OtherCurrentLiabilities", is_parent: false, parent_id: "current_liab_group" },
            HierarchyRowConfig { tag_name: "ProvisionsCurrent", is_parent: false, parent_id: "current_liab_group" },

            // ─── CORE BALANCING OVERHEADS
            HierarchyRowConfig { tag_name: "Liabilities", is_parent: false, parent_id: "" },
            HierarchyRowConfig { tag_name: "EquityAndLiabilities", is_parent: false, parent_id: "" },

            // ─── EXTRA DATA / FOOTNOTES
            HierarchyRowConfig { tag_name: "UnAllocableAssets", is_parent: false, parent_id: "" },
            HierarchyRowConfig { tag_name: "NetSegmentAssets", is_parent: false, parent_id: "" },
            HierarchyRowConfig { tag_name: "SegmentLiabilities", is_parent: false, parent_id: "" },
            HierarchyRowConfig { tag_name: "UnAllocableLiabilities", is_parent: false, parent_id: "" },
            HierarchyRowConfig { tag_name: "NetSegmentLiabilities", is_parent: false, parent_id: "" },
        ];

        // 📊 STEP 4: DATA MATRIX GRID ROW COMPILER
        let mut compiled_rows = Vec::new();
        let mut table_headers = vec!["Schedule III Balance Metric Component".to_string()];
        
        for date in &unique_filing_dates {
            table_headers.push(date.clone());
        }

        for config in &structured_balance_sheet_tree {
            let mut row_cells = Vec::new();
            let clean_row_header = transform_camel_case(config.tag_name);

            row_cells.push(json!({ "type": "text", "value": clean_row_header }));

            for date in &unique_filing_dates {
                let lookup_key = format!("{}__{}", date, config.tag_name);
                let raw_amount = matrix_data_map.get(&lookup_key).map(|s| s.as_str()).unwrap_or("");
                let formatted_amount = format_financial_number(raw_amount);

                row_cells.push(json!({ "type": "text", "value": formatted_amount }));
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

        // 📊 STEP 5: OUTPUT ACCORDION-SUPPORTED UI SCHEMATICS
        Ok(json!({
            "type": "card",
            "title": format!("{} Balance Sheet ({})", transform_camel_case(&active_report_type.to_lowercase()), if active_period_type == "ANNUALLY" { "Annually" } else { "Quarterly" }),
            "subtitle": format!("// SEBI SELECTION MATRIX // {} Snapshots", current_active_select_label),
            "footer": format!("Total active metrics tracked: {} rows spanning {} historical intervals", structured_balance_sheet_tree.len(), unique_filing_dates.len()),
            "children": [
                {
                    "type": "container",
                    "className": "flex flex-row justify-between items-center w-full mt-1 mb-4 pointer-events-auto",
                    "style": { "display": "flex", "flexDirection": "row", "justifyContent": "between" },
                    "children": [
                        {
                            "type": "text",
                            "className": "text-xs font-semibold font-mono uppercase opacity-60 text-neutral-400", 
                            "value": "Balance Sheet Perspective Control Panel:"
                        },
                        {
                            "type": "select",
                            "action_target": "balance_sheet_financials", 
                            "default_value": current_active_select_label,
                            "options": unified_dropdown_options
                        }
                    ]
                },
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