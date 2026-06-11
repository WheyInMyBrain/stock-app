use crate::commands::memory_pool::store_parsed_table;
use polars::prelude::*;
use std::collections::{HashMap, BTreeSet, HashSet};

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AnalysisMetadataRow {
    pub year: i32,
    pub dividend_paid: i64,
    pub basic_eps: f64,
    pub net_profit_after_tax: i64,
    pub total_equity: i64,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CashFlowMetadataRow {
    pub year: i32,
    pub operating_cash_flow: i64,
    pub capex_outflow: i64,
    pub capex_inflow: i64,
    pub net_capex: i64,
    pub free_cash_flow: i64,
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
}

fn process_ocr_income_statement(bytes: Vec<u8>) -> Result<(HashMap<i32, f64>, HashMap<i32, f64>, BTreeSet<i32>), PolarsError> {
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

        let is_basic = part.trim().starts_with("basic") || part.contains("basic and diluted");
        let is_eps = part.contains("earnings per") && part.contains("share");
        if is_basic { rule.b_c = c_num; rule.b_p = p_num; }
        if is_eps { rule.h_c = c_num; rule.h_p = p_num; }

        let is_excluded = part.contains("before tax") || part.contains("operating") || part.contains("before exceptional") || part.contains("comprising") || part.contains("comprehensive");
        if !is_excluded && (c_num != 0.0 || p_num != 0.0) {
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

    let mut eps_ledger: HashMap<i32, f64> = HashMap::new();
    let mut prof_ledger: HashMap<i32, f64> = HashMap::new();

    let mut sorted_rules: Vec<_> = file_map.into_values().filter(|r| r.file_year != 0).collect();
    sorted_rules.sort_by_key(|r| r.file_year);

    for r in &sorted_rules {
        let eps_curr = if r.b_c == 0.0 && r.b_p == 0.0 { r.h_c } else { r.b_c };
        if eps_curr != 0.0 { eps_ledger.insert(r.file_year, eps_curr); years.insert(r.file_year); }
        if r.prof_c != 0.0 { prof_ledger.insert(r.file_year, r.prof_c); years.insert(r.file_year); }
    }
    for r in &sorted_rules {
        let eps_prev = if r.b_c == 0.0 && r.b_p == 0.0 { r.h_p } else { r.b_p };
        if eps_prev != 0.0 { eps_ledger.insert(r.file_year - 1, eps_prev); years.insert(r.file_year - 1); }
        if r.prof_p != 0.0 { prof_ledger.insert(r.file_year - 1, r.prof_p); years.insert(r.file_year - 1); }
    }

    Ok((eps_ledger, prof_ledger, years))
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

        // Parse Dividend Allocations (.last() emulation)
        if (part.contains("dividend") && part.contains("paid")) || (part.contains("paid") && part.contains("dividend")) {
            div_file_map.insert(file_name.clone(), (yr, c_num.abs(), p_num.abs()));
        }

        // Parse Operating Cash Flow (.last() emulation)
        if part.contains("net") && part.contains("cash") && part.contains("operating") ||
           part.contains("cash") && part.contains("generated") && part.contains("operations") ||
           part.contains("net") && part.contains("cash") && part.contains("flow") && part.contains("operating") {
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
// 4. OCR TRACK C: BALANCE SHEET (EQUITY MIN)
// =========================================================================

#[derive(Default, Debug, Clone)]
struct BsFileRules {
    file_year: i32,
    t1_c: f64, t1_p: f64,
    cap_c: f64, cap_p: f64,
    res_c: f64, res_p: f64,
    t3_c: f64, t3_p: f64,
    f_t1: bool, f_cap: bool, f_res: bool, f_t3: bool,
}

fn process_ocr_balance_sheet(bytes: Vec<u8>) -> Result<(HashMap<i32, f64>, BTreeSet<i32>), PolarsError> {
    let df = ParquetReader::new(std::io::Cursor::new(bytes)).finish()?;
    let mut eq_ledger: HashMap<i32, f64> = HashMap::new();
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
        if part.contains("liabilities") || part.contains("minority") { continue; }
        let file_name = match f_ca.get(i) { Some(f) => f.to_string(), None => continue };
        let c_num = curr_ca.get(i).unwrap_or("0").trim().parse::<f64>().unwrap_or(0.0);
        let p_num = prev_ca.get(i).unwrap_or("0").trim().parse::<f64>().unwrap_or(0.0);

        let rule = file_map.entry(file_name.clone()).or_insert_with(|| BsFileRules {
            file_year: extract_year_from_filename(&file_name).unwrap_or(0),
            ..Default::default()
        });

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
        let eq_curr = if r.t1_c != 0.0 || r.t1_p != 0.0 { r.t1_c } else if (r.cap_c + r.res_c) != 0.0 || (r.cap_p + r.res_p) != 0.0 { r.cap_c + r.res_c } else { r.t3_c };
        let eq_prev = if r.t1_c != 0.0 || r.t1_p != 0.0 { r.t1_p } else if (r.cap_c + r.res_c) != 0.0 || (r.cap_p + r.res_p) != 0.0 { r.cap_p + r.res_p } else { r.t3_p };

        if eq_curr != 0.0 {
            let e = eq_ledger.entry(r.file_year).or_insert(eq_curr);
            if eq_curr < *e { *e = eq_curr; }
            years.insert(r.file_year);
        }
        if eq_prev != 0.0 {
            let e = eq_ledger.entry(r.file_year - 1).or_insert(eq_prev);
            if eq_prev < *e { *e = eq_prev; }
            years.insert(r.file_year - 1);
        }
    }

    Ok((eq_ledger, years))
}

// =========================================================================
// 5. XBRL EXCHANGE EXTRACTION ENGINE (WITH DCF CASH VECTORS EXTENSION)
// =========================================================================

#[derive(Default, Debug, Clone)]
struct XmlFileRules {
    year: i32,
    eps_total: f64, eps_cont: f64, eps_disc: f64,
    prof_owner: f64, prof_period: f64, prof_comp: f64,
    equity: f64, div: f64,
    // New DCF Matrix extensions
    operating_cash_flow: f64,
    capex_outflow: f64,
    capex_inflow: f64,
}

fn process_exchange_xbrl(
    bytes_opt: Option<Vec<u8>>,
    div_ledg: &mut HashMap<i32, f64>,
    eps_ledg: &mut HashMap<i32, f64>,
    prof_ledg: &mut HashMap<i32, f64>,
    eq_ledg: &mut HashMap<i32, f64>,
    // Extended target output map anchors
    ocf_ledg: &mut HashMap<i32, f64>,
    out_ledg: &mut HashMap<i32, f64>,
    in_ledg: &mut HashMap<i32, f64>,
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
        // Fallback context validation routing matches OneD logic checks
        if ctx != "oned" && ctx != "fourd" && ctx != "onei" { continue; }
        let file = match src_ca.get(i) { Some(f) => f.to_string(), None => continue };
        if !consolidated_files.contains(&file) || !file_map.contains_key(&file) || file_map[&file].year == 0 { continue; }
        
        let tag = tag_ca.get(i).unwrap_or("");
        let val = val_ca.get(i).unwrap_or("0").trim().parse::<f64>().unwrap_or(0.0);

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
            // DCF Core Tag Identifiers Map
            "CashFlowsFromUsedInOperatingActivities" => rule.operating_cash_flow = val,
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

    for (_, r) in file_map {
        if r.year == 0 { continue; }
        let basic_eps = if r.eps_total != 0.0 { r.eps_total } else { r.eps_cont + r.eps_disc };
        let net_profit = if r.prof_owner != 0.0 { r.prof_owner } else if r.prof_period != 0.0 { r.prof_period } else { r.prof_comp };

        if r.div != 0.0 { div_ledg.insert(r.year, r.div); years.insert(r.year); }
        if basic_eps != 0.0 { eps_ledg.insert(r.year, basic_eps); years.insert(r.year); }
        if net_profit != 0.0 { prof_ledg.insert(r.year, net_profit); years.insert(r.year); }
        if r.equity != 0.0 { eq_ledg.insert(r.year, r.equity); years.insert(r.year); }

        // Unroll custom vertical horizontal capex outflows
        if r.operating_cash_flow != 0.0 { ocf_ledg.insert(r.year, r.operating_cash_flow); years.insert(r.year); }
        if r.capex_outflow != 0.0 { out_ledg.insert(r.year, r.capex_outflow * -1.0); years.insert(r.year); }
        if r.capex_inflow != 0.0 { in_ledg.insert(r.year, r.capex_inflow); years.insert(r.year); }
    }

    Ok(())
}

// =========================================================================
// 6. MAIN ORCHESTRATION PIPELINE
// =========================================================================

pub fn hydrate_analysis_metadata(ticker: &str) -> Result<(), String> {
    let loader = crate::database::WorkspaceDataLoader::bind(ticker);

    let nse_int_bytes = loader.load_raw_bytes("parquets/nse_integrated-finance-results.parquet").ok();
    let nse_corp_bytes = loader.load_raw_bytes("parquets/nse_corporates-financial-results.parquet").ok();
    let bse_fin_bytes = loader.load_raw_bytes("parquets/bse_financial-results-docs.parquet").ok();
    let bse_int_bytes = loader.load_raw_bytes("parquets/bse_integrated-finance-data.parquet").ok();
    
    let cash_flow_bytes = loader.load_raw_bytes("parquets/annual_report/cash_flow.parquet").ok();
    let income_statement_bytes = loader.load_raw_bytes("parquets/annual_report/income_statement.parquet").ok();
    let balance_sheet_bytes = loader.load_raw_bytes("parquets/annual_report/balance_sheet.parquet").ok();

    let mut ocr_div_ledger = HashMap::new(); let mut ocr_eps_ledger = HashMap::new();
    let mut ocr_prof_ledger = HashMap::new(); let mut ocr_eq_ledger = HashMap::new();
    let mut ocr_ocf_ledger = HashMap::new(); let mut ocr_out_ledger = HashMap::new();
    let mut ocr_in_ledger = HashMap::new();

    let mut global_years = BTreeSet::new();

    if let Some(bytes) = income_statement_bytes {
        if let Ok((eps, prof, yrs)) = process_ocr_income_statement(bytes) {
            ocr_eps_ledger = eps; ocr_prof_ledger = prof; global_years.extend(yrs);
        }
    }
    if let Some(bytes) = cash_flow_bytes {
        if let Ok((div, ocf, out, r_in, yrs)) = process_ocr_cash_flow(bytes) {
            ocr_div_ledger = div; ocr_ocf_ledger = ocf; ocr_out_ledger = out; ocr_in_ledger = r_in;
            global_years.extend(yrs);
        }
    }
    if let Some(bytes) = balance_sheet_bytes {
        if let Ok((eq, yrs)) = process_ocr_balance_sheet(bytes) {
            ocr_eq_ledger = eq; global_years.extend(yrs);
        }
    }

    let mut nse_div = HashMap::new(); let mut nse_eps = HashMap::new(); let mut nse_prof = HashMap::new(); let mut nse_eq = HashMap::new();
    let mut nse_ocf = HashMap::new(); let mut nse_out = HashMap::new(); let mut nse_in = HashMap::new();

    let mut bse_div = HashMap::new(); let mut bse_eps = HashMap::new(); let mut bse_prof = HashMap::new(); let mut bse_eq = HashMap::new();
    let mut bse_ocf = HashMap::new(); let mut bse_out = HashMap::new(); let mut bse_in = HashMap::new();

    let _ = process_exchange_xbrl(nse_int_bytes, &mut nse_div, &mut nse_eps, &mut nse_prof, &mut nse_eq, &mut nse_ocf, &mut nse_out, &mut nse_in, &mut global_years);
    let _ = process_exchange_xbrl(nse_corp_bytes, &mut nse_div, &mut nse_eps, &mut nse_prof, &mut nse_eq, &mut nse_ocf, &mut nse_out, &mut nse_in, &mut global_years);
    let _ = process_exchange_xbrl(bse_fin_bytes, &mut bse_div, &mut bse_eps, &mut bse_prof, &mut bse_eq, &mut bse_ocf, &mut bse_out, &mut bse_in, &mut global_years);
    let _ = process_exchange_xbrl(bse_int_bytes, &mut bse_div, &mut bse_eps, &mut bse_prof, &mut bse_eq, &mut bse_ocf, &mut bse_out, &mut bse_in, &mut global_years);

    // =========================================================================
    // COALESCE AMALGAMATION PIPELINE (METRIC-BY-METRIC SCAN)
    // =========================================================================
    let mut meta_analysis = Vec::with_capacity(global_years.len());
    let mut meta_cashflow = Vec::with_capacity(global_years.len());

    for year in global_years {
        // DDM Metrics Allocation Group
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

        // DCF Metrics Allocation Group
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

        meta_analysis.push(AnalysisMetadataRow {
            year, dividend_paid, basic_eps, net_profit_after_tax, total_equity
        });

        meta_cashflow.push(CashFlowMetadataRow {
            year,
            operating_cash_flow: operating_cash_flow as i64,
            capex_outflow: capex_outflow as i64,
            capex_inflow: capex_inflow as i64,
            net_capex: net_capex as i64,
            free_cash_flow: free_cash_flow as i64,
        });
    }

    store_parsed_table("analysis_metadata", meta_analysis);
    store_parsed_table("cashflow_metadata", meta_cashflow);
    Ok(())
}