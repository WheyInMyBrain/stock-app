use std::collections::{BTreeMap, BTreeSet};
use serde_json::{json, Value};
use crate::pipeline::{WorkspaceModule, WorkspaceDataContext, CatalogItem};

pub struct MertonBatesCard;

#[derive(Default, Clone)]
struct SnapshotRiskSummary {
    base_stock_price: f64,
    implied_volatility: f64,
    min_var_95: f64,
    min_var_99: f64,
    max_var_99: f64,
    sum_sim_value: f64,
    count: f64,
}

impl WorkspaceModule for MertonBatesCard {
    fn catalog_definition(&self) -> CatalogItem {
        CatalogItem {
            id: "merton_bates_jump_diffusion".to_string(),
            name: "Merton-Bates Jump-Diffusion & Tail Risk Canvas".to_string(),
            description: "Advanced multi-layered vector visualization charting actual spot prices against non-linear Poisson jump liquidation floors over time.".to_string(),
        }
    }

    fn compile(
        &self, 
        _ticker: &str, 
        _timeframe: &str, 
        data: &WorkspaceDataContext
    ) -> Result<Value, String> {

        // 📊 STEP 1: INGEST UNDERLYING JUMP DIFFUSION PERMUTATIONS DATA STREAM
        let mut mb_payload = data.get_dataset("analysis/nse_merton_bates_credit_risk.json");
        if mb_payload.is_null() || mb_payload.as_array().map(|a| a.is_empty()).unwrap_or(true) {
            mb_payload = data.get_dataset("analysis/bse_merton_bates_credit_risk.json");
        }

        let array_records = match mb_payload.as_array() {
            Some(arr) if !arr.is_empty() => arr,
            _ => return Err("Merton-Bates scenario simulation datasets not found or empty inside workspace profile cache".to_string()),
        };

        // 📊 STEP 2: CHRONOLOGICAL ACCUMULATION & WORST-CASE ISOLATION BOUNDS
        let mut chronological_map: BTreeMap<String, SnapshotRiskSummary> = BTreeMap::new();
        
        let mut peak_volatility = 0.0;
        let mut peak_vol_date = String::new();
        let mut deepest_crash_floor = f64::MAX;
        let mut deepest_crash_date = String::new();

        for cell in array_records {
            let date = cell["snapshot_date"].as_str().unwrap_or("Unknown").to_string();
            let base_price = cell["base_stock_price"].as_f64().unwrap_or(0.0);
            let vol = cell["implied_annual_volatility"].as_f64().unwrap_or(0.0);
            let var_95 = cell["value_at_risk_95"].as_f64().unwrap_or(0.0);
            let var_99 = cell["value_at_risk_99"].as_f64().unwrap_or(0.0);
            let sim_val = cell["simulated_expected_value"].as_f64().unwrap_or(0.0);

            if var_95.is_nan() || var_99.is_nan() || var_95.is_infinite() || var_99.is_infinite() {
                continue;
            }

            let summary = chronological_map.entry(date.clone()).or_insert_with(|| SnapshotRiskSummary {
                base_stock_price: base_price,
                implied_volatility: vol,
                min_var_95: f64::MAX,
                min_var_99: f64::MAX,
                max_var_99: f64::MIN,
                ..Default::default()
            });

            summary.count += 1.0;
            summary.sum_sim_value += sim_val;

            if var_95 < summary.min_var_95 { summary.min_var_95 = var_95; }
            if var_99 < summary.min_var_99 { summary.min_var_99 = var_99; }
            if var_99 > summary.max_var_99 { summary.max_var_99 = var_99; }

            // Track Anomaly moments to populate milestone filters
            if vol > peak_volatility {
                peak_volatility = vol;
                peak_vol_date = date.clone();
            }
            if summary.min_var_99 < deepest_crash_floor {
                deepest_crash_floor = summary.min_var_99;
                deepest_crash_date = date;
            }
        }

        if chronological_map.is_empty() {
            return Err("Zero valid operational records parsed following scenario aggregation matrix limits.".to_string());
        }

        // ==============================================================================
        // 📊 STEP 3: ASSEMBLE CLEAN JSON DATA STREAM ARRAY FOR FRONTEND INTERACTION
        // ==============================================================================
        let mut chart_stream_data = Vec::with_capacity(chronological_map.len());
        for (date, summary) in &chronological_map {
            chart_stream_data.push(json!({
                "date": date,
                "price": summary.base_stock_price,
                "var95": summary.min_var_95,
                "var99": summary.min_var_99
            }));
        }

        // ==============================================================================
        // 📊 STEP 4: FILTER DOWN TO EXCLUSIVE CRITICAL MILESTONES (Descending Row Sort)
        // ==============================================================================
        let mut milestone_dates = BTreeSet::new();
        
        if let Some(first_date) = chronological_map.keys().next() { milestone_dates.insert(first_date.clone()); }
        if let Some(last_date) = chronological_map.keys().last() { milestone_dates.insert(last_date.clone()); }
        if !peak_vol_date.is_empty() { milestone_dates.insert(peak_vol_date); }
        if !deepest_crash_date.is_empty() { milestone_dates.insert(deepest_crash_date); }

        for (date, summary) in &chronological_map {
            let local_drawdown = ((summary.base_stock_price - summary.min_var_99) / summary.base_stock_price) * 100.0;
            if local_drawdown >= 45.0 {
                milestone_dates.insert(date.clone());
            }
        }

        let mut compiled_rows = Vec::new();
        let mut latest_spot = 0.0;
        let mut latest_worst_var_99 = 0.0;
        let mut latest_vol = 0.0;
        let mut latest_date = "Unknown".to_string();

        for date in milestone_dates.iter().rev() {
            if let Some(summary) = chronological_map.get(date) {
                let avg_sim_value = if summary.count > 0.0 { summary.sum_sim_value / summary.count } else { summary.base_stock_price };
                
                if latest_date == "Unknown" {
                    latest_spot = summary.base_stock_price;
                    latest_worst_var_99 = summary.min_var_99;
                    latest_vol = summary.implied_volatility;
                    latest_date = date.clone();
                }

                let max_drawdown_pct = ((summary.base_stock_price - summary.min_var_99) / summary.base_stock_price) * 100.0;
                let (var_99_class, var_99_hex) = if max_drawdown_pct >= 40.0 {
                    ("text-rose-400 font-mono font-bold bg-rose-950/20 px-1.5 py-0.5 rounded border border-rose-900/40", "#f43f5e")
                } else {
                    ("text-amber-400 font-mono font-medium", "#fbbf24")
                };

                let sim_drift = avg_sim_value - summary.base_stock_price;
                let sim_class = if sim_drift >= 0.0 { "text-emerald-400 font-mono" } else { "text-rose-400 font-mono" };

                compiled_rows.push(json!({
                    "type": "table_row",
                    "is_parent": false,
                    "is_child": false,
                    "align_right_values": true,
                    "cells": [
                        { "type": "text", "value": date, "className": "font-bold text-neutral-200 text-xs" },
                        { "type": "text", "value": format!("₹ {:.2}", summary.base_stock_price), "className": "text-neutral-300 font-mono text-xs" },
                        { "type": "text", "value": format!("{:.2}%", summary.implied_volatility * 100.0), "className": "text-neutral-400 font-mono text-xs" },
                        { "type": "text", "value": format!("₹ {:.2}", summary.min_var_95), "className": "text-amber-400/90 font-mono text-xs" },
                        { "type": "text", "value": format!("₹ {:.2}", summary.min_var_99), "className": var_99_class, "style": { "color": var_99_hex } },
                        { "type": "text", "value": format!("₹ {:.2}", avg_sim_value), "className": format!("{} text-xs", sim_class) }
                    ]
                }));
            }
        }

        // ==============================================================================
        // 📊 STEP 5: QUANTITATIVE TAIL RISK NARRATIVE (Strict LaTeX Compliance)
        // ==============================================================================
        let total_risk_drawdown_pct = if latest_spot > 0.0 {
            ((latest_spot - latest_worst_var_99) / latest_spot) * 100.0
        } else {
            0.0
        };

        let alert_status = if total_risk_drawdown_pct >= 45.0 { "⚠️ CRITICAL STRATEGIC RISK BREAK:" } else { "🟢 SYSTEMIC LIQUIDITY BUFFER SECURE:" };

        let insight_narrative = format!(
            "{} Merton-Bates jump-diffusion modeling maps a continuous asset volatility baseline ($\\sigma = {:.2}\\%$) super-imposed with discrete Poisson shock structures. To protect viewport frame render capacity and completely bypass DOM layout scrolling stutter, this system routes raw parameters into your interactive graph compositor context. As of the latest reporting horizon snapshot ({}), the maximum modeled stress boundary sets an absolute 99% confidence Value at Risk liquidation cushion ($\\text{{VaR}}_{{0.99}}$) at ₹ {:.2}, representing a maximum potential drawdown limit of {:.2}% from the spot price level of ₹ {:.2}.",
            alert_status, latest_vol * 100.0, latest_date, latest_worst_var_99, total_risk_drawdown_pct, latest_spot
        );

        // ==============================================================================
        // 📊 STEP 6: OUTPUT CONSOLIDATED INTEGRATION FRAMEWORK (Interactive Case)
        // ==============================================================================
        Ok(json!({
            "type": "card",
            "title": "Merton-Bates Jump-Diffusion Canvas Matrix",
            "subtitle": "// INTERACTIVE RISK OVERLAY HISTORY // NO-LAG COMPACT VIEWPORT LAYOUT //",
            "footer": format!("Interactive Canvas Metadata: Streamed {} chronological tracking nodes to the client UI canvas layer.", chronological_map.len()),
            "children": [
                {
                    "type": "text",
                    "value": insight_narrative,
                    "className": "text-xs text-neutral-300 leading-relaxed bg-neutral-900/40 p-3 rounded border border-neutral-800/60 font-sans my-2 shadow-inner"
                },
                {
                    "type": "container",
                    "className": "flex flex-row justify-start gap-4 items-center w-full mt-2 mb-1 px-1 text-[10px] font-mono tracking-wider text-neutral-400",
                    "children": [
                        { "type": "text", "value": "📈 Spot Stock Price", "className": "text-neutral-200 font-bold" },
                        { "type": "text", "value": "🟡 95% Confidence VaR Support", "className": "text-amber-400 font-bold" },
                        { "type": "text", "value": "🔴 99% Black-Swan Shock Floor", "className": "text-rose-400 font-bold" }
                    ]
                },
                // 🚀 THE NEW GENERALLY REUSABLE INTERACTIVE CHART WORKSPACE CASE
                {
                    "type": "interactive_chart",
                    "xAxisKey": "date",
                    "series": [
                        { "key": "price", "label": "Spot Share Price", "stroke": "#e5e5e5", "strokeWidth": 1.5 },
                        { "key": "var95", "label": "95% VaR Cushion", "stroke": "#f59e0b", "strokeWidth": 1.0 },
                        { "key": "var99", "label": "99% Black-Swan Floor", "stroke": "#ef4444", "strokeWidth": 1.5 }
                    ],
                    "data": chart_stream_data
                },
                {
                    "type": "text",
                    "value": "CRITICAL RISK INFLECTION LANDMARKS & ANOMALIES",
                    "className": "text-[10px] font-bold text-neutral-400 tracking-widest mt-4 mb-1 block"
                },
                {
                    "type": "container",
                    "className": "w-full overflow-x-auto [content-visibility:auto] [contain-intrinsic-size:120px]",
                    "children": [
                        {
                            "type": "table",
                            "className": "w-full min-w-[750px] text-left border-collapse",
                            "headers": [
                                "Risk Milestone Horizon",
                                "Spot Share Price",
                                "Continuous Vol (σ)",
                                "95% VaR Cushion",
                                "Worst 99% VaR Limit",
                                "Sim Expected Value"
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