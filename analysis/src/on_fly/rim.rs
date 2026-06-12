// stock-app/analysis/src/on_fly/rim.rs

/// Struct to hold the raw financial metric slice inputs for the RIM calculations
pub struct RimInputMetrics {
    pub total_equity: i64,          // Book Value of Equity
    pub net_profit_after_tax: i64,  // Net Income / PAT
    pub outstanding_shares: i64,
    pub nse_beta: f64,
    pub bse_beta: f64,
}

/// Struct to hold the compiled numerical results of the RIM calculation pass
pub struct RimCalculatedOutput {
    pub calculated_ke: f64,
    pub intrinsic_value: f64,
    pub status_ok: bool,
    pub error_msg: String,
}

/// Statelessly computes the Residual Income Model Intrinsic Value over a 5-year multi-stage horizon
pub fn calculate_rim_on_fly(
    metrics: &RimInputMetrics,
    rf_str: &str,
    rm_str: &str,
    g_str: &str,
) -> RimCalculatedOutput {
    let mut output = RimCalculatedOutput {
        calculated_ke: 0.10,
        intrinsic_value: 0.0,
        status_ok: false,
        error_msg: String::new(),
    };

    // 1. Parse Input Assumptions safely from strings
    let rf = rf_str.parse::<f64>().unwrap_or(7.0) / 100.0;
    let rm = rm_str.parse::<f64>().unwrap_or(12.0) / 100.0;
    let growth = g_str.parse::<f64>().unwrap_or(8.0) / 100.0; // Expected long-term income growth rate

    // 2. Map Metrics out of Input Block
    let eq_base = metrics.total_equity as f64;
    let pat_base = metrics.net_profit_after_tax as f64;
    let shares = metrics.outstanding_shares as f64;
    
    let beta = (metrics.nse_beta + metrics.bse_beta) / 2.0;

    // 3. Derived Required Return on Equity (CAPM Cost of Equity)
    let ke = rf + beta * (rm - rf);
    output.calculated_ke = ke;

    // 4. Invariant Financial Guardrails Validation
    if shares <= 0.0 {
        output.error_msg = "Missing Shares".to_string();
        return output;
    }
    if eq_base <= 0.0 {
        output.error_msg = "Negative/Zero Equity".to_string();
        return output;
    }
    if ke <= 0.0 {
        output.error_msg = "Invalid Ke Rate".to_string();
        return output;
    }

    // 5. Run 5-Year Multi-Stage Residual Income Projection Engine Loop
    let mut pv_residual_income = 0.0;
    let mut projected_equity = eq_base;
    let mut projected_pat = pat_base;
    let mut last_residual_income = 0.0;

    for step in 1..=5 {
        // Required Equity Earnings Charge = Equity at start of period * Ke
        let equity_charge = projected_equity * ke;
        let residual_income = projected_pat - equity_charge;
        
        // Present value of this stage's residual economic profit
        pv_residual_income += residual_income / (1.0 + ke).powi(step);

        last_residual_income = residual_income;

        // Transition matrix vectors ahead to project the next terminal sequence point:
        // Clean Surplus Accounting assumption: Equity_t = Equity_t-1 + NetIncome_t - Dividends_t
        // Assuming earnings retention reinvestment is driving the forward compound sequence
        projected_pat *= 1.0 + growth;
        projected_equity += residual_income; // Adding abnormal earnings increments to step adjustments
    }

    // 6. Terminal Capitalization Pass
    // Capitalizing the terminal economic residual value using perpetuity bounds
    let terminal_pv_ri = if ke > growth && last_residual_income > 0.0 {
        let terminal_ri = last_residual_income * (1.0 + growth);
        let capitalized_ri = terminal_ri / (ke - growth);
        capitalized_ri / (1.0 + ke).powi(5)
    } else {
        0.0 // Conservatively assume zero premium growth beyond year 5 if constraints fail
    };

    // 7. Intrinsic Valuation Bridge 
    // Value = Current Book Value + PV of Discrete RI (Years 1-5) + PV of Terminal RI
    let total_firm_value = eq_base + pv_residual_income + terminal_pv_ri;
    let intrinsic_value_per_share = total_firm_value / shares;

    output.intrinsic_value = intrinsic_value_per_share;
    output.status_ok = true;
    output
}