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

// 🚀 INDICATOR BADGE UTILITY: Formats clean directional subtext layout properties
fn format_growth_badge(value: f64) -> String {
    if value.is_nan() || value.is_infinite() || value == 0.0 {
        "0.0%".to_string()
    } else if value > 0.0 {
        format!("▲+{:.1}%", value)
    } else {
        format!("▼{:.1}%", value)
    }
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

        // Helper function to extract numerical values safely
        let get_float_val = |date: &str, tag: &str| -> f64 {
            let lookup_key = format!("{}__{}", date, tag);
            matrix_data_map.get(&lookup_key)
                .and_then(|s| s.trim().parse::<f64>().ok())
                .unwrap_or(0.0)
        };

        let mut velocity_indicators = Vec::new();
        let mut anomalies_and_warnings = Vec::new();

        if unique_filing_dates.len() >= 2 {
            let latest_date = &unique_filing_dates[0];
            let prev_date = &unique_filing_dates[1];

            let revenue_latest = get_float_val(latest_date, "RevenueFromOperations");
            let revenue_prev = get_float_val(prev_date, "RevenueFromOperations");
            let expenses_latest = get_float_val(latest_date, "Expenses");
            let expenses_prev = get_float_val(prev_date, "Expenses");
            let profit_latest = get_float_val(latest_date, "ProfitLossForPeriod");
            let profit_prev = get_float_val(prev_date, "ProfitLossForPeriod");

            let rev_growth = if revenue_prev != 0.0 { (revenue_latest - revenue_prev) / revenue_prev * 100.0 } else { 0.0 };
            let exp_growth = if expenses_prev != 0.0 { (expenses_latest - expenses_prev) / expenses_prev * 100.0 } else { 0.0 };
            let profit_growth = if profit_prev != 0.0 { (profit_latest - profit_prev) / profit_prev * 100.0 } else { 0.0 };

            velocity_indicators.push(json!({
                "type": "metric",
                "title": "Revenue QoQ Velocity",
                "value": format!("{:.2}%", rev_growth),
                "variant": if rev_growth >= 0.0 { "success" } else { "danger" }
            }));
            velocity_indicators.push(json!({
                "type": "metric",
                "title": "Net Profit QoQ Velocity",
                "value": format!("{:.2}%", profit_growth),
                "variant": if profit_growth >= 0.0 { "success" } else { "danger" }
            }));

            if unique_filing_dates.len() >= 3 {
                let third_date = &unique_filing_dates[2];
                let profit_third = get_float_val(third_date, "ProfitLossForPeriod");
                if profit_latest < profit_prev && profit_prev < profit_third {
                    anomalies_and_warnings.push(json!({
                        "type": "text",
                        "className": "text-xs font-mono text-red-400 bg-red-950/20 px-2 py-1 rounded border border-red-900/30",
                        "value": "⚠️ ALERT: Consecutive multi-quarter Net Profit contraction streak observed."
                    }));
                } else if profit_latest > profit_prev && profit_prev > profit_third {
                    velocity_indicators.push(json!({
                        "type": "metric",
                        "title": "Earning Trajectory",
                        "value": "Expansion Streak 🚀"
                    }));
                }
            }

            if exp_growth > rev_growth && rev_growth < 5.0 {
                anomalies_and_warnings.push(json!({
                    "type": "text",
                    "className": "text-xs font-mono text-amber-400 bg-amber-950/20 px-2 py-1 rounded border border-amber-900/30",
                    "value": format!("⚠️ NEGATIVE JAWS ALERT: Expense growth ({:.1}%) is outpacing Revenue velocity ({:.1}%).", exp_growth, rev_growth)
                }));
            }

            let pbt_latest = get_float_val(latest_date, "ProfitBeforeTax");
            let tax_latest = get_float_val(latest_date, "TaxExpense");
            if pbt_latest > 0.0 && tax_latest <= 0.0 {
                anomalies_and_warnings.push(json!({
                    "type": "text",
                    "className": "text-xs font-mono text-red-400 bg-red-950/20 px-2 py-1 rounded border border-red-900/30",
                    "value": "⚠️ TAX DISCONNECT: Positive Pre-Tax Profit registered alongside zero or negative tax provisions."
                }));
            }

            let basic_eps = get_float_val(latest_date, "BasicEarningsLossPerShareFromContinuingOperations");
            let diluted_eps = get_float_val(latest_date, "DilutedEarningsLossPerShareFromContinuingOperations");
            if basic_eps > 0.0 && (basic_eps - diluted_eps) / basic_eps > 0.05 {
                anomalies_and_warnings.push(json!({
                    "type": "text",
                    "className": "text-xs font-mono text-blue-400 bg-blue-950/20 px-2 py-1 rounded border border-blue-900/30",
                    "value": format!("⚠️ CAPITAL DILUTION RISK: Diluted EPS ({:.2}) lags Basic EPS ({:.2}) by over 5%.", diluted_eps, basic_eps)
                }));
            }
        }

        if anomalies_and_warnings.is_empty() {
            anomalies_and_warnings.push(json!({
                "type": "text",
                "className": "text-xs font-mono text-green-400 bg-green-950/20 px-2 py-1 rounded border border-green-900/30",
                "value": "✓ HEALTH LEDGER: Operational data profile within nominal analytical parameters."
            }));
        }

        let mut compiled_rows = Vec::new();

        // 🚀 INJECT FINANCIAL RATIOS GROUP AS THE FIRST COLLAPSIBLE PARENT ITEM AT THE TOP OF THE TABLE
        compiled_rows.push(json!({
            "type": "table_row",
            "is_parent": true,
            "is_child": false,
            "parent_id": Some("ratios_group".to_string()),
            "align_right_values": true,
            "cells": [ { "type": "text", "value": "Financial Ratios & Percentages" } ]
        }));

        let ratio_row_definitions = vec![
            ("Operating Profit Margin", "OPM"),
            ("Net Profit Margin", "NPM"),
            ("Employee Cost Intensity", "ECI"),
            ("Debt Coverage Stress Factor", "DCSF"),
            ("Effective Tax Rate", "ETR"),
        ];

        // 1. Precompute floats into an indexable coordinate table for reliable multi-quarter time lookups
        let mut ratio_float_matrix: HashMap<(String, String), f64> = HashMap::new();
        for (_, identifier) in &ratio_row_definitions {
            for date in &unique_filing_dates {
                let rev = get_float_val(date, "RevenueFromOperations");
                let exp = get_float_val(date, "Expenses");
                let net_profit = get_float_val(date, "ProfitLossForPeriod");

                let calculated_val = match *identifier {
                    "OPM" => if rev > 0.0 { ((rev - exp) / rev) * 100.0 } else { 0.0 },
                    "NPM" => if rev > 0.0 { (net_profit / rev) * 100.0 } else { 0.0 },
                    "ECI" => {
                        let emp = get_float_val(date, "EmployeeBenefitExpense");
                        if rev > 0.0 { (emp / rev) * 100.0 } else { 0.0 }
                    },
                    "DCSF" => {
                        let finance = get_float_val(date, "FinanceCosts");
                        let pbt_items = get_float_val(date, "ProfitBeforeExceptionalItemsAndTax");
                        if pbt_items > 0.0 { finance / pbt_items } else { 0.0 }
                    },
                    "ETR" => {
                        let tax = get_float_val(date, "TaxExpense");
                        let pbt = get_float_val(date, "ProfitBeforeTax");
                        if pbt > 0.0 { (tax / pbt) * 100.0 } else { 0.0 }
                    },
                    _ => 0.0
                };
                ratio_float_matrix.insert((identifier.to_string(), date.clone()), calculated_val);
            }
        }

        // 2. Build the structural ratio rows with raw hex color mapping (No nested children velocity popups)
        for (label, identifier) in &ratio_row_definitions {
            let mut ratio_cells = Vec::new();
            ratio_cells.push(json!({ "type": "text", "value": label.to_string() }));

            for date in &unique_filing_dates {
                let current_val = *ratio_float_matrix.get(&(identifier.to_string(), date.clone())).unwrap_or(&0.0);

                // Enforce zero-anchored sigmoid gradient hex parameters natively
                let hex_color = match *identifier {
                    "OPM" | "NPM" => {
                        if current_val <= 0.0 { "#EF4444" }      // Deep Warning Crimson Red
                        else if current_val < 10.0 { "#FB923C" } // Soft Amber Orange
                        else if current_val < 20.0 { "#FBBF24" } // Neutral Baseline Yellow
                        else { "#34D399" }                       // Healthy Growth Green
                    },
                    "ECI" | "DCSF" => {
                        if current_val > 35.0 { "#EF4444" }      // High-risk Overhead Strain
                        else if current_val > 15.0 { "#FB923C" }
                        else { "#34D399" }
                    },
                    _ => "#E5E5E5"
                };

                let base_formatted_text = if *identifier == "DCSF" { format!("{:.2}x", current_val) } else { format!("{:.2}%", current_val) };

                ratio_cells.push(json!({
                    "type": "text",
                    "style": { "color": hex_color, "fontWeight": "600" },
                    "value": base_formatted_text
                }));
            }

            compiled_rows.push(json!({
                "type": "table_row",
                "is_parent": false,
                "is_child": true,
                "parent_id": Some("ratios_group".to_string()),
                "align_right_values": true,
                "cells": ratio_cells
            }));
        }

        // 🚀 3. EXPLICIT CORE GROWTH ROWS (Natively integrated as standalone collapsible metrics)
        let core_growth_definitions = vec![
            ("Revenue QoQ Growth Speed", "RevenueFromOperations", true),
            ("Revenue YoY Growth Speed", "RevenueFromOperations", false),
            ("Net Profit QoQ Growth Speed", "ProfitLossForPeriod", true),
            ("Net Profit YoY Growth Speed", "ProfitLossForPeriod", false),
        ];

        for (row_label, target_tag, is_qoq) in core_growth_definitions {
            let mut trend_cells = Vec::new();
            trend_cells.push(json!({ "type": "text", "value": row_label.to_string() }));

            for (d_idx, date) in unique_filing_dates.iter().enumerate() {
                let current_raw = get_float_val(date, target_tag);
                let lookback_offset = if is_qoq { 1 } else { 4 };

                let calculated_growth = if d_idx + lookback_offset < unique_filing_dates.len() {
                    let historical_date = &unique_filing_dates[d_idx + lookback_offset];
                    let historical_raw = get_float_val(historical_date, target_tag);
                    if historical_raw != 0.0 {
                        ((current_raw - historical_raw) / historical_raw.abs()) * 100.0
                    } else {
                        0.0
                    }
                } else {
                    0.0
                };

                let metric_color = if calculated_growth > 0.0 {
                    "#34D399" // Dynamic Green for growth improvement
                } else if calculated_growth < 0.0 {
                    "#EF4444" // Dynamic Crimson Red for decline drops
                } else {
                    "#737373" // Neutral Muted Grey
                };

                trend_cells.push(json!({
                    "type": "text",
                    "style": { "color": metric_color, "fontFamily": "monospace", "fontSize": "11px", "fontWeight": "500" },
                    "value": format_growth_badge(calculated_growth)
                }));
            }

            compiled_rows.push(json!({
                "type": "table_row",
                "is_parent": false,
                "is_child": true,
                "parent_id": Some("ratios_group".to_string()),
                "align_right_values": true,
                "cells": trend_cells
            }));
        }

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

        let total_children_count = structured_accounting_tree.len() + ratio_row_definitions.len() + 1;

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
            "footer": format!("Total active tracked accounting metrics: {} tags spanning {} quarters", total_children_count, unique_filing_dates.len()),
            "children": [
                /* 🚀 CHILD ONE: HEALTH AND TRAJECTORY SIGNALS LEDGER */
                {
                    "type": "container",
                    "className": "w-full flex flex-col gap-3 p-4 rounded-xl mb-4 bg-neutral-500/5 border border-neutral-500/10",
                    "children": [
                        {
                            "type": "text",
                            "className": "text-[10px] uppercase font-bold tracking-widest font-mono opacity-60 mb-1",
                            "value": "Automated Financial Health & Trajectory Signals"
                        },
                        {
                            "type": "container",
                            "className": "w-full grid grid-cols-1 sm:grid-cols-3 gap-3",
                            "children": velocity_indicators
                        },
                        {
                            "type": "container",
                            "className": "w-full flex flex-col gap-2 mt-2",
                            "children": anomalies_and_warnings
                        }
                    ]
                },
                /* 🚀 CHILD TWO: INTERACTIVE DATA TABLE VIEWPORT FRAME */
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