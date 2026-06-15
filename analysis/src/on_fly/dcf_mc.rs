// analysis/src/on_fly/dcf_mc.rs

use rayon::prelude::*;

#[derive(Debug, Clone)]
pub struct DcfMcHistoricalRow {
    pub year: i32,
    pub operating_cash_flow: i64,
    pub capex_outflow: i64,
    pub total_debt: i64,
    pub total_equity: i64,
    pub outstanding_shares: i64,
    pub profit_before_tax: i64,
    pub net_profit_after_tax: i64,
    pub finance_interest_expense: i64,
    pub beta: f64,
    pub risk_free_rate: f64,    
    pub market_return: f64,     
    pub terminal_gn: f64,       
}

#[derive(Debug, Clone)]
pub struct DcfMcConfigInput {
    pub total_simulations: usize, 
}

#[derive(Debug, Clone, Default)]
pub struct DcfMcPercentileSummary {
    pub year: i32,
    pub p100: f64, pub p97_5: f64, pub p95: f64, pub p90: f64,
    pub p75: f64,  pub p50: f64,   pub p25: f64, pub p10: f64,
    pub p5: f64,   pub p2_5: f64,  pub p0: f64,  pub average: f64,
    pub mean_g: f64, pub vol_g: f64, pub status_ok: bool, pub error_msg: String,
}

fn evaluate_tree(r: &DcfMcHistoricalRow, g: f64) -> Option<f64> {
    let shares = r.outstanding_shares as f64;
    if shares <= 0.0 { return None; }
    
    let ocf = r.operating_cash_flow as f64;
    let capex = r.capex_outflow as f64;
    let base_fcf = ocf + capex; 

    let debt = r.total_debt as f64;
    let equity = r.total_equity as f64;
    let pbt = r.profit_before_tax as f64;
    let pat = r.net_profit_after_tax as f64;
    let interest = r.finance_interest_expense as f64;
    
    let rf = r.risk_free_rate;
    let rm = r.market_return;
    let term_g = r.terminal_gn;
    let beta = r.beta;

    let tax_rate = if pbt > 0.0 && pbt > pat { (pbt - pat) / pbt } else { 0.0 };
    
    let kd = if debt > 0.0 { 
        let calculated_kd = interest / debt;
        if calculated_kd > 1.0 { return None; }
        calculated_kd
    } else { 
        0.0 
    };

    let ke = rf + beta * (rm - rf);
    let total_cap = debt + equity;
    if total_cap <= 0.0 { return None; }

    let wacc = (ke * (equity / total_cap)) + ((kd * (1.0 - tax_rate)) * (debt / total_cap));
    if wacc <= term_g || wacc <= 0.0 { return None; }

    // 5-Year Projection Loop (Symmetric and unclipped to preserve distribution integrity)
    let mut pv_stage_1 = 0.0;
    let mut running_fcf = base_fcf;
    for step in 1..=5 {
        running_fcf *= 1.0 + g;
        pv_stage_1 += running_fcf / (1.0 + wacc).powi(step);
    }

    let terminal_value = (running_fcf * (1.0 + term_g)) / (wacc - term_g);
    let pv_terminal = terminal_value / (1.0 + wacc).powi(5);

    // Intrinsic value calculation with limited liability floor protection
    let intrinsic_share = ((pv_stage_1 + pv_terminal) - debt).max(0.0) / shares;

    if intrinsic_share.is_finite() { Some(intrinsic_share) } else { None }
}

pub fn run_stochastic_dcf(history: &[DcfMcHistoricalRow], cfg: &DcfMcConfigInput) -> Vec<DcfMcPercentileSummary> {
    let mut timeline_results = Vec::with_capacity(history.len());

    for i in 0..history.len() {
        let current_year_row = &history[i];
        let target_year = current_year_row.year;

        if current_year_row.outstanding_shares <= 0 {
            timeline_results.push(DcfMcPercentileSummary {
                year: target_year,
                status_ok: false,
                error_msg: format!("No Share"),
                ..Default::default()
            });
            continue;
        }

        let sub_history = &history[0..=i];

        if sub_history.len() < 2 {
            timeline_results.push(DcfMcPercentileSummary {
                year: target_year,
                status_ok: false,
                error_msg: "No History".to_string(),
                ..Default::default()
            });
            continue;
        }

        // 1. Compute symmetric growth metrics across rolling history window slices
        let mut rolling_growth_rates = Vec::new();
        for w in sub_history.windows(2) {
            let prev = (w[0].operating_cash_flow + w[0].capex_outflow) as f64;
            let curr = (w[1].operating_cash_flow + w[1].capex_outflow) as f64;

            let denom = (prev.abs() + curr.abs()) / 2.0;
            if denom > 0.0 {
                let rate = (curr - prev) / denom;
                rolling_growth_rates.push(rate.clamp(-0.25, 0.30)); 
            }
        }

        let mean_g = if !rolling_growth_rates.is_empty() {
            rolling_growth_rates.iter().sum::<f64>() / rolling_growth_rates.len() as f64
        } else { 0.05 };

        let variance = if !rolling_growth_rates.is_empty() {
            rolling_growth_rates.iter().map(|&g| (g - mean_g).powi(2)).sum::<f64>() / rolling_growth_rates.len() as f64
        } else { 0.01 };
        
        // 2. Clamp the underlying volatility coefficient directly (Enforces 0% to 6% standard deviation bound)
        // This keeps the normal distribution curve structure mathematically sound while avoiding tail explosions
        let vol_g = if variance > 0.0 { 
            variance.sqrt().clamp(0.0, 0.06) 
        } else { 
            0.03 
        };

        // 3. Parallel Worker Path Simulations Pass
        let prices: Vec<f64> = (0..cfg.total_simulations)
            .into_par_iter()
            .filter_map(|idx| {
                let mut state = (idx + 7) as u64;
                state = state.wrapping_add(0x9E3779B97F4A7C15);
                let mut z = state;
                z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
                z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
                let u1 = ((z ^ (z >> 31)) as f64 / u64::MAX as f64).max(1e-10);

                state = state.wrapping_add(0x9E3779B97F4A7C15);
                let mut z2 = state;
                z2 = (z2 ^ (z2 >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
                z2 = (z2 ^ (z2 >> 27)).wrapping_mul(0x94D049BB133111EB);
                let u2 = ((z2 ^ (z2 >> 31)) as f64 / u64::MAX as f64).max(1e-10);

                let rand_norm = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
                let simulated_g = mean_g + (vol_g * rand_norm);

                evaluate_tree(current_year_row, simulated_g)
            })
            .collect();

        if prices.is_empty() {
            timeline_results.push(DcfMcPercentileSummary { 
                year: target_year, 
                status_ok: false, 
                error_msg: "Bad Math.".to_string(), 
                ..Default::default() 
            });
            continue;
        }

        let mut sorted = prices;
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let len = sorted.len();
        let sum: f64 = sorted.iter().sum();
        let calculated_average = sum / len as f64;

        timeline_results.push(DcfMcPercentileSummary {
            year: target_year,
            p100: sorted[len - 1],
            p97_5: sorted[((len as f64 * 0.975) as usize).min(len - 1)],
            p95: sorted[((len as f64 * 0.95) as usize).min(len - 1)],
            p90: sorted[((len as f64 * 0.90) as usize).min(len - 1)],
            p75: sorted[((len as f64 * 0.75) as usize).min(len - 1)],
            p50: sorted[((len as f64 * 0.50) as usize).min(len - 1)],
            p25: sorted[((len as f64 * 0.25) as usize).min(len - 1)],
            p10: sorted[((len as f64 * 0.10) as usize).min(len - 1)],
            p5: sorted[((len as f64 * 0.05) as usize).min(len - 1)],
            p2_5: sorted[((len as f64 * 0.025) as usize).min(len - 1)],
            p0: sorted[0],
            average: calculated_average,
            mean_g, 
            vol_g, 
            status_ok: true, 
            error_msg: String::new(),
        });
    }

    timeline_results
}