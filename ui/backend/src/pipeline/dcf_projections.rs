use std::collections::{BTreeMap};
use serde_json::{json, Value};
use crate::pipeline::{WorkspaceModule, WorkspaceDataContext, CatalogItem};

pub struct DcfProjectionsCard;

#[derive(Default)]
struct AnnualDcfSummary {
    year_end: String,
    base_revenue: f64,
    min_rolling: f64,
    max_rolling: f64,
    avg_rolling: f64,
    avg_omniscient: f64,
    sample_count: f64,
    rolling_sum: f64,
    omni_sum: f64,
}

impl WorkspaceModule for DcfProjectionsCard {
    fn catalog_definition(&self) -> CatalogItem {
        CatalogItem {
            id: "dcf_sensitivity_projections".to_string(),
            name: "Discounted Cash Flow (DCF) Sensitivity Projections".to_string(),
            description: "Aggregated results of the concurrent matrix valuation showing historical rolling models vs global omniscient baselines.".to_string(),
        }
    }

    fn compile(
        &self, 
        _ticker: &str, 
        _timeframe: &str, 
        data: &WorkspaceDataContext
    ) -> Result<Value, String> {

        // 📊 STEP 1: RESOLVE FILE KEY REPOSITORIES (FALLING BACK TO NSE PREFERENCES)
        let mut dcf_payload = data.get_dataset("analysis/nse_dcf_projections.json");
        if dcf_payload.is_null() || dcf_payload.as_array().map(|a| a.is_empty()).unwrap_or(true) {
            dcf_payload = data.get_dataset("analysis/bse_dcf_projections.json");
        }

        let array_records = match dcf_payload.as_array() {
            Some(arr) if !arr.is_empty() => arr,
            _ => return Err("DCF projection data matrices not found or contain empty arrays in Workspace Context".to_string()),
        };

        // 📊 STEP 2: AGGREGATE SENSITIVITY RECORDS CHRONOLOGICALLY
        let mut annual_map: BTreeMap<String, AnnualDcfSummary> = BTreeMap::new();

        for cell in array_records {
            let year = cell["year_end"].as_str().unwrap_or("Unknown").to_string();
            let base_rev = cell["base_revenue"].as_f64().unwrap_or(0.0);
            let roll_fv = cell["rolling_fair_value"].as_f64().unwrap_or(0.0);
            let omni_fv = cell["omniscient_fair_value"].as_f64().unwrap_or(0.0);

            // Filter outliers or division-by-zero bounds safely
            if roll_fv.is_infinite() || roll_fv.is_nan() || omni_fv.is_infinite() || omni_fv.is_nan() {
                continue;
            }

            let summary = annual_map.entry(year.clone()).or_insert_with(|| AnnualDcfSummary {
                year_end: year,
                base_revenue: base_rev,
                min_rolling: f64::MAX,
                max_rolling: f64::MIN,
                ..Default::default()
            });

            summary.sample_count += 1.0;
            summary.rolling_sum += roll_fv;
            summary.omni_sum += omni_fv;

            if roll_fv < summary.min_rolling { summary.min_rolling = roll_fv; }
            if roll_fv > summary.max_rolling { summary.max_rolling = roll_fv; }
        }

        // 📊 STEP 3: RE-MAP RAW STATISTICAL MOMENTS AND NARRATIVE PARAMETERS
        let mut compiled_rows = Vec::new();
        let mut macro_avg_rolling = 0.0;
        let mut macro_avg_omni = 0.0;
        let total_years = annual_map.len() as f64;

        for (_, mut summary) in annual_map {
            if summary.sample_count > 0.0 {
                summary.avg_rolling = summary.rolling_sum / summary.sample_count;
                summary.avg_omniscient = summary.omni_sum / summary.sample_count;
            } else {
                summary.min_rolling = 0.0;
                summary.max_rolling = 0.0;
            }

            macro_avg_rolling += summary.avg_rolling;
            macro_avg_omni += summary.avg_omniscient;

            let formatted_rev = format!("₹ {:.2} Cr", summary.base_revenue / 10_000_000.0);

            compiled_rows.push(json!({
                "type": "table_row",
                "is_parent": false,
                "is_child": false,
                "align_right_values": true,
                "cells": [
                    { "type": "text", "value": summary.year_end, "className": "font-bold text-neutral-200" },
                    { "type": "text", "value": formatted_rev, "className": "text-neutral-400 font-mono" },
                    { "type": "text", "value": format!("₹ {:.2}", summary.min_rolling), "className": "text-rose-400 font-mono font-medium" },
                    { "type": "text", "value": format!("₹ {:.2}", summary.max_rolling), "className": "text-emerald-400 font-mono font-medium" },
                    { "type": "text", "value": format!("₹ {:.2}", summary.avg_rolling), "className": "text-amber-400 font-mono font-bold bg-amber-950/10 px-1.5 py-0.5 rounded" },
                    { "type": "text", "value": format!("₹ {:.2}", summary.avg_omniscient), "className": "text-indigo-400 font-mono font-bold bg-indigo-950/20 px-1.5 py-0.5 rounded" }
                ]
            }));
        }

        if total_years > 0.0 {
            macro_avg_rolling /= total_years;
            macro_avg_omni /= total_years;
        }

        // 📊 STEP 4: AUTONOMOUS QUANTITATIVE TEXT TRANSLATOR
        let gap_ratio = if macro_avg_rolling > 0.0 { macro_avg_omni / macro_avg_rolling } else { 1.0 };
        
        let insight_narrative = format!(
            "CRITICAL STRATEGIC AUDIT SUMMARY: Across this computational window, the global Omniscient Value tracks at an average of ₹ {:.2}, creating a {:.2}x gap over the historical Rolling Model average of ₹ {:.2}. {} This gap underscores a standard information-asymmetry variance: the rolling matrix evaluates the enterprise using strict local trailing boundaries, while the look-ahead omniscient model retroactively infuses structural scaling and terminal-stage margin efficiencies into earlier corporate snapshots.",
            macro_avg_omni,
            gap_ratio,
            macro_avg_rolling,
            if gap_ratio > 3.0 {
                "⚠️ CRITICAL VARIANCE DETECTED: The asset displays extreme operational scaling asymmetry. The full-horizon baseline suggests that forward operational visibility shifts value upward significantly compared to point-in-time trailing estimates."
            } else {
                "🟢 MODERATE VARIANCE TRACK: Valuation models display standard, steady corporate scaling trajectories without extreme structural volatility."
            }
        );

        // 📊 STEP 5: OUTPUT CONFIGURATION ARCHITECTURE GRID
        Ok(json!({
            "type": "card",
            "title": "DCF Valuation & Sensitivity Summary Matrix",
            "subtitle": "// MULTI-SCENARIO CONCURRENT ITERATION ITERATIONS // ROLLING HISTORICAL BOUNDS VS LOOK-AHEAD BASES //",
            "footer": format!("Aggregated matrix metadata: Indexed {} raw sensitivity outcomes over {} annual tracking periods.", array_records.len(), total_years),
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
                            "className": "w-full min-w-[750px] text-left border-collapse",
                            "headers": [
                                "Timeline Horizon",
                                "Base Revenue",
                                "Pessimistic Bound (Min)",
                                "Optimistic Bound (Max)",
                                "Rolling Matrix Avg",
                                "Omniscient Baseline"
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