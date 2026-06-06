use serde_json::{json, Value};
use crate::pipeline::{WorkspaceModule, WorkspaceDataContext, CatalogItem};

pub struct MertonBatesCard;

impl WorkspaceModule for MertonBatesCard {
    fn catalog_definition(&self) -> CatalogItem {
        CatalogItem {
            id: "merton_bates_jump_diffusion".to_string(),
            name: "Merton-Bates Jump-Diffusion & Tail Risk Matrix".to_string(),
            description: "Asset price path stress-testing combining continuous geometric Brownian motion with discrete Poisson jump shocks to simulate Value at Risk (VaR) floors.".to_string(),
        }
    }

    fn compile(
        &self, 
        _ticker: &str, 
        _timeframe: &str, 
        data: &WorkspaceDataContext
    ) -> Result<Value, String> {

        // 📊 STEP 1: LOAD JUMP DIFFUSION PAYLOAD MATRIX VIA CONTEXT REPOSITORY
        let mut mb_payload = data.get_dataset("analysis/nse_merton_bates_credit_risk.json");
        if mb_payload.is_null() || mb_payload.as_array().map(|a| a.is_empty()).unwrap_or(true) {
            mb_payload = data.get_dataset("analysis/bse_merton_bates_credit_risk.json");
        }

        let array_records = match mb_payload.as_array() {
            Some(arr) if !arr.is_empty() => arr,
            _ => return Err("Merton-Bates scenario simulation data sets not found or completely empty inside workspace profile cache".to_string()),
        };

        // 📊 STEP 2: SCANNABILITY LOOPS & INTERPOLATION COMPILER
        let mut compiled_rows = Vec::new();
        let mut extreme_tail_risk_floor = f64::MAX;
        let mut baseline_price = 0.0;
        let mut active_date = "Unknown".to_string();

        for cell in array_records {
            let date = cell["snapshot_date"].as_str().unwrap_or("Unknown").to_string();
            let base_price = cell["base_stock_price"].as_f64().unwrap_or(0.0);
            let vol = cell["implied_annual_volatility"].as_f64().unwrap_or(0.0);
            let lambda = cell["jump_intensity_lambda"].as_f64().unwrap_or(0.0);
            let mu_j = cell["expected_jump_size_mu_j"].as_f64().unwrap_or(0.0);
            let var_95 = cell["value_at_risk_95"].as_f64().unwrap_or(0.0);
            let var_99 = cell["value_at_risk_99"].as_f64().unwrap_or(0.0);
            let sim_val = cell["simulated_expected_value"].as_f64().unwrap_or(0.0);

            if base_price > 0.0 {
                baseline_price = base_price;
                active_date = date.clone();
            }

            // Capture absolute lowest tail risk boundary (99% confidence maximum crash shock floor)
            if var_99 > 0.0 && var_99 < extreme_tail_risk_floor {
                extreme_tail_risk_floor = var_99;
            }

            // Conditional formatting for jump dimension parameters
            let jump_size_pct = mu_j * 100.0;
            let (mu_j_text, mu_j_class, mu_j_hex) = if mu_j < 0.0 {
                (format!("{:.1}% Shock", jump_size_pct), "text-rose-400 font-medium", "#f43f5e")
            } else {
                (format!("+{:.1}% Boost", jump_size_pct), "text-emerald-400 font-medium", "#34d399")
            };

            // Evaluate severe drawdown parameters (If VaR floor breaches >40% from base price)
            let is_severe_drawdown = var_99 < (base_price * 0.60);
            let var_99_class = if is_severe_drawdown {
                "text-rose-400 font-bold bg-rose-950/20 px-1.5 py-0.5 rounded border border-rose-900/40"
            } else {
                "text-amber-400 font-medium"
            };
            let var_99_hex = if is_severe_drawdown { "#f43f5e" } else { "#fbbf24" };

            let sim_drift = sim_val - base_price;
            let sim_class = if sim_drift >= 0.0 { "text-emerald-400 font-mono font-bold" } else { "text-rose-400 font-mono font-bold" };
            let sim_hex = if sim_drift >= 0.0 { "#34d399" } else { "#f43f5e" };

            compiled_rows.push(json!({
                "type": "table_row",
                "is_parent": false,
                "is_child": false,
                "align_right_values": true,
                "cells": [
                    { "type": "text", "value": format!("{:.1} / yr", lambda), "className": "text-neutral-300 font-mono" },
                    { "type": "text", "value": mu_j_text, "className": mu_j_class, "style": { "color": mu_j_hex } },
                    { "type": "text", "value": format!("₹ {:.2}", base_price), "className": "text-neutral-400 font-mono" },
                    { "type": "text", "value": format!("{:.2}%", vol * 100.0), "className": "text-neutral-400 font-mono" },
                    { "type": "text", "value": format!("₹ {:.2}", var_95), "className": "text-neutral-200 font-mono" },
                    { "type": "text", "value": format!("₹ {:.2}", var_99), "className": var_99_class, "style": { "color": var_99_hex } },
                    { "type": "text", "value": format!("₹ {:.2}", sim_val), "className": sim_class, "style": { "color": sim_hex } }
                ]
            }));
        }

        // 📊 STEP 3: NARRATIVE RISK INTERPRETER (LaTeX Strict Compliance)
        let total_risk_drawdown_pct = if baseline_price > 0.0 {
            ((baseline_price - extreme_tail_risk_floor) / baseline_price) * 100.0
        } else {
            0.0
        };

        let insight_narrative = format!(
            "STRESS-TEST AUDIT NARRATIVE (As of {}): The Merton-Bates framework superimposes discrete Poisson jump events onto standard continuous asset volatility paths. Under normal parameters, continuous implied annual volatility sits at {:.2}%. However, when black-swan jump anomalies are modeled, non-linear tail risks expand rapidly. In the worst-case high-intensity shock environment ($\\lambda = 3.0$ events/yr with a mean jump size of $\\mu_j = -10.0\\%$), the 99% confidence Value at Risk ($\\text{{VaR}}_{{0.99}}$) drops to an extreme structural protection floor of ₹ {:.2}. This represents an absolute asset drawdown bound of {:.2}% from the current spot price of ₹ {:.2}. If daily price trends breach past local technical supports, this framework flags the exact capital liquidation floor where systemic risk clusters dissolve and fundamental asset value stabilizes.",
            active_date,
            array_records[0]["implied_annual_volatility"].as_f64().unwrap_or(0.0) * 100.0,
            extreme_tail_risk_floor,
            total_risk_drawdown_pct,
            baseline_price
        );

        // 📊 STEP 4: PACKAGE CARD STRUCT LAYOUT SPECIFICATION
        Ok(json!({
            "type": "card",
            "title": "Merton-Bates Jump-Diffusion Risk Summary",
            "subtitle": "// POISSON JUMP-SHOCK SIMULATIONS // NON-LINEAR VALUE AT RISK BOUNDARIES //",
            "footer": format!("Aggregated simulation matrix: Compiled {} jump-intensity parameter fields for horizon target: {}.", array_records.len(), active_date),
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
                                "Jump Intensity (λ)",
                                "Expected Jump Size (μ_j)",
                                "Base Price",
                                "Continuous Vol",
                                "95% VaR Floor",
                                "99% VaR Max Floor",
                                "Simulated Expected Price"
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