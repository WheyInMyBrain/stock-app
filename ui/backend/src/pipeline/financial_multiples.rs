use std::collections::{HashMap, BTreeSet};
use serde_json::{json, Value};
use crate::pipeline::{WorkspaceModule, WorkspaceDataContext, CatalogItem};

pub struct CorporateMultiplesCard;

struct HierarchyRowConfig {
    tag_name: &'static str,
    display_name: &'static str,
    format_type: &'static str,
    is_parent: bool,
    parent_id: &'static str,
}

// ─────────────────────────────────────────────────────────────────────────────
// 🚀 SCANNABILITY ICONOGRAPHY & COLOR FORMATTING ENGINE
// ─────────────────────────────────────────────────────────────────────────────

/// Parses numbers and evaluates them against strict accounting thresholds, returning
/// a tuple of (Formatted Text with Emojis, Tailwind ClassName, Inline Hex Color Code).
/// Parses numerical inputs and checks them against structural financial thresholds,
/// returning a unified tuple of (Formatted Text with Icons, Tailwind ClassName, Inline Hex Color).
fn format_metric_with_visuals(tag_name: &str, val: &Value, format_type: &str) -> (String, String, String) {
    if val.is_null() {
        return ("-".to_string(), "text-neutral-500 font-mono".to_string(), "#737373".to_string());
    }

    let num_opt = if val.is_f64() {
        val.as_f64()
    } else if val.is_i64() {
        val.as_i64().map(|v| v as f64)
    } else if let Some(s) = val.as_str() {
        s.trim().parse::<f64>().ok()
    } else {
        None
    };

    let num = match num_opt {
        Some(n) => n,
        None => return (val.to_string().replace('"', ""), "text-neutral-200".to_string(), "#e5e5e5".to_string()),
    };

    let mut class_name = "text-neutral-200 font-mono".to_string();
    let mut hex_color = "#e5e5e5".to_string();

    // Indian Standard Comma Grouping Separator Closure
    let format_commas = |n: f64, decimals: usize| -> String {
        let formatted_str = format!("{:.1$}", n.abs(), decimals);
        let parts: Vec<&str> = formatted_str.split('.').collect();
        let integer_part = parts[0];
        
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
        if parts.len() > 1 {
            format!("{}{}.{}", if n < 0.0 { "-" } else { "" }, grouped_integer, parts[1])
        } else {
            format!("{}{}", if n < 0.0 { "-" } else { "" }, grouped_integer)
        }
    };

    // 🎯 REFACTOR OPTIMIZATION: The match tree now assigns the text string directly
    let display_text = match tag_name {
        // 📈 PROFITABILITY & RETURNS (Thresholds: >=15% Excellent, >=7% Fair, <7% Thin)
        "ebit_margin" | "net_margin" | "fcf_margin" | "roic" | "roe" | "roa" | "dupond_operating_margin" => {
            let pct = num * 100.0;
            if num >= 0.15 {
                class_name = "text-emerald-400 font-bold bg-emerald-950/30 px-2 py-0.5 rounded border border-emerald-900/50".to_string();
                hex_color = "#34d399".to_string();
                format!("🟢 {:.2}%", pct)
            } else if num >= 0.07 {
                class_name = "text-amber-400 font-medium bg-amber-950/20 px-2 py-0.5 rounded".to_string();
                hex_color = "#fbbf24".to_string();
                format!("🟡 {:.2}%", pct)
            } else {
                class_name = "text-rose-400 font-bold bg-rose-950/30 px-2 py-0.5 rounded border border-rose-900/50".to_string();
                hex_color = "#f43f5e".to_string();
                format!("🔴 {:.2}%", pct)
            }
        },

        // 🛡️ SOLVENCY & LEVERAGE DEBT MULTIPLIERS (D/E: <=0.5 Conservative, <=1.5 Fair, >1.5 Dangerous)
        "debt_to_equity" | "dupond_leverage_multiplier" => {
            let leverage_offset = if tag_name == "dupond_leverage_multiplier" { num - 1.0 } else { num };
            if leverage_offset <= 0.5 {
                class_name = "text-emerald-400 font-semibold".to_string();
                hex_color = "#34d399".to_string();
                format!("🛡️ {:.2}x", num)
            } else if leverage_offset <= 1.5 {
                class_name = "text-amber-400".to_string();
                hex_color = "#fbbf24".to_string();
                format!("🟡 {:.2}x", num)
            } else {
                class_name = "text-rose-400 font-bold bg-rose-950/20 px-1.5 py-0.5 rounded".to_string();
                hex_color = "#f43f5e".to_string();
                format!("⚠️ {:.2}x", num)
            }
        },

        // 💸 LIQUIDITY CRITERIA (Threshold: >=1.8 Liquid, >=1.0 Tight, <1.0 Cash Crunch)
        "current_ratio" | "quick_ratio" => {
            if num >= 1.8 {
                class_name = "text-emerald-400 font-medium".to_string();
                hex_color = "#34d399".to_string();
                format!("🟢 {:.2}x", num)
            } else if num >= 1.0 {
                class_name = "text-amber-400".to_string();
                hex_color = "#fbbf24".to_string();
                format!("🟡 {:.2}x", num)
            } else {
                class_name = "text-rose-400 font-extrabold bg-rose-950/40 px-2 py-0.5 rounded border border-rose-900".to_string();
                hex_color = "#f43f5e".to_string();
                format!("🚨 {:.2}x", num)
            }
        },

        // ⚡ INTEREST SERVICE BARS
        "interest_coverage" => {
            if num >= 4.0 {
                class_name = "text-emerald-400 font-medium".to_string();
                hex_color = "#34d399".to_string();
                format!("🟢 {:.2}x", num)
            } else if num >= 1.5 {
                class_name = "text-amber-400".to_string();
                hex_color = "#fbbf24".to_string();
                format!("🟡 {:.2}x", num)
            } else {
                class_name = "text-rose-400 font-black bg-rose-950/40 px-2 py-0.5 rounded border border-rose-900".to_string();
                hex_color = "#f43f5e".to_string();
                format!("💥 {:.2}x", num)
            }
        },

        // 🏆 PIOTROSKI FINANCIAL HEALTH SCORE (Max 9; >=7 Safe, 4-6 Grey, <=3 Weak)
        "piotroski_f_score" => {
            if num >= 7.0 {
                class_name = "text-emerald-400 font-black bg-emerald-950/50 border border-emerald-700 px-2.5 py-0.5 rounded text-xs tracking-wider".to_string();
                hex_color = "#34d399".to_string();
                format!("🏆 ⭐ {:.0} / 9", num)
            } else if num >= 4.0 {
                class_name = "text-amber-400 font-bold bg-amber-950/30 px-2.5 py-0.5 rounded text-xs".to_string();
                hex_color = "#fbbf24".to_string();
                format!("⚖️ {:.0} / 9", num)
            } else {
                class_name = "text-rose-400 font-black bg-rose-950/50 border border-rose-800 px-2.5 py-0.5 rounded text-xs tracking-wider".to_string();
                hex_color = "#f43f5e".to_string();
                format!("💀 🚨 {:.0} / 9", num)
            }
        },

        // 💀 ALTMAN Z-SCORE BANKRUPTCY ENGINE
        "altman_z_score" => {
            if num > 2.99 {
                class_name = "text-emerald-400 font-bold".to_string();
                hex_color = "#34d399".to_string();
                format!("🟢 {:.2} [Safe]", num)
            } else if num >= 1.81 {
                class_name = "text-amber-400".to_string();
                hex_color = "#fbbf24".to_string();
                format!("🟡 {:.2} [Grey]", num)
            } else {
                class_name = "text-rose-400 font-black bg-rose-950/30 px-2 py-0.5 rounded border border-rose-900/40".to_string();
                hex_color = "#f43f5e".to_string();
                format!("💀 {:.2} [Distress]", num)
            }
        },

        // 🚩 BENEISH M-SCORE MANIPULATION AUDITOR (Threshold -1.78; greater means high risk)
        "beneish_m_score" => {
            if num > -1.78 {
                class_name = "text-rose-400 font-extrabold bg-rose-950/40 border border-rose-800 px-2 py-0.5 rounded".to_string();
                hex_color = "#f43f5e".to_string();
                format!("🚩 {:.2} [Anomaly]", num)
            } else {
                class_name = "text-emerald-400 font-medium".to_string();
                hex_color = "#34d399".to_string();
                format!("🟢 {:.2} [Secure]", num)
            }
        },

        // 🛡️ INTRINSIC MARGIN OF SAFETY
        "margin_of_safety_pct" => {
            if num >= 30.0 {
                class_name = "text-emerald-400 font-black bg-emerald-950/20 px-1.5 py-0.5 rounded".to_string();
                hex_color = "#34d399".to_string();
                format!("🛡️ {:.2}%", num)
            } else if num >= 10.0 {
                class_name = "text-amber-400".to_string();
                hex_color = "#fbbf24".to_string();
                format!("🟡 {:.2}%", num)
            } else {
                class_name = "text-rose-400 font-semibold".to_string();
                hex_color = "#f43f5e".to_string();
                format!("🔴 {:.2}%", num)
            }
        },

        // ⚡ REVENUE SHOCK MATRIX COEFFICIENTS
        s if s.starts_with("elasticity_shock_up") => {
            class_name = "text-emerald-400 font-bold".to_string();
            hex_color = "#34d399".to_string();
            format!("▲ +{:.2}%", num)
        },
        s if s.starts_with("elasticity_shock_down") => {
            class_name = "text-rose-400 font-bold".to_string();
            hex_color = "#f43f5e".to_string();
            format!("▼ {:.2}%", num)
        },

        // ⚙️ STANDARDIZED SCALAR METRIC CONVERSIONS
        _ => match format_type {
            "currency_cr" => format!("₹ {} Cr", format_commas(num, 2)),
            "currency_raw" => format!("₹ {:.2}", num),
            "percentage_raw" => format!("{:.2}%", num),
            "multiplier" => format!("{:.2}x", num),
            "days" => format!("{:.1} days", num),
            "years" => format!("{:.1} yrs", num),
            "months" => format!("{:.1} mos", num),
            "count_raw" => format_commas(num, 0),
            _ => format!("{:.2}", num),
        }
    };

    (display_text, class_name, hex_color)
}

// ─────────────────────────────────────────────────────────────────────────────
// 🚀 WORKSPACE TRAIT IMPLEMENTATION LAYER
// ─────────────────────────────────────────────────────────────────────────────

impl WorkspaceModule for CorporateMultiplesCard {
    fn catalog_definition(&self) -> CatalogItem {
        CatalogItem {
            id: "corporate_financial_multiples".to_string(),
            name: "Corporate Valuation & Multiples Matrix".to_string(),
            description: "Advanced financial health metrics, stress-test slump models, and DuPont breakdowns with conditional scannability shading.".to_string(),
        }
    }

    fn compile(
        &self, 
        _ticker: &str, 
        _timeframe: &str, 
        data: &WorkspaceDataContext
    ) -> Result<Value, String> {

        let multiples_payload = data.get_dataset("analysis/nse_corporate_financial_multiples.json");
        
        let array_records = match multiples_payload.as_array() {
            Some(arr) => arr,
            None => return Err("Dataset file not found or failed to parse multiples as a JSON array".to_string()),
        };

        let mut unique_dates = BTreeSet::new();
        let mut data_lookup_matrix: HashMap<String, Value> = HashMap::new();

        for item in array_records {
            let date_str = item["snapshot_date"].as_str().unwrap_or("Unknown Date").to_string();
            unique_dates.insert(date_str.clone());

            if let Some(obj) = item.as_object() {
                for (metric_key, metric_val) in obj {
                    let matrix_lookup_key = format!("{}__{}", date_str, metric_key);
                    data_lookup_matrix.insert(matrix_lookup_key, metric_val.clone());
                }
            }
        }

        let mut chronological_headers: Vec<String> = unique_dates.into_iter().collect();
        chronological_headers.sort_by(|a, b| b.cmp(a));

        let structured_multiples_tree = vec![
            // ─── SCALE & VALUATION
            HierarchyRowConfig { tag_name: "scale_valuation_group", display_name: "Valuation & Scale Multiples", format_type: "", is_parent: true, parent_id: "scale_valuation_group" },
            HierarchyRowConfig { tag_name: "revenue", display_name: "Revenue from Operations", format_type: "currency_cr", is_parent: false, parent_id: "scale_valuation_group" },
            HierarchyRowConfig { tag_name: "stock_price", display_name: "Stock Closing Price", format_type: "currency_raw", is_parent: false, parent_id: "scale_valuation_group" },
            HierarchyRowConfig { tag_name: "total_shares", display_name: "Total Outstanding Shares", format_type: "count_raw", is_parent: false, parent_id: "scale_valuation_group" },
            HierarchyRowConfig { tag_name: "enterprise_value", display_name: "Enterprise Value (EV)", format_type: "currency_cr", is_parent: false, parent_id: "scale_valuation_group" },
            HierarchyRowConfig { tag_name: "ev_to_ebitda", display_name: "EV / EBITDA Multiple", format_type: "multiplier", is_parent: false, parent_id: "scale_valuation_group" },

            // ─── PROFITABILITY & RETURNS
            HierarchyRowConfig { tag_name: "profitability_returns_group", display_name: "Profitability & Return Matrix", format_type: "", is_parent: true, parent_id: "profitability_returns_group" },
            HierarchyRowConfig { tag_name: "ebit_margin", display_name: "Operating Margin (EBIT)", format_type: "percentage_decimal", is_parent: false, parent_id: "profitability_returns_group" },
            HierarchyRowConfig { tag_name: "net_margin", display_name: "Net Profit Margin", format_type: "percentage_decimal", is_parent: false, parent_id: "profitability_returns_group" },
            HierarchyRowConfig { tag_name: "fcf_margin", display_name: "Free Cash Flow (FCF) Margin", format_type: "percentage_decimal", is_parent: false, parent_id: "profitability_returns_group" },
            HierarchyRowConfig { tag_name: "roic", display_name: "Return on Invested Capital (ROIC)", format_type: "percentage_decimal", is_parent: false, parent_id: "profitability_returns_group" },
            HierarchyRowConfig { tag_name: "roe", display_name: "Return on Equity (ROE)", format_type: "percentage_decimal", is_parent: false, parent_id: "profitability_returns_group" },
            HierarchyRowConfig { tag_name: "roa", display_name: "Return on Assets (ROA)", format_type: "percentage_decimal", is_parent: false, parent_id: "profitability_returns_group" },

            // ─── DUPONT RATIOS
            HierarchyRowConfig { tag_name: "dupont_tree_group", display_name: "DuPont Expansion Tree", format_type: "", is_parent: true, parent_id: "dupont_tree_group" },
            HierarchyRowConfig { tag_name: "dupond_operating_margin", display_name: "DuPont Operating Margin", format_type: "percentage_decimal", is_parent: false, parent_id: "dupont_tree_group" },
            HierarchyRowConfig { tag_name: "dupond_asset_turnover", display_name: "DuPont Asset Turnover", format_type: "multiplier", is_parent: false, parent_id: "dupont_tree_group" },
            HierarchyRowConfig { tag_name: "dupond_leverage_multiplier", display_name: "DuPont Leverage Multiplier", format_type: "multiplier", is_parent: false, parent_id: "dupont_tree_group" },
            HierarchyRowConfig { tag_name: "dupond_interest_burden", display_name: "DuPont Interest Burden Ratio", format_type: "multiplier", is_parent: false, parent_id: "dupont_tree_group" },
            HierarchyRowConfig { tag_name: "dupond_tax_burden", display_name: "DuPont Tax Burden Ratio", format_type: "multiplier", is_parent: false, parent_id: "dupont_tree_group" },

            // ─── LIQUIDITY & DEBT SOLVENCY
            HierarchyRowConfig { tag_name: "solvency_liquidity_group", display_name: "Liquidity & Debt Solvency", format_type: "", is_parent: true, parent_id: "solvency_liquidity_group" },
            HierarchyRowConfig { tag_name: "debt_to_equity", display_name: "Debt to Equity Ratio", format_type: "multiplier", is_parent: false, parent_id: "solvency_liquidity_group" },
            HierarchyRowConfig { tag_name: "current_ratio", display_name: "Current Ratio", format_type: "multiplier", is_parent: false, parent_id: "solvency_liquidity_group" },
            HierarchyRowConfig { tag_name: "quick_ratio", display_name: "Quick Ratio (Acid Test)", format_type: "multiplier", is_parent: false, parent_id: "solvency_liquidity_group" },
            HierarchyRowConfig { tag_name: "interest_coverage", display_name: "Interest Coverage Ratio", format_type: "multiplier", is_parent: false, parent_id: "solvency_liquidity_group" },
            HierarchyRowConfig { tag_name: "cash_conversion_cycle_days", display_name: "Cash Conversion Cycle", format_type: "days", is_parent: false, parent_id: "solvency_liquidity_group" },
            HierarchyRowConfig { tag_name: "inventory_turnover", display_name: "Inventory Turnover Ratio", format_type: "multiplier", is_parent: false, parent_id: "solvency_liquidity_group" },
            HierarchyRowConfig { tag_name: "defensive_cash_burn_months", display_name: "Defensive Cash Interval", format_type: "months", is_parent: false, parent_id: "solvency_liquidity_group" },

            // ─── EFFICIENCY & LEVERAGE
            HierarchyRowConfig { tag_name: "operating_leverage_group", display_name: "Operating Efficiency & Leverage", format_type: "", is_parent: true, parent_id: "operating_leverage_group" },
            HierarchyRowConfig { tag_name: "degree_of_operating_leverage", display_name: "Degree of Operating Leverage (DOL)", format_type: "multiplier", is_parent: false, parent_id: "operating_leverage_group" },
            HierarchyRowConfig { tag_name: "breakeven_operating_revenue", display_name: "Breakeven Operating Revenue", format_type: "currency_cr", is_parent: false, parent_id: "operating_leverage_group" },
            HierarchyRowConfig { tag_name: "accruals_to_sales_intensity", display_name: "Accruals-to-Sales Intensity", format_type: "percentage_decimal", is_parent: false, parent_id: "operating_leverage_group" },
            HierarchyRowConfig { tag_name: "capex_to_depreciation_coverage", display_name: "CapEx-to-Depreciation Coverage", format_type: "multiplier", is_parent: false, parent_id: "operating_leverage_group" },
            HierarchyRowConfig { tag_name: "estimated_infrastructure_nbv_age_years", display_name: "Estimated Fixed Asset Age", format_type: "years", is_parent: false, parent_id: "operating_leverage_group" },

            // ─── RISKS & DIAGNOSTIC SCORES
            HierarchyRowConfig { tag_name: "health_scores_group", display_name: "Financial Health & Risk Scores", format_type: "", is_parent: true, parent_id: "health_scores_group" },
            HierarchyRowConfig { tag_name: "piotroski_f_score", display_name: "Piotroski F-Score", format_type: "score", is_parent: false, parent_id: "health_scores_group" },
            HierarchyRowConfig { tag_name: "beneish_m_score", display_name: "Beneish M-Score (Earnings Manipulation)", format_type: "score", is_parent: false, parent_id: "health_scores_group" },
            HierarchyRowConfig { tag_name: "altman_z_score", display_name: "Altman Z-Score (Bankruptcy Risk)", format_type: "score", is_parent: false, parent_id: "health_scores_group" },

            // ─── SHAREHOLDING STRUCTURE
            HierarchyRowConfig { tag_name: "shareholding_pattern_group", display_name: "Ownership Structure Pattern", format_type: "", is_parent: true, parent_id: "shareholding_pattern_group" },
            HierarchyRowConfig { tag_name: "promoter_pct", display_name: "Promoter Holding Block", format_type: "percentage_raw", is_parent: false, parent_id: "shareholding_pattern_group" },
            HierarchyRowConfig { tag_name: "fii_pct", display_name: "Foreign Institutional Holding (FII)", format_type: "percentage_raw", is_parent: false, parent_id: "shareholding_pattern_group" },
            HierarchyRowConfig { tag_name: "dii_pct", display_name: "Domestic Institutional Holding (DII)", format_type: "percentage_raw", is_parent: false, parent_id: "shareholding_pattern_group" },
            HierarchyRowConfig { tag_name: "government_pct", display_name: "Sovereign / Government Holding", format_type: "percentage_raw", is_parent: false, parent_id: "shareholding_pattern_group" },
            HierarchyRowConfig { tag_name: "public_retail_pct", display_name: "Public Retail holding Float", format_type: "percentage_raw", is_parent: false, parent_id: "shareholding_pattern_group" },

            // ─── STRESS TESTS & LIQUIDATING DISMANTLES
            HierarchyRowConfig { tag_name: "asset_shocks_group", display_name: "Asset Stress Tests & Dissolution Safety Margin", format_type: "", is_parent: true, parent_id: "asset_shocks_group" },
            HierarchyRowConfig { tag_name: "margin_of_safety_pct", display_name: "Intrinsic Valuation Margin of Safety", format_type: "percentage_raw", is_parent: false, parent_id: "asset_shocks_group" },
            HierarchyRowConfig { tag_name: "net_liquidating_dissolution_cash", display_name: "Net Liquidating Dissolution Cash Floor", format_type: "currency_cr", is_parent: false, parent_id: "asset_shocks_group" },
            HierarchyRowConfig { tag_name: "simulated_assets_post_10_percent_slump", display_name: "Simulated Assets (Post -10% Asset Slump)", format_type: "currency_cr", is_parent: false, parent_id: "asset_shocks_group" },
            HierarchyRowConfig { tag_name: "simulated_assets_post_20_percent_slump", display_name: "Simulated Assets (Post -20% Asset Slump)", format_type: "currency_cr", is_parent: false, parent_id: "asset_shocks_group" },
            HierarchyRowConfig { tag_name: "simulated_assets_post_30_percent_slump", display_name: "Simulated Assets (Post -30% Asset Slump)", format_type: "currency_cr", is_parent: false, parent_id: "asset_shocks_group" },
            HierarchyRowConfig { tag_name: "simulated_assets_post_40_percent_slump", display_name: "Simulated Assets (Post -40% Asset Slump)", format_type: "currency_cr", is_parent: false, parent_id: "asset_shocks_group" },
            HierarchyRowConfig { tag_name: "simulated_assets_post_50_percent_slump", display_name: "Simulated Assets (Post -50% Asset Slump)", format_type: "currency_cr", is_parent: false, parent_id: "asset_shocks_group" },

            // ─── ELASTICITY EARNING SHOCKS
            HierarchyRowConfig { tag_name: "elasticity_shocks_group", display_name: "Operating Elasticity Revenue Sensitivity Shock Models", format_type: "", is_parent: true, parent_id: "elasticity_shocks_group" },
            HierarchyRowConfig { tag_name: "elasticity_shock_up_5", display_name: "EBIT Change Shock (Sales Revenue +5%)", format_type: "percentage_raw", is_parent: false, parent_id: "elasticity_shocks_group" },
            HierarchyRowConfig { tag_name: "elasticity_shock_down_5", display_name: "EBIT Change Shock (Sales Revenue -5%)", format_type: "percentage_raw", is_parent: false, parent_id: "elasticity_shocks_group" },
            HierarchyRowConfig { tag_name: "elasticity_shock_up_10", display_name: "EBIT Change Shock (Sales Revenue +10%)", format_type: "percentage_raw", is_parent: false, parent_id: "elasticity_shocks_group" },
            HierarchyRowConfig { tag_name: "elasticity_shock_down_10", display_name: "EBIT Change Shock (Sales Revenue -10%)", format_type: "percentage_raw", is_parent: false, parent_id: "elasticity_shocks_group" },
            HierarchyRowConfig { tag_name: "elasticity_shock_up_15", display_name: "EBIT Change Shock (Sales Revenue +15%)", format_type: "percentage_raw", is_parent: false, parent_id: "elasticity_shocks_group" },
            HierarchyRowConfig { tag_name: "elasticity_shock_down_15", display_name: "EBIT Change Shock (Sales Revenue -15%)", format_type: "percentage_raw", is_parent: false, parent_id: "elasticity_shocks_group" },
            HierarchyRowConfig { tag_name: "elasticity_shock_up_20", display_name: "EBIT Change Shock (Sales Revenue +20%)", format_type: "percentage_raw", is_parent: false, parent_id: "elasticity_shocks_group" },
            HierarchyRowConfig { tag_name: "elasticity_shock_down_20", display_name: "EBIT Change Shock (Sales Revenue -20%)", format_type: "percentage_raw", is_parent: false, parent_id: "elasticity_shocks_group" },
        ];

        // 📊 STEP 4: GRID GENERATOR COMPILER LOOP
        let mut compiled_rows = Vec::new();
        let mut table_headers = vec!["Financial Operational & Valuation Multiple Metrics".to_string()];
        
        for date in &chronological_headers {
            table_headers.push(date.clone());
        }

        for config in &structured_multiples_tree {
            let mut row_cells = Vec::new();

            if config.is_parent {
                row_cells.push(json!({ "type": "text", "value": config.display_name, "className": "font-bold text-neutral-100" }));
                for _ in &chronological_headers {
                    row_cells.push(json!({ "type": "text", "value": "" }));
                }
            } else {
                row_cells.push(json!({ "type": "text", "value": config.display_name, "className": "text-neutral-300 font-medium" }));

                for date in &chronological_headers {
                    let lookup_key = format!("{}__{}", date, config.tag_name);
                    let raw_val = data_lookup_matrix.get(&lookup_key).unwrap_or(&Value::Null);
                    
                    // 🎯 DUAL-LAYER SCANNABILITY DISPATCH
                    let (formatted_text, tailwind_class, inline_hex) = format_metric_with_visuals(config.tag_name, raw_val, config.format_type);

                    row_cells.push(json!({ 
                        "type": "text", 
                        "value": formatted_text,
                        "className": tailwind_class,
                        "style": { "color": inline_hex } // Enforces correct color even if standard classes are overridden
                    }));
                }
            }

            let has_parent_group = !config.parent_id.is_empty();
            let is_child_row = has_parent_group && !config.is_parent;

            compiled_rows.push(json!({
                "type": "table_row",
                "is_parent": config.is_parent,
                "is_child": is_child_row,
                "parent_id": if has_parent_group { Some(config.parent_id.to_string()) } else { None },
                "align_right_values": true,
                "cells": row_cells
            }));
        }

        // 📊 STEP 5: OUTPUT ACCORDION-SUPPORTED UI INTERFACE MATRIX
        Ok(json!({
            "type": "card",
            "title": "Corporate Multiples & Valuation Matrix",
            "subtitle": "// CONDITIONAL FINANCIAL AUDITING SHADING // AT-A-GLANCE STRESS-TEST SCANNABILITY //",
            "footer": format!("Total metrics indexed: {} parameters mapped across {} historical snapshot intervals", structured_multiples_tree.len() - 9, chronological_headers.len()),
            "children": [
                {
                    "type": "container",
                    "className": "w-full overflow-x-auto overflow-y-visible mt-2",
                    "children": [
                        {
                            "type": "table",
                            "className": "w-full min-w-[850px] text-left border-collapse",
                            "headers": table_headers,
                            "align_right_columns": true,
                            "children": compiled_rows
                        }
                    ]
                }
            ]
        }))
    }
}