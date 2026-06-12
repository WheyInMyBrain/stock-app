// stock-app/analysis/src/on_fly/dcf.rs

/// Struct to hold the raw financial metric slice inputs for a specific year
pub struct DcfInputMetrics {
    pub operating_cash_flow: i64,
    pub capex_outflow: i64,
    pub total_debt: i64,
    pub total_equity: i64,
    pub outstanding_shares: i64,
    pub profit_before_tax: i64,
    pub net_profit_after_tax: i64,
    pub finance_interest_expense: i64,
    pub nse_beta: f64,
    pub bse_beta: f64,
}

/// Struct to hold the compiled numerical results of the calculation pass
pub struct DcfCalculatedOutput {
    pub calculated_tax_rate: f64,
    pub calculated_kd: f64,
    pub calculated_ke: f64,
    pub calculated_wacc: f64,
    pub intrinsic_value: f64,
    pub status_ok: bool,
    pub error_msg: String,
}

/// Statelessly computes the Discounted Cash Flow Intrinsic Value for a given year block
pub fn calculate_dcf_on_fly(
    metrics: &DcfInputMetrics,
    rf_str: &str,
    rm_str: &str,
    g_str: &str,
    gn_str: &str,
) -> DcfCalculatedOutput {
    let mut output = DcfCalculatedOutput {
        calculated_tax_rate: 0.25,
        calculated_kd: 0.085,
        calculated_ke: 0.10,
        calculated_wacc: 0.10,
        intrinsic_value: 0.0,
        status_ok: false,
        error_msg: String::new(),
    };

    // 1. Parse Input Assumptions safely from strings
    let rf = rf_str.parse::<f64>().unwrap_or(7.0) / 100.0;
    let rm = rm_str.parse::<f64>().unwrap_or(12.0) / 100.0;
    let growth = g_str.parse::<f64>().unwrap_or(10.0) / 100.0;
    let term_g = gn_str.parse::<f64>().unwrap_or(4.5) / 100.0;

    // 2. Map Metrics out of Input Block
    let ocf = metrics.operating_cash_flow as f64;
    let capex = metrics.capex_outflow as f64;
    let base_fcf = ocf + capex;
    
    let debt = metrics.total_debt as f64;
    let equity = metrics.total_equity as f64;
    let shares = metrics.outstanding_shares as f64;
    let pbt = metrics.profit_before_tax as f64;
    let pat = metrics.net_profit_after_tax as f64;
    let interest = metrics.finance_interest_expense as f64;
    
    let beta = (metrics.nse_beta + metrics.bse_beta) / 2.0;

    // 3. Derived Corporate Financial Math Formulas
    let tax_rate = if pbt > 0.0 && pbt > pat { (pbt - pat) / pbt } else { 0.25 };
    let kd = if debt > 0.0 { interest / debt } else { 0.085 };
    let ke = rf + beta * (rm - rf);
    
    let total_cap = debt + equity;
    let wacc = if total_cap > 0.0 {
        (ke * (equity / total_cap)) + ((kd * (1.0 - tax_rate)) * (debt / total_cap))
    } else {
        ke
    };

    output.calculated_tax_rate = tax_rate;
    output.calculated_kd = kd;
    output.calculated_ke = ke;
    output.calculated_wacc = wacc;

    // 4. Invariant Financial Guardrails Validation
    if shares <= 0.0 {
        output.error_msg = "Missing Shares".to_string();
        return output;
    }
    if base_fcf <= 0.0 {
        output.error_msg = "Negative FCF".to_string();
        return output;
    }
    if wacc <= term_g {
        output.error_msg = "WACC < gn".to_string();
        return output;
    }

    // 5. Run 5-Year Forward Cash Flow Projection Matrix Multiplier
    let mut pv_stage_1 = 0.0;
    let mut running_fcf = base_fcf;
    for step in 1..=5 {
        running_fcf *= 1.0 + growth;
        pv_stage_1 += running_fcf / (1.0 + wacc).powi(step);
    }

    // 6. Capitalize Perpetual Growth Terminal Horizon Value
    let terminal_value = (running_fcf * (1.0 + term_g)) / (wacc - term_g);
    let pv_terminal = terminal_value / (1.0 + wacc).powi(5);

    // 7. Deduct Net Debt Claims to isolate Equity Value Per Share
    let intrinsic_share = ((pv_stage_1 + pv_terminal) - debt).max(0.0) / shares;

    output.intrinsic_value = intrinsic_share;
    output.status_ok = true;
    output
}