// stock-app/analysis/src/on_fly/ddm.rs

/// Struct to hold the raw financial metric slice inputs for the DDM calculations
pub struct DdmInputMetrics {
    pub dividend_paid: i64,
    pub outstanding_shares: i64,
    pub nse_beta: f64,
    pub bse_beta: f64,
}

/// Struct to hold the compiled numerical results of the DDM calculation pass
pub struct DdmCalculatedOutput {
    pub calculated_ke: f64,
    pub intrinsic_value: f64,
    pub status_ok: bool,
    pub error_msg: String,
}

/// Statelessly computes the Dividend Discount Model Intrinsic Value for a given year block
pub fn calculate_ddm_on_fly(
    metrics: &DdmInputMetrics,
    rf_str: &str,
    rm_str: &str,
    g_str: &str,
) -> DdmCalculatedOutput {
    let mut output = DdmCalculatedOutput {
        calculated_ke: 0.10,
        intrinsic_value: 0.0,
        status_ok: false,
        error_msg: String::new(),
    };

    // 1. Parse Input Assumptions safely from strings
    let rf = rf_str.parse::<f64>().unwrap_or(7.0) / 100.0;
    let rm = rm_str.parse::<f64>().unwrap_or(12.0) / 100.0;
    let div_growth = g_str.parse::<f64>().unwrap_or(5.0) / 100.0;

    // 2. Map Metrics out of Input Block
    let total_div = metrics.dividend_paid as f64;
    let shares = metrics.outstanding_shares as f64;
    
    let beta = (metrics.nse_beta + metrics.bse_beta) / 2.0;

    // 3. Derived Financial Metrics (CAPM Cost of Equity)
    let ke = rf + beta * (rm - rf);
    output.calculated_ke = ke;

    // 4. Invariant Financial Guardrails Validation
    if shares <= 0.0 {
        output.error_msg = "Missing Shares".to_string();
        return output;
    }
    if total_div <= 0.0 {
        output.error_msg = "No Dividends".to_string();
        return output;
    }
    if ke <= div_growth {
        output.error_msg = "Ke <= g".to_string();
        return output;
    }

    // 5. Calculate Base Dividend Per Share (DPS_0)
    let dps_base = total_div / shares;

    // 6. Run Gordon Growth Model Perpetuity Capitalization Formula
    // DPS_1 = DPS_0 * (1 + g)
    // Value = DPS_1 / (Ke - g)
    let dps_forward = dps_base * (1.0 + div_growth);
    let value_per_share = dps_forward / (ke - div_growth);

    output.intrinsic_value = value_per_share;
    output.status_ok = true;
    output
}