use crate::commands::memory_pool::store_parsed_table;
use polars::prelude::*;
use std::collections::{HashMap, BTreeSet, BTreeMap, HashSet};
use serde::{Serialize, Deserialize};

#[derive(Default, Debug, Clone)]
pub struct AnalysisMetadataRow {
    pub year: i32,
    pub dividend_paid: i64,
    pub basic_eps: f64,
    pub net_profit_after_tax: i64,
    pub total_equity: i64,
    pub total_debt: i64,
    pub operating_cash_flow: i64,
    pub capex_outflow: i64,
    pub capex_inflow: i64,
    pub net_capex: i64,
    pub free_cash_flow: i64,
    pub outstanding_shares: i64,
    pub profit_before_tax: i64,
    pub finance_interest_expense: i64,
    pub effective_tax_rate: f64,
    pub nse_beta: f64,
    pub bse_beta: f64,
    pub dynamic_rf: f64,
    pub average_beta: f64,
    pub dynamic_rm: f64,
    pub sustainable_g: f64,
    pub terminal_gn: f64,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct ValuationResultRow {
    pub year: i32,
    pub calculated_tax_rate: f64,
    pub calculated_kd: f64,
    pub calculated_ke: f64,
    pub calculated_wacc: f64,
    pub intrinsic_value: f64,
    pub status_ok: bool,
    pub error_msg: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonteCarloResultSummary {
    pub ticker: String,
    pub expected_value: f64,    
    pub upper_bound: f64,       
    pub lower_bound: f64,       
    pub forecast_horizon: u32,  
    pub total_simulations: u32, 
    pub status_ok: bool,
    pub error_msg: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonteCarloPathPoint {
    pub path_index: u32,       
    pub step_date: String,              
    pub simulated_price: f64,   
}

// =========================================================================
// 1. UTILITY PARSING HELPERS
// =========================================================================

fn extract_year_from_filename(file_name: &str) -> Option<i32> {
    let parts: Vec<&str> = file_name.split('-').collect();
    if let Some(last) = parts.last() {
        let clean_yr: String = last.chars().filter(|c| c.is_ascii_digit()).collect();
        if clean_yr.len() >= 4 {
            return clean_yr[0..4].parse::<i32>().ok();
        }
    }
    let clean_yr: String = file_name.chars().filter(|c| c.is_ascii_digit()).collect();
    if clean_yr.len() >= 4 {
        clean_yr[clean_yr.len() - 4..].parse::<i32>().ok()
    } else {
        None
    }
}

// =========================================================================
// 2. OCR TRACK A: INCOME STATEMENT (EPS & RANKED PROFIT)
// =========================================================================

#[derive(Default, Debug, Clone)]
struct IsFileRules {
    file_year: i32,
    b_c: f64, b_p: f64, 
    h_c: f64, h_p: f64,
    prof_rank: i32,
    prof_c: f64, prof_p: f64,
    pbt_c: f64, pbt_p: f64,
    pbt_found: bool,
}

fn process_ocr_income_statement(bytes: Vec<u8>) -> Result<(
    HashMap<i32, f64>, // eps_ledger
    HashMap<i32, f64>, // prof_ledger (After Tax)
    HashMap<i32, f64>, // pbt_ledger (Before Tax)
    HashMap<i32, f64>, // tax_ledger (Computed Tax Expense)
    HashMap<i32, f64>, // tax_rate_ledger (Effective Tax Rate)
    BTreeSet<i32>      // years timeline
), PolarsError> {
    let df = ParquetReader::new(std::io::Cursor::new(bytes)).finish()?;
    let mut file_map: HashMap<String, IsFileRules> = HashMap::new();
    let mut years = BTreeSet::new();

    let p_ca = df.column("particulars")?.str()?;
    let ctx_ca = df.column("context_id")?.str()?;
    let f_ca = df.column("file_name")?.str()?;
    let curr_ca = df.column("curr_year")?.str()?;
    let prev_ca = df.column("prev_year")?.str()?;

    for i in 0..df.height() {
        if ctx_ca.get(i).unwrap_or("").to_lowercase() != "consolidated" { continue; }
        let file_name = match f_ca.get(i) { Some(f) => f.to_string(), None => continue };
        let part = p_ca.get(i).unwrap_or("").to_lowercase();
        let c_num = curr_ca.get(i).unwrap_or("0").trim().parse::<f64>().unwrap_or(0.0);
        let p_num = prev_ca.get(i).unwrap_or("0").trim().parse::<f64>().unwrap_or(0.0);

        let rule = file_map.entry(file_name.clone()).or_insert_with(|| IsFileRules {
            file_year: extract_year_from_filename(&file_name).unwrap_or(0),
            prof_rank: 99,
            ..Default::default()
        });

        // -----------------------------------------------------------------
        // TRACK A: SHARE EARNINGS EXTRACTION (EPS)
        // -----------------------------------------------------------------
        let is_basic = part.trim().starts_with("basic") || part.contains("basic and diluted");
        let is_eps = part.contains("earnings per") && part.contains("share");
        if is_basic { rule.b_c = c_num; rule.b_p = p_num; }
        if is_eps { rule.h_c = c_num; rule.h_p = p_num; }

        // -----------------------------------------------------------------
        // TRACK B: PROFIT BEFORE TAX (PBT) EXTRACTION
        // -----------------------------------------------------------------
        let part_lower = part.to_lowercase();
        let has_profit_before = part_lower.contains("profit") && part_lower.contains("before");
        let is_pbt = has_profit_before && (part_lower.contains("tax") || part_lower.contains("exceptional"));
        let is_operating = part_lower.contains("operating");
        if is_pbt && !is_operating && !rule.pbt_found && (c_num != 0.0 || p_num != 0.0) {
            rule.pbt_c = c_num;
            rule.pbt_p = p_num;
            rule.pbt_found = true;
        }

        // -----------------------------------------------------------------
        // TRACK C: NET PROFIT AFTER TAX (PAT) HIERARCHICAL RANKING
        // -----------------------------------------------------------------
        let is_pat_excluded = part.contains("before tax") || part.contains("operating") || 
                              part.contains("before exceptional") || part.contains("comprising") || 
                              part.contains("comprehensive");
                              
        if !is_pat_excluded && (c_num != 0.0 || p_num != 0.0) {
            let is_r1 = part.contains("owner") || part.contains("parent") || part.contains("holding");
            let is_r2 = (part.contains("profit") && part.contains("after") && part.contains("tax")) ||
                        (part.contains("profit") && part.contains("for") && part.contains("the") && part.contains("year")) ||
                        (part.contains("profit") && part.contains("after") && part.contains("taxation"));
            
            let rank = if is_r1 { 1 } else if is_r2 { 2 } else { 99 };
            if rank < rule.prof_rank {
                rule.prof_rank = rank;
                rule.prof_c = c_num;
                rule.prof_p = p_num;
            }
        }
    }

    let mut eps_ledger = HashMap::new();
    let mut prof_ledger = HashMap::new();
    let mut pbt_ledger = HashMap::new();
    let mut tax_ledger = HashMap::new();
    let mut tax_rate_ledger = HashMap::new();

    let mut sorted_rules: Vec<_> = file_map.into_values().filter(|r| r.file_year != 0).collect();
    sorted_rules.sort_by_key(|r| r.file_year);

    // Unroll data for Current Years
    for r in &sorted_rules {
        let eps_curr = if r.b_c == 0.0 && r.b_p == 0.0 { r.h_c } else { r.b_c };
        if eps_curr != 0.0 { eps_ledger.insert(r.file_year, eps_curr); years.insert(r.file_year); }
        if r.prof_c != 0.0 { prof_ledger.insert(r.file_year, r.prof_c); years.insert(r.file_year); }
        
        if r.pbt_c != 0.0 {
            pbt_ledger.insert(r.file_year, r.pbt_c);
            years.insert(r.file_year);

            // Mathematically derive tax values on the fly
            let computed_tax = r.pbt_c - r.prof_c;
            tax_ledger.insert(r.file_year, computed_tax);

            let rate = if r.pbt_c > 0.0 { computed_tax / r.pbt_c } else { 0.0 };
            tax_rate_ledger.insert(r.file_year, rate);
        }
    }
    
    // Unroll data for Previous Years (Chronological fallback offsets)
    for r in &sorted_rules {
        let prev_yr = r.file_year - 1;
        let eps_prev = if r.b_c == 0.0 && r.b_p == 0.0 { r.h_p } else { r.b_p };
        if eps_prev != 0.0 { eps_ledger.insert(prev_yr, eps_prev); years.insert(prev_yr); }
        if r.prof_p != 0.0 { prof_ledger.insert(prev_yr, r.prof_p); years.insert(prev_yr); }
        
        if r.pbt_p != 0.0 {
            pbt_ledger.insert(prev_yr, r.pbt_p);
            years.insert(prev_yr);

            let computed_tax = r.pbt_p - r.prof_p;
            tax_ledger.insert(prev_yr, computed_tax);

            let rate = if r.pbt_p > 0.0 { computed_tax / r.pbt_p } else { 0.0 };
            tax_rate_ledger.insert(prev_yr, rate);
        }
    }

    Ok((eps_ledger, prof_ledger, pbt_ledger, tax_ledger, tax_rate_ledger, years))
}

// =========================================================================
// 3. OCR TRACK B: CASH FLOW (DIVIDENDS, OPERATING CASH FLOW & CAPEX EXTRACTION)
// =========================================================================

#[derive(Default, Debug, Clone)]
struct OcrCfYearSummary {
    curr_outflow: f64, curr_inflow: f64,
    prev_outflow: f64, prev_inflow: f64,
}

fn process_ocr_cash_flow(
    bytes: Vec<u8>
) -> Result<(HashMap<i32, f64>, HashMap<i32, f64>, HashMap<i32, f64>, HashMap<i32, f64>, BTreeSet<i32>), PolarsError> {
    let df = ParquetReader::new(std::io::Cursor::new(bytes)).finish()?;
    let mut div_ledger = HashMap::new();
    let mut ocf_ops_parsed = HashMap::new();
    let mut capex_summary_map: HashMap<i32, OcrCfYearSummary> = HashMap::new();
    let mut years = BTreeSet::new();

    let p_ca = df.column("particulars")?.str()?;
    let ctx_ca = df.column("context_id")?.str()?;
    let f_ca = df.column("file_name")?.str()?;
    let curr_ca = df.column("curr_year")?.str()?;
    let prev_ca = df.column("prev_year")?.str()?;

    let mut div_file_map = HashMap::new();
    let mut ocf_file_map = HashMap::new();
    let mut boundaries_map: HashMap<i32, (i64, i64)> = HashMap::new();

    // Pass 1: Identify bounds indices and parse structured rows per file year group
    for i in 0..df.height() {
        if ctx_ca.get(i).unwrap_or("").to_lowercase() != "consolidated" { continue; }
        let file_name = match f_ca.get(i) { Some(f) => f.to_string(), None => continue };
        let yr = match extract_year_from_filename(&file_name) { Some(y) => y, None => continue };
        
        let part = p_ca.get(i).unwrap_or("").to_lowercase();
        let idx = i as i64;

        let bounds = boundaries_map.entry(yr).or_insert((i64::MAX, i64::MAX));
        if part.contains("cash flow") && part.contains("investing") && bounds.0 == i64::MAX { bounds.0 = idx; }
        if part.contains("cash flow") && part.contains("financ") && bounds.1 == i64::MAX { bounds.1 = idx; }

        let c_raw = curr_ca.get(i).unwrap_or("0").trim();
        let p_raw = prev_ca.get(i).unwrap_or("0").trim();
        let c_num = c_raw.parse::<f64>().unwrap_or(0.0);
        let p_num = p_raw.parse::<f64>().unwrap_or(0.0);

        if part.contains("dividend") && part.contains("paid") {
            div_file_map.insert(file_name.clone(), (yr, c_num.abs(), p_num.abs()));
        }

        // Parse Operating Cash Flow
        if (part.contains("net") && part.contains("cash") && part.contains("operating")) ||
        (part.contains("cash") && part.contains("generated") && part.contains("operations")) ||
        (part.contains("net") && part.contains("cash") && part.contains("flow") && part.contains("operating")) {
            ocf_file_map.insert(file_name.clone(), (yr, c_num, p_num));
        }
    }

    // Unroll Dividend Paid Mappings
    for (_, (yr, c_val, p_val)) in div_file_map {
        if c_val > 0.0 { div_ledger.insert(yr, c_val); years.insert(yr); }
        if p_val > 0.0 { div_ledger.insert(yr - 1, p_val); years.insert(yr - 1); }
    }

    // Unroll Operating Cash Flow Mappings
    for (_, (yr, c_val, p_val)) in ocf_file_map {
        if c_val != 0.0 { ocf_ops_parsed.insert(yr, c_val); years.insert(yr); }
        if p_val != 0.0 { ocf_ops_parsed.insert(yr - 1, p_val); years.insert(yr - 1); }
    }

    // Pass 2: Sandwich Data Extraction for Capex Rules Execution
    for i in 0..df.height() {
        if ctx_ca.get(i).unwrap_or("").to_lowercase() != "consolidated" { continue; }
        let file_name = match f_ca.get(i) { Some(f) => f.to_string(), None => continue };
        let yr = match extract_year_from_filename(&file_name) { Some(y) => y, None => continue };
        let part = p_ca.get(i).unwrap_or("");
        if part.trim().is_empty() { continue; }
        let part_lower = part.to_lowercase();

        let bounds = match boundaries_map.get(&yr) { Some(&b) => b, None => continue };
        let start_idx = bounds.0;
        let end_idx = if bounds.1 == i64::MAX { start_idx + 15 } else { bounds.1 };
        let idx = i as i64;

        if idx >= start_idx && idx < end_idx {
            let exclusion = part_lower.contains("investment") || part_lower.contains("deposit") || 
                            part_lower.contains("bank balance") || part_lower.contains("interest received") || 
                            part_lower.contains("dividend") || part_lower.contains("net cash") || part_lower.contains("total");
            if !exclusion {
                let curr_val = curr_ca.get(i).unwrap_or("0").trim().parse::<f64>().unwrap_or(0.0);
                let prev_val = prev_ca.get(i).unwrap_or("0").trim().parse::<f64>().unwrap_or(0.0);

                let summary = capex_summary_map.entry(yr).or_default();
                if curr_val < 0.0 { summary.curr_outflow += curr_val; } else { summary.curr_inflow += curr_val; }
                if prev_val < 0.0 { summary.prev_outflow += prev_val; } else { summary.prev_inflow += prev_val; }
            }
        }
    }

    // Unroll Capex Timelines
    let mut capex_outflow_ledger = HashMap::new();
    let mut capex_inflow_ledger = HashMap::new();

    for (yr, summary) in capex_summary_map {
        // Current Year Timeline
        capex_outflow_ledger.insert(yr, summary.curr_outflow);
        capex_inflow_ledger.insert(yr, summary.curr_inflow);
        years.insert(yr);

        // Previous Year Timeline
        capex_outflow_ledger.insert(yr - 1, summary.prev_outflow);
        capex_inflow_ledger.insert(yr - 1, summary.prev_inflow);
        years.insert(yr - 1);
    }

    Ok((div_ledger, ocf_ops_parsed, capex_outflow_ledger, capex_inflow_ledger, years))
}

// =========================================================================
// 4. OCR TRACK C: BALANCE SHEET (EQUITY & TOTAL DEBT EXTRACTION)
// =========================================================================

#[derive(Default, Debug, Clone)]
struct BsFileRules {
    file_year: i32,
    t1_c: f64, t1_p: f64,
    cap_c: f64, cap_p: f64,
    res_c: f64, res_p: f64,
    t3_c: f64, t3_p: f64,
    f_t1: bool, f_cap: bool, f_res: bool, f_t3: bool,
    // New total debt dynamic accumulators per file group
    debt_curr_sum: f64,
    debt_prev_sum: f64,
}

fn process_ocr_balance_sheet(bytes: Vec<u8>) -> Result<(HashMap<i32, f64>, HashMap<i32, f64>, BTreeSet<i32>), PolarsError> {
    let df = ParquetReader::new(std::io::Cursor::new(bytes)).finish()?;
    let mut eq_ledger: HashMap<i32, f64> = HashMap::new();
    let mut debt_ledger: HashMap<i32, f64> = HashMap::new();
    let mut years = BTreeSet::new();

    let p_ca = df.column("particulars")?.str()?;
    let ctx_ca = df.column("context_id")?.str()?;
    let f_ca = df.column("file_name")?.str()?;
    let curr_ca = df.column("curr_year")?.str()?;
    let prev_ca = df.column("prev_year")?.str()?;

    let mut file_map: HashMap<String, BsFileRules> = HashMap::new();

    for i in 0..df.height() {
        if ctx_ca.get(i).unwrap_or("").to_lowercase() != "consolidated" { continue; }
        let part = p_ca.get(i).unwrap_or("").to_lowercase();
        let file_name = match f_ca.get(i) { Some(f) => f.to_string(), None => continue };
        
        let c_num = curr_ca.get(i).unwrap_or("0").trim().parse::<f64>().unwrap_or(0.0);
        let p_num = prev_ca.get(i).unwrap_or("0").trim().parse::<f64>().unwrap_or(0.0);

        let rule = file_map.entry(file_name.clone()).or_insert_with(|| BsFileRules {
            file_year: extract_year_from_filename(&file_name).unwrap_or(0),
            ..Default::default()
        });

        let is_borrowing = part.contains("borrowing");
        let is_debt_excluded = part.contains("equity") || part.contains("total liabilities") || 
                              part.contains("cost") || part.contains("finance");

        if is_borrowing && !is_debt_excluded {
            rule.debt_curr_sum += c_num;
            rule.debt_prev_sum += p_num;
        }

        if part.contains("liabilities") || part.contains("minority") { continue; }

        if part.contains("equity") && part.contains("owner") && !rule.f_t1 {
            rule.t1_c = c_num; rule.t1_p = p_num; rule.f_t1 = true;
        }
        if (part.contains("share capital") || part.contains("equity share capital")) && !rule.f_cap {
            rule.cap_c = c_num; rule.cap_p = p_num; rule.f_cap = true;
        }
        if (part.contains("reserves") && part.contains("surplus")) || (part.contains("other") && part.contains("equity")) && !rule.f_res {
            rule.res_c = c_num; rule.res_p = p_num; rule.f_res = true;
        }
        if part.trim() == "total equity" && !rule.f_t3 {
            rule.t3_c = c_num; rule.t3_p = p_num; rule.f_t3 = true;
        }
    }

    for (_, r) in file_map {
        if r.file_year == 0 { continue; }
        
        // Finalize Equity selectors
        let eq_curr = if r.t1_c != 0.0 || r.t1_p != 0.0 { r.t1_c } else if (r.cap_c + r.res_c) != 0.0 || (r.cap_p + r.res_p) != 0.0 { r.cap_c + r.res_c } else { r.t3_c };
        let eq_prev = if r.t1_c != 0.0 || r.t1_p != 0.0 { r.t1_p } else if (r.cap_c + r.res_c) != 0.0 || (r.cap_p + r.res_p) != 0.0 { r.cap_p + r.res_p } else { r.t3_p };

        // Unroll Current Year Timeline Data
        if eq_curr != 0.0 {
            let e = eq_ledger.entry(r.file_year).or_insert(eq_curr);
            if eq_curr < *e { *e = eq_curr; }
            years.insert(r.file_year);
        }
        if r.debt_curr_sum != 0.0 {
            debt_ledger.insert(r.file_year, r.debt_curr_sum);
            years.insert(r.file_year);
        }

        // Unroll Previous Year Timeline Data (Appends file_year - 1 keys)
        if eq_prev != 0.0 {
            let e = eq_ledger.entry(r.file_year - 1).or_insert(eq_prev);
            if eq_prev < *e { *e = eq_prev; }
            years.insert(r.file_year - 1);
        }
        if r.debt_prev_sum != 0.0 {
            // If a previous year sum is already calculated by an earlier chronologically processed file,
            // we override/insert cleanly to let Python-like groupings align.
            debt_ledger.insert(r.file_year - 1, r.debt_prev_sum);
            years.insert(r.file_year - 1);
        }
    }

    Ok((eq_ledger, debt_ledger, years))
}

// =========================================================================
// 5. XBRL EXCHANGE EXTRACTION ENGINE (WITH DCF CASH VECTORS EXTENSION)
// =========================================================================

// =========================================================================
// STRUCT RULES STRUCT EXTENSION
// =========================================================================
#[derive(Default, Debug, Clone)]
struct XmlFileRules {
    year: i32,
    eps_total: f64, eps_cont: f64, eps_disc: f64,
    prof_owner: f64, prof_period: f64, prof_comp: f64,
    equity: f64, div: f64,
    operating_cash_flow: f64,
    capex_outflow: f64,
    capex_inflow: f64,
    total_debt: f64,
    // Add tracking layers for WACC parameters
    pbt: f64,
    finance_costs: f64,
}

fn process_exchange_xbrl(
    bytes_opt: Option<Vec<u8>>,
    div_ledg: &mut HashMap<i32, f64>,
    eps_ledg: &mut HashMap<i32, f64>,
    prof_ledg: &mut HashMap<i32, f64>,
    eq_ledg: &mut HashMap<i32, f64>,
    ocf_ledg: &mut HashMap<i32, f64>,
    out_ledg: &mut HashMap<i32, f64>,
    in_ledg: &mut HashMap<i32, f64>,
    debt_ledg: &mut HashMap<i32, f64>,
    pbt_ledg: &mut HashMap<i32, f64>,
    interest_ledg: &mut HashMap<i32, f64>,
    tax_rate_ledg: &mut HashMap<i32, f64>,
    years: &mut BTreeSet<i32>,
) -> Result<(), PolarsError> {
    let bytes = match bytes_opt {
        Some(b) => b,
        None => return Ok(()),
    };
    let df = ParquetReader::new(std::io::Cursor::new(bytes)).finish()?;
    
    let tag_ca = df.column("tag_name")?.str()?;
    let ctx_ca = df.column("context_id")?.str()?;
    let val_ca = df.column("raw_value")?.str()?;
    let src_ca = df.column("source_file")?.str()?;

    let mut file_map: HashMap<String, XmlFileRules> = HashMap::new();
    let mut consolidated_files: HashSet<String> = HashSet::new();

    for i in 0..df.height() {
        if let Some(tag) = tag_ca.get(i) {
            if tag == "NatureOfReportStandaloneConsolidated" {
                if let (Some(file), Some(val)) = (src_ca.get(i), val_ca.get(i)) {
                    if val.to_lowercase().contains("consolidated") {
                        consolidated_files.insert(file.to_string());
                    }
                }
            }
        }
    }

    for i in 0..df.height() {
        if tag_ca.get(i) == Some("DateOfEndOfReportingPeriod") {
            if let (Some(file), Some(val)) = (src_ca.get(i), val_ca.get(i)) {
                if consolidated_files.contains(file) && val.ends_with("-03-31") && val.len() >= 4 {
                    if let Ok(yr) = val[0..4].parse::<i32>() {
                        file_map.entry(file.to_string()).or_default().year = yr;
                    }
                }
            }
        }
    }

    for i in 0..df.height() {
        let ctx = ctx_ca.get(i).unwrap_or("").to_lowercase();
        if ctx != "oned" && ctx != "fourd" && ctx != "onei" { continue; }
        let file = match src_ca.get(i) { Some(f) => f.to_string(), None => continue };
        if !consolidated_files.contains(&file) || !file_map.contains_key(&file) || file_map[&file].year == 0 { continue; }
        
        let tag = tag_ca.get(i).unwrap_or("");
        
        // Clean comma/bracket string formatting safely on the fly before parsing f64
        let raw_val_str = val_ca.get(i).unwrap_or("0").trim().to_string();
        let cleaned_val_str = raw_val_str.replace(",", "").replace("(", "").replace(")", "");
        let val = cleaned_val_str.parse::<f64>().unwrap_or(0.0);

        let rule = file_map.get_mut(&file).unwrap();
        match tag {
            "BasicEarningsLossPerShareFromContinuingAndDiscontinuedOperations" => rule.eps_total = val,
            "BasicEarningsLossPerShareFromContinuingOperations" => rule.eps_cont = val,
            "BasicEarningsLossPerShareFromDiscontinuedOperations" => rule.eps_disc = val,
            "ProfitOrLossAttributableToOwnersOfParent" => rule.prof_owner = val,
            "ProfitLossForPeriod" => rule.prof_period = val,
            "ComprehensiveIncomeForThePeriodAttributableToOwnersOfParent" => rule.prof_comp = val,
            "EquityAttributableToOwnersOfParent" => rule.equity = val,
            "DividendsPaidClassifiedAsFinancingActivities" => rule.div = val.abs(),
            "CashFlowsFromUsedInOperatingActivities" => rule.operating_cash_flow = val,
            
            "BorrowingsNoncurrent" | "BorrowingsCurrent" => rule.total_debt += val,

            "ProfitLossBeforeTax" | "ProfitBeforeTax" => rule.pbt = val,
            "FinanceCosts" => rule.finance_costs = val,

            "PurchaseOfPropertyPlantAndEquipmentClassifiedAsInvestingActivities" |
            "PurchaseOfInvestmentPropertyClassifiedAsInvestingActivities" |
            "PurchaseOfIntangibleAssetsUnderDevelopment" |
            "PurchaseOfGoodwillClassifiedAsInvestingActivities" |
            "PurchaseOfIntangibleAssetsClassifiedAsInvestingActivities" |
            "PurchaseOfBiologicalAssetsOtherThanBearerPlantsClassifiedAsInvestingActivities" |
            "PurchaseOfOtherLongTermAssetsClassifiedAsInvestingActivities" => rule.capex_outflow += val,
            
            "ProceedsFromSalesOfPropertyPlantAndEquipmentClassifiedAsInvestingActivities" |
            "ProceedsFromSalesOfInvestmentPropertyClassifiedAsInvestingActivities" |
            "ProceedsFromSalesOfIntangibleAssetsUnderDevelopment" |
            "ProceedsFromSalesOfGoodwillClassifiedAsInvestingActivities" |
            "ProceedsFromSalesOfIntangibleAssetsClassifiedAsInvestingActivities" |
            "ProceedsFromBiologicalAssetsOtherThanBearerPlantsClassifiedAsInvestingActivities" |
            "ProceedsFromSalesOfOtherLongTermAssetsClassifiedAsInvestingActivities" => rule.capex_inflow += val,
            _ => {}
        }
    }

    // =========================================================================
    // UNROLL DATA STREAMS & COMPUTE DYNAMIC CALCULATED TAX
    // =========================================================================
    for (_, r) in file_map {
        if r.year == 0 { continue; }
        let basic_eps = if r.eps_total != 0.0 { r.eps_total } else { r.eps_cont + r.eps_disc };
        let net_profit = if r.prof_owner != 0.0 { r.prof_owner } else if r.prof_period != 0.0 { r.prof_period } else { r.prof_comp };

        if r.div != 0.0 { div_ledg.insert(r.year, r.div); years.insert(r.year); }
        if basic_eps != 0.0 { eps_ledg.insert(r.year, basic_eps); years.insert(r.year); }
        if net_profit != 0.0 { prof_ledg.insert(r.year, net_profit); years.insert(r.year); }
        if r.equity != 0.0 { eq_ledg.insert(r.year, r.equity); years.insert(r.year); }
        if r.total_debt != 0.0 { debt_ledg.insert(r.year, r.total_debt); years.insert(r.year); }

        if r.operating_cash_flow != 0.0 { ocf_ledg.insert(r.year, r.operating_cash_flow); years.insert(r.year); }
        if r.capex_outflow != 0.0 { out_ledg.insert(r.year, r.capex_outflow * -1.0); years.insert(r.year); }
        if r.capex_inflow != 0.0 { in_ledg.insert(r.year, r.capex_inflow); years.insert(r.year); }

        // Unroll WACC Parameters
        if r.pbt != 0.0 { pbt_ledg.insert(r.year, r.pbt); years.insert(r.year); }
        if r.finance_costs != 0.0 { interest_ledg.insert(r.year, r.finance_costs); years.insert(r.year); }

        // Execute your PBT - PAT Total Tax calculation shortcut on the fly
        if r.pbt > 0.0 {
            let total_tax_expense = r.pbt - net_profit;
            let eff_rate = total_tax_expense / r.pbt;
            // Prevent skewed negative tax rate records due to anomalies/credits
            let protected_rate = if eff_rate < 0.0 { 0.0 } else { eff_rate };
            
            tax_rate_ledg.insert(r.year, protected_rate);
            years.insert(r.year);
        }
    }

    Ok(())
}

// =========================================================================
// ISOLATED SHAREHOLDING PATTERN TIMELINE RESOLUTION
// =========================================================================
fn process_shareholding_patterns(
    bse_sh_bytes: Option<Vec<u8>>,
    nse_sh_bytes: Option<Vec<u8>>,
    global_years: &mut BTreeSet<i32>,
) -> HashMap<i32, i64> {
    let mut file_date_registry: HashMap<String, i32> = HashMap::new();
    let mut file_shares_registry: HashMap<String, f64> = HashMap::new();
    let mut stitched_shares_timeline: HashMap<i32, i64> = HashMap::new();

    // Reusable internal closure to iterate through the dataframes
    let mut parse_dataframe = |bytes: Vec<u8>| -> Result<(), polars::prelude::PolarsError> {
        let df = ParquetReader::new(std::io::Cursor::new(bytes)).finish()?;
        let tag_ca = df.column("tag_name")?.str()?;
        let ctx_ca = df.column("context_id")?.str()?;
        let val_ca = df.column("raw_value")?.str()?;
        let src_ca = df.column("source_file")?.str()?;

        for i in 0..df.height() {
            let file = match src_ca.get(i) { Some(f) => f.to_string(), None => continue };
            let tag = tag_ca.get(i).unwrap_or("");
            let ctx = ctx_ca.get(i).unwrap_or("").to_lowercase();
            let raw_val = val_ca.get(i).unwrap_or("");

            // Capture the reporting calendar year
            if ctx == "onei" && tag == "DateOfReport" && raw_val.len() >= 4 {
                if let Ok(yr) = raw_val[0..4].parse::<i32>() {
                    file_date_registry.insert(file.clone(), yr);
                }
            }

            // Isolate the maximum share allocation block encountered
            if (ctx == "shareholdingpatterni" || ctx == "shareholdingpattern_contexti") && tag == "NumberOfShares" {
                let clean_shares = raw_val.trim().replace(',', "").parse::<f64>().unwrap_or(0.0);
                if clean_shares > 0.0 {
                    let entry = file_shares_registry.entry(file).or_insert(0.0);
                    if clean_shares > *entry {
                        *entry = clean_shares;
                    }
                }
            }
        }
        Ok(())
    };

    // Run both raw blocks through the parser if they exist
    if let Some(b) = bse_sh_bytes { let _ = parse_dataframe(b); }
    if let Some(b) = nse_sh_bytes { let _ = parse_dataframe(b); }

    // Intersect the file datasets to stitch the final chronology map
    for (file, year) in file_date_registry {
        if let Some(&max_shares) = file_shares_registry.get(&file) {
            let entry = stitched_shares_timeline.entry(year).or_insert(0);
            if (max_shares as i64) > *entry {
                *entry = max_shares as i64;
            }
            global_years.insert(year); // Ensure this year exists in the master map loops
        }
    }

    stitched_shares_timeline
}

// =========================================================================
// ISOLATED HISTORICAL MARKET DAILY CLOSE EXTRACTOR
// =========================================================================
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HistoricalChartRow {
    pub date: String, // format: "YYYY-MM-DD"
    pub nse_close: Option<f64>,
    pub bse_close: Option<f64>,
}

fn process_historical_chart_matrix(
    nse_chart_value: Option<serde_json::Value>,
    bse_chart_value: Option<serde_json::Value>,
) -> Vec<HistoricalChartRow> {
    let mut unified_chart_matrix: BTreeMap<String, (Option<f64>, Option<f64>)> = BTreeMap::new();

    if let Some(parsed_json) = nse_chart_value {
        if let Some(datapoints) = parsed_json.get("grapthData").and_then(|v| v.as_array()) {
            for item in datapoints {
                if let (Some(ts_val), Some(close_val)) = (item.get(0), item.get(1)) {
                    let ms_epoch = ts_val.as_i64().unwrap_or(0);
                    let price = close_val.as_f64().unwrap_or(0.0);
                    
                    if let Some(date_str) = format_epoch_ms_to_date(ms_epoch) {
                        let entry = unified_chart_matrix.entry(date_str).or_insert((None, None));
                        entry.0 = Some(price);
                    }
                }
            }
        }
    }

    if let Some(parsed_json) = bse_chart_value {
        if let Some(datapoints) = parsed_json.get("grapthData").and_then(|v| v.as_array()) {
            for item in datapoints {
                if let (Some(ts_val), Some(close_val)) = (item.get(0), item.get(1)) {
                    let ms_epoch = ts_val.as_i64().unwrap_or(0);
                    let price = close_val.as_f64().unwrap_or(0.0);
                    
                    if let Some(date_str) = format_epoch_ms_to_date(ms_epoch) {
                        let entry = unified_chart_matrix.entry(date_str).or_insert((None, None));
                        entry.1 = Some(price);
                    }
                }
            }
        }
    }
    let mut chart_rows: Vec<HistoricalChartRow> = unified_chart_matrix
        .into_iter()
        .map(|(date, (nse_close, bse_close))| HistoricalChartRow {
            date,
            nse_close,
            bse_close,
        })
        .collect();

    chart_rows.sort_by(|a, b| a.date.cmp(&b.date));

    chart_rows
}

fn format_epoch_ms_to_date(mut ms: i64) -> Option<String> {
    if ms <= 0 { return None; }
    
    if ms > 9_999_999_999 {
        ms /= 1000;
    }
    
    let days_since_epoch = ms / 86400;
    let mut year = 1970;
    let mut days_left = days_since_epoch;
    
    while days_left >= 365 {
        let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
        let days_in_year = if is_leap { 366 } else { 365 };
        if days_left >= days_in_year {
            days_left -= days_in_year;
            year += 1;
        } else {
            break;
        }
    }
    
    let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
    let month_days = if is_leap {
        vec![31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        vec![31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    
    let mut month = 1;
    for days in month_days {
        if days_left >= days {
            days_left -= days;
            month += 1;
        } else {
            break;
        }
    }
    let day = days_left + 1;
    Some(format!("{:04}-{:02}-{:02}", year, month, day))
}

fn calculate_empirical_betas(
    chart_rows: &[HistoricalChartRow],
    nifty_value: Option<serde_json::Value>,
    sensex_raw_str: Option<String>,
) -> (f64, f64) {
    let mut final_nse_beta = 1.0;
    let mut final_bse_beta = 1.0;

    // -----------------------------------------------------------------
    // PHASE A: PARSE NIFTY 50 TIMELINE
    // -----------------------------------------------------------------
    let mut nifty_map: BTreeMap<String, f64> = BTreeMap::new();
    if let Some(parsed_json) = nifty_value {
        if let Some(datapoints) = parsed_json.get("data").and_then(|d| d.get("grapthData")).and_then(|v| v.as_array()) {
            for item in datapoints {
                if let (Some(ts_val), Some(close_val)) = (item.get(0), item.get(1)) {
                    let ms_epoch = ts_val.as_i64().unwrap_or(0);
                    let price = close_val.as_f64().unwrap_or(0.0);
                    if let Some(date_str) = format_epoch_ms_to_date(ms_epoch) {
                        nifty_map.insert(date_str, price);
                    }
                }
            }
        }
    }

    // -----------------------------------------------------------------
    // PHASE B: REPAIR SENSEX DATA LOG VIA NATIVE STRING SPLITTING
    // -----------------------------------------------------------------
    let mut sensex_map: BTreeMap<String, f64> = BTreeMap::new();
    if let Some(raw_text) = sensex_raw_str {
        if let Some(historical_block) = raw_text.split("#@#").nth(1) {
            let normalized_block = historical_block.replace("\\\"", "\"");
            
            let month_map = std::collections::HashMap::from([
                ("Jan", "01"), ("Feb", "02"), ("Mar", "03"), ("Apr", "04"), ("May", "05"), ("Jun", "06"),
                ("Jul", "07"), ("Aug", "08"), ("Sep", "09"), ("Oct", "10"), ("Nov", "11"), ("Dec", "12")
            ]);

            // Split string entries using the terminating curly brace token
            for chunk in normalized_block.split('}') {
                if !chunk.contains("\"date\"") || !chunk.contains("\"value\"") {
                    continue;
                }
                
                // Native slice split parsing to extract field keys
                let mut date_extracted = String::new();
                let mut value_extracted = 0.0;

                for part in chunk.split(',') {
                    if part.contains("\"date\"") {
                        if let Some(d_val) = part.split(':').nth(1) {
                            date_extracted = d_val.replace('"', "").trim().to_string();
                        }
                    }
                    if part.contains("\"value\"") {
                        if let Some(v_val) = part.split(':').nth(1) {
                            value_extracted = v_val.replace('"', "").trim().parse::<f64>().unwrap_or(0.0);
                        }
                    }
                }

                // Normalizing date formats out of string fragments
                let date_parts: Vec<&str> = date_extracted.split_whitespace().collect();
                if date_parts.len() >= 4 {
                    let month_abbr = date_parts[1];
                    let day_str = date_parts[2];
                    let year_str = date_parts[3];
                    
                    if let Some(month_num) = month_map.get(month_abbr) {
                        let parsed_day = day_str.parse::<i32>().unwrap_or(1);
                        let formatted_date = format!("{}-{}-{:02}", year_str, month_num, parsed_day);
                        sensex_map.insert(formatted_date, value_extracted);
                    }
                }
            }
        }
    }

    // -----------------------------------------------------------------
    // PHASE C: STATISTICAL MATH MATRIX UTILITIES
    // -----------------------------------------------------------------
    let calculate_beta = |stock_returns: &[f64], index_returns: &[f64]| -> f64 {
        if stock_returns.len() < 2 { return 1.0; }
        let len = stock_returns.len() as f64;
        
        let mean_stock = stock_returns.iter().sum::<f64>() / len;
        let mean_index = index_returns.iter().sum::<f64>() / len;
        
        let mut covariance = 0.0;
        let mut variance = 0.0;
        
        for i in 0..stock_returns.len() {
            let diff_stock = stock_returns[i] - mean_stock;
            let diff_index = index_returns[i] - mean_index;
            covariance += diff_stock * diff_index;
            variance += diff_index * diff_index;
        }
        
        if variance == 0.0 { 1.0 } else { covariance / variance }
    };

    // -----------------------------------------------------------------
    // PHASE D: RUN STATS FOR NSE (IMFA NSE vs NIFTY 50)
    // -----------------------------------------------------------------
    let mut nse_stock_returns = Vec::new();
    let mut nifty_returns = Vec::new();
    let mut prev_stock_nse: Option<f64> = None;
    let mut prev_idx_nifty: Option<f64> = None;

    for row in chart_rows {
        if let (Some(stock_p), Some(&idx_p)) = (row.nse_close, nifty_map.get(&row.date)) {
            if let (Some(p_stock), Some(p_idx)) = (prev_stock_nse, prev_idx_nifty) {
                if p_stock > 0.0 && p_idx > 0.0 {
                    nse_stock_returns.push((stock_p - p_stock) / p_stock);
                    nifty_returns.push((idx_p - p_idx) / p_idx);
                }
            }
            prev_stock_nse = Some(stock_p);
            prev_idx_nifty = Some(idx_p);
        }
    }
    if !nse_stock_returns.is_empty() {
        final_nse_beta = calculate_beta(&nse_stock_returns, &nifty_returns);
    }

    // -----------------------------------------------------------------
    // PHASE E: RUN STATS FOR BSE (IMFA BSE vs SENSEX)
    // -----------------------------------------------------------------
    let mut bse_stock_returns = Vec::new();
    let mut sensex_returns = Vec::new();
    let mut prev_stock_bse: Option<f64> = None;
    let mut prev_idx_sensex: Option<f64> = None;

    for row in chart_rows {
        if let (Some(stock_p), Some(&idx_p)) = (row.bse_close, sensex_map.get(&row.date)) {
            if let (Some(p_stock), Some(p_idx)) = (prev_stock_bse, prev_idx_sensex) {
                if p_stock > 0.0 && p_idx > 0.0 {
                    bse_stock_returns.push((stock_p - p_stock) / p_stock);
                    sensex_returns.push((idx_p - p_idx) / p_idx);
                }
            }
            prev_stock_bse = Some(stock_p);
            prev_idx_sensex = Some(idx_p);
        }
    }
    if !bse_stock_returns.is_empty() {
        final_bse_beta = calculate_beta(&bse_stock_returns, &sensex_returns);
    }

    (final_nse_beta, final_bse_beta)
}

fn parse_march_rf_timeline(rf_json: &serde_json::Value) -> HashMap<i32, f64> {
    let mut rf_timeline = HashMap::new();

    if let Some(data_array) = rf_json.get("data").and_then(|d| d.as_array()) {
        for entry in data_array {
            // Read the date field to check if it's March
            if let Some(date_str) = entry.get("rowDate").and_then(|s| s.as_str()) {
                if date_str.starts_with("Mar") {
                    // Extract the year directly from the text string or timestamp
                    if let Some(year_str) = date_str.split_whitespace().nth(1) {
                        if let Ok(year) = year_str.parse::<i32>() {
                            if let Some(close_str) = entry.get("last_close").and_then(|v| v.as_str()) {
                                if let Ok(close_val) = close_str.parse::<f64>() {
                                    rf_timeline.insert(year, close_val);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    rf_timeline
}

// =========================================================================
// ISOLATED MACRO GDP PARSER
// =========================================================================
fn extract_historical_avg_gdp(loader: &crate::database::WorkspaceDataLoader) -> f64 {
    let raw_json: serde_json::Value = match loader.load_json_struct("misc_gdp-data/endpoint-metadata.json") {
        Ok(json) => json,
        Err(_) => return 5.5, // Fallback historical average if file is missing
    };

    let mut recent_gdp_values = Vec::new();

    if let Some(values_obj) = raw_json
        .pointer("/indicators/NGDP_RPCH/values/IND")
        .and_then(|v| v.as_object()) 
    {
        for (_year_str, val) in values_obj {
            if let Some(gdp_num) = val.as_f64() {
                recent_gdp_values.push(gdp_num);
            }
        }
    }

    if recent_gdp_values.is_empty() {
        return 5.5;
    }

    // Sort to get the most recent years (assuming keys are chronological or we just take the last 10)
    // A simple average of the whole dataset or the last decade works. We'll average everything found.
    let sum: f64 = recent_gdp_values.iter().sum();
    let count = recent_gdp_values.len() as f64;
    
    sum / count
}

// =========================================================================
// ISOLATED BACKEND ASSUMPTIONS ENGINE
// =========================================================================
fn compute_dynamic_assumptions(
    nse_beta: f64, 
    bse_beta: f64,
    dynamic_rf: f64,
    historical_gdp: f64,
    net_profit: i64,
    total_equity: i64,
    dividend_paid: i64,
) -> (f64, f64, f64, f64) {
    let average_beta = if nse_beta > 0.0 && bse_beta > 0.0 {
        (nse_beta + bse_beta) / 2.0
    } else if nse_beta > 0.0 {
        nse_beta
    } else if bse_beta > 0.0 {
        bse_beta
    } else {
        1.0
    };

    let dynamic_rm = dynamic_rf + 5.5;

    let dynamic_rf_spread = dynamic_rf - 1.0;
    let terminal_gn = dynamic_rf_spread.min(historical_gdp).max(2.0);

    let mut sustainable_g = 12.0; 
    
    if total_equity > 0 && net_profit > 0 {
        let roe = (net_profit as f64) / (total_equity as f64);
        let payout_ratio = ((dividend_paid as f64) / (net_profit as f64)).clamp(0.0, 1.0);
        let retention_ratio = 1.0 - payout_ratio;
        
        let calculated_g = roe * retention_ratio * 100.0;
        sustainable_g = calculated_g.clamp(2.0, 20.0);
    }

    (average_beta, dynamic_rm, terminal_gn, sustainable_g)
}

// =========================================================================
// 6. MAIN ORCHESTRATION PIPELINE
// =========================================================================

pub fn hydrate_analysis_metadata(ticker: &str) -> Result<(), String> {
    let loader = crate::database::WorkspaceDataLoader::bind(ticker);

    // 1. Gather all financial statement raw parquets
    let nse_int_bytes = loader.load_raw_bytes("parquets/nse_integrated-finance-results.parquet").ok();
    let nse_corp_bytes = loader.load_raw_bytes("parquets/nse_corporates-financial-results.parquet").ok();
    let bse_fin_bytes = loader.load_raw_bytes("parquets/bse_financial-results-docs.parquet").ok();
    let bse_int_bytes = loader.load_raw_bytes("parquets/bse_integrated-finance-data.parquet").ok();
    
    let cash_flow_bytes = loader.load_raw_bytes("parquets/annual_report/cash_flow.parquet").ok();
    let income_statement_bytes = loader.load_raw_bytes("parquets/annual_report/income_statement.parquet").ok();
    let balance_sheet_bytes = loader.load_raw_bytes("parquets/annual_report/balance_sheet.parquet").ok();

    // 2. Gather Shareholding Pattern parquets
    let bse_sh_bytes = loader.load_raw_bytes("parquets/bse_shareholding-pattern-docs.parquet").ok();
    let nse_sh_bytes = loader.load_raw_bytes("parquets/nse_corporate-shareholding-master.parquet").ok();

    // 3. Gather Historical Chart data payloads directly within the orchestrator load group
    let nse_chart_value = loader.load_json_struct::<serde_json::Value>("nse_historical-chart-data/endpoint-metadata.json").ok();
    let bse_chart_value = loader.load_json_struct::<serde_json::Value>("bse_historical-chart-data/10Y.json").ok();

    let nifty_chart_value = loader.load_json_struct::<serde_json::Value>("nse_historical-index-chart-data/NIFTY_50.json").ok();
    
    let sensex_chart_string = loader.load_raw_bytes("bse_sensex-historical-data/endpoint-metadata.json")
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok());

    let rf_macro_json = loader.load_json_struct::<serde_json::Value>("misc_investing-historical-monthly/endpoint-metadata.json").ok();

    // -----------------------------------------------------------------
    // STEP A: PROCESSING HISTORICAL CHART DAILY DATA MATRIX
    // -----------------------------------------------------------------
    let chart_rows = process_historical_chart_matrix(nse_chart_value, bse_chart_value);
    store_parsed_table("historical_chart_data", chart_rows.clone());

    let (calculated_nse_beta, calculated_bse_beta) = calculate_empirical_betas(
        &chart_rows,
        nifty_chart_value,
        sensex_chart_string
    );

    let rf_timeline_map = if let Some(ref json_payload) = rf_macro_json {
        parse_march_rf_timeline(json_payload)
    } else {
        HashMap::new()
    };

    let macro_historical_gdp = extract_historical_avg_gdp(&loader);

    // -----------------------------------------------------------------
    // STEP B: RUN EXTRACTS ACROSS INTERACTIVE STATEMENT LEDGERS
    // -----------------------------------------------------------------
    let mut ocr_div_ledger = HashMap::new(); let mut ocr_eps_ledger = HashMap::new();
    let mut ocr_prof_ledger = HashMap::new(); let mut ocr_eq_ledger = HashMap::new();
    let mut ocr_ocf_ledger = HashMap::new(); let mut ocr_out_ledger = HashMap::new();
    let mut ocr_in_ledger = HashMap::new(); let mut ocr_debt_ledger = HashMap::new();
    let mut ocr_pbt_ledger = HashMap::new(); let mut ocr_interest_ledger = HashMap::new();
    let mut ocr_tax_rate_ledger = HashMap::new();

    let mut global_years = BTreeSet::new();

    if let Some(bytes) = income_statement_bytes {
        if let Ok((eps, prof, pbt, interest, tax_rate, yrs)) = process_ocr_income_statement(bytes) {
            ocr_eps_ledger = eps; 
            ocr_prof_ledger = prof; 
            ocr_pbt_ledger = pbt;
            ocr_interest_ledger = interest;
            ocr_tax_rate_ledger = tax_rate;
            global_years.extend(yrs);
        }
    }
    if let Some(bytes) = cash_flow_bytes {
        if let Ok((div, ocf, out, r_in, yrs)) = process_ocr_cash_flow(bytes) {
            ocr_div_ledger = div; ocr_ocf_ledger = ocf; ocr_out_ledger = out; ocr_in_ledger = r_in;
            global_years.extend(yrs);
        }
    }
    if let Some(bytes) = balance_sheet_bytes {
        if let Ok((eq, debt, yrs)) = process_ocr_balance_sheet(bytes) {
            ocr_eq_ledger = eq; ocr_debt_ledger = debt; global_years.extend(yrs);
        }
    }

    let mut nse_div = HashMap::new(); let mut nse_eps = HashMap::new(); let mut nse_prof = HashMap::new(); let mut nse_eq = HashMap::new();
    let mut nse_ocf = HashMap::new(); let mut nse_out = HashMap::new(); let mut nse_in = HashMap::new(); let mut nse_debt = HashMap::new();
    let mut nse_pbt = HashMap::new(); let mut nse_interest = HashMap::new(); let mut nse_tax_rate = HashMap::new();

    let mut bse_div = HashMap::new(); let mut bse_eps = HashMap::new(); let mut bse_prof = HashMap::new(); let mut bse_eq = HashMap::new();
    let mut bse_ocf = HashMap::new(); let mut bse_out = HashMap::new(); let mut bse_in = HashMap::new(); let mut bse_debt = HashMap::new();
    let mut bse_pbt = HashMap::new(); let mut bse_interest = HashMap::new(); let mut bse_tax_rate = HashMap::new();

    let _ = process_exchange_xbrl(nse_int_bytes, &mut nse_div, &mut nse_eps, &mut nse_prof, &mut nse_eq, &mut nse_ocf, &mut nse_out, &mut nse_in, &mut nse_debt, &mut nse_pbt, &mut nse_interest, &mut nse_tax_rate, &mut global_years);
    let _ = process_exchange_xbrl(nse_corp_bytes, &mut nse_div, &mut nse_eps, &mut nse_prof, &mut nse_eq, &mut nse_ocf, &mut nse_out, &mut nse_in, &mut nse_debt, &mut nse_pbt, &mut nse_interest, &mut nse_tax_rate, &mut global_years);
    let _ = process_exchange_xbrl(bse_fin_bytes, &mut bse_div, &mut bse_eps, &mut bse_prof, &mut bse_eq, &mut bse_ocf, &mut bse_out, &mut bse_in, &mut bse_debt, &mut bse_pbt, &mut bse_interest, &mut bse_tax_rate, &mut global_years);
    let _ = process_exchange_xbrl(bse_int_bytes, &mut bse_div, &mut bse_eps, &mut bse_prof, &mut bse_eq, &mut bse_ocf, &mut bse_out, &mut bse_in, &mut bse_debt, &mut bse_pbt, &mut bse_interest, &mut bse_tax_rate, &mut global_years);

    let mut stitched_shares_timeline = process_shareholding_patterns(bse_sh_bytes, nse_sh_bytes, &mut global_years);

    // -----------------------------------------------------------------
    // STEP C: TWO-WAY (BACKWARD & FORWARD) TIMELINE BOUNDARY PADDING ENGINE
    // -----------------------------------------------------------------
    let valid_share_years: BTreeSet<i32> = stitched_shares_timeline
        .iter()
        .filter(|&(_, &v)| v > 0)
        .map(|(&yr, _)| yr)
        .collect();

    if !valid_share_years.is_empty() {
        let min_share_yr = *valid_share_years.first().unwrap();
        let max_share_yr = *valid_share_years.last().unwrap();
        
        let first_available_shares = *stitched_shares_timeline.get(&min_share_yr).unwrap();
        let last_available_shares = *stitched_shares_timeline.get(&max_share_yr).unwrap();
        for &target_year in &global_years {
            let existing_val = stitched_shares_timeline.get(&target_year).copied().unwrap_or(0);
            
            if existing_val == 0 {
                if target_year < min_share_yr {
                    stitched_shares_timeline.insert(target_year, first_available_shares);
                } else if target_year > max_share_yr {
                    stitched_shares_timeline.insert(target_year, last_available_shares);
                } else {
                    let mut lower_bound_yr = min_share_yr;
                    for &valid_yr in &valid_share_years {
                        if valid_yr < target_year { lower_bound_yr = valid_yr; }
                        else { break; }
                    }
                    let interpolation_proxy = *stitched_shares_timeline.get(&lower_bound_yr).unwrap();
                    stitched_shares_timeline.insert(target_year, interpolation_proxy);
                }
            }
        }
    }

    // -----------------------------------------------------------------
    // STEP D: COALESCE EVERYTHING INTO UNIFIED HORIZONTAL ROWS MATRIX
    // -----------------------------------------------------------------
    let mut meta_analysis = Vec::with_capacity(global_years.len());

    for year in global_years {
        let dividend_paid = nse_div.get(&year).copied().filter(|&v| v != 0.0)
            .or_else(|| bse_div.get(&year).copied().filter(|&v| v != 0.0))
            .or_else(|| ocr_div_ledger.get(&year).copied()).unwrap_or(0.0) as i64;

        let basic_eps = nse_eps.get(&year).copied().filter(|&v| v != 0.0)
            .or_else(|| bse_eps.get(&year).copied().filter(|&v| v != 0.0))
            .or_else(|| ocr_eps_ledger.get(&year).copied()).unwrap_or(0.0);

        let net_profit_after_tax = nse_prof.get(&year).copied().filter(|&v| v != 0.0)
            .or_else(|| bse_prof.get(&year).copied().filter(|&v| v != 0.0))
            .or_else(|| ocr_prof_ledger.get(&year).copied()).unwrap_or(0.0) as i64;

        let total_equity = nse_eq.get(&year).copied().filter(|&v| v != 0.0)
            .or_else(|| bse_eq.get(&year).copied().filter(|&v| v != 0.0))
            .or_else(|| ocr_eq_ledger.get(&year).copied()).unwrap_or(0.0) as i64;

        let total_debt = nse_debt.get(&year).copied().filter(|&v| v != 0.0)
            .or_else(|| bse_debt.get(&year).copied().filter(|&v| v != 0.0))
            .or_else(|| ocr_debt_ledger.get(&year).copied()).unwrap_or(0.0) as i64;

        let operating_cash_flow = nse_ocf.get(&year).copied().filter(|&v| v != 0.0)
            .or_else(|| bse_ocf.get(&year).copied().filter(|&v| v != 0.0))
            .or_else(|| ocr_ocf_ledger.get(&year).copied()).unwrap_or(0.0);

        let capex_outflow = nse_out.get(&year).copied().filter(|&v| v != 0.0)
            .or_else(|| bse_out.get(&year).copied().filter(|&v| v != 0.0))
            .or_else(|| ocr_out_ledger.get(&year).copied()).unwrap_or(0.0);

        let capex_inflow = nse_in.get(&year).copied().filter(|&v| v != 0.0)
            .or_else(|| bse_in.get(&year).copied().filter(|&v| v != 0.0))
            .or_else(|| ocr_in_ledger.get(&year).copied()).unwrap_or(0.0);

        let net_capex = capex_outflow + capex_inflow;
        let free_cash_flow = operating_cash_flow + net_capex;

        let outstanding_shares = stitched_shares_timeline.get(&year).copied().unwrap_or(0);

        let profit_before_tax = nse_pbt.get(&year).copied().filter(|&v| v != 0.0)
            .or_else(|| bse_pbt.get(&year).copied().filter(|&v| v != 0.0))
            .or_else(|| ocr_pbt_ledger.get(&year).copied()).unwrap_or(0.0) as i64;

        let finance_interest_expense = nse_interest.get(&year).copied().filter(|&v| v != 0.0)
            .or_else(|| bse_interest.get(&year).copied().filter(|&v| v != 0.0))
            .or_else(|| ocr_interest_ledger.get(&year).copied()).unwrap_or(0.0) as i64;

        let effective_tax_rate = nse_tax_rate.get(&year).copied().filter(|&v| v != 0.0)
            .or_else(|| bse_tax_rate.get(&year).copied().filter(|&v| v != 0.0))
            .or_else(|| ocr_tax_rate_ledger.get(&year).copied()).unwrap_or(0.0);
        
        let dynamic_rf = rf_timeline_map.get(&year).copied().unwrap_or(7.0);

        let (average_beta, dynamic_rm, terminal_gn, sustainable_g) = compute_dynamic_assumptions(
            calculated_nse_beta, 
            calculated_bse_beta,
            dynamic_rf,
            macro_historical_gdp,
            net_profit_after_tax as i64,
            total_equity as i64,
            dividend_paid as i64,
        );

        meta_analysis.push(AnalysisMetadataRow {
            year,
            dividend_paid,
            basic_eps,
            net_profit_after_tax,
            total_equity,
            total_debt,
            operating_cash_flow: operating_cash_flow as i64,
            capex_outflow: capex_outflow as i64,
            capex_inflow: capex_inflow as i64,
            net_capex: net_capex as i64,
            free_cash_flow: free_cash_flow as i64,
            outstanding_shares,
            profit_before_tax,
            finance_interest_expense,
            effective_tax_rate,
            nse_beta: calculated_nse_beta,
            bse_beta: calculated_bse_beta,
            dynamic_rf,
            average_beta,
            dynamic_rm,
            sustainable_g,
            terminal_gn,
        });
    }

    store_parsed_table("analysis_metadata", meta_analysis);
    Ok(())
}