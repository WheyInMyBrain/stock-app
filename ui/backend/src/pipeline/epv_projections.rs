use std::collections::BTreeMap;
use serde_json::{json, Value};
use crate::pipeline::{WorkspaceModule, WorkspaceDataContext, CatalogItem};

pub struct EpvProjectionsCard;

#[derive(Default)]
struct AnnualEpvSummary {
    year_end: String,
    base_revenue: f64,
    min_epv: f64,
    max_epv: f64,
    avg_epv: f64,
    avg_ebit_margin: f64,
    avg_normalized_fcf: f64,
    sample_count: f64,
    epv_sum: f64,
    ebit_margin_sum: f64,
    fcf_sum: f64,
}

impl WorkspaceModule for EpvProjectionsCard {
    fn catalog_definition(&self) -> CatalogItem {
        CatalogItem {
            id: "epv_no_growth_projections".to_string(),
            name: "Earnings Power Value (EPV) Floor Projections".to_string(),
            description: "Bruce Greenwald's zero-growth valuation model isolating current sustainable operating earnings power from speculative growth assumptions.".to_string(),
        }
    }

    fn compile(
        &self, 
        _ticker: &str, 
        _timeframe: &str, 
        data: &WorkspaceDataContext
    ) -> Result<Value, String> {

        // 📊 STEP 1: ESTABLISH SYSTEM RESILIENT DATA STREAM CROSS-EXCHANGE FALLBACKS
        let mut epv_payload = data.get_dataset("analysis/nse_epv_projections.json");
        if epv_payload.is_null() || epv_payload.as_array().map(|a| a.is_empty()).unwrap_or(true) {
            epv_payload = data.get_dataset("analysis/bse_epv_projections.json");
        }

        let array_records = match epv_payload.as_array() {
            Some(arr) if !arr.is_empty() => arr,
            _ => return Err("EPV scenario matrix data sets not found or completely empty inside workspace profile cache".to_string()),
        };

        // 📊 STEP 2: CHRONOLOGICAL ACCUMULATION MATRIX BUCKETER
        let mut annual_map: BTreeMap<String, AnnualEpvSummary> = BTreeMap::new();

        for cell in array_records {
            let year = cell["year_end"].as_str().unwrap_or("Unknown").to_string();
            let base_rev = cell["base_revenue"].as_f64().unwrap_or(0.0);
            let ebit_margin = cell["historical_ebit_margin"].as_f64().unwrap_or(0.0);
            let norm_fcf = cell["normalized_fcf"].as_f64().unwrap_or(0.0);
            let epv_val = cell["epv_fair_value"].as_f64().unwrap_or(0.0);

            if epv_val.is_infinite() || epv_val.is_nan() || norm_fcf.is_infinite() || norm_fcf.is_nan() {
                continue;
            }

            let summary = annual_map.entry(year.clone()).or_insert_with(|| AnnualEpvSummary {
                year_end: year,
                base_revenue: base_rev,
                min_epv: f64::MAX,
                max_epv: f64::MIN,
                ..Default::default()
            });

            summary.sample_count += 1.0;
            summary.epv_sum += epv_val;
            summary.ebit_margin_sum += ebit_margin;
            summary.fcf_sum += norm_fcf;

            if epv_val < summary.min_epv { summary.min_epv = epv_val; }
            if epv_val > summary.max_epv { summary.max_epv = epv_val; }
        }

        // 📊 STEP 3: NATIVE VALUE COMPILER PIPELINE GRID GENERATOR
        let mut compiled_rows = Vec::new();
        let mut macro_avg_epv = 0.0;
        let mut macro_avg_fcf = 0.0;
        let total_years = annual_map.len() as f64;

        for (_, mut summary) in annual_map {
            if summary.sample_count > 0.0 {
                summary.avg_epv = summary.epv_sum / summary.sample_count;
                summary.avg_ebit_margin = summary.ebit_margin_sum / summary.sample_count;
                summary.avg_normalized_fcf = summary.fcf_sum / summary.sample_count;
            } else {
                summary.min_epv = 0.0;
                summary.max_epv = 0.0;
            }

            macro_avg_epv += summary.avg_epv;
            macro_avg_fcf += summary.avg_normalized_fcf;

            let formatted_rev = format!("₹ {:.2} Cr", summary.base_revenue / 10_000_000.0);
            let formatted_ebit = format!("{:.2}%", summary.avg_ebit_margin * 100.0);
            let formatted_fcf = format!("₹ {:.2} Cr", summary.avg_normalized_fcf / 10_000_000.0);

            compiled_rows.push(json!({
                "type": "table_row",
                "is_parent": false,
                "is_child": false,
                "align_right_values": true,
                "cells": [
                    { "type": "text", "value": summary.year_end, "className": "font-bold text-neutral-200" },
                    { "type": "text", "value": formatted_rev, "className": "text-neutral-400 font-mono" },
                    { "type": "text", "value": formatted_ebit, "className": "text-neutral-400 font-mono font-medium" },
                    { "type": "text", "value": formatted_fcf, "className": "text-teal-400 font-mono font-medium" },
                    { "type": "text", "value": format!("₹ {:.2}", summary.min_epv), "className": "text-rose-400 font-mono" },
                    { "type": "text", "value": format!("₹ {:.2}", summary.max_epv), "className": "text-emerald-400 font-mono" },
                    { "type": "text", "value": format!("₹ {:.2}", summary.avg_epv), "className": "text-indigo-400 font-mono font-bold bg-indigo-950/30 px-1.5 py-0.5 rounded border border-indigo-900/30" }
                ]
            }));
        }

        if total_years > 0.0 {
            macro_avg_epv /= total_years;
            macro_avg_fcf /= total_years;
        }

        // 📊 STEP 4: AUTONOMOUS QUANTITATIVE TEXT TRANSLATOR
        let insight_narrative = format!(
            "QUANTITATIVE EPV FRAMEWORK ANALYSIS: The Earnings Power Value engine provides a rigorous zero-growth operational baseline evaluation ($g=0$). Across our multi-scenario matrix matrix evaluation, the macro sustainable EPV Fair Value floor stabilizes at an average of ₹ {:.2}, supported by a normalized sustainable Free Cash Flow generation midpoint of ₹ {:.2} Cr. This metrics matrix represents 'fundamental gravity': if this corporate entity completely stops scaling sales expansion today but maintains its current baseline operating margin efficiencies indefinitely, this is its standalone intrinsic worth. Cross-referencing this zero-growth anchor directly against your dynamic DCF matrix allows you to cleanly isolate exactly what portion of the current stock price premium is based on verified metrics vs speculative future growth projections.",
            macro_avg_epv,
            macro_avg_fcf / 10_000_000.0
        );

        // 📊 STEP 5: COMPREHENSIVE CARD LAYOUT DESIGN PACKAGING
        Ok(json!({
            "type": "card",
            "title": "Earnings Power Value (EPV) Operational Floor Matrix",
            "subtitle": "// SUSTAINABLE ZERO-GROWTH OPERATIONAL ENGINE // CORE MOAT ISOLATION MODELS //",
            "footer": format!("Aggregated EPV matrix data: Processed {} scenario points across {} chronological tracking blocks.", array_records.len(), total_years),
            "children": [
                {
                    "type": "text",
                    "value": insight_narrative,
                    "className": "text-xs text-neutral-300 leading-relaxed bg-neutral-900/40 p-3 rounded border border-neutral-800/60 font-sans my-3 shadow-inner"
                },
                {
                    "type": "container",
                    "className": "w-full overflow-x-auto mt-2",
                    "children": [
                        {
                            "type": "table",
                            "className": "w-full min-w-[850px] text-left border-collapse",
                            "headers": [
                                "Timeline Horizon",
                                "Sustainable Revenue",
                                "Avg EBIT Margin",
                                "Normalized Cash Flow",
                                "EPV Floor (Min)",
                                "EPV Ceiling (Max)",
                                "EPV Matrix Avg"
                            ],
                            "align_right_columns": true,
                            "children": compiled_rows
                        }
                    ]
                }
            ]
        }))
    }
}