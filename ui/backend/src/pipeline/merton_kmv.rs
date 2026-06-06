use serde_json::{json, Value};
use crate::pipeline::{WorkspaceModule, WorkspaceDataContext, CatalogItem};

pub struct MertonKmvCard;

impl WorkspaceModule for MertonKmvCard {
    fn catalog_definition(&self) -> CatalogItem {
        CatalogItem {
            id: "merton_kmv_credit_solvency".to_string(),
            name: "Merton-KMV Structural Credit Risk Matrix".to_string(),
            description: "Structural credit solvency model mapping equity option boundaries onto short and long term liabilities to calculate asset values, Distance to Default (DD), and Expected Default Frequency (EDF).".to_string(),
        }
    }

    fn compile(
        &self, 
        _ticker: &str, 
        _timeframe: &str, 
        data: &WorkspaceDataContext
    ) -> Result<Value, String> {

        // 📊 STEP 1: LOAD METRIC MATRIX DATA DIRECTLY FROM WORKSPACE CONTEXT REPOSITORY
        let mut kmv_payload = data.get_dataset("analysis/nse_merton_kmv_default_risk.json");
        if kmv_payload.is_null() || kmv_payload.as_array().map(|a| a.is_empty()).unwrap_or(true) {
            kmv_payload = data.get_dataset("analysis/bse_merton_kmv_default_risk.json");
        }

        let array_records = match kmv_payload.as_array() {
            Some(arr) if !arr.is_empty() => arr,
            _ => return Err("Merton-KMV scenario processing records not found or empty inside workspace cache profile".to_string()),
        };

        // 📊 STEP 2: SCANNABILITY LOOPS & CONDITIONAL CLASS MAPS
        let mut compiled_rows = Vec::new();
        let mut macro_avg_dd = 0.0;
        let mut macro_avg_edf = 0.0;
        let mut sample_count = 0.0;
        let mut latest_date = "Unknown".to_string();

        // Indian Standard Format Separation Closure
        let format_crores = |n: f64| -> String {
            let crores = n / 10_000_000.0;
            if crores == 0.0 { return "₹ 0.00 Cr".to_string(); }
            
            let formatted_str = format!("{:.2}", crores.abs());
            let parts: Vec<&str> = formatted_str.split('.').collect();
            let integer_part = parts[0];
            let decimal_part = parts[1];
            
            let mut grouped_integer = String::new();
            let chars: Vec<char> = integer_part.chars().collect();
            let len = chars.len();
            for (i, ch) in chars.into_iter().enumerate() {
                grouped_integer.push(ch);
                let remaining = len - 1 - i;
                if remaining > 0 && remaining % 3 == 0 {
                    grouped_integer.push(',');
                }
            }
            if n < 0.0 {
                format!("-₹ {}.{} Cr", grouped_integer, decimal_part)
            } else {
                format!("₹ {}.{} Cr", grouped_integer, decimal_part)
            }
        };

        for cell in array_records {
            let date = cell["snapshot_date"].as_str().unwrap_or("Unknown").to_string();
            let equity_cap = cell["equity_value_market_cap"].as_f64().unwrap_or(0.0);
            let barrier = cell["structural_default_barrier"].as_f64().unwrap_or(0.0);
            let asset_val = cell["inferred_asset_value"].as_f64().unwrap_or(0.0);
            let asset_vol = cell["inferred_asset_volatility"].as_f64().unwrap_or(0.0);
            let dd = cell["distance_to_default_dd"].as_f64().unwrap_or(0.0);
            let edf = cell["expected_default_frequency_edf"].as_f64().unwrap_or(0.0);

            latest_date = date.clone();
            sample_count += 1.0;
            macro_avg_dd += dd;
            macro_avg_edf += edf;

            // 🎯 DISTANCE TO DEFAULT (DD) RULES: >2.0 High Safety, >1.0 Moderate, <1.0 High Tail Risk
            let (dd_text, dd_class, dd_hex) = if dd >= 2.0 {
                (format!("🛡️ {:.2} σ", dd), "text-emerald-400 font-bold bg-emerald-950/20 px-2 py-0.5 rounded border border-emerald-900/50", "#34d399")
            } else if dd >= 1.0 {
                (format!("🟡 {:.2} σ", dd), "text-amber-400 font-semibold bg-amber-950/20 px-2 py-0.5 rounded", "#fbbf24")
            } else {
                (format!("🚨 {:.2} σ", dd), "text-rose-400 font-extrabold bg-rose-950/30 px-2 py-0.5 rounded border border-rose-900/50", "#f43f5e")
            };

            // 🎯 EXPECTED DEFAULT FREQUENCY (EDF) RULES: <2.0% Safe, <10.0% Guarded, >=10.0% Elevated
            let edf_pct = edf * 100.0;
            let (edf_class, edf_hex) = if edf < 0.02 {
                ("text-emerald-400 font-mono font-medium", "#34d399")
            } else if edf < 0.10 {
                ("text-amber-400 font-mono", "#fbbf24")
            } else {
                ("text-rose-400 font-mono font-bold bg-rose-950/20 px-1 py-0.5 rounded", "#f43f5e")
            };

            compiled_rows.push(json!({
                "type": "table_row",
                "is_parent": false,
                "is_child": false,
                "align_right_values": true,
                "cells": [
                    { "type": "text", "value": date, "className": "font-bold text-neutral-200" },
                    { "type": "text", "value": format_crores(equity_cap), "className": "text-neutral-400 font-mono" },
                    { "type": "text", "value": format_crores(barrier), "className": "text-neutral-400 font-mono" },
                    { "type": "text", "value": format_crores(asset_val), "className": "text-neutral-300 font-mono font-medium" },
                    { "type": "text", "value": format!("{:.2}%", asset_vol * 100.0), "className": "text-neutral-400 font-mono" },
                    { "type": "text", "value": dd_text, "className": dd_class, "style": { "color": dd_hex } },
                    { "type": "text", "value": format!("{:.2}%", edf_pct), "className": edf_class, "style": { "color": edf_hex } }
                ]
            }));
        }

        if sample_count > 0.0 {
            macro_avg_dd /= sample_count;
            macro_avg_edf /= sample_count;
        }

        // 📊 STEP 3: NARRATIVE RISK EVALUATOR DIRECTIVE (Strict LaTeX Notation Compliance)
        let alert_flag = if macro_avg_dd < 1.20 { "⚠️ ELEVATED CREDIT FLAGGING:" } else { "🟢 SYSTEMIC STABILITY SECURE:" };
        
        let insight_narrative = format!(
            "{} The Merton-KMV struct maps option pricing mechanics onto corporate debt boundaries. By modeling the equity market capitalization as a call option on underlying firm assets, the framework derives the true unobservable Inferred Asset Value ($V_A$) and Inferred Asset Volatility ($\\sigma_A$). Across this chronological timeline, the average Distance to Default ($DD$) stabilizes at {:.2} standard deviations ($\\sigma$) from the default barrier, generating a blended Expected Default Frequency ($EDF$) of {:.2}%. A $DD$ value of {:.2} means the structural asset base sits close to its liability trigger. If market value contracts further or interest-bearing leverage expands without a matching scale in core earnings power, the asset's structural boundary buffer will narrow, pushing default probabilities higher.",
            alert_flag, macro_avg_dd, macro_avg_edf * 100.0, macro_avg_dd
        );

        // 📊 STEP 4: GRID ARCHITECTURE EXPORT FORMAT CONFIGURATION
        Ok(json!({
            "type": "card",
            "title": "Merton-KMV Structural Credit & Solvency Matrix",
            "subtitle": "// STRUCTURAL CREDIT BUFFER MODULATION // ASSET VOLATILITY INVERSIONS & DEBT BOUNDARIES //",
            "footer": format!("Aggregated credit metrics: Tracked {} chronological structural parameters up to closing horizon target: {}.", array_records.len(), latest_date),
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
                                "Reporting Horizon",
                                "Market Capitalization",
                                "Default Barrier (Short + 0.5×Long)",
                                "Inferred Asset Value (V_A)",
                                "Asset Volatility (σ_A)",
                                "Distance to Default (DD)",
                                "Expected Default Freq (EDF)"
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