// stock-app/ui/backend/src/commands/analysis_engine.rs

use crate::commands::memory_pool;
use crate::database::analysis::{AnalysisMetadataRow, ValuationResultRow};

use analysis::on_fly::dcf::{DcfInputMetrics, calculate_dcf_on_fly};
use analysis::on_fly::ddm::{DdmInputMetrics, calculate_ddm_on_fly};
use analysis::on_fly::rim::{RimInputMetrics, calculate_rim_on_fly};
use analysis::on_fly::epv::{EpvInputMetrics, calculate_epv_on_fly};
use analysis::on_fly::bgvm::{GrahamInputMetrics, calculate_graham_on_fly};
use analysis::on_fly::eva::{EvaInputMetrics, calculate_eva_on_fly};
use analysis::on_fly::monte_carlo::{MonteCarloInputMetrics, calculate_monte_carlo_on_fly};
use analysis::on_fly::dcf_mc::{DcfMcConfigInput, DcfMcHistoricalRow, run_stochastic_dcf};

pub fn compute_on_fly_valuation(ticker: &str, tab_key: &str) {
    let (metadata_key, result_slot_key) = match tab_key {
        "DCF" => ("dcf_metadata", "dcf_calculated_results"),
        "DDM" => ("ddm_metadata", "ddm_calculated_results"),
        "EPV" => ("epv_metadata", "epv_calculated_results"),
        "BGVM" => ("bgvm_metadata", "bgvm_calculated_results"),
        "EVA" => ("eva_metadata", "eva_calculated_results"),
        "MONTE_CARLO" => ("monte_carlo_metadata", "monte_carlo_summary_results"),
        "DCF_MONTE_CARLO" => ("dcfmc_metadata", "dcfmc_calculated_results"),
        _ => ("rem_metadata", "rem_calculated_results"),
    };

    if tab_key == "MONTE_CARLO" {
        run_monte_carlo_pipeline(ticker, metadata_key, result_slot_key);
        return;
    }

    if tab_key == "DCF_MONTE_CARLO" {
        run_dcf_monte_carlo_standalone_pipeline(ticker, metadata_key, result_slot_key);
        return;
    }

    let mut inputs: Vec<AnalysisMetadataRow> = Vec::new();
    memory_pool::with_active_table::<Vec<AnalysisMetadataRow>, _, _>(metadata_key, |table| {
        inputs = table.clone();
    });

    let mut final_results = Vec::with_capacity(inputs.len());
    let mut lookup_key = String::with_capacity(64);

    for row in inputs {
        let year = row.year;

        // Extract localized macro assumption baseline rates directly out of metadata rows
        let mut rf = row.dynamic_rf.to_string();
        let mut rm = row.dynamic_rm.to_string();
        let mut g  = row.sustainable_g.to_string();
        let mut gn = row.terminal_gn.to_string();

        let mut check_override = |suffix: &str, current_val: &mut String| {
            lookup_key.clear();
            use std::fmt::Write;
            let _ = write!(lookup_key, "{}_{}_{}", metadata_key, year, suffix);
            
            memory_pool::with_active_table::<Vec<String>, _, _>(&lookup_key, |t| {
                if !t.is_empty() { *current_val = t[0].clone(); }
            });
        };

        check_override("rf", &mut rf);
        check_override("rm", &mut rm);
        check_override("g",  &mut g);
        check_override("gn", &mut gn);

        // Clean delegation down to dedicated calculation helpers
        let result_row = match tab_key {
            "DCF"  => execute_dcf(&row, &rf, &rm, &g, &gn),
            "DDM"  => execute_ddm(&row, &rf, &rm, &g),
            "EPV"  => execute_epv(&row, &rf, &rm),
            "BGVM" => execute_graham(&row, &rf, &g),
            "EVA"  => execute_eva(&row, &rf, &rm),
            _      => execute_rim(&row, &rf, &rm, &g),
        };

        final_results.push(result_row);
    }

    memory_pool::store_parsed_table(result_slot_key, final_results);
}

// =========================================================================
// ISOLATED SUB-MODULE EXTRACTED ROUTINES
// =========================================================================

fn execute_dcf(r: &AnalysisMetadataRow, rf: &str, rm: &str, g: &str, gn: &str) -> ValuationResultRow {
    let output = calculate_dcf_on_fly(&DcfInputMetrics {
        operating_cash_flow: r.operating_cash_flow,
        capex_outflow: r.capex_outflow,
        total_debt: r.total_debt,
        total_equity: r.total_equity,
        outstanding_shares: r.outstanding_shares,
        profit_before_tax: r.profit_before_tax,
        net_profit_after_tax: r.net_profit_after_tax,
        finance_interest_expense: r.finance_interest_expense,
        nse_beta: r.nse_beta,
        bse_beta: r.bse_beta,
    }, rf, rm, g, gn);

    ValuationResultRow {
        year: r.year,
        calculated_tax_rate: output.calculated_tax_rate,
        calculated_kd: output.calculated_kd,
        calculated_ke: output.calculated_ke,
        calculated_wacc: output.calculated_wacc,
        intrinsic_value: output.intrinsic_value,
        status_ok: output.status_ok,
        error_msg: output.error_msg,
    }
}

fn execute_ddm(r: &AnalysisMetadataRow, rf: &str, rm: &str, g: &str) -> ValuationResultRow {
    let output = calculate_ddm_on_fly(&DdmInputMetrics {
        dividend_paid: r.dividend_paid,
        outstanding_shares: r.outstanding_shares,
        nse_beta: r.nse_beta,
        bse_beta: r.bse_beta,
    }, rf, rm, g);

    ValuationResultRow {
        year: r.year,
        calculated_ke: output.calculated_ke,
        intrinsic_value: output.intrinsic_value,
        status_ok: output.status_ok,
        error_msg: output.error_msg,
        calculated_tax_rate: 0.0, calculated_kd: 0.0, calculated_wacc: 0.0,
    }
}

fn execute_epv(r: &AnalysisMetadataRow, rf: &str, rm: &str) -> ValuationResultRow {
    let output = calculate_epv_on_fly(&EpvInputMetrics {
        outstanding_shares: r.outstanding_shares,
        profit_before_tax: r.profit_before_tax,
        net_profit_after_tax: r.net_profit_after_tax,
        total_debt: r.total_debt,
        total_equity: r.total_equity,
        finance_interest_expense: r.finance_interest_expense,
        nse_beta: r.nse_beta,
        bse_beta: r.bse_beta,
    }, rf, rm);

    ValuationResultRow {
        year: r.year,
        calculated_tax_rate: output.calculated_tax_rate,
        calculated_kd: output.calculated_kd,
        calculated_ke: output.calculated_ke,
        calculated_wacc: output.calculated_wacc,
        intrinsic_value: output.intrinsic_value,
        status_ok: output.status_ok,
        error_msg: output.error_msg,
    }
}

fn execute_graham(r: &AnalysisMetadataRow, rf: &str, g: &str) -> ValuationResultRow {
    let output = calculate_graham_on_fly(&GrahamInputMetrics {
        outstanding_shares: r.outstanding_shares,
        net_profit_after_tax: r.net_profit_after_tax,
        total_equity: r.total_equity,
        nse_beta: r.nse_beta,
        bse_beta: r.bse_beta,
    }, rf, g);

    ValuationResultRow {
        year: r.year,
        intrinsic_value: output.intrinsic_value, 
        status_ok: output.status_ok,
        error_msg: output.error_msg,
        calculated_tax_rate: 0.0, calculated_kd: 0.0, calculated_ke: 0.0, calculated_wacc: 0.0, 
    }
}

fn execute_eva(r: &AnalysisMetadataRow, rf: &str, rm: &str) -> ValuationResultRow {
    let output = calculate_eva_on_fly(&EvaInputMetrics {
        profit_before_tax: r.profit_before_tax,
        net_profit_after_tax: r.net_profit_after_tax,
        total_equity: r.total_equity,
        total_debt: r.total_debt,
        finance_interest_expense: r.finance_interest_expense,
        outstanding_shares: r.outstanding_shares,
        nse_beta: r.nse_beta,
        bse_beta: r.bse_beta,
    }, rf, rm);

    ValuationResultRow {
        year: r.year,
        intrinsic_value: output.eva_per_share, 
        calculated_wacc: output.calculated_wacc,
        calculated_tax_rate: output.calculated_tax_rate,
        status_ok: output.status_ok,
        error_msg: output.error_msg,
        calculated_kd: 0.0, calculated_ke: 0.0,
    }
}

fn execute_rim(r: &AnalysisMetadataRow, rf: &str, rm: &str, g: &str) -> ValuationResultRow {
    let output = calculate_rim_on_fly(&RimInputMetrics {
        total_equity: r.total_equity,
        net_profit_after_tax: r.net_profit_after_tax,
        outstanding_shares: r.outstanding_shares,
        nse_beta: r.nse_beta,
        bse_beta: r.bse_beta,
    }, rf, rm, g);

    ValuationResultRow {
        year: r.year,
        calculated_ke: output.calculated_ke,
        intrinsic_value: output.intrinsic_value,
        status_ok: output.status_ok,
        error_msg: output.error_msg,
        calculated_tax_rate: 0.0, calculated_kd: 0.0, calculated_wacc: 0.0,
    }
}

fn run_monte_carlo_pipeline(ticker: &str, metadata_key: &str, result_slot_key: &str) {
    let mut chart_rows: Vec<crate::database::analysis::HistoricalChartRow> = Vec::new();
    memory_pool::with_active_table::<Vec<crate::database::analysis::HistoricalChartRow>, _, _>("historical_chart_data", |table| {
        chart_rows = table.clone();
    });

    let mut mc_days = String::new();
    let mut mc_sims = String::new();
    let mut mc_conf = String::new();
    let mut mc_date = String::new();
    let mut mc_lookback = String::new();

    memory_pool::with_active_table::<Vec<String>, _, _>(&format!("{}_mc_days", metadata_key), |t| if !t.is_empty() { mc_days = t[0].clone(); });
    memory_pool::with_active_table::<Vec<String>, _, _>(&format!("{}_mc_sims", metadata_key), |t| if !t.is_empty() { mc_sims = t[0].clone(); });
    memory_pool::with_active_table::<Vec<String>, _, _>(&format!("{}_mc_conf", metadata_key), |t| if !t.is_empty() { mc_conf = t[0].clone(); });
    memory_pool::with_active_table::<Vec<String>, _, _>(&format!("{}_mc_date", metadata_key), |t| if !t.is_empty() { mc_date = t[0].clone(); });
    memory_pool::with_active_table::<Vec<String>, _, _>(&format!("{}_mc_lookback", metadata_key), |t| if !t.is_empty() { mc_lookback = t[0].clone(); });

    let days_parsed = match mc_days.parse::<usize>() { Ok(val) => val, Err(_) => return, };
    let sims_parsed = match mc_sims.parse::<usize>() { Ok(val) => val, Err(_) => return, };
    let conf_parsed = match mc_conf.parse::<f64>() { Ok(val) => val, Err(_) => return, };
    let lookback_parsed = match mc_lookback.parse::<usize>() { Ok(val) => val, Err(_) => return, };

    let cutoff_index = match chart_rows.iter().position(|row| row.date == mc_date) {
        Some(idx) => idx,
        None => return,
    };

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
        ticker: ticker.to_string(),
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
}

pub fn run_dcf_monte_carlo_standalone_pipeline(_ticker: &str, metadata_key: &str, result_slot_key: &str) {
    let mut inputs: Vec<AnalysisMetadataRow> = Vec::new();
    memory_pool::with_active_table::<Vec<AnalysisMetadataRow>, _, _>(metadata_key, |table| {
        inputs = table.clone();
    });

    if inputs.is_empty() { return; }
    inputs.sort_by_key(|r| r.year);

    let latest_year = match inputs.last() {
        Some(row) => row.year,
        None => return,
    };

    let mut mc_sims = String::new();
    memory_pool::with_active_table::<Vec<String>, _, _>(
        &format!("{}_{}_sims", metadata_key, latest_year), 
        |t| if !t.is_empty() { mc_sims = t[0].clone(); }
    );

    let total_simulations = match mc_sims.parse::<usize>() {
        Ok(val) => val,
        Err(_) => return,
    };

    let mut history = Vec::with_capacity(inputs.len());
    let mut lookup_key = String::with_capacity(64);

    for row in inputs {
        let year = row.year;

        let mut rf = row.dynamic_rf.to_string();
        let mut rm = row.dynamic_rm.to_string();
        let mut gn = row.terminal_gn.to_string();

        let mut check_override = |suffix: &str, current_val: &mut String| {
            lookup_key.clear();
            use std::fmt::Write;
            let _ = write!(lookup_key, "{}_{}_{}", metadata_key, year, suffix);
            
            memory_pool::with_active_table::<Vec<String>, _, _>(&lookup_key, |t| {
                if !t.is_empty() { *current_val = t[0].clone(); }
            });
        };

        check_override("rf", &mut rf);
        check_override("rm", &mut rm);
        check_override("gn", &mut gn);

        let parsed_rf = match rf.parse::<f64>() { Ok(v) => v / 100.0, Err(_) => return, };
        let parsed_rm = match rm.parse::<f64>() { Ok(v) => v / 100.0, Err(_) => return, };
        let parsed_gn = match gn.parse::<f64>() { Ok(v) => v / 100.0, Err(_) => return, };

        history.push(DcfMcHistoricalRow {
            year,
            operating_cash_flow: row.operating_cash_flow,
            capex_outflow: row.capex_outflow,
            total_debt: row.total_debt,
            total_equity: row.total_equity,
            outstanding_shares: row.outstanding_shares,
            profit_before_tax: row.profit_before_tax,
            net_profit_after_tax: row.net_profit_after_tax,
            finance_interest_expense: row.finance_interest_expense,
            beta: row.average_beta,
            risk_free_rate: parsed_rf,
            market_return: parsed_rm,
            terminal_gn: parsed_gn,
        });
    }

    let config = DcfMcConfigInput { total_simulations };
    
    // 1. Run simulation engine 
    let summary = run_stochastic_dcf(&history, &config);

    // 2. Map structures cleanly over type paths from calculation space into memory pool definitions
    let database_ready_collection: Vec<crate::database::analysis::DcfMcPercentileSummary> = summary
        .into_iter()
        .map(|s| crate::database::analysis::DcfMcPercentileSummary {
            year: s.year,
            p100: s.p100,
            p97_5: s.p97_5,
            p95: s.p95,
            p90: s.p90,
            p75: s.p75,
            p50: s.p50,
            p25: s.p25,
            p10: s.p10,
            p5: s.p5,
            p2_5: s.p2_5,
            p0: s.p0,
            average: s.average,
            mean_g: s.mean_g,
            vol_g: s.vol_g,
            status_ok: s.status_ok,
            error_msg: s.error_msg,
        })
        .collect();

    // 3. Store data under the pristine ticker token key used by the frontend chart canvas lookups
    memory_pool::store_parsed_table(result_slot_key, database_ready_collection);
}