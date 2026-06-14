// stock-app/ui/backend/src/commands/analysis_engine.rs

use crate::commands::memory_pool;
use crate::database::analysis::{AnalysisMetadataRow, ValuationResultRow};

// Import the stateless calculators from your dedicated library crate
use analysis::on_fly::dcf::{DcfInputMetrics, calculate_dcf_on_fly};
use analysis::on_fly::ddm::{DdmInputMetrics, calculate_ddm_on_fly};
use analysis::on_fly::rim::{RimInputMetrics, calculate_rim_on_fly};
use analysis::on_fly::epv::{EpvInputMetrics, calculate_epv_on_fly};
use analysis::on_fly::bgvm::{GrahamInputMetrics, calculate_graham_on_fly};
use analysis::on_fly::eva::{EvaInputMetrics, calculate_eva_on_fly};
use analysis::on_fly::monte_carlo::{MonteCarloInputMetrics, calculate_monte_carlo_on_fly};

/// Core non-blocking engine processing pipeline. Takes separate metadata slots,
/// routes them down to on-fly calculation scripts, and flushes output slots.
pub fn compute_on_fly_valuation(_ticker: &str, tab_key: &str) {
    // 1. Resolve distinct metadata and result slot tracking strings based on active view tabs
    let metadata_key = match tab_key {
        "DCF" => "dcf_metadata",
        "DDM" => "ddm_metadata",
        "EPV" => "epv_metadata",
        "BGVM" => "bgvm_metadata",
        "EVA" => "eva_metadata",
        "MONTE_CARLO" => "monte_carlo_metadata",
        _ => "rem_metadata",
    };

    let result_slot_key = match tab_key {
        "DCF" => "dcf_calculated_results",
        "DDM" => "ddm_calculated_results",
        "EPV" => "epv_calculated_results",
        "BGVM" => "bgvm_calculated_results",
        "EVA" => "eva_calculated_results",
        "MONTE_CARLO" => "monte_carlo_summary_results",
        _ => "rem_calculated_results",
    };

    // 2. Safely pull down raw frontend user snapshot arrays out of the memory pool table
    let mut inputs: Vec<AnalysisMetadataRow> = Vec::new();
    memory_pool::with_active_table::<Vec<AnalysisMetadataRow>, _, _>(metadata_key, |table| {
        inputs = table.clone();
    });

    // =========================================================================
    // SPECIALIZED MONTE CARLO PROBABILISTIC INTERCEPTOR
    // =========================================================================
    if tab_key == "MONTE_CARLO" {
        let mut chart_rows: Vec<crate::database::analysis::HistoricalChartRow> = Vec::new();
        memory_pool::with_active_table::<Vec<crate::database::analysis::HistoricalChartRow>, _, _>("historical_chart_data", |table| {
            chart_rows = table.clone();
        });

        let mut mc_days = String::new();
        let mut mc_sims = String::new();
        let mut mc_conf = String::new();
        let mut mc_date = String::new();
        let mut mc_lookback = String::new();

        memory_pool::with_active_table::<Vec<String>, _, _>(&format!("{}_mc_days", metadata_key), |t| {
            if !t.is_empty() { mc_days = t[0].clone(); }
        });
        memory_pool::with_active_table::<Vec<String>, _, _>(&format!("{}_mc_sims", metadata_key), |t| {
            if !t.is_empty() { mc_sims = t[0].clone(); }
        });
        memory_pool::with_active_table::<Vec<String>, _, _>(&format!("{}_mc_conf", metadata_key), |t| {
            if !t.is_empty() { mc_conf = t[0].clone(); }
        });
        memory_pool::with_active_table::<Vec<String>, _, _>(&format!("{}_mc_date", metadata_key), |t| {
            if !t.is_empty() { mc_date = t[0].clone(); }
        });
        memory_pool::with_active_table::<Vec<String>, _, _>(&format!("{}_mc_lookback", metadata_key), |t| {
            if !t.is_empty() { mc_lookback = t[0].clone(); }
        });

        let days_parsed = match mc_days.parse::<usize>() { Ok(val) => val, Err(_) => return, };
        let sims_parsed = match mc_sims.parse::<usize>() { Ok(val) => val, Err(_) => return, };
        let conf_parsed = match mc_conf.parse::<f64>() { Ok(val) => val, Err(_) => return, };
        let lookback_parsed = match mc_lookback.parse::<usize>() { Ok(val) => val, Err(_) => return, };

        let cutoff_index = match chart_rows.iter().position(|row| row.date == mc_date) {
            Some(idx) => idx,
            None => return,
        };

        // Extract complete price data sequence up to anchor cutoff index
        let close_prices: Vec<f64> = chart_rows[0..=cutoff_index]
            .iter()
            .filter_map(|row| row.nse_close.or(row.bse_close))
            .collect();

        if close_prices.is_empty() { return; }

        let mc_inputs = MonteCarloInputMetrics {
            forecast_days: days_parsed,
            num_simulations: sims_parsed,
            confidence_level: conf_parsed,
            visual_paths_to_return: 10, 
            historical_lookback: lookback_parsed,
        };

        let output = calculate_monte_carlo_on_fly(&close_prices, &mc_inputs);

        let summary = vec![crate::database::analysis::MonteCarloResultSummary {
            ticker: _ticker.to_string(),
            expected_value: output.expected_value,
            upper_bound: output.upper_bound,
            lower_bound: output.lower_bound,
            forecast_horizon: mc_inputs.forecast_days as u32,
            total_simulations: mc_inputs.num_simulations as u32,
            status_ok: output.status_ok,
            error_msg: output.error_msg,
        }];

        let mut path_points = Vec::new();
        for (path_idx, path) in output.visual_paths.iter().enumerate() {
            for (step_idx, price) in path.iter().enumerate() {
                let step_label = if step_idx == 0 {
                    mc_date.clone()
                } else {
                    format!("{} T+{:03}", mc_date, step_idx)
                };

                path_points.push(crate::database::analysis::MonteCarloPathPoint {
                    path_index: path_idx as u32,
                    step_date: step_label,
                    simulated_price: *price,
                });
            }
        }

        memory_pool::store_parsed_table(result_slot_key, summary);
        memory_pool::store_parsed_table("monte_carlo_path_results", path_points);
        return; 
    }

    let mut final_results = Vec::with_capacity(inputs.len());

    // 3. Process each year step independently using localized timeline configurations
    for row in inputs {
        let year = row.year;

        // Establish default fallback string matrices for macro assumptions
        let mut rf = row.dynamic_rf.to_string();
        let mut rm = row.dynamic_rm.to_string();
        let mut g  = row.sustainable_g.to_string();
        let mut gn = row.terminal_gn.to_string();

        // Now override ONLY if the user has provided a custom override in the memory pool
        memory_pool::with_active_table::<Vec<String>, _, _>(&format!("{}_{}_rf", metadata_key, year), |t| {
            if !t.is_empty() { rf = t[0].clone(); }
        });
        memory_pool::with_active_table::<Vec<String>, _, _>(&format!("{}_{}_rm", metadata_key, year), |t| {
            if !t.is_empty() { rm = t[0].clone(); }
        });
        memory_pool::with_active_table::<Vec<String>, _, _>(&format!("{}_{}_g", metadata_key, year), |t| {
            if !t.is_empty() { g = t[0].clone(); }
        });
        memory_pool::with_active_table::<Vec<String>, _, _>(&format!("{}_{}_gn", metadata_key, year), |t| {
            if !t.is_empty() { gn = t[0].clone(); }
        });

        // 5. Direct analytical routing down to sub-module calculators
        match tab_key {
            "DCF" => {
                // Map to the library's standalone DCF input structure
                let dcf_inputs = DcfInputMetrics {
                    operating_cash_flow: row.operating_cash_flow,
                    capex_outflow: row.capex_outflow,
                    total_debt: row.total_debt,
                    total_equity: row.total_equity,
                    outstanding_shares: row.outstanding_shares,
                    profit_before_tax: row.profit_before_tax,
                    net_profit_after_tax: row.net_profit_after_tax,
                    finance_interest_expense: row.finance_interest_expense,
                    nse_beta: row.nse_beta,
                    bse_beta: row.bse_beta,
                };

                let output = calculate_dcf_on_fly(&dcf_inputs, &rf, &rm, &g, &gn);
                
                final_results.push(ValuationResultRow {
                    year,
                    calculated_tax_rate: output.calculated_tax_rate,
                    calculated_kd: output.calculated_kd,
                    calculated_ke: output.calculated_ke,
                    calculated_wacc: output.calculated_wacc,
                    intrinsic_value: output.intrinsic_value,
                    status_ok: output.status_ok,
                    error_msg: output.error_msg,
                });
            }
            "DDM" => {
                // Map to the library's standalone DDM input structure
                let ddm_inputs = DdmInputMetrics {
                    dividend_paid: row.dividend_paid,
                    outstanding_shares: row.outstanding_shares,
                    nse_beta: row.nse_beta,
                    bse_beta: row.bse_beta,
                };

                let output = calculate_ddm_on_fly(&ddm_inputs, &rf, &rm, &g);

                final_results.push(ValuationResultRow {
                    year,
                    calculated_ke: output.calculated_ke,
                    intrinsic_value: output.intrinsic_value,
                    status_ok: output.status_ok,
                    error_msg: output.error_msg,
                    calculated_tax_rate: 0.0,
                    calculated_kd: 0.0,
                    calculated_wacc: 0.0,
                });
            }
            "EPV" => {
                // Map inputs to the standalone library EPV core
                let epv_inputs = EpvInputMetrics {
                    outstanding_shares: row.outstanding_shares,
                    profit_before_tax: row.profit_before_tax,
                    net_profit_after_tax: row.net_profit_after_tax,
                    total_debt: row.total_debt,
                    total_equity: row.total_equity,
                    finance_interest_expense: row.finance_interest_expense,
                    nse_beta: row.nse_beta,
                    bse_beta: row.bse_beta,
                };

                let output = calculate_epv_on_fly(&epv_inputs, &rf, &rm);

                final_results.push(ValuationResultRow {
                    year,
                    calculated_tax_rate: output.calculated_tax_rate,
                    calculated_kd: output.calculated_kd,
                    calculated_ke: output.calculated_ke,
                    calculated_wacc: output.calculated_wacc,
                    intrinsic_value: output.intrinsic_value,
                    status_ok: output.status_ok,
                    error_msg: output.error_msg,
                });
            }
            "BGVM" => {
                let graham_inputs = GrahamInputMetrics {
                    outstanding_shares: row.outstanding_shares,
                    net_profit_after_tax: row.net_profit_after_tax,
                    total_equity: row.total_equity,
                    nse_beta: row.nse_beta,
                    bse_beta: row.bse_beta,
                };

                let output = calculate_graham_on_fly(&graham_inputs, &rf, &g);

                final_results.push(ValuationResultRow {
                    year,
                    intrinsic_value: output.intrinsic_value, 
                    status_ok: output.status_ok,
                    error_msg: output.error_msg,
                    calculated_tax_rate: 0.0,
                    calculated_kd: 0.0,
                    calculated_ke: 0.0,
                    calculated_wacc: 0.0, 
                });
            }
            "EVA" => {
                let eva_inputs = EvaInputMetrics {
                    profit_before_tax: row.profit_before_tax,
                    net_profit_after_tax: row.net_profit_after_tax,
                    total_equity: row.total_equity,
                    total_debt: row.total_debt,
                    finance_interest_expense: row.finance_interest_expense,
                    outstanding_shares: row.outstanding_shares,
                    nse_beta: row.nse_beta,
                    bse_beta: row.bse_beta,
                };

                let output = calculate_eva_on_fly(&eva_inputs, &rf, &rm);

                final_results.push(ValuationResultRow {
                    year,
                    intrinsic_value: output.eva_per_share, 
                    calculated_wacc: output.calculated_wacc,
                    calculated_tax_rate: output.calculated_tax_rate,
                    status_ok: output.status_ok,
                    error_msg: output.error_msg,
                    calculated_kd: 0.0,
                    calculated_ke: 0.0,
                });
            }
            _ => {
                // Map to the library's standalone RIM input structure
                let rim_inputs = RimInputMetrics {
                    total_equity: row.total_equity,
                    net_profit_after_tax: row.net_profit_after_tax,
                    outstanding_shares: row.outstanding_shares,
                    nse_beta: row.nse_beta,
                    bse_beta: row.bse_beta,
                };

                let output = calculate_rim_on_fly(&rim_inputs, &rf, &rm, &g);

                final_results.push(ValuationResultRow {
                    year,
                    calculated_ke: output.calculated_ke,
                    intrinsic_value: output.intrinsic_value,
                    status_ok: output.status_ok,
                    error_msg: output.error_msg,
                    calculated_tax_rate: 0.0,
                    calculated_kd: 0.0,
                    calculated_wacc: 0.0,
                });
            }
        }
    }

    // 6. Flush the finalized compiled results directly back into the memory pool table slot 
    // for the presentation user interface layers to query and map onto the screen grids instantly!
    memory_pool::store_parsed_table(result_slot_key, final_results);
}