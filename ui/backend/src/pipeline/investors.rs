use serde_json::{json, Value};
use std::collections::{HashMap, BTreeSet};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use parquet::file::reader::FileReader;
use parquet::file::serialized_reader::SerializedFileReader;
use parquet::record::RowAccessor;
use crate::commands::pipeline::CatalogItem;
use crate::pipeline::{WorkspaceModule, WorkspaceDataContext};

pub struct InvestorsCard;

fn transform_camel_case(input: &str) -> String {
    // Strip trailing SEBI data markers if present
    let clean_input = if input.ends_with('I') && !input.ends_with("HUF") {
        &input[..input.len() - 1]
    } else if input.ends_with("SI") {
        &input[..input.len() - 2]
    } else {
        input
    };

    let mut result = String::new();
    for (i, ch) in clean_input.chars().enumerate() {
        if i > 0 && ch.is_uppercase() {
            // Keep acronyms or markers tight but split regular camelCase words
            result.push(' ');
        }
        result.push(ch);
    }
    result
}

fn classify_entity_type(name: &str, categories: &str) -> &'static str {
    let combined = format!("{} {}", name.to_uppercase(), categories.to_uppercase());
    if combined.contains("TRUST") {
        "Family Trust / Employee Benefit"
    } else if combined.contains("LTD") || combined.contains("PVT") || combined.contains("LIMITED") || combined.contains("BODIES CORPORATE") {
        "Corporate Body / Entity"
    } else if combined.contains("CLEARING") {
        "Clearing Member"
    } else {
        "Individual / HNI / Promoter"
    }
}

fn format_share_count(val_str: &str) -> String {
    let trimmed = val_str.trim();
    if trimmed.is_empty() || trimmed == "NaN" || trimmed == "NA" || trimmed == "-" {
        return "0".to_string();
    }
    if let Ok(float_val) = trimmed.parse::<f64>() {
        if float_val == 0.0 {
            return "0".to_string();
        }
        if float_val >= 10_000_000.0 {
            return format!("{:.2} Cr", float_val / 10_000_000.0);
        } else if float_val >= 100_000.0 {
            return format!("{:.2} Lk", float_val / 100_000.0);
        }
        return format!("{:.0}", float_val);
    }
    trimmed.to_string()
}

impl WorkspaceModule for InvestorsCard {
    fn catalog_definition(&self) -> CatalogItem {
        CatalogItem {
            id: "shareholding_investors".to_string(),
            name: "Shareholding Structure & SBO Whales Matrix".to_string(),
            description: "Dynamic crawler tracking SEBI capital structure metrics, HNI whales, and ultimate human governance control layers over cross-year timelines.".to_string(),
        }
    }

    fn compile(&self, _ticker: &str, _timeframe: &str, data: &WorkspaceDataContext) -> Result<Value, String> {
        let parquet_payload = data.get_dataset("parquets/nse_corporate-shareholding-master.parquet");
        let mut raw_records: Vec<(String, String, String, String)> = Vec::new();

        if let Some(b64_str) = parquet_payload["bytes_base64"].as_str() {
            if let Ok(vec_bytes) = STANDARD.decode(b64_str) {
                let bytes_container = bytes::Bytes::from(vec_bytes);
                if let Ok(file_reader) = SerializedFileReader::new(bytes_container) {
                    let num_groups = file_reader.metadata().num_row_groups();
                    let mut group_idx = 0;
                    while group_idx < num_groups {
                        if let Ok(group) = file_reader.get_row_group(group_idx) {
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
                        group_idx += 1;
                    }
                }
            }
        }

        if raw_records.is_empty() {
            return Err("Zero data entries processed from the shareholding master Parquet registry".to_string());
        }

        // 🚀 PASS 1: VALIDATE CODES & MAP COMPLIANT SUBMISSIONS TO LOGICAL REPORTING DATES
        let mut file_to_report_date: HashMap<String, String> = HashMap::new();
        let mut files_with_valid_data: BTreeSet<String> = BTreeSet::new();
        let mut unique_report_dates: BTreeSet<String> = BTreeSet::new();

        // Pass 1a: Extract explicit target dates
        for (source_file, tag_name, _, raw_value) in &raw_records {
            if tag_name == "DateOfReport" {
                let cleaned_date = raw_value.trim().to_string();
                if !cleaned_date.is_empty() && cleaned_date != "NA" && cleaned_date != "0" {
                    file_to_report_date.insert(source_file.clone(), cleaned_date);
                }
            }
        }

        // Pass 1b: Verify that the file isn't just an empty skeleton container 
        for (source_file, tag_name, _, raw_value) in &raw_records {
            if tag_name == "NumberOfShares" {
                let cleaned_val = raw_value.trim();
                if !cleaned_val.is_empty() && cleaned_val != "NA" && cleaned_val != "0" {
                    if let Some(true_date) = file_to_report_date.get(source_file) {
                        files_with_valid_data.insert(source_file.clone());
                        unique_report_dates.insert(true_date.clone());
                    }
                }
            }
        }

        // Freeze timeline columns from newest quarter to oldest quarter
        let timeline_axis: Vec<String> = unique_report_dates.into_iter().rev().collect();

        // Guard Break: If no clean, populated dates are left, exit out safely
        if timeline_axis.is_empty() {
            return Err("Data Integrity Exception: Zero valid reporting quarters identified with active share data.".to_string());
        }

        // 🚀 PASS 2: COMPOSITE SEBI COORDINATE MATRIX EXTRACTOR (UNIFIED GOVERNANCE ROUTING)
        let mut master_coordinate_matrix: HashMap<String, String> = HashMap::new();
        let mut discovered_macro_contexts: BTreeSet<String> = BTreeSet::new();
        let mut dynamic_whale_keys: BTreeSet<(String, String)> = BTreeSet::new(); 
        let mut dynamic_sbo_indices: BTreeSet<String> = BTreeSet::new();

        for (source_file, tag_name, context_id, raw_value) in &raw_records {
            if !files_with_valid_data.contains(source_file) {
                continue; 
            }

            let true_date = file_to_report_date.get(source_file).unwrap();
            let lookup_coordinate = format!("{}__{}__{}", true_date, context_id, tag_name);
            master_coordinate_matrix.insert(lookup_coordinate, raw_value.clone());

            let len = context_id.len();
            if len >= 4 {
                let suffix = &context_id[len - 4..];
                let chars: Vec<char> = suffix.chars().collect();
                
                // Check for standard SEBI 3-digit number + D/I letter structure
                if chars.len() == 4 && chars[0].is_ascii_digit() && chars[1].is_ascii_digit() && chars[2].is_ascii_digit() && (chars[3] == 'D' || chars[3] == 'I') {
                    let prefix_base = context_id[..len - 4].to_string();
                    let index_code = format!("{}{}{}", chars[0], chars[1], chars[2]);
                    
                    // 🛡️ UNIFIED GOVERNANCE FILTER: Catch either variant (with or without spaces)
                    if prefix_base.contains("Significant Beneficial")
                        || prefix_base.contains("SignificantBeneficial") 
                        || prefix_base.contains("SBO") 
                        || tag_name.contains("Beneficial") 
                        || tag_name.contains("RegisteredOwner") 
                    {
                        dynamic_sbo_indices.insert(index_code);
                    } else {
                        dynamic_whale_keys.insert((prefix_base, index_code));
                    }
                } else {
                    if !context_id.is_empty() && context_id != "One" && context_id != "OneD" && context_id != "OneI" {
                        discovered_macro_contexts.insert(context_id.clone());
                    }
                }
            } else {
                if !context_id.is_empty() && context_id != "One" && context_id != "OneD" && context_id != "OneI" {
                    discovered_macro_contexts.insert(context_id.clone());
                }
            }
        }

        let matrix_lookup = |date: &str, ctx: &str, tag: &str| -> String {
            let key = format!("{}__{}__{}", date, ctx, tag);
            master_coordinate_matrix.get(&key).cloned().unwrap_or_default()
        };

        let matrix_lookup_f64 = |date: &str, ctx: &str, tag: &str| -> f64 {
            matrix_lookup(date, ctx, tag).trim().parse::<f64>().unwrap_or(0.0)
        };

        // 🚀 ENGINE LAYER 1: COMPILE THE MACRO CAP-TABLE TREE (ALL IN PERCENTAGES + HOVER METRICS PARCELS)
        let primary_sebi_roots = vec![
            ("ShareholdingOfPromoterAndPromoterGroupI", "Promoters & Promoter Group", "promoter_root", true, ""),
            ("IndianI", "Indian Promoters Breakdown", "promoter_root", false, "promoter_root"),
            ("IndividualsOrHinduUndividedFamilyI", "↳ Individuals / HUF Base", "promoter_root", false, "promoter_root"),
            ("OtherIndianShareholdersI", "↳ Central/State Gov & Corporate Entities", "promoter_root", false, "promoter_root"),
            ("ForeignI", "Foreign Promoters Breakdown", "promoter_root", false, "promoter_root"),
            
            ("InstitutionsI", "Institutional Open Market Float", "institution_root", true, ""),
            ("MutualFundsOrUtiI", "↳ Institutional Mutual Funds", "institution_root", false, "institution_root"),
            ("InstitutionsForeignPortfolioInvestorI", "↳ Foreign Portfolio Investors (FPI)", "institution_root", false, "institution_root"),
            ("FinancialInstitutionOrBanksI", "↳ Indian Banks & Financial Hubs", "institution_root", false, "institution_root"),
            
            ("NonInstitutionsI", "Public Non-Institutional Float", "public_root", true, ""),
            ("IndividualShareholdersHoldingNominalShareCapitalUpToRs2LakhsI", "↳ Retail Public (Nominal Up to ₹2 Lk)", "public_root", false, "public_root"),
            ("IndividualShareholdersHoldingNominalShareCapitalInExcessOfRs2LakhsI", "↳ HNI Public Whales (Above ₹2 Lk)", "public_root", false, "public_root"),
            ("OtherNonInstitutionsI", "↳ Bodies Corporate, Clearing & NRIs", "public_root", false, "public_root"),
            
            ("ShareholdingPatternI", "Grand Total Capitalization Base", "", false, ""),
        ];

        let mut macro_compiled_rows = Vec::new();

        for (ctx_id, visual_label, parent_group, is_parent, nested_under) in primary_sebi_roots {
            let mut row_cells = Vec::new();
            row_cells.push(json!({ "type": "text", "value": visual_label.to_string() }));

            for date in &timeline_axis {
                let shares = matrix_lookup_f64(date, ctx_id, "NumberOfShares");
                let total_shares = matrix_lookup_f64(date, "ShareholdingPatternI", "NumberOfShares");
                let shareholders = matrix_lookup(date, ctx_id, "NumberOfShareholders");
                let demat_shares = matrix_lookup(date, ctx_id, "NumberOfEquitySharesHeldInDematerializedForm");
                let pledged_shares = matrix_lookup(date, ctx_id, "PledgedOrEncumberedNumberOfShares");

                let weight_percentage = if total_shares > 0.0 { (shares / total_shares) * 100.0 } else { 0.0 };

                // 🎯 HOVER INTERACTION DESIGN: Primary string value holds the clean percentage weight,
                // while detailed raw values are packaged as hover properties.
                row_cells.push(json!({
                    "type": "text",
                    "value": format!("{:.2}%", weight_percentage),
                    "className": "font-semibold text-neutral-200 cursor-pointer hover:text-blue-400 transition-colors",
                    "hover_breakdown": {
                        "Reporting Quarter": date,
                        "Ownership Stake Weight": format!("{:.4}%", weight_percentage),
                        "Absolute Share Volume": format_share_count(&shares.to_string()),
                        "Total Active Shareholders": if shareholders.is_empty() { "-" } else { &shareholders },
                        "Dematerialized Digital Ratio": format_share_count(&demat_shares),
                        "Promoter Pledged / Encumbered": if pledged_shares.is_empty() || pledged_shares == "0" { "None Registered" } else { &pledged_shares }
                    }
                }));
            }

            macro_compiled_rows.push(json!({
                "type": "table_row",
                "is_parent": is_parent,
                "is_child": !nested_under.is_empty(),
                "parent_id": if !parent_group.is_empty() { Some(parent_group.to_string()) } else { None },
                "align_right_values": true,
                "cells": row_cells
            }));
        }

        // 🚀 ENGINE LAYER 2: DIRECT INSIDER HISTORICAL TRACKER (EXACT NAME MATRIX ENGINE)
        let mut whales_compiled_rows = Vec::new();

        // Separate discovered dynamic whale hashes into clear logical vectors natively
        let mut grouped_whales_map: HashMap<String, Vec<String>> = HashMap::new();
        for (prefix, index) in &dynamic_whale_keys {
            grouped_whales_map.entry(prefix.clone()).or_insert_with(Vec::new).push(index.clone());
        }

        // --- SUB-ROUTINE HELPER: UNIFY SEBI SCHEMA MUTATIONS TO KEEPS DROPDOWNS BALANCED ---
        let normalize_context_prefix = |ctx: &str| -> String {
            ctx.replace("ResidentIndividual", "Individual")
               .replace("OtherNonInstitutions", "NonInstitutions")
               .replace("OthersIndianShareholders", "IndianShareholders")
        };

        // 1. Map every exact legal name string to its historical reporting coordinates
        // Key: (normalized_prefix, exact_investor_name) -> Map of: date -> BTreeSet of (raw_prefix, index_str)
        let mut exact_name_registry: HashMap<(String, String), HashMap<String, BTreeSet<(String, String)>>> = HashMap::new();

        for prefix_base in grouped_whales_map.keys() {
            let normalized_prefix = normalize_context_prefix(prefix_base);
            if let Some(indices) = grouped_whales_map.get(prefix_base) {
                for index_str in indices {
                    let context_coordinate_d = format!("{}{}{}", prefix_base, index_str, "D");
                    
                    for date in &timeline_axis {
                        let name_probe = matrix_lookup(date, &context_coordinate_d, "NameOfTheShareholder");
                        let cleaned_name = name_probe.trim().to_string();
                        
                        if !cleaned_name.is_empty() && cleaned_name != "NA" && cleaned_name != "-" && cleaned_name != "0" {
                            let registry_key = (normalized_prefix.clone(), cleaned_name);
                            
                            exact_name_registry
                                .entry(registry_key)
                                .or_insert_with(HashMap::new)
                                .entry(date.clone())
                                .or_insert_with(BTreeSet::new)
                                .insert((prefix_base.clone(), index_str.clone()));
                        }
                    }
                }
            }
        }

        // 2. Group the clean name anchors back underneath their normalized dropdown headers
        let mut category_to_investors: HashMap<String, Vec<String>> = HashMap::new();
        for (normalized_prefix, investor_name) in exact_name_registry.keys() {
            category_to_investors.entry(normalized_prefix.clone()).or_insert_with(Vec::new).push(investor_name.clone());
        }

        // 3. Compile the structural table grid
        for normalized_prefix in category_to_investors.keys() {
            let processed_label = transform_camel_case(normalized_prefix);
            let investors_list = category_to_investors.get(normalized_prefix).unwrap();

            // 🧮 DE-DUPLICATED DROPDOWN SUMMARY TOTAL COMPILER
            let mut dropdown_cells = Vec::new();
            dropdown_cells.push(json!({ "type": "text", "value": format!("Category Block: {}", processed_label), "className": "font-bold text-neutral-200" }));
            dropdown_cells.push(json!({ "type": "text", "value": "Summary Aggregate" }));

            for date in &timeline_axis {
                let mut total_category_block_shares = 0.0;
                let total_cap = matrix_lookup_f64(date, "ShareholdingPatternI", "NumberOfShares");

                for investor_name in investors_list {
                    if let Some(date_maps) = exact_name_registry.get(&(normalized_prefix.clone(), investor_name.clone())) {
                        if let Some(coord_set) = date_maps.get(date) {
                            for (c_prefix, index_str) in coord_set {
                                let context_coordinate_i = format!("{}{}{}", c_prefix, index_str, "I");
                                if let Ok(parsed_shares) = matrix_lookup(date, &context_coordinate_i, "NumberOfShares").trim().parse::<f64>() {
                                    total_category_block_shares += parsed_shares;
                                }
                            }
                        }
                    }
                }

                let category_aggregate_weight = if total_cap > 0.0 { (total_category_block_shares / total_cap) * 100.0 } else { 0.0 };
                
                dropdown_cells.push(json!({
                    "type": "text",
                    "value": format!("{:.2}% ({})", category_aggregate_weight, format_share_count(&total_category_block_shares.to_string())),
                    "className": "font-bold text-neutral-300 border-b border-neutral-800",
                    "hover": { // 🎯 FIXED FRONTIER KEY LINK
                        "Reporting Quarter": date,
                        "Combined Block Stake": format!("{:.4}%", category_aggregate_weight),
                        "Total Aggregate Shares": format!("{} shares", total_category_block_shares as i64)
                    }
                }));
            }

            whales_compiled_rows.push(json!({
                "type": "table_row",
                "is_parent": true,
                "is_child": false,
                "parent_id": Some(normalized_prefix.clone()),
                "cells": dropdown_cells
            }));

            // 4. Compile individual investor tracking lines
            for investor_name in investors_list {
                let mut whale_cells = Vec::new();
                whale_cells.push(json!({
                    "type": "text",
                    "value": investor_name.clone(),
                    "className": "font-medium text-emerald-400 pl-4"
                }));

                // Extract structural classification type tags safely
                let mut resolved_class_tag = "Individual / HNI / Promoter";
                if let Some(date_maps) = exact_name_registry.get(&(normalized_prefix.clone(), investor_name.clone())) {
                    'class_finder: for coord_set in date_maps.values() {
                        for (c_prefix, index_str) in coord_set {
                            let context_coordinate_d = format!("{}{}{}", c_prefix, index_str, "D");
                            for date in &timeline_axis {
                                let cat_probe = matrix_lookup(date, &context_coordinate_d, "CategoryOfOtherIndianShareholders");
                                if !cat_probe.is_empty() && cat_probe != "NA" {
                                    resolved_class_tag = classify_entity_type(investor_name, &cat_probe);
                                    break 'class_finder;
                                }
                            }
                        }
                    }
                }
                if resolved_class_tag == "Individual / HNI / Promoter" {
                    resolved_class_tag = classify_entity_type(investor_name, "");
                }
                whale_cells.push(json!({ "type": "text", "value": resolved_class_tag }));

                // 5. Generate consolidated timeline columns
                let date_maps = exact_name_registry.get(&(normalized_prefix.clone(), investor_name.clone())).unwrap();

                for date in &timeline_axis {
                    let mut aggregated_shares = 0.0;

                    if let Some(coord_set) = date_maps.get(date) {
                        for (c_prefix, index_str) in coord_set {
                            let context_coordinate_i = format!("{}{}{}", c_prefix, index_str, "I");
                            if let Ok(parsed_shares) = matrix_lookup(date, &context_coordinate_i, "NumberOfShares").trim().parse::<f64>() {
                                aggregated_shares += parsed_shares;
                            }
                        }
                    }

                    let total_cap = matrix_lookup_f64(date, "ShareholdingPatternI", "NumberOfShares");
                    let weight_percentage = if total_cap > 0.0 { (aggregated_shares / total_cap) * 100.0 } else { 0.0 };
                    let formatted_shares = format_share_count(&aggregated_shares.to_string());

                    // 🎯 VISUAL UPGRADE: Shows both Percentage and formatted Volume directly in the grid
                    let grid_display_text = if aggregated_shares > 0.0 {
                        format!("{:.2}% ({})", weight_percentage, formatted_shares)
                    } else {
                        "0.00% (0)".to_string()
                    };

                    let mut cell_payload = json!({ 
                        "type": "text", 
                        "value": grid_display_text,
                        "className": if aggregated_shares > 0.0 { "text-neutral-200 cursor-pointer hover:text-blue-400 transition-colors" } else { "text-neutral-600" }
                    });

                    if aggregated_shares > 0.0 {
                        cell_payload["hover"] = json!({ // 🎯 FIXED FRONTIER KEY LINK
                            "Filing Date Scope": date,
                            "Absolute Stake Position": format!("{} shares", aggregated_shares as i64),
                            "Exact Total Net Weight": format!("{:.4}%", weight_percentage)
                        });
                    } else {
                        cell_payload["hover"] = json!({
                            "Filing Date Scope": date,
                            "Status": "Exited / Position Liquidated or Dropped Below 1% Regulatory Disclosure Boundary"
                        });
                    }
                    whale_cells.push(cell_payload);
                }

                whales_compiled_rows.push(json!({
                    "type": "table_row",
                    "is_parent": false,
                    "is_child": true,
                    "parent_id": Some(normalized_prefix.clone()),
                    "align_right_values": true,
                    "cells": whale_cells
                }));
            }
        }

        // 🚀 ENGINE LAYER 3: DYNAMIC HUMAN SIGNIFICANT BENEFICIAL OWNERSHIP (SBO) CONTROLLER LOOP (UNIFIED SPACE RESOLUTION)
        let mut sbo_compiled_rows = Vec::new();

        for index_str in &dynamic_sbo_indices {
            // 🎯 FIXED DATA LINK: Check options WITH and WITHOUT literal spacing architectures
            let context_variants = vec![
                format!("Significant Beneficial Owners Axis{}D", index_str),
                format!("Significant Beneficial Owners Axis{}I", index_str),
                format!("SignificantBeneficialOwnersAxis{}D", index_str),
                format!("SignificantBeneficialOwnersAxis{}I", index_str),
                format!("SignificantBeneficialOwners{}D", index_str),
                format!("SignificantBeneficialOwners{}I", index_str),
                format!("SignificantBeneficialOwner{}D", index_str),
                format!("SignificantBeneficialOwner{}I", index_str),
            ];

            let mut resolved_human_sbo = String::new();
            let mut resolved_proxy_owner = String::new();
            let mut resolved_nationality = String::new();
            let mut resolved_acquisition_date = String::new();
            let mut has_any_historical_sbo_activity = false;

            // 1. Scan across all context path variations and quarters to anchor our base row text labels
            for date in &timeline_axis {
                for ctx in &context_variants {
                    let sbo_name = matrix_lookup(date, ctx, "NameOfSignificantBeneficialOwners");
                    let proxy_name = matrix_lookup(date, ctx, "NameOfRegisteredOwner");
                    let nat = matrix_lookup(date, ctx, "NationalityOfSignificantBeneficialOwners");
                    let acq = matrix_lookup(date, ctx, "DateOfCreationOrAcquisitionOfSignificantBeneficialInterest");

                    if !sbo_name.is_empty() && sbo_name != "NA" && sbo_name != "-" { resolved_human_sbo = sbo_name; has_any_historical_sbo_activity = true; }
                    if !proxy_name.is_empty() && proxy_name != "NA" && proxy_name != "-" { resolved_proxy_owner = proxy_name; has_any_historical_sbo_activity = true; }
                    if !nat.is_empty() && nat != "NA" && nat != "-" { resolved_nationality = nat; }
                    if !acq.is_empty() && acq != "NA" && acq != "-" { resolved_acquisition_date = acq; }

                    // Also verify if legal flag rows exist even if name strings were omitted by the company clerk
                    if !has_any_historical_sbo_activity {
                        let probe_shares = matrix_lookup(date, ctx, "DetailsOfHoldingExerciseOfRightOfTheSBOInTheReportingCompanyWhetherByVirtueOfShares");
                        if !probe_shares.is_empty() && probe_shares != "NA" {
                            has_any_historical_sbo_activity = true;
                        }
                    }
                }
            }

            // 🛡️ SAFETY VALVE: If no historical data points exist for this node index at all, skip it.
            if !has_any_historical_sbo_activity {
                continue; 
            }

            if resolved_human_sbo.is_empty() { resolved_human_sbo = format!("Disclosed SBO Individual #{}", index_str); }
            if resolved_proxy_owner.is_empty() { resolved_proxy_owner = "Direct Shareholding Body / Trust".to_string(); }
            if resolved_nationality.is_empty() { resolved_nationality = "Indian".to_string(); }
            if resolved_acquisition_date.is_empty() { resolved_acquisition_date = "Nominal Base Setup".to_string(); }

            let mut sbo_cells = Vec::new();
            sbo_cells.push(json!({ "type": "text", "value": resolved_proxy_owner, "className": "font-semibold text-amber-400 text-xs" }));
            sbo_cells.push(json!({ "type": "text", "value": resolved_human_sbo, "className": "text-neutral-100 font-medium text-xs" }));
            sbo_cells.push(json!({ "type": "text", "value": resolved_nationality, "className": "text-xs opacity-80" }));
            sbo_cells.push(json!({ "type": "text", "value": resolved_acquisition_date, "className": "text-xs opacity-80 font-mono" }));

            // 2. Loop over columns to compile the legal control rights visual ribbons
            for date in &timeline_axis {
                let mut exercise_shares = "false".to_string();
                let mut exercise_voting = "false".to_string();
                let mut exercise_dividend = "false".to_string();
                let mut exercise_control = "false".to_string();
                let mut exercise_influence = "false".to_string();
                let mut cell_has_active_quarter_data = false;

                for ctx in &context_variants {
                    let sh = matrix_lookup(date, ctx, "DetailsOfHoldingExerciseOfRightOfTheSBOInTheReportingCompanyWhetherByVirtueOfShares");
                    let vt = matrix_lookup(date, ctx, "DetailsOfHoldingExerciseOfRightOfTheSBOInTheReportingCompanyWhetherByVirtueOfVotingRights");
                    let dv = matrix_lookup(date, ctx, "DetailsOfHoldingExerciseOfRightOfTheSBOInTheReportingCompanyWhetherByVirtueOfRightsOnDistributableDividendOrAnyOtherDistribution");
                    let cn = matrix_lookup(date, ctx, "DetailsOfHoldingExerciseOfRightOfTheSBOInTheReportingCompanyWhetherByVirtueOfExerciseOfControl");
                    let inf = matrix_lookup(date, ctx, "DetailsOfHoldingExerciseOfRightOfTheSBOInTheReportingCompanyWhetherByVirtueOfExerciseOfSignificantInfluence");

                    if !sh.is_empty() && sh != "NA" { exercise_shares = sh; cell_has_active_quarter_data = true; }
                    if !vt.is_empty() && vt != "NA" { exercise_voting = vt; cell_has_active_quarter_data = true; }
                    if !dv.is_empty() && dv != "NA" { exercise_dividend = dv; cell_has_active_quarter_data = true; }
                    if !cn.is_empty() && cn != "NA" { exercise_control = cn; cell_has_active_quarter_data = true; }
                    if !inf.is_empty() && inf != "NA" { exercise_influence = inf; cell_has_active_quarter_data = true; }
                }

                let check_flag = |f_str: &str| -> bool {
                    let tr = f_str.to_lowercase();
                    tr == "true" || tr == "1" || tr == "yes" || tr == "y"
                };

                let has_shares = check_flag(&exercise_shares);
                let has_voting = check_flag(&exercise_voting);
                let has_dividend = check_flag(&exercise_dividend);
                let has_control = check_flag(&exercise_control);
                let has_influence = check_flag(&exercise_influence);

                let control_ribbon = if cell_has_active_quarter_data {
                    format!(
                        "{}  {}  {}  {}  {}",
                        if has_shares { "S" } else { "·" },
                        if has_voting { "V" } else { "·" },
                        if has_dividend { "D" } else { "·" },
                        if has_control { "C" } else { "·" },
                        if has_influence { "I" } else { "·" }
                    )
                } else {
                    "·  ·  ·  ·  ·".to_string()
                };

                let mut cell_payload = json!({
                    "type": "text",
                    "value": control_ribbon,
                    "className": if cell_has_active_quarter_data { "font-mono font-bold tracking-widest text-center text-xs text-blue-400 cursor-help border-b border-dotted border-blue-500/30 pb-0.5" } else { "font-mono tracking-widest text-center text-xs text-neutral-600" }
                });

                if cell_has_active_quarter_data {
                    cell_payload["children"] = json!([
                        { "type": "text", "title": "Filing Scope Window", "value": date },
                        { "type": "text", "title": "[S] Direct Equity Shares Held", "value": if has_shares { "YES" } else { "NO" } },
                        { "type": "text", "title": "[V] Board Voting Weight Rights", "value": if has_voting { "YES" } else { "NO" } },
                        { "type": "text", "title": "[D] Distributable Dividend Claims", "value": if has_dividend { "YES" } else { "NO" } },
                        { "type": "text", "title": "[C] De-Facto Executive Control", "value": if has_control { "VETO ACTIVATED" } else { "NO" } },
                        { "type": "text", "title": "[I] Significant Influence Held", "value": if has_influence { "YES" } else { "NO" } }
                    ]);
                } else {
                    cell_payload["children"] = json!([
                        { "type": "text", "title": "Filing Scope Window", "value": date },
                        { "type": "text", "title": "Status Flag", "value": "No active SBO regulatory disclosures recorded for this quarter" }
                    ]);
                }
                sbo_cells.push(cell_payload);
            }

            sbo_compiled_rows.push(json!({
                "type": "table_row",
                "is_parent": false,
                "is_child": false,
                "align_right_values": false,
                "cells": sbo_cells
            }));
        }

        // 🚀 PASS 4: GENERATE INTEGRATED MULTI-TABLE DATA WORKSPACE PAYLOAD
        let mut macro_headers = vec!["Cap-Table Tier Allocation Segment".to_string()];
        macro_headers.extend(timeline_axis.clone());

        let mut whale_headers = vec!["Whale Disclosure Identity".to_string(), "Structural Classification".to_string()];
        whale_headers.extend(timeline_axis.clone());

        let mut sbo_headers = vec![
            "Registered Proxy Owner (Trust/Firm)".to_string(),
            "Ultimate Human SBO".to_string(),
            "Nationality".to_string(),
            "Acquisition Date".to_string(),
        ];
        sbo_headers.extend(timeline_axis.clone());

        Ok(json!({
            "type": "card",
            "title": "Corporate Ownership & Strategic Governance Matrix",
            "subtitle": "// MULTI-QUARTER CAP-TABLE MACHINE // INTERACTIVE WHALES DISCLOSURE NODE // SEBI SBO TRACK",
            "footer": format!("Successfully tracking {} financial reporting timeline quarters across 3 standalone disclosure layouts", timeline_axis.len()),
            "children": [
                /* TABLE ROW 1: THE PRIMARY SCALE ALLOCATION MATRIX */
                {
                    "type": "container",
                    "className": "w-full flex flex-col gap-2 p-4 rounded-xl border border-neutral-800 bg-neutral-900/40 mb-6",
                    "children": [
                        {
                            "type": "table",
                            "className": "w-full min-w-[750px] text-left border-collapse text-sm",
                            "headers": macro_headers,
                            "align_right_columns": true,
                            "children": macro_compiled_rows
                        }
                    ]
                },
                /* TABLE ROW 2: THE WHALES AND HIGH NET WORTH DISCLOSURE REGISTRY */
                {
                    "type": "container",
                    "className": "w-full flex flex-col gap-2 p-4 rounded-xl border border-neutral-800 bg-neutral-900/40 mb-6",
                    "children": [
                        {
                            "type": "table",
                            "className": "w-full min-w-[750px] text-left border-collapse text-sm",
                            "headers": whale_headers,
                            "align_right_columns": true,
                            "children": whales_compiled_rows
                        }
                    ]
                },
                /* TABLE ROW 3: SIGNIFICANT BENEFICIAL OWNERSHIP GOVERNANCE AUDIT SYSTEM */
                {
                    "type": "container",
                    "className": "w-full flex flex-col gap-2 p-4 rounded-xl border border-neutral-800 bg-neutral-900/40",
                    "children": [
                        {
                            "type": "table",
                            "className": "w-full min-w-[950px] text-left border-collapse text-sm",
                            "headers": sbo_headers,
                            "align_right_columns": false,
                            "children": sbo_compiled_rows
                        }
                    ]
                }
            ]
        }))
    }
}