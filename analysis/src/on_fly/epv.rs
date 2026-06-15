// stock-app/analysis/src/on_fly/epv.rs

/// Struct to hold the raw financial metric slice inputs for the EPV calculations
pub struct EpvInputMetrics {
    pub outstanding_shares: i64,
    pub profit_before_tax: i64,
    pub net_profit_after_tax: i64,
    pub total_debt: i64,
    pub total_equity: i64,
    pub finance_interest_expense: i64,
    pub nse_beta: f64,
    pub bse_beta: f64,
}

/// Struct to hold the compiled numerical results of the EPV calculation pass
pub struct EpvCalculatedOutput {
    pub calculated_tax_rate: f64,
    pub calculated_kd: f64,
    pub calculated_ke: f64,
    pub calculated_wacc: f64,
    pub intrinsic_value: f64,
    pub status_ok: bool,
    pub error_msg: String,
}

/// Statelessly computes the Earnings Power Value (Zero Growth Floor) for a given year block
pub fn calculate_epv_on_fly(
    metrics: &EpvInputMetrics,
    rf_str: &str,
    rm_str: &str,
) -> EpvCalculatedOutput {
    let mut output = EpvCalculatedOutput {
        calculated_tax_rate: 0.0,
        calculated_kd: 0.0,
        calculated_ke: 0.0,
        calculated_wacc: 0.0,
        intrinsic_value: 0.0,
        status_ok: false,
        error_msg: String::new(),
    };

    // 1. Strict Input Parsing from strings (NO ASSUMPTIONS)
    let rf = match rf_str.parse::<f64>() {
        Ok(v) => v / 100.0,
        Err(_) => { output.error_msg = "Bad Rf".to_string(); return output; }
    };
    let rm = match rm_str.parse::<f64>() {
        Ok(v) => v / 100.0,
        Err(_) => { output.error_msg = "Bad Rm".to_string(); return output; }
    };

    // 2. Map Metrics out of Input Block
    let shares = metrics.outstanding_shares as f64;
    let pbt = metrics.profit_before_tax as f64;
    let pat = metrics.net_profit_after_tax as f64;
    let debt = metrics.total_debt as f64;
    let equity = metrics.total_equity as f64;
    let interest = metrics.finance_interest_expense as f64;
    let beta = (metrics.nse_beta + metrics.bse_beta) / 2.0;

    // 3. Derived Corporate Financial Math Formulas
    let tax_rate = if pbt > 0.0 && pbt > pat { (pbt - pat) / pbt } else { 0.0 };
    
    let kd = if debt > 0.0 { 
        let calculated_kd = interest / debt;
        if calculated_kd > 1.0 {
            rf + 0.02 // Fallback to risk-free rate + spread if interest outpaces debt structure
        } else {
            calculated_kd
        }
    } else { 
        0.0 
    };

    let ke = rf + beta * (rm - rf);
    
    let total_cap = debt + equity;
    if total_cap <= 0.0 {
        output.error_msg = "Cap <= 0".to_string();
        return output;
    }

    let wacc = (ke * (equity / total_cap)) + ((kd * (1.0 - tax_rate)) * (debt / total_cap));

    output.calculated_tax_rate = tax_rate;
    output.calculated_kd = kd;
    output.calculated_ke = ke;
    output.calculated_wacc = wacc;

    // 4. Invariant Financial Guardrails Validation
    if shares <= 0.0 { output.error_msg = "No Shares".to_string(); return output; }
    if pat <= 0.0 { output.error_msg = "PAT <= 0".to_string(); return output; }
    if wacc <= 0.0 { output.error_msg = "WACC <= 0".to_string(); return output; }

    // 5. Run Bruce Greenwald EPV Capitalization Formula
    // Operating Earnings Power (Adjusted Earnings) = Net Income / WACC (unlevered adjustment base)
    // For a standard public equity pass, we capitalize normalized earnings and subtract net debt.
    let capitalized_earnings_power = pat / wacc;
    let intrinsic_share = (capitalized_earnings_power - debt).max(0.0) / shares;

    output.intrinsic_value = intrinsic_share;
    output.status_ok = true;
    output
}