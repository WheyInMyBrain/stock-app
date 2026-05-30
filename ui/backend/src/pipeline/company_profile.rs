// stock-app/ui/backend/src/pipeline/company_profile.rs

use serde_json::{json, Value};
use crate::commands::pipeline::CatalogItem;
use crate::pipeline::{WorkspaceModule, WorkspaceDataContext};

pub struct CompanyProfileCard;

impl WorkspaceModule for CompanyProfileCard {
    fn catalog_definition(&self) -> CatalogItem {
        CatalogItem {
            id: "company_profile".to_string(),
            name: "Company Directory Profile".to_string(),
            description: "Static corporate identification tracking metadata, listings timelines, and sector classifications.".to_string(),
        }
    }

    fn compile(&self, ticker: &str, _timeframe: &str, data: &WorkspaceDataContext) -> Result<Value, String> {
        
        // Fallback placeholders
        let mut company_name = format!("{} Limited", ticker.to_uppercase());
        let mut listing_date = "N/A".to_string();
        let mut macro_grp = "N/A".to_string();
        let mut sector = "N/A".to_string();
        let mut industry = "N/A".to_string();
        let mut basic_industry = "N/A".to_string();
        let mut isin_code = "N/A".to_string();
        let mut segment = "N/A".to_string();
        let mut status = "N/A".to_string();
        let mut board = "N/A".to_string();
        let mut share_class = "N/A".to_string();
        let mut tracking_indexes = "N/A".to_string();

        // Target pre-parsed JSON payload variable memory context
        let nse_json = &data.endpoint_metadata;

        if let Some(eq_resp) = nse_json["equityResponse"].as_array().and_then(|a| a.first()) {
            
            // Extract metadata sub-tree (Company Identities)
            if let Some(meta) = eq_resp["metaData"].as_object() {
                if let Some(name) = meta.get("companyName").and_then(|v| v.as_str()) { 
                    company_name = name.trim().to_string(); 
                }
                if let Some(isin) = meta.get("isinCode").and_then(|v| v.as_str()) { 
                    isin_code = isin.trim().to_string(); 
                }
            }

            // Extract extensive security classification tracking loops
            if let Some(sec) = eq_resp["secInfo"].as_object() {
                if let Some(ld) = sec.get("listingDate").and_then(|v| v.as_str()) { listing_date = ld.trim().to_string(); }
                if let Some(mac) = sec.get("macro").and_then(|v| v.as_str()) { macro_grp = mac.trim().to_string(); }
                if let Some(sec_val) = sec.get("sector").and_then(|v| v.as_str()) { sector = sec_val.trim().to_string(); }
                if let Some(ind) = sec.get("industryInfo").and_then(|v| v.as_str()) { industry = ind.trim().to_string(); }
                if let Some(bi) = sec.get("basicIndustry").and_then(|v| v.as_str()) { basic_industry = bi.trim().to_string(); }
                if let Some(seg) = sec.get("tradingSegment").and_then(|v| v.as_str()) { segment = seg.trim().to_string(); }
                if let Some(brd) = sec.get("boardStatus").and_then(|v| v.as_str()) { board = brd.trim().to_string(); }
                if let Some(cls) = sec.get("classShare").and_then(|v| v.as_str()) { share_class = cls.trim().to_string(); }
                
                // 🎯 DYNAMIC STATUS LOOKUP: Maps straight to the real data attribute state string
                if let Some(susp) = sec.get("isSuspended").and_then(|v| v.as_str()) { 
                    status = susp.trim().to_string(); 
                }

                // Map out all indices this asset tracks within dynamically
                if let Some(idx_list) = sec.get("indexList").and_then(|v| v.as_array()) {
                    let indices: Vec<String> = idx_list
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.trim().to_string()))
                        .collect();
                    if !indices.is_empty() {
                        tracking_indexes = indices.join(" | ");
                    }
                }
            }
        }

        // Output visual card tree matrix
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
                    "className": "w-full mt-4 flex flex-col",
                    "style": { "background": "transparent", "border": "none", "padding": "0px", "boxShadow": "none" },
                    "children": [
                        { 
                            "type": "text", 
                            "className": "text-[10px] uppercase font-bold tracking-widest font-mono mb-2 opacity-50", 
                            "value": "Registered Exchange Benchmark Indexes" 
                        },
                        {
                            "type": "container",
                            "className": "w-full flex flex-row flex-wrap gap-2 p-3 rounded-xl has-border",
                            "children": tracking_indexes
                                .split('|')
                                .map(|idx| idx.trim())
                                .filter(|idx| !idx.is_empty() && *idx != "N/A")
                                .map(|idx| {
                                    json!({
                                        "type": "text",
                                        "className": "text-xs font-mono px-2.5 py-1 has-border rounded-md tracking-tight shadow-sm whitespace-nowrap",
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