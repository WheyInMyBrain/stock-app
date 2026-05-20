// src/multiples/engine.rs

use polars::prelude::*;
use std::path::Path;
use std::fs::File;
use std::collections::HashMap;
use serde_json::Value;

use crate::multiples::CorporateMultiplesReport;

/// Ingests historical market pricing arrays and financial statements to extract all valuation multiples
pub fn execute_multiples_analytical_pipeline(ticker: &str) -> PolarsResult<Vec<CorporateMultiplesReport>> {
    let nse_path = format!("../data/{}/parquets/nse_corporates-financial-results.parquet", ticker);
    let chart_path = format!("../data/{}/nse_historical-chart-data/10Y.json", ticker);
    let shp_path = format!("../data/{}/parquets/nse_corporate-shareholding-master.parquet", ticker);

    if !Path::new(&nse_path).exists() || !Path::new(&chart_path).exists() {
        return Err(PolarsError::ComputeError(
            format!("Missing financial parquet tables or JSON market chart files for ticker: {}.", ticker).into()
        ));
    }

    // ==============================================================================
    // 📊 STEP 1: PARSE AND MAP THE HISTORICAL CHART PRICE POINTS (JSON EXTRACTOR)
    // ==============================================================================
    let file = File::open(&chart_path).map_err(|e| PolarsError::ComputeError(e.to_string().into()))?;
    let chart_json: Value = serde_json::from_reader(file).map_err(|e| PolarsError::ComputeError(e.to_string().into()))?;
    
    let mut price_timeline: HashMap<String, f64> = HashMap::new();
    
    if let Some(graph_data) = chart_json.get("grapthData").and_then(|v| v.as_array()) {
        for entry in graph_data {
            if let (Some(ms_val), Some(price_val)) = (entry.get(0).and_then(|v| v.as_i64()), entry.get(1).and_then(|v| v.as_f64())) {
                let seconds = ms_val / 1000;
                let day_raw = seconds / 86400;
                
                let r_year = 1970 + (day_raw / 365); 
                let r_month = ((day_raw % 365) / 30) + 1;
                let r_day = (day_raw % 30) + 1;
                let clean_date_key = format!("{:04}-{:02}-{:02}", r_year, r_month, r_day);
                
                price_timeline.insert(clean_date_key, price_val);
            }
        }
    }

    let find_aligned_price = |target_date: &str| -> f64 {
        if let Some(&price) = price_timeline.get(target_date) { return price; }
        
        if target_date.len() == 10 {
            let base_day_str = &target_date[8..10];
            if let Ok(base_day) = base_day_str.parse::<i32>() {
                for offset in [-2, -1, 1, 2, -3, 3] {
                    let adjusted_day = base_day + offset;
                    if adjusted_day > 0 && adjusted_day <= 31 {
                        let check_str = format!("{}{:02}", &target_date[0..8], adjusted_day);
                        if let Some(&price) = price_timeline.get(&check_str) { return price; }
                    }
                }
            }
        }
        
        if !price_timeline.is_empty() {
            let sum: f64 = price_timeline.values().sum();
            return sum / price_timeline.len() as f64;
        }
        0.0
    };

    // ==============================================================================
    // 📊 STEP 2: PARSE HISTORICAL SHARES OUTSTANDING (DYNAMIC MATRIX)
    // ==============================================================================
    let mut share_history_timeline: HashMap<String, f64> = HashMap::new();

    if Path::new(&shp_path).exists() {
        let df_shp = LazyFrame::scan_parquet(&shp_path, Default::default())?.collect()?;
        let shp_tag = df_shp.column("tag_name")?.str()?;
        let shp_ctx = df_shp.column("context_id")?.str()?;
        let shp_bounds = df_shp.column("date_bounds")?.str()?;
        let shp_val = df_shp.column("raw_value")?.str()?;

        for idx in 0..df_shp.shape().0 {
            if shp_tag.get(idx) == Some("NumberOfShares") {
                let context = shp_ctx.get(idx).unwrap_or("");
                if context == "ShareholdingPatternI" || context == "ShareholdingPattern_ContextI" {
                    let date_key = shp_bounds.get(idx).unwrap_or("").to_string();
                    let raw_str = shp_val.get(idx).unwrap_or("0");
                    let parsed_shares: f64 = raw_str.replace(",", "").replace(" ", "").trim().parse().unwrap_or(0.0);
                    
                    if parsed_shares > 0.0 {
                        share_history_timeline.insert(date_key, parsed_shares);
                    }
                }
            }
        }
    }

    let find_historical_shares = |target_date: &str| -> f64 {
        if let Some(&shares) = share_history_timeline.get(target_date) {
            return shares;
        }
        
        let target_year = target_date.split('-').next().unwrap_or("2024");
        let mut sorted_dates: Vec<&String> = share_history_timeline.keys().collect();
        sorted_dates.sort();
        
        for date_key in sorted_dates.iter().rev() {
            if date_key.starts_with(target_year) {
                return *share_history_timeline.get(*date_key).unwrap_or(&53_954_106.0);
            }
        }
        
        *share_history_timeline.values().next().unwrap_or(&53_954_106.0)
    };

    // ==============================================================================
    // 📊 STEP 3: PARQUET INGESTION & IN-MEMORY PIVOT MATCHING
    // ==============================================================================
    let df_raw = LazyFrame::scan_parquet(&nse_path, Default::default())?.collect()?;

    let target_tags = [
        "RevenueFromOperations", "ProfitBeforeTax", "FinanceCosts", "TaxExpense",
        "DepreciationDepletionAndAmortisationExpense", "CashFlowsFromUsedInOperatingActivities",
        "Assets", "CurrentAssets", "NoncurrentAssets", "Liabilities", 
        "CurrentLiabilities", "NoncurrentLiabilities", "Equity", "PropertyPlantAndEquipment",
        "TradeReceivablesCurrent", "Inventories", "PurchaseOfPropertyPlantAndEquipmentClassifiedAsInvestingActivities"
    ];

    let tag_col = df_raw.column("tag_name")?.str()?;
    let mask: BooleanChunked = tag_col.into_iter().map(|opt| opt.map_or(false, |v| target_tags.contains(&v))).collect();
    let df_filtered = df_raw.filter(&mask)?;

    let file_col = df_filtered.column("source_file")?.str()?;
    let tag_name_col = df_filtered.column("tag_name")?.str()?;
    let raw_value_col = df_filtered.column("raw_value")?.str()?;

    let mut document_matrix: HashMap<String, HashMap<String, f64>> = HashMap::new();

    for idx in 0..df_filtered.shape().0 {
        let file = file_col.get(idx).unwrap_or("").to_string();
        let tag = tag_name_col.get(idx).unwrap_or("").to_string();
        let raw_val = raw_value_col.get(idx).unwrap_or("");

        if file.contains("Consolidated") {
            let cleaned_val: f64 = raw_val.replace(",", "").replace(" ", "").trim().parse().unwrap_or(0.0);
            document_matrix.entry(file).or_insert_with(HashMap::new).insert(tag, cleaned_val);
        }
    }

    let mut chron_files: Vec<String> = document_matrix.keys().cloned().collect();
    chron_files.sort();

    let mut multiples_timeline = Vec::new();

    // ==============================================================================
    // 📊 STEP 4: EXECUTE HIGH-SPEED RATIO MULTIPLIERS GENERATION
    // ==============================================================================
    for (idx, file_key) in chron_files.iter().enumerate() {
        if let Some(metrics) = document_matrix.get(file_key) {
            
            let clean_file_prefix = file_key.split('_').next().unwrap_or("31-Mar-2024");
            let components: Vec<&str> = clean_file_prefix.split('-').collect();

            let parsed_snapshot_date = if components.len() >= 3 {
                let day = components[0];
                let month_str = components[1].to_lowercase();
                let year = components[2];
                
                let month_num = match month_str.as_str() {
                    "jan" => "01", "feb" => "02", "mar" => "03", "apr" => "04",
                    "may" => "05", "jun" => "06", "jul" => "07", "aug" => "08",
                    "sep" => "09", "oct" => "10", "nov" => "11", "dec" => "12",
                    _ => "03",
                };
                format!("{}-{}-{}", year, month_num, day)
            } else {
                "2024-03-31".to_string()
            };

            let rev = *metrics.get("RevenueFromOperations").unwrap_or(&0.0);
            let pbt = *metrics.get("ProfitBeforeTax").unwrap_or(&0.0);
            let interest = *metrics.get("FinanceCosts").unwrap_or(&0.0);
            let tax = *metrics.get("TaxExpense").unwrap_or(&0.0);
            let depr = *metrics.get("DepreciationDepletionAndAmortisationExpense").unwrap_or(&0.0);
            let cfo = *metrics.get("CashFlowsFromUsedInOperatingActivities").unwrap_or(&0.0);
            let capex = metrics.get("PurchaseOfPropertyPlantAndEquipmentClassifiedAsInvestingActivities").cloned().unwrap_or(0.0).abs();

            if rev <= 0.0 { continue; } 

            let ebit = pbt + interest;
            let ebitda = ebit + depr;
            let net_profit = pbt - tax;
            let total_expenses = rev - net_profit;

            let ebit_margin = ebit / rev;
            let net_margin = net_profit / rev;
            let fcf_margin = (cfo - capex) / rev;
            let interest_coverage = if interest > 0.0 { ebit / interest } else { 0.0 };
            let accruals_to_sales_intensity = (net_profit - cfo) / rev;

            let estimated_variable_costs = (total_expenses - depr - interest) * 0.65;
            let estimated_fixed_costs = (total_expenses - depr - interest) * 0.35 + depr;
            
            let degree_of_operating_leverage = if ebit > 0.0 { (rev - estimated_variable_costs) / ebit } else { 0.0 };
            let contribution_margin_ratio = if rev > 0.0 { (rev - estimated_variable_costs) / rev } else { 0.1 };
            let breakeven_operating_revenue = if contribution_margin_ratio > 0.0 { estimated_fixed_costs / contribution_margin_ratio } else { 0.0 };

            let capex_to_depreciation_coverage = if depr > 0.0 { capex / depr } else { 0.0 };
            let ppe = *metrics.get("PropertyPlantAndEquipment").unwrap_or(&0.0);
            let estimated_infrastructure_nbv_age_years = if depr > 0.0 { ppe / depr } else { 0.0 };

            let ca = *metrics.get("CurrentAssets").unwrap_or(&0.0);
            let assets = *metrics.get("Assets").unwrap_or(&0.0);
            let has_balance_sheet = ca > 0.0 || assets > 0.0;

            let stock_price = find_aligned_price(&parsed_snapshot_date);
            let active_shares_outstanding = find_historical_shares(&parsed_snapshot_date);

            let (mut roic, mut roe, mut roa, mut debt_to_equity, mut current_ratio, mut quick_ratio) = (None, None, None, None, None, None);
            let (mut inventory_turnover, mut cash_conversion_cycle_days, mut enterprise_value, mut ev_to_ebitda) = (None, None, None, None);
            let (mut piotroski_f_score, mut beneish_m_score, mut altman_z_score) = (None, None, None);
            
            let (mut defensive_cash_burn_months, mut net_liquidating_dissolution_cash) = (None, None);
            let (mut simulated_assets_post_10_percent_slump, mut simulated_assets_post_20_percent_slump, mut simulated_assets_post_30_percent_slump) = (None, None, None);
            let (mut simulated_assets_post_40_percent_slump, mut simulated_assets_post_50_percent_slump) = (None, None);

            if has_balance_sheet {
                let total_assets = if assets > 0.0 { assets } else { ca + metrics.get("NoncurrentAssets").unwrap_or(&0.0) };
                let total_liabilities = if *metrics.get("Liabilities").unwrap_or(&0.0) > 0.0 { *metrics.get("Liabilities").unwrap_or(&0.0) } else { metrics.get("CurrentLiabilities").unwrap_or(&0.0) + metrics.get("NoncurrentLiabilities").unwrap_or(&0.0) };
                let mut net_equity = if *metrics.get("Equity").unwrap_or(&0.0) > 0.0 { *metrics.get("Equity").unwrap_or(&0.0) } else { total_assets - total_liabilities };
                if net_equity <= 0.0 { net_equity = 1.0; }

                let cl = *metrics.get("CurrentLiabilities").unwrap_or(&1.0);
                let cl_guard = if cl <= 0.0 { 1.0 } else { cl };
                let inventories = *metrics.get("Inventories").unwrap_or(&0.0);
                let receivables = *metrics.get("TradeReceivablesCurrent").unwrap_or(&0.0);
                let working_capital = ca - cl_guard;

                current_ratio = Some(ca / cl_guard);
                quick_ratio = Some((ca - inventories) / cl_guard);
                debt_to_equity = Some(total_liabilities / net_equity);
                roa = Some(net_profit / total_assets);
                roe = Some(net_profit / net_equity);
                
                let effective_tax_rate = if ebit > 0.0 { tax / ebit } else { 0.25 };
                let tax_proxy = effective_tax_rate.max(0.0).min(0.45);
                roic = Some((ebit * (1.0 - tax_proxy)) / (ppe + working_capital).max(1.0));
                
                inventory_turnover = Some(rev / inventories.max(1.0));
                cash_conversion_cycle_days = Some(((inventories / rev) * 365.0) + ((receivables / rev) * 365.0));

                let asset_turnover_calc = if total_assets > 0.0 { rev / total_assets } else { 0.0 };

                if stock_price > 0.0 {
                    let market_cap = stock_price * active_shares_outstanding;
                    let ev = market_cap + total_liabilities - ca;
                    enterprise_value = Some(ev);
                    ev_to_ebitda = Some(ev / ebitda.max(1.0));
                }

                let monthly_cash_burn_rate = ((total_expenses - depr) / 12.0).max(1.0);
                let liquid_assets = ca - inventories;
                defensive_cash_burn_months = Some(liquid_assets / monthly_cash_burn_rate);
                
                net_liquidating_dissolution_cash = Some(ca - total_liabilities);
                
                simulated_assets_post_10_percent_slump = Some(total_assets - (inventories * 0.10));
                simulated_assets_post_20_percent_slump = Some(total_assets - (inventories * 0.20));
                simulated_assets_post_30_percent_slump = Some(total_assets - (inventories * 0.30));
                simulated_assets_post_40_percent_slump = Some(total_assets - (inventories * 0.40));
                simulated_assets_post_50_percent_slump = Some(total_assets - (inventories * 0.50));

                if idx > 0 {
                    if let Some(prev) = document_matrix.get(&chron_files[idx - 1]) {
                        let p_ca = *prev.get("CurrentAssets").unwrap_or(&0.0);
                        if p_ca > 0.0 {
                            let p_rev = *prev.get("RevenueFromOperations").unwrap_or(&0.0);
                            let p_receivables = *prev.get("TradeReceivablesCurrent").unwrap_or(&0.0);
                            let p_ppe = *prev.get("PropertyPlantAndEquipment").unwrap_or(&0.0);
                            let p_total_assets = if *prev.get("Assets").unwrap_or(&0.0) > 0.0 { *prev.get("Assets").unwrap_or(&0.0) } else { p_ca + prev.get("NoncurrentAssets").unwrap_or(&0.0) };
                            
                            let p_total_liabilities = if *prev.get("Liabilities").unwrap_or(&0.0) > 0.0 { *prev.get("Liabilities").unwrap_or(&0.0) } else { prev.get("CurrentLiabilities").cloned().unwrap_or(0.0) + prev.get("NoncurrentLiabilities").cloned().unwrap_or(0.0) };

                            let dsri = (receivables / rev) / (p_receivables / p_rev).max(0.0001);
                            let g_margin_curr = (rev - ebit) / rev;
                            let g_margin_prev = (p_rev - (prev.get("ProfitBeforeTax").unwrap_or(&0.0) + prev.get("FinanceCosts").unwrap_or(&0.0))) / p_rev;
                            
                            let gmi = g_margin_prev / g_margin_curr.max(0.0001);
                            let aqi = (1.0 - ((ca + ppe) / total_assets)) / (1.0 - ((p_ca + p_ppe) / p_total_assets)).max(0.0001);
                            let sgi = rev / p_rev.max(0.0001);
                            let lvgi = (total_liabilities / total_assets) / (p_total_liabilities / p_total_assets).max(0.0001);

                            beneish_m_score = Some(-4.84 + (0.92 * dsri) + (0.528 * aqi) + (0.472 * sgi) + (0.404 * lvgi) + (0.1 * gmi));
                        }
                    }
                }

                let mut f_points = 0;
                if net_profit > 0.0 { f_points += 1; }
                if cfo > 0.0 { f_points += 1; }
                if cfo > net_profit { f_points += 1; }
                if idx > 0 {
                    if let Some(prev) = document_matrix.get(&chron_files[idx - 1]) {
                        let p_ca = *prev.get("CurrentAssets").unwrap_or(&0.0);
                        if p_ca > 0.0 {
                            let p_total_assets = if *prev.get("Assets").unwrap_or(&0.0) > 0.0 { *prev.get("Assets").unwrap_or(&0.0) } else { p_ca + prev.get("NoncurrentAssets").unwrap_or(&0.0) };
                            let p_roa = (*prev.get("ProfitBeforeTax").unwrap_or(&0.0) - *prev.get("TaxExpense").unwrap_or(&0.0)) / p_total_assets.max(1.0);
                            let p_lever = (*prev.get("Liabilities").unwrap_or(&0.0)) / p_total_assets.max(1.0);
                            
                            let p_current = p_ca / prev.get("CurrentLiabilities").cloned().unwrap_or(1.0).max(1.0);
                            let p_turnover = *prev.get("RevenueFromOperations").unwrap_or(&0.0) / p_total_assets.max(1.0);

                            if roa.unwrap_or(0.0) > p_roa { f_points += 1; }
                            if (total_liabilities / total_assets) < p_lever { f_points += 1; }
                            if current_ratio.unwrap_or(0.0) > p_current { f_points += 1; }
                            if asset_turnover_calc > p_turnover { f_points += 1; }
                        }
                    }
                }
                piotroski_f_score = Some(f_points);

                altman_z_score = Some((1.2 * (working_capital / total_assets)) + (1.4 * ((net_equity * 0.65) / total_assets)) + (3.3 * (ebit / total_assets)) + (0.6 * (net_equity / total_liabilities.max(1.0))) + (0.999 * (rev / total_assets)));
            }

            multiples_timeline.push(CorporateMultiplesReport {
                source_file: file_key.clone(),
                snapshot_date: parsed_snapshot_date,
                revenue: rev, ebit_margin, net_margin, fcf_margin, interest_coverage, accruals_to_sales_intensity,
                degree_of_operating_leverage, breakeven_operating_revenue, capex_to_depreciation_coverage, estimated_infrastructure_nbv_age_years,
                stock_price,
                total_shares: active_shares_outstanding, 
                roic, roe, roa, debt_to_equity, current_ratio, quick_ratio, inventory_turnover,
                cash_conversion_cycle_days, enterprise_value, ev_to_ebitda, piotroski_f_score, beneish_m_score, altman_z_score,
                defensive_cash_burn_months, net_liquidating_dissolution_cash,
                simulated_assets_post_10_percent_slump, simulated_assets_post_20_percent_slump,
                simulated_assets_post_30_percent_slump, simulated_assets_post_40_percent_slump,
                simulated_assets_post_50_percent_slump,
            });
        }
    }

    Ok(multiples_timeline)
}