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
        calculated_ke: 0.0,
        intrinsic_value: 0.0,
        status_ok: false,
        error_msg: String::new(),
    };

    // 1. Strict Input Parsing from strings (Zero Assumptions, relies fully on your backend keys)
    let rf = match rf_str.parse::<f64>() {
        Ok(v) => v / 100.0,
        Err(_) => { output.error_msg = "Bad Rf".to_string(); return output; }
    };
    let rm = match rm_str.parse::<f64>() {
        Ok(v) => v / 100.0,
        Err(_) => { output.error_msg = "Bad Rm".to_string(); return output; }
    };
    let raw_growth = match g_str.parse::<f64>() {
        Ok(v) => v / 100.0,
        Err(_) => { output.error_msg = "Bad g".to_string(); return output; }
    };

    // 2. Macroeconomic Cap: Long-term internal income expansion growth cannot outpace the Risk-Free Rate.
    // This stops abnormal earnings from compounding into trillions inside the 5-year transition loop.
    let growth = raw_growth.min(rf);

    // 3. Map Metrics out of Input Block
    let eq_base = metrics.total_equity as f64;
    let pat_base = metrics.net_profit_after_tax as f64;
    let shares = metrics.outstanding_shares as f64;
    
    let beta = (metrics.nse_beta + metrics.bse_beta) / 2.0;

    // 4. Derived Required Return on Equity (CAPM Cost of Equity)
    let ke = rf + beta * (rm - rf);
    output.calculated_ke = ke;

    // 5. Invariant Financial Guardrails Validation
    if shares <= 0.0 {
        output.error_msg = "No Shares".to_string();
        return output;
    }
    if eq_base <= 0.0 {
        output.error_msg = "Eq <= 0".to_string();
        return output;
    }
    if ke <= 0.0 {
        output.error_msg = "Ke <= 0".to_string();
        return output;
    }
    if ke <= growth {
        output.error_msg = "Ke <= g".to_string();
        return output;
    }

    // 6. Run 5-Year Multi-Stage Residual Income Projection Engine Loop
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

        // Transition matrix vectors ahead using capped economic growth projection constraints
        projected_pat *= 1.0 + growth;
        projected_equity += residual_income; 
    }

    // 7. Terminal Capitalization Pass
    let terminal_pv_ri = if ke > growth && last_residual_income > 0.0 {
        let terminal_ri = last_residual_income * (1.0 + growth);
        let capitalized_ri = terminal_ri / (ke - growth);
        capitalized_ri / (1.0 + ke).powi(5)
    } else {
        0.0 
    };

    // 8. Intrinsic Valuation Bridge 
    let total_firm_value = eq_base + pv_residual_income + terminal_pv_ri;
    let intrinsic_value_per_share = total_firm_value / shares;

    output.intrinsic_value = intrinsic_value_per_share;
    output.status_ok = true;
    output
}