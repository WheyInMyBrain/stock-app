// stock-app/ui/backend/src/commands/analysis_engine.rs

use crate::commands::memory_pool;
use crate::database::analysis::{AnalysisMetadataRow, ValuationResultRow};

// Import the stateless calculators from your dedicated library crate
use analysis::on_fly::dcf::{DcfInputMetrics, calculate_dcf_on_fly};
use analysis::on_fly::ddm::{DdmInputMetrics, calculate_ddm_on_fly};
use analysis::on_fly::rim::{RimInputMetrics, calculate_rim_on_fly};

/// Core non-blocking engine processing pipeline. Takes separate metadata slots,
/// routes them down to on-fly calculation scripts, and flushes output slots.
pub fn compute_on_fly_valuation(_ticker: &str, tab_key: &str) {
    // 1. Resolve distinct metadata and result slot tracking strings based on active view tabs
    let metadata_key = match tab_key {
        "DCF" => "dcf_metadata",
        "DDM" => "ddm_metadata",
        _ => "rem_metadata",
    };

    let result_slot_key = match tab_key {
        "DCF" => "dcf_calculated_results",
        "DDM" => "ddm_calculated_results",
        _ => "rem_calculated_results",
    };

    // 2. Safely pull down raw frontend user snapshot arrays out of the memory pool table
    let mut inputs: Vec<AnalysisMetadataRow> = Vec::new();
    memory_pool::with_active_table::<Vec<AnalysisMetadataRow>, _, _>(metadata_key, |table| {
        inputs = table.clone();
    });

    let mut final_results = Vec::with_capacity(inputs.len());

    // 3. Process each year step independently using localized timeline configurations
    for row in inputs {
        let year = row.year;

        // Establish default fallback string matrices for macro assumptions
        let mut rf = row.dynamic_rf.to_string();
        let mut rm = row.dynamic_rm.to_string();
        let mut g = match tab_key {
            "DCF" => row.dcf_g.to_string(),
            "DDM" => row.ddm_g.to_string(),
            _ => row.rem_g.to_string(),
        };
        let mut gn = row.dcf_gn.to_string();

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
                    // Use zero defaults for non-DDM parameters
                    calculated_tax_rate: 0.0,
                    calculated_kd: 0.0,
                    calculated_wacc: 0.0,
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
                    // Use zero defaults for non-RIM parameters
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