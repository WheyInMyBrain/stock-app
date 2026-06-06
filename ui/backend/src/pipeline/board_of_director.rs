use serde_json::{json, Value};
use std::collections::HashMap;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use parquet::file::reader::FileReader;
use parquet::file::serialized_reader::SerializedFileReader;
use parquet::record::RowAccessor;
use crate::commands::pipeline::CatalogItem;
use crate::pipeline::{WorkspaceModule, WorkspaceDataContext};

pub struct BoardOfDirectorCard;

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

fn sanitize_name_spacing(name: &str) -> String {
    name.split_whitespace().collect::<Vec<&str>>().join(" ")
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

#[derive(Debug, Default)]
struct UnifiedDirectorProfile {
    title: String,
    designation: String,
    is_active_currently: bool,
    timeline_attributes: HashMap<String, Vec<String>>,
}

#[derive(Debug, Default)]
struct UnifiedCommitteeProfile {
    committee_name: String,
    member_name: String,
    is_active_currently: bool,
    timeline_attributes: HashMap<String, Vec<String>>,
}

impl WorkspaceModule for BoardOfDirectorCard {
    fn catalog_definition(&self) -> CatalogItem {
        CatalogItem {
            id: "board_of_director".to_string(),
            name: "Board Governance Directory".to_string(),
            description: "Aggregates multi-year corporate governance files, ignoring corrupted empty quarters to track accurate executive timelines.".to_string(),
        }
    }

    fn compile(&self, _ticker: &str, _timeframe: &str, data: &WorkspaceDataContext) -> Result<Value, String> {
        let parquet_raw_payload = data.get_dataset("parquets/nse_corporate-governance-master.parquet");
        
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
            return Err("Zero data records parsed from file registry".to_string());
        }

        // Get unique filenames sorted from Newest to Oldest
        let mut unique_filings: Vec<String> = raw_records.iter().map(|r| r.0.clone()).collect();
        unique_filings.sort();
        unique_filings.dedup();
        unique_filings.sort_by(|a, b| map_filing_to_sortable_score(b).cmp(&map_filing_to_sortable_score(a)));

        // 🚀 FIX: Validate files newest-to-oldest. Skip corrupted files that have 0 director names.
        let mut latest_filing_target = String::new();
        for target_file in &unique_filings {
            let has_names = raw_records.iter().any(|r| {
                r.0 == *target_file 
                && r.2.starts_with("CompositionOfBoardOfDirectors") 
                && r.1 == "NameOftheDirector" 
                && !r.3.trim().is_empty()
            });

            if has_names {
                latest_filing_target = target_file.clone();
                break;
            }
        }

        // Complete safety fallback if everything was empty
        if latest_filing_target.is_empty() {
            latest_filing_target = unique_filings.first().cloned().unwrap_or_default();
        }

        // Sort data chronologically (oldest to newest) so timelines build forward properly
        raw_records.sort_by(|a, b| {
            map_filing_to_sortable_score(&a.0).cmp(&map_filing_to_sortable_score(&b.0))
        });

        // Resolve names mapping per file block context
        let mut board_name_resolver: HashMap<String, String> = HashMap::new();
        let mut committee_name_resolver: HashMap<String, String> = HashMap::new();
        let mut directors_in_latest_file: HashMap<String, bool> = HashMap::new();

        for (source_file, tag_name, context_id, raw_value) in &raw_records {
            let clean_val = sanitize_name_spacing(raw_value.trim()).to_uppercase();
            if clean_val.is_empty() { continue; }

            let numeric_id = if context_id.ends_with('D') || context_id.ends_with('I') {
                context_id[..context_id.len() - 1].to_string()
            } else {
                context_id.clone()
            };
            let resolver_key = format!("{}__{}", source_file, numeric_id);

            if context_id.starts_with("CompositionOfBoardOfDirectors") && tag_name == "NameOftheDirector" {
                board_name_resolver.insert(resolver_key, clean_val.clone());
                if *source_file == latest_filing_target {
                    directors_in_latest_file.insert(clean_val, true);
                }
            } else if context_id.starts_with("CompositionOfCommittees") && tag_name == "NameOfCommitteeMembers" {
                committee_name_resolver.insert(resolver_key, clean_val);
            }
        }

        let mut director_registry: HashMap<String, UnifiedDirectorProfile> = HashMap::new();
        let mut committee_registry: HashMap<String, UnifiedCommitteeProfile> = HashMap::new();

        // Pass 2: Process attributes chronologically
        for (source_file, tag_name, context_id, raw_value) in &raw_records {
            let clean_val = raw_value.trim().to_string();
            if clean_val.is_empty() { continue; }

            let numeric_id = if context_id.ends_with('D') || context_id.ends_with('I') {
                context_id[..context_id.len() - 1].to_string()
            } else {
                context_id.clone()
            };
            let resolver_key = format!("{}__{}", source_file, numeric_id);
            let is_latest_file = *source_file == latest_filing_target;

            if context_id.starts_with("CompositionOfBoardOfDirectors") {
                if let Some(director_name) = board_name_resolver.get(&resolver_key) {
                    let profile = director_registry.entry(director_name.clone()).or_insert_with(UnifiedDirectorProfile::default);
                    
                    if directors_in_latest_file.contains_key(director_name) {
                        profile.is_active_currently = true;
                    }

                    if tag_name == "Title" && profile.title.is_empty() {
                        profile.title = clean_val.clone();
                    }

                    if (tag_name == "PositionOfDirectorInBoardOne" || tag_name == "PositionOfDirectorInBoardThree")
                        && !clean_val.is_empty() && clean_val != "Not Applicable" && clean_val != "NA"
                    {
                        // Update designation continuously up to the validated active file target
                        if profile.is_active_currently {
                            if is_latest_file || profile.designation.is_empty() {
                                profile.designation = clean_val.clone();
                            }
                        } else {
                            profile.designation = "-".to_string();
                        }
                    }

                    if tag_name != "NameOftheDirector" && tag_name != "Title" {
                        let historical_values = profile.timeline_attributes.entry(tag_name.clone()).or_insert_with(Vec::new);
                        if historical_values.last() != Some(&clean_val) {
                            historical_values.push(clean_val);
                        }
                    }
                }
            } else if context_id.starts_with("CompositionOfCommittees") {
                if let Some(member_name) = committee_name_resolver.get(&resolver_key) {
                    let committee_name = board_identity_map_key_fallback(source_file, context_id, &raw_records);
                    if committee_name.is_empty() { continue; }

                    let composite_key = format!("{}//{}", committee_name, member_name);
                    let profile = committee_registry.entry(composite_key).or_insert_with(|| {
                        UnifiedCommitteeProfile {
                            committee_name,
                            member_name: member_name.clone(),
                            ..Default::default()
                        }
                    });

                    if *source_file == latest_filing_target {
                        profile.is_active_currently = true;
                    }

                    if tag_name != "NameOfCommittee" && tag_name != "NameOfCommitteeMembers" {
                        let historical_values = profile.timeline_attributes.entry(tag_name.clone()).or_insert_with(Vec::new);
                        if historical_values.last() != Some(&clean_val) {
                            historical_values.push(clean_val);
                        }
                    }
                }
            }
        }

        // --- RENDER TABLE 1: BOARD SEATS ---
        let mut board_table_rows = Vec::new();
        let mut sorted_directors: Vec<_> = director_registry.keys().collect();
        sorted_directors.sort();

        for name in sorted_directors {
            if let Some(profile) = director_registry.get(name) {
                let title_prefix = if profile.title.is_empty() { "".to_string() } else { format!("{} ", profile.title) };
                
                let display_words: Vec<String> = name.split_whitespace().map(|w| {
                    let mut chars = w.chars();
                    match chars.next() {
                        Some(f) => format!("{}{}", f.to_uppercase().collect::<String>(), chars.as_str().to_lowercase()),
                        None => String::new(),
                    }
                }).collect();
                let display_name = format!("{}{}", title_prefix, display_words.join(" "));
                
                let final_designation = if profile.is_active_currently {
                    if profile.designation.is_empty() { "Director".to_string() } else { profile.designation.clone() }
                } else {
                    "-".to_string()
                };

                let mut hover_children = Vec::new();
                let mut sorted_tags: Vec<_> = profile.timeline_attributes.keys().collect();
                sorted_tags.sort();

                for tag in sorted_tags {
                    if let Some(vals) = profile.timeline_attributes.get(tag) {
                        hover_children.push(json!({
                            "type": "text",
                            "title": transform_camel_case(tag),
                            "value": vals.join(" → ")
                        }));
                    }
                }

                board_table_rows.push(json!({
                    "type": "table_row",
                    "cells": [
                        { "type": "text", "className": "font-medium font-mono text-xs opacity-80", "value": final_designation },
                        { 
                            "type": "text", 
                            "className": "font-bold text-xs tracking-tight text-indigo-500 cursor-help border-b border-dotted border-indigo-500/40 pb-0.5", 
                            "value": display_name,
                            "children": hover_children
                        }
                    ]
                }));
            }
        }

        // --- RENDER TABLE 2: COMMITTEES ---
        let mut committee_table_rows = Vec::new();
        let mut sorted_committees: Vec<_> = committee_registry.keys().collect();
        sorted_committees.sort();

        for key in sorted_committees {
            if let Some(profile) = committee_registry.get(key) {
                let display_comm_name = if profile.is_active_currently { profile.committee_name.clone() } else { "-".to_string() };
                
                let member_words: Vec<String> = profile.member_name.split_whitespace().map(|w| {
                    let mut chars = w.chars();
                    match chars.next() {
                        Some(f) => format!("{}{}", f.to_uppercase().collect::<String>(), chars.as_str().to_lowercase()),
                        None => String::new(),
                    }
                }).collect();
                let display_member_name = member_words.join(" ");

                let mut hover_children = Vec::new();
                let mut sorted_tags: Vec<_> = profile.timeline_attributes.keys().collect();
                sorted_tags.sort();

                hover_children.push(json!({ "type": "text", "title": "Target working group", "value": profile.committee_name.clone() }));

                for tag in sorted_tags {
                    if let Some(vals) = profile.timeline_attributes.get(tag) {
                        hover_children.push(json!({
                            "type": "text",
                            "title": transform_camel_case(tag),
                            "value": vals.join(" → ")
                        }));
                    }
                }

                committee_table_rows.push(json!({
                    "type": "table_row",
                    "cells": [
                        { "type": "text", "className": "font-medium font-mono text-xs opacity-80", "value": display_comm_name },
                        { 
                            "type": "text", 
                            "className": "font-bold text-xs tracking-tight text-indigo-500 cursor-help border-b border-dotted border-indigo-500/40 pb-0.5", 
                            "value": display_member_name,
                            "children": hover_children
                        }
                    ]
                }));
            }
        }

        if board_table_rows.is_empty() {
            board_table_rows.push(json!({ "type": "table_row", "cells": [{ "type": "text", "className": "opacity-40 text-xs italic", "value": "No records found" }, { "type": "text", "value": "" }] }));
        }
        if committee_table_rows.is_empty() {
            committee_table_rows.push(json!({ "type": "table_row", "cells": [{ "type": "text", "className": "opacity-40 text-xs italic", "value": "No records found" }, { "type": "text", "value": "" }] }));
        }

        Ok(json!({
            "type": "card",
            "title": "Corporate Governance Directory",
            "subtitle": format!("// TIMELINE AGGREGATOR // VALIDATED FILING TARGET: {}", latest_filing_target.replace(".xml", "")),
            "footer": format!("Total unique monitored executives: {}", director_registry.len()),
            "children": [
                {
                    "type": "container",
                    "className": "flex flex-col gap-6 w-full max-w-full overflow-x-auto overflow-y-visible",
                    "children": [
                        {
                            "type": "container",
                            "className": "flex flex-col gap-1.5 w-full overflow-visible",
                            "children": [
                                {
                                    "type": "table",
                                    "className": "w-full text-left font-mono border-collapse",
                                    "headers": ["Current Designation", "Director Executive Name (Hover for Timeline)"],
                                    "children": board_table_rows
                                }
                            ]
                        },
                        {
                            "type": "container",
                            "className": "flex flex-col gap-1.5 w-full overflow-visible pt-2 border-t border-dashed border-neutral-500/10",
                            "children": [
                                {
                                    "type": "table",
                                    "className": "w-full text-left font-mono border-collapse",
                                    "headers": ["Current Committee", "Assigned Member Name (Hover for Timeline)"],
                                    "children": committee_table_rows
                                }
                            ]
                        }
                    ]
                }
            ]
        }))
    }
}

fn board_identity_map_key_fallback(file: &str, ctx: &str, records: &[(String, String, String, String)]) -> String {
    records.iter()
        .find(|r| r.0 == *file && r.2 == *ctx && r.1 == "NameOfCommittee")
        .map(|r| r.3.trim().to_uppercase())
        .unwrap_or_default()
}