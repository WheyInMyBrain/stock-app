// stock-app/analysis/src/on_fly/eva.rs

/// Struct to hold the raw financial metric inputs for EVA calculations
pub struct EvaInputMetrics {
    pub profit_before_tax: i64,
    pub net_profit_after_tax: i64,
    pub total_equity: i64,
    pub total_debt: i64,
    pub finance_interest_expense: i64,
    pub outstanding_shares: i64,
    pub nse_beta: f64,
    pub bse_beta: f64,
}

/// Struct to hold the compiled numerical results of the EVA pass
pub struct EvaCalculatedOutput {
    pub calculated_tax_rate: f64,
    pub calculated_wacc: f64,
    pub capital_employed: f64,
    pub nopat: f64,
    pub eva_value: f64,              // Total corporate Economic Value Added (in ₹)
    pub eva_per_share: f64,          // EVA scaled down to share level
    pub status_ok: bool,
    pub error_msg: String,
}

/// Statelessly computes the Economic Value Added (EVA) for a given year block
pub fn calculate_eva_on_fly(
    metrics: &EvaInputMetrics,
    rf_str: &str,
    rm_str: &str,
) -> EvaCalculatedOutput {
    let mut output = EvaCalculatedOutput {
        calculated_tax_rate: 0.0,
        calculated_wacc: 0.0,
        capital_employed: 0.0,
        nopat: 0.0,
        eva_value: 0.0,
        eva_per_share: 0.0,
        status_ok: false,
        error_msg: String::new(),
    };

    // 1. Strict Input Parsing from strings
    let rf = match rf_str.parse::<f64>() {
        Ok(v) => v / 100.0,
        Err(_) => { output.error_msg = "Bad Rf".to_string(); return output; }
    };
    let rm = match rm_str.parse::<f64>() {
        Ok(v) => v / 100.0,
        Err(_) => { output.error_msg = "Bad Rm".to_string(); return output; }
    };

    // 2. Map Metrics out of Input Block
    let pbt = metrics.profit_before_tax as f64;
    let pat = metrics.net_profit_after_tax as f64;
    let debt = metrics.total_debt as f64;
    let equity = metrics.total_equity as f64;
    let interest = metrics.finance_interest_expense as f64;
    let shares = metrics.outstanding_shares as f64;
    let beta = (metrics.nse_beta + metrics.bse_beta) / 2.0;

    // 3. Derived Corporate WACC Core (Identical to your DCF logic)
    let tax_rate = if pbt > 0.0 && pbt > pat { (pbt - pat) / pbt } else { 0.0 };
    
    let kd = if debt > 0.0 { 
        let calculated_kd = interest / debt;
        if calculated_kd > 1.0 {
            rf + 0.02 // Fallback credit spread proxy
        } else {
            calculated_kd
        }
    } else { 
        0.0 
    };

    let ke = rf + beta * (rm - rf);
    let capital_employed = debt + equity;

    if capital_employed <= 0.0 {
        output.error_msg = "Cap Employed <= 0".to_string();
        return output;
    }

    let wacc = (ke * (equity / capital_employed)) + ((kd * (1.0 - tax_rate)) * (debt / capital_employed));

    // 4. EVA Core Calculations
    // NOPAT = EBIT * (1 - t). Since interest is subtracted to get PBT, we approximate EBIT as PBT + Interest
    let ebit = (pbt + interest).max(0.0);
    let nopat = ebit * (1.0 - tax_rate);
    
    // Capital Charge = Capital Employed * WACC
    let capital_charge = capital_employed * wacc;
    let eva_value = nopat - capital_charge;

    // 5. Invariant Guardrails Validation
    if shares <= 0.0 {
        output.error_msg = "No Shares".to_string();
        return output;
    }

    output.calculated_tax_rate = tax_rate;
    output.calculated_wacc = wacc;
    output.capital_employed = capital_employed;
    output.nopat = nopat;
    output.eva_value = eva_value;
    output.eva_per_share = eva_value / shares;
    output.status_ok = true;
    output
}