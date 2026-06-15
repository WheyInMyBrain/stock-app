// stock-app/analysis/src/on_fly/bgvm.rs

/// Struct to hold the raw financial metric slice inputs for the Graham calculations
pub struct GrahamInputMetrics {
    pub outstanding_shares: i64,
    pub net_profit_after_tax: i64,  // Used to calculate baseline EPS
    pub total_equity: i64,          // Used to calculate Book Value Per Share
    pub nse_beta: f64,
    pub bse_beta: f64,
}

/// Struct to hold the compiled numerical results of the Graham pass
pub struct GrahamCalculatedOutput {
    pub intrinsic_value: f64,       // Modern Benjamin Graham Formula Value
    pub graham_number: f64,         // Defensive Graham Number Floor: sqrt(22.5 * EPS * BVPS)
    pub status_ok: bool,
    pub error_msg: String,
}

/// Statelessly computes both classical Benjamin Graham valuation methodologies for a given year block
pub fn calculate_graham_on_fly(
    metrics: &GrahamInputMetrics,
    rf_str: &str,
    g_str: &str,
) -> GrahamCalculatedOutput {
    let mut output = GrahamCalculatedOutput {
        intrinsic_value: 0.0,
        graham_number: 0.0,
        status_ok: false,
        error_msg: String::new(),
    };

    // 1. Strict Input Parsing from strings (Zero Assumptions)
    let rf = match rf_str.parse::<f64>() {
        Ok(v) => v, // Kept as absolute number (e.g., 7.5 for 7.5%) because Graham's formula expects raw numbers
        Err(_) => { output.error_msg = "Bad Rf".to_string(); return output; }
    };
    let growth = match g_str.parse::<f64>() {
        Ok(v) => v, // Kept as raw percentage number (e.g., 8.5 for 8.5% expected growth)
        Err(_) => { output.error_msg = "Bad g".to_string(); return output; }
    };

    // 2. Map Metrics out of Input Block
    let shares = metrics.outstanding_shares as f64;
    let pat = metrics.net_profit_after_tax as f64;
    let equity = metrics.total_equity as f64;

    if shares <= 0.0 {
        output.error_msg = "No Shares".to_string();
        return output;
    }

    // 3. Derived Fundamentals
    let eps = pat / shares;
    let bvps = equity / shares;

    // 4. Invariant Financial Guardrails Validation
    if eps <= 0.0 {
        output.error_msg = "EPS <= 0".to_string();
        return output;
    }
    if bvps <= 0.0 {
        output.error_msg = "BVPS <= 0".to_string();
        return output;
    }
    if rf <= 0.0 {
        output.error_msg = "Rf <= 0".to_string();
        return output;
    }

    // 5. Formula 1: The Classic Graham Number (Defensive Asset/Earnings Blended Ceiling)
    // Formula: Graham Number = Sqrt(22.5 * EPS * BVPS)
    let product = 22.5 * eps * bvps;
    output.graham_number = product.sqrt();

    // 6. Formula 2: The Modern Revised Benjamin Graham Intrinsic Value Formula
    // Formula: Value = (EPS * (8.5 + 2g) * 4.4) / Y
    // Where 4.4 is the historical risk-free baseline AAA corporate bond rate when Graham wrote it, 
    // and Y (rf) is the current local long-term Risk-Free Rate proxy.
    let expected_growth = growth.clamp(0.0, 20.0); // Keep growth expectations realistic
    let numerator = eps * (8.5 + 2.0 * expected_growth) * 4.4;
    let value_per_share = numerator / rf;

    output.intrinsic_value = value_per_share.max(0.0);
    output.status_ok = true;
    output
}