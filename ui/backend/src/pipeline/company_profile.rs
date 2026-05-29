use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use serde_json::{json, Value};
use crate::commands::pipeline::CatalogItem;
use crate::pipeline::WorkspaceModule;

pub struct CompanyProfileCard;

fn get_shared_data_directory() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); path.pop(); // Back out of ui/backend/ to stock-app/
    path.push("data");
    path
}

impl WorkspaceModule for CompanyProfileCard {
    fn catalog_definition(&self) -> CatalogItem {
        CatalogItem {
            id: "company_profile".to_string(),
            name: "Company Directory Profile".to_string(),
            description: "Static corporate identification tracking metadata, listings timelines, and sector classifications.".to_string(),
        }
    }

    fn compile(&self, ticker: &str, _timeframe: &str) -> Result<Value, String> {
        let mut path = get_shared_data_directory();
        path.push("IMFA");
        path.push("nse_symbol-core-data");
        path.push("endpoint-metadata.json");

        // Fallback placeholders
        let mut company_name = format!("{} Limited", ticker.to_uppercase());
        let mut listing_date = "N/A".to_string();
        let mut macro_grp = "N/A".to_string();
        let mut sector = "N/A".to_string();
        let mut industry = "N/A".to_string();
        let mut basic_industry = "N/A".to_string();
        let mut isin_code = "N/A".to_string();
        let mut segment = "Normal Market".to_string();
        let mut status = "Active".to_string();
        let mut board = "Main".to_string();
        let mut share_class = "Equity".to_string();
        let mut tracking_indexes = "N/A".to_string();

        if path.exists() {
            if let Ok(mut file) = File::open(&path) {
                let mut content = String::new();
                if file.read_to_string(&mut content).is_ok() {
                    if let Ok(nse_json) = serde_json::from_str::<Value>(&content) {
                        if let Some(eq_resp) = nse_json["equityResponse"].as_array().and_then(|a| a.first()) {
                            
                            // Extract metadata sub-tree
                            if let Some(meta) = eq_resp["metaData"].as_object() {
                                if let Some(name) = meta.get("companyName").and_then(|v| v.as_str()) { company_name = name.to_string(); }
                                if let Some(isin) = meta.get("isinCode").and_then(|v| v.as_str()) { isin_code = isin.to_string(); }
                            }

                            // Extract extensive security classification tracking loops
                            if let Some(sec) = eq_resp["secInfo"].as_object() {
                                if let Some(ld) = sec.get("listingDate").and_then(|v| v.as_str()) { listing_date = ld.to_string(); }
                                if let Some(mac) = sec.get("macro").and_then(|v| v.as_str()) { macro_grp = mac.to_string(); }
                                if let Some(sec_val) = sec.get("sector").and_then(|v| v.as_str()) { sector = sec_val.to_string(); }
                                if let Some(ind) = sec.get("industryInfo").and_then(|v| v.as_str()) { industry = ind.to_string(); }
                                if let Some(bi) = sec.get("basicIndustry").and_then(|v| v.as_str()) { basic_industry = bi.to_string(); }
                                if let Some(seg) = sec.get("tradingSegment").and_then(|v| v.as_str()) { segment = seg.to_string(); }
                                if let Some(susp) = sec.get("isSuspended").and_then(|v| v.as_str()) { status = susp.to_string(); }
                                if let Some(brd) = sec.get("boardStatus").and_then(|v| v.as_str()) { board = brd.to_string(); }
                                if let Some(cls) = sec.get("classShare").and_then(|v| v.as_str()) { share_class = cls.to_string(); }

                                // Map out all indices this asset tracks within
                                if let Some(idx_list) = sec.get("indexList").and_then(|v| v.as_array()) {
                                    let indices: Vec<String> = idx_list.iter().filter_map(|v| v.as_str().map(|s| s.trim().to_string())).collect();
                                    if !indices.is_empty() {
                                        tracking_indexes = indices.join(" | ");
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(json!({
            "type": "card",
            "title": company_name,
            "subtitle": format!("// REGULATORY COMPANY INFO NODE: {}", ticker.to_uppercase()),
            "footer": format!("ISIN Reference Hash: {}", isin_code),
            "children": [
                {
                    "type": "container",
                    "className": "w-full",
                    "children": [
                        { "type": "metric", "title": "Security Class Shared", "value": share_class },
                        { "type": "metric", "title": "Listing Date", "value": listing_date },
                        { "type": "metric", "title": "Macro Sector", "value": macro_grp },
                        { "type": "metric", "title": "Sector Group", "value": sector },
                        { "type": "metric", "title": "Operational Status", "value": status }
                    ]
                },
                {
                    "type": "container",
                    "className": "w-full mt-4",
                    "children": [
                        { "type": "metric", "title": "Industry Sector", "value": industry },
                        { "type": "metric", "title": "Core Basic Industry", "value": basic_industry },
                        { "type": "metric", "title": "Exchange Segment", "value": segment },
                        { "type": "metric", "title": "Board Class", "value": board }
                    ]
                },
                {
                    "type": "container",
                    "className": "w-full mt-4",
                    "style": { "background": "transparent", "border": "none", "padding": "0px", "boxShadow": "none", "display": "flex", "flexDirection": "column" },
                    "children": [
                        { "type": "text", "className": "text-[10px] uppercase font-bold tracking-widest text-neutral-500 font-mono mb-2", "value": "Registered Exchange Benchmark Indexes" },
                        {
                            "type": "container",
                            "className": "w-full flex flex-row flex-wrap gap-2 p-3 bg-neutral-950/60 rounded-xl border border-neutral-900/60",
                            "style": { "display": "flex", "flexDirection": "row" },
                            "children": tracking_indexes
                                .split('|')
                                .map(|idx| idx.trim())
                                .filter(|idx| !idx.is_empty() && *idx != "N/A")
                                .map(|idx| {
                                    json!({
                                        "type": "text",
                                        "className": "text-xs font-mono px-2.5 py-1 bg-neutral-900/40 text-neutral-300 rounded-md border border-neutral-800/40 tracking-tight shadow-sm whitespace-nowrap",
                                        "value": idx.to_string()
                                    })
                                })
                                .collect::<Vec<Value>>()
                        }
                    ]
                }
            ]
        }))
    }
}