use serde_json::{json, Value};
use crate::commands::pipeline::CatalogItem;
use crate::pipeline::{WorkspaceModule, WorkspaceDataContext};

pub struct BoardDirectoryCard;

impl WorkspaceModule for BoardDirectoryCard {
    fn catalog_definition(&self) -> CatalogItem {
        CatalogItem {
            id: "board_directory".to_string(),
            name: "Board of Directors".to_string(),
            description: "Direct tracking directory mapping prefixes, full structural name vectors, and corporate execution designations.".to_string(),
        }
    }

    fn compile(&self, ticker: &str, _timeframe: &str, data: &WorkspaceDataContext) -> Result<Value, String> {
        let bse_json = data.get_dataset("bse_corporate-info-directory/endpoint-metadata.json");
        let mut table_rows = Vec::new();

        if let Some(directors_list) = bse_json["Table"].as_array() {
            for director in directors_list {
                let prefix = director.get("sPrefix").and_then(|v| v.as_str()).unwrap_or("").trim();
                let first = director.get("sFirstname").and_then(|v| v.as_str()).unwrap_or("").trim();
                let middle = director.get("sMiddlename").and_then(|v| v.as_str()).unwrap_or("").trim();
                let last = director.get("sLastname").and_then(|v| v.as_str()).unwrap_or("").trim();
                let designation = director.get("sDesignation").and_then(|v| v.as_str()).unwrap_or("N/A").trim();

                let mut name_parts = Vec::new();
                if !prefix.is_empty() { name_parts.push(prefix); }
                if !first.is_empty() { name_parts.push(first); }
                if !middle.is_empty() { name_parts.push(middle); }
                if !last.is_empty() { name_parts.push(last); }

                let full_name = if name_parts.is_empty() {
                    "Unknown Identity".to_string()
                } else {
                    name_parts.join(" ")
                };

                table_rows.push(json!({
                    "type": "table_row",
                    "cells": [
                        { "type": "text", "className": "font-medium font-mono text-xs", "value": designation },
                        { "type": "text", "className": "font-bold text-xs tracking-tight", "value": full_name }
                    ]
                }));
            }
        }

        if table_rows.is_empty() {
            table_rows.push(json!({
                "type": "table_row",
                "cells": [
                    { "type": "text", "className": "opacity-50 italic text-xs", "value": "No data available" },
                    { "type": "text", "className": "opacity-50 italic text-xs", "value": "N/A" }
                ]
            }));
        }

        Ok(json!({
            "type": "card",
            "title": "Board of Directors",
            "subtitle": format!("// GOVERNANCE STRUCTURE INDEX Node: {}", ticker.to_uppercase()),
            "footer": format!("Total Registered Officials: {}", table_rows.len()),
            "children": [
                {
                    "type": "container",
                    "className": "w-full overflow-x-auto max-h-[400px] overflow-y-auto",
                    "children": [
                        {
                            "type": "table",
                            "className": "w-full text-left font-mono border-collapse",
                            "headers": ["Designation", "Name"],
                            "children": table_rows
                        }
                    ]
                }
            ]
        }))
    }
}