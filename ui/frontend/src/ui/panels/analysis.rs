use egui::{Ui, Color32};
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use crate::core::data_manager::DataManager;
use crate::ui::layouts::canvas::{AbstractSubTab, draw_nav_canvas_orchestrator, paint_abstract_chart_canvas, GenericChartLine, GenericChartPoint};
use backend::database::analysis::AnalysisMetadataRow;

// =========================================================================
// THREAD-SAFE CELLS CACHE CONTROL LAYER
// =========================================================================
#[derive(Clone)]
struct DynamicCellCache {
    inputs: HashMap<(i32, String), String>,
}

impl Default for DynamicCellCache {
    fn default() -> Self {
        Self { inputs: HashMap::new() }
    }
}

std::thread_local! {
    static ACTIVE_PANEL_TICKER: RefCell<String> = RefCell::new(String::new());
    static INTERACTIVE_CELL_CACHE: RefCell<DynamicCellCache> = RefCell::new(DynamicCellCache::default());
}

fn access_cell_state(year: i32, metric: &str, fallback_val: String) -> String {
    INTERACTIVE_CELL_CACHE.with(|cache| {
        let mut cache_ref = cache.borrow_mut();
        cache_ref.inputs.entry((year, metric.to_string())).or_insert(fallback_val).clone()
    })
}

fn update_cell_state(year: i32, metric: &str, current_text: String) {
    INTERACTIVE_CELL_CACHE.with(|cache| {
        let mut cache_ref = cache.borrow_mut();
        cache_ref.inputs.insert((year, metric.to_string()), current_text);
    });
}

fn check_column_filled(year: i32, metrics: &[&str]) -> bool {
    INTERACTIVE_CELL_CACHE.with(|cache| {
        let cache_ref = cache.borrow();
        for &metric in metrics {
            if let Some(val) = cache_ref.inputs.get(&(year, metric.to_string())) {
                let trimmed = val.trim();
                if trimmed.is_empty() || trimmed == "0" || trimmed == "0.0" { return false; }
            } else { return false; }
        }
        true
    })
}

// =========================================================================
// BACKEND COORDINATION INGESTION WRITER
// =========================================================================
pub fn push_interactive_state_to_pool(years: &[i32]) {
    let mut master_rows = Vec::with_capacity(years.len());

    for &year in years {
        let basic_eps = access_cell_state(year, "eps", "0".to_string()).parse::<f64>().unwrap_or(0.0);
        let net_profit_after_tax = access_cell_state(year, "pat", "0".to_string()).parse::<i64>().unwrap_or(0);
        let dividend_paid = access_cell_state(year, "div", "0".to_string()).parse::<i64>().unwrap_or(0);
        let total_equity = access_cell_state(year, "eq", "0".to_string()).parse::<i64>().unwrap_or(0);
        let total_debt = access_cell_state(year, "debt", "0".to_string()).parse::<i64>().unwrap_or(0);    
        let operating_cash_flow = access_cell_state(year, "ocf", "0".to_string()).parse::<i64>().unwrap_or(0);
        let capex_outflow = access_cell_state(year, "capex_out", "0".to_string()).parse::<i64>().unwrap_or(0);
        let capex_inflow = access_cell_state(year, "capex_in", "0".to_string()).parse::<i64>().unwrap_or(0);
        let outstanding_shares = access_cell_state(year, "shares", "0".to_string()).parse::<i64>().unwrap_or(0);
        let profit_before_tax = access_cell_state(year, "pbt", "0".to_string()).parse::<i64>().unwrap_or(0);
        let finance_interest_expense = access_cell_state(year, "interest", "0".to_string()).parse::<i64>().unwrap_or(0);
        let effective_tax_rate = access_cell_state(year, "tax_rate", "0.25".to_string()).parse::<f64>().unwrap_or(0.25);
        let user_beta = access_cell_state(year, "beta", "1.0".to_string()).parse::<f64>().unwrap_or(1.0);
        
        let net_capex = capex_outflow + capex_inflow;
        let free_cash_flow = operating_cash_flow + net_capex;

        master_rows.push(AnalysisMetadataRow {
            year,
            dividend_paid,
            basic_eps,
            net_profit_after_tax,
            total_equity,
            total_debt,
            operating_cash_flow,
            capex_outflow,
            capex_inflow,
            net_capex,
            free_cash_flow,
            outstanding_shares,
            profit_before_tax,
            finance_interest_expense,
            effective_tax_rate,
            nse_beta: user_beta,
            bse_beta: user_beta,
        });
    }

    backend::commands::memory_pool::store_parsed_table("analysis_metadata", master_rows);
}

fn get_valuation_maps(tab_metrics: &[&str]) -> (Vec<i32>, HashMap<i32, AnalysisMetadataRow>) {
    let active_ticker = ACTIVE_PANEL_TICKER.with(|ticker| ticker.borrow().clone());
    if active_ticker.is_empty() { return (Vec::new(), HashMap::new()); }

    let mut analysis_rows: Vec<AnalysisMetadataRow> = Vec::new();
    backend::commands::memory_pool::with_active_table::<Vec<AnalysisMetadataRow>, _, _>("analysis_metadata", |table| {
        analysis_rows = table.clone();
    });

    let mut analysis_map = HashMap::new();
    let mut unique_years = BTreeSet::new();
    for row in analysis_rows {
        unique_years.insert(row.year);
        analysis_map.insert(row.year, row);
    }

    let mut years_vector: Vec<i32> = unique_years.into_iter().collect();
    if !years_vector.is_empty() {
        let mut current_min = years_vector[0];
        while check_column_filled(current_min - 1, tab_metrics) { current_min -= 1; }
        let absolute_start = current_min - 1;

        let mut current_max = years_vector[years_vector.len() - 1];
        while check_column_filled(current_max + 1, tab_metrics) { current_max += 1; }
        let absolute_end = current_max + 1;

        years_vector = (absolute_start..=absolute_end).collect();
    }
    (years_vector, analysis_map)
}

fn render_workspace_chart(ui: &mut Ui) {
    let mut entries: Vec<backend::database::analysis::HistoricalChartRow> = Vec::new();
    backend::commands::memory_pool::with_active_table::<Vec<backend::database::analysis::HistoricalChartRow>, _, _>("historical_chart_data", |table| {
        entries = table.clone();
    });

    let mut nse_points = Vec::with_capacity(entries.len());
    let mut bse_points = Vec::with_capacity(entries.len());

    for row in entries {
        if let Some(val) = row.nse_close { nse_points.push(GenericChartPoint { date: row.date.clone(), value: val }); }
        if let Some(val) = row.bse_close { bse_points.push(GenericChartPoint { date: row.date, value: val }); }
    }

    let chart_lines = vec![
        GenericChartLine { label: "NSE", color: Color32::from_rgb(250, 210, 50), stroke_width: 1.5, points: nse_points },
        GenericChartLine { label: "BSE", color: Color32::from_rgb(50, 150, 250), stroke_width: 1.5, points: bse_points },
    ];
    paint_abstract_chart_canvas(ui, &chart_lines);
}

// =========================================================================
// REUSABLE TABULAR COMPONENTS
// =========================================================================
fn render_horizontal_grid_header(ui: &mut Ui, years: &[i32], title: &str) {
    ui.label(egui::RichText::new(title).strong().color(Color32::from_rgb(150, 150, 150)));
    for year in years { ui.label(egui::RichText::new(format!("{}", year)).strong()); }
    ui.end_row();
}

fn render_editable_row(
    ui: &mut Ui,
    years: &[i32],
    label: &str,
    metric_id: &str,
    extract_fallback: impl Fn(&AnalysisMetadataRow) -> String,
    analysis_map: &HashMap<i32, AnalysisMetadataRow>,
) {
    ui.label(label);
    for yr in years {
        let fallback = analysis_map.get(yr).map(&extract_fallback).unwrap_or_else(|| "0".to_string());
        let mut value_buffer = access_cell_state(*yr, metric_id, fallback);
        if ui.add(egui::TextEdit::singleline(&mut value_buffer).desired_width(80.0)).changed() {
            update_cell_state(*yr, metric_id, value_buffer);
            push_interactive_state_to_pool(years);
        }
    }
    ui.end_row();
}

// =========================================================================
// DISCOUNTED CASH FLOW SUBTAB - WITH TWO-AXIS SCROLLING
// =========================================================================
struct DcfTab;
impl AbstractSubTab<Vec<AnalysisMetadataRow>> for DcfTab {
    fn id(&self) -> usize { 0 }
    fn label(&self) -> &'static str { "Discounted Cash Flow (DCF)" }
    fn render_main(&self, ui: &mut Ui, _data: &Vec<AnalysisMetadataRow>) { render_workspace_chart(ui); }
    
    fn render_bottom(&self, ui: &mut Ui, data: &Vec<AnalysisMetadataRow>) {
        let metrics = vec!["ocf", "capex_out", "debt", "eq", "shares", "pbt", "pat", "interest", "rf", "rm", "g", "gn"];
        let (years, analysis_map) = get_valuation_maps(&metrics);
        if years.is_empty() { return; }

        let calculated_beta = data.first().map(|r| (r.nse_beta + r.bse_beta) / 2.0).unwrap_or(1.0);

        ui.label(egui::RichText::new("Discounted Cash Flow (TABULAR CALCULATOR)").strong().size(14.0));
        ui.add_space(4.0);

        // FEATURE UPGRADE: Enable bidirectional scrolling (.both()) for oversized vertical tables
        egui::ScrollArea::both().id_source("dcf_matrix_scroll_area").show(ui, |ui| {
            egui::Frame::none().stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 45, 45))).show(ui, |ui| {
                egui::Grid::new("dcf_matrix_grid").striped(true).spacing(egui::vec2(12.0, 8.0)).show(ui, |ui| {
                    render_horizontal_grid_header(ui, &years, "METRICS FROM INTEGRATED PARQUETS");
                    
                    render_editable_row(ui, &years, "Operating Cash Flow (OCF)", "ocf", |r| r.operating_cash_flow.to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Capital Expenditure (Capex)", "capex_out", |r| r.capex_outflow.to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Total Debt (Short + Long Term)", "debt", |r| r.total_debt.to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Total Shareholder Equity", "eq", |r| r.total_equity.to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Outstanding Shares Count", "shares", |r| r.outstanding_shares.to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Profit Before Tax (PBT)", "pbt", |r| r.profit_before_tax.to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Net Profit After Tax (PAT)", "pat", |r| r.net_profit_after_tax.to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Finance Interest Expenses", "interest", |r| r.finance_interest_expense.to_string(), &analysis_map);

                    ui.separator(); for _ in &years { ui.separator(); } ui.end_row();
                    render_horizontal_grid_header(ui, &years, "USER FORECAST ASSUMPTIONS");
                    
                    render_editable_row(ui, &years, "Risk Free Rate (Rf)", "rf", |_| "7.0".to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Expected Market Return (Rm)", "rm", |_| "12.0".to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Stage 1 Forecast Growth (g)", "g", |_| "10.0".to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Terminal Perpetuity Growth (gn)", "gn", |_| "4.5".to_string(), &analysis_map);

                    ui.separator(); for _ in &years { ui.separator(); } ui.end_row();
                    render_horizontal_grid_header(ui, &years, "DERIVED DATA ATTRIBUTES ENGINE");

                    ui.label("Calculated Marginal Tax Rate");
                    for yr in &years {
                        let pbt = access_cell_state(*yr, "pbt", "0".to_string()).parse::<f64>().unwrap_or(0.0);
                        let pat = access_cell_state(*yr, "pat", "0".to_string()).parse::<f64>().unwrap_or(0.0);
                        let tax = if pbt > 0.0 && pbt > pat { (pbt - pat) / pbt } else { 0.25 };
                        ui.label(format!("{:.1}%", tax * 100.0));
                    }
                    ui.end_row();

                    ui.label("Calculated Pre-tax Cost of Debt (Kd)");
                    for yr in &years {
                        let debt = access_cell_state(*yr, "debt", "0".to_string()).parse::<f64>().unwrap_or(0.0);
                        let interest = access_cell_state(*yr, "interest", "0".to_string()).parse::<f64>().unwrap_or(0.0);
                        let kd = if debt > 0.0 { interest / debt } else { 0.085 };
                        ui.label(format!("{:.2}%", kd * 100.0));
                    }
                    ui.end_row();

                    ui.label("Calculated Cost of Equity (Ke)");
                    for yr in &years {
                        let rf = access_cell_state(*yr, "rf", "7.0".to_string()).parse::<f64>().unwrap_or(7.0) / 100.0;
                        let rm = access_cell_state(*yr, "rm", "12.0".to_string()).parse::<f64>().unwrap_or(12.0) / 100.0;
                        let ke = rf + calculated_beta * (rm - rf);
                        ui.label(format!("{:.2}%", ke * 100.0));
                    }
                    ui.end_row();

                    ui.label(egui::RichText::new("Calculated WACC").strong().color(Color32::from_rgb(50, 160, 240)));
                    for yr in &years {
                        let debt = access_cell_state(*yr, "debt", "0".to_string()).parse::<f64>().unwrap_or(0.0);
                        let equity = access_cell_state(*yr, "eq", "0".to_string()).parse::<f64>().unwrap_or(0.0);
                        let pbt = access_cell_state(*yr, "pbt", "0".to_string()).parse::<f64>().unwrap_or(0.0);
                        let pat = access_cell_state(*yr, "pat", "0".to_string()).parse::<f64>().unwrap_or(0.0);
                        let interest = access_cell_state(*yr, "interest", "0".to_string()).parse::<f64>().unwrap_or(0.0);
                        
                        let rf = access_cell_state(*yr, "rf", "7.0".to_string()).parse::<f64>().unwrap_or(7.0) / 100.0;
                        let rm = access_cell_state(*yr, "rm", "12.0".to_string()).parse::<f64>().unwrap_or(12.0) / 100.0;
                        
                        let tax = if pbt > 0.0 && pbt > pat { (pbt - pat) / pbt } else { 0.25 };
                        let kd = if debt > 0.0 { interest / debt } else { 0.085 };
                        let ke = rf + calculated_beta * (rm - rf);
                        
                        let total_cap = debt + equity;
                        let wacc = if total_cap > 0.0 {
                            (ke * (equity / total_cap)) + ((kd * (1.0 - tax)) * (debt / total_cap))
                        } else { ke };
                        ui.label(format!("{:.2}%", wacc * 100.0));
                    }
                    ui.end_row();

                    ui.label(egui::RichText::new("DCF INTRINSIC VALUE").strong().color(Color32::from_rgb(50, 220, 120)));
                    for yr in &years {
                        let ocf = access_cell_state(*yr, "ocf", "0".to_string()).parse::<f64>().unwrap_or(0.0);
                        let capex = access_cell_state(*yr, "capex_out", "0".to_string()).parse::<f64>().unwrap_or(0.0);
                        let debt = access_cell_state(*yr, "debt", "0".to_string()).parse::<f64>().unwrap_or(0.0);
                        let equity = access_cell_state(*yr, "eq", "0".to_string()).parse::<f64>().unwrap_or(0.0);
                        let pbt = access_cell_state(*yr, "pbt", "0".to_string()).parse::<f64>().unwrap_or(0.0);
                        let pat = access_cell_state(*yr, "pat", "0".to_string()).parse::<f64>().unwrap_or(0.0);
                        let interest = access_cell_state(*yr, "interest", "0".to_string()).parse::<f64>().unwrap_or(0.0);
                        let shares = access_cell_state(*yr, "shares", "0".to_string()).parse::<f64>().unwrap_or(0.0);
                        
                        let rf = access_cell_state(*yr, "rf", "7.0".to_string()).parse::<f64>().unwrap_or(7.0) / 100.0;
                        let rm = access_cell_state(*yr, "rm", "12.0".to_string()).parse::<f64>().unwrap_or(12.0) / 100.0;
                        let growth = access_cell_state(*yr, "g", "10.0".to_string()).parse::<f64>().unwrap_or(10.0) / 100.0;
                        let term_g = access_cell_state(*yr, "gn", "4.5".to_string()).parse::<f64>().unwrap_or(4.5) / 100.0;

                        let base_fcf = ocf + capex;
                        let tax = if pbt > 0.0 && pbt > pat { (pbt - pat) / pbt } else { 0.25 };
                        let kd = if debt > 0.0 { interest / debt } else { 0.085 };
                        let ke = rf + calculated_beta * (rm - rf);
                        
                        let total_cap = debt + equity;
                        let wacc = if total_cap > 0.0 {
                            (ke * (equity / total_cap)) + ((kd * (1.0 - tax)) * (debt / total_cap))
                        } else { ke };

                        if wacc > term_g && base_fcf > 0.0 && shares > 0.0 {
                            let mut pv_stage_1 = 0.0;
                            let mut running_fcf = base_fcf;
                            for step in 1..=5 {
                                running_fcf *= 1.0 + growth;
                                pv_stage_1 += running_fcf / (1.0 + wacc).powi(step);
                            }
                            let terminal_value = (running_fcf * (1.0 + term_g)) / (wacc - term_g);
                            let pv_terminal = terminal_value / (1.0 + wacc).powi(5);
                            let intrinsic_share = ((pv_stage_1 + pv_terminal) - debt).max(0.0) / shares;
                            
                            ui.label(egui::RichText::new(format!("₹ {:.2}", intrinsic_share)).strong().color(Color32::GREEN));
                        } else if shares == 0.0 {
                            ui.label(egui::RichText::new("Missing Shares").weak().color(Color32::LIGHT_RED));
                        } else if base_fcf <= 0.0 {
                            ui.label(egui::RichText::new("Negative FCF").weak());
                        } else {
                            ui.label(egui::RichText::new("WACC < gn").weak());
                        }
                    }
                    ui.end_row();
                });
            });
        });
    }
}

// =========================================================================
// DIVIDEND DISCOUNT MODEL SUBTAB - WITH TWO-AXIS SCROLLING
// =========================================================================
struct DdmTab;
impl AbstractSubTab<Vec<AnalysisMetadataRow>> for DdmTab {
    fn id(&self) -> usize { 1 }
    fn label(&self) -> &'static str { "Dividend Discount Model (DDM)" }
    fn render_main(&self, ui: &mut Ui, _data: &Vec<AnalysisMetadataRow>) { render_workspace_chart(ui); }

    fn render_bottom(&self, ui: &mut Ui, data: &Vec<AnalysisMetadataRow>) {
        let metrics = vec!["div", "shares", "rf", "rm", "g"];
        let (years, analysis_map) = get_valuation_maps(&metrics);
        if years.is_empty() { return; }

        let calculated_beta = data.first().map(|r| (r.nse_beta + r.bse_beta) / 2.0).unwrap_or(1.0);

        ui.label(egui::RichText::new("Dividend Discount Model (Gordon Growth Grid)").strong().size(14.0));
        ui.add_space(4.0);

        egui::ScrollArea::both().id_source("ddm_matrix_scroll_area").show(ui, |ui| {
            egui::Frame::none().stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 45, 45))).show(ui, |ui| {
                egui::Grid::new("ddm_matrix_grid").striped(true).spacing(egui::vec2(12.0, 8.0)).show(ui, |ui| {
                    render_horizontal_grid_header(ui, &years, "METRICS / STATIONS");
                    render_editable_row(ui, &years, "Aggregate Dividend Paid", "div", |r| r.dividend_paid.to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Outstanding Shares", "shares", |r| r.outstanding_shares.to_string(), &analysis_map);
                    
                    ui.separator(); for _ in &years { ui.separator(); } ui.end_row();

                    render_editable_row(ui, &years, "Risk Free Rate (Rf)", "rf", |_| "7.0".to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Market Premium (Rm)", "rm", |_| "12.0".to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Dividend Growth Rate (g)", "g", |_| "5.0".to_string(), &analysis_map);

                    ui.separator(); for _ in &years { ui.separator(); } ui.end_row();

                    ui.label(egui::RichText::new("DDM Intrinsic Share Price").strong().color(Color32::from_rgb(50, 220, 120)));
                    for yr in &years {
                        let total_div = access_cell_state(*yr, "div", "0".to_string()).parse::<f64>().unwrap_or(0.0);
                        let shares = access_cell_state(*yr, "shares", "0".to_string()).parse::<f64>().unwrap_or(0.0);
                        
                        let rf = access_cell_state(*yr, "rf", "7.0".to_string()).parse::<f64>().unwrap_or(7.0) / 100.0;
                        let rm = access_cell_state(*yr, "rm", "12.0".to_string()).parse::<f64>().unwrap_or(12.0) / 100.0;
                        let div_g = access_cell_state(*yr, "g", "5.0".to_string()).parse::<f64>().unwrap_or(5.0) / 100.0;

                        let ke = rf + calculated_beta * (rm - rf);
                        let dps_base = if shares > 0.0 { total_div / shares } else { 0.0 };

                        if ke > div_g && dps_base > 0.0 && shares > 0.0 {
                            let value_per_share = (dps_base * (1.0 + div_g)) / (ke - div_g);
                            ui.label(egui::RichText::new(format!("₹ {:.2}", value_per_share)).strong().color(Color32::GREEN));
                        } else if shares == 0.0 {
                            ui.label(egui::RichText::new("Missing Shares").weak().color(Color32::LIGHT_RED));
                        } else if dps_base == 0.0 {
                            ui.label(egui::RichText::new("No Dividends").weak());
                        } else {
                            ui.label(egui::RichText::new("Ke < g").weak());
                        }
                    }
                    ui.end_row();
                });
            });
        });
    }
}

// =========================================================================
// RESIDUAL INCOME MODEL SUBTAB - WITH TWO-AXIS SCROLLING
// =========================================================================
struct ResidualIncomeTab;
impl AbstractSubTab<Vec<AnalysisMetadataRow>> for ResidualIncomeTab {
    fn id(&self) -> usize { 2 }
    fn label(&self) -> &'static str { "Residual Income" }
    fn render_main(&self, ui: &mut Ui, _data: &Vec<AnalysisMetadataRow>) { render_workspace_chart(ui); }

    fn render_bottom(&self, ui: &mut Ui, data: &Vec<AnalysisMetadataRow>) {
        let metrics = vec!["eq", "pat", "shares", "rf", "rm", "g"];
        let (years, analysis_map) = get_valuation_maps(&metrics);
        if years.is_empty() { return; }

        let calculated_beta = data.first().map(|r| (r.nse_beta + r.bse_beta) / 2.0).unwrap_or(1.0);

        ui.label(egui::RichText::new("Residual Income Multi-Stage Capital Table").strong().size(14.0));
        ui.add_space(4.0);

        egui::ScrollArea::both().id_source("ri_matrix_scroll_area").show(ui, |ui| {
            egui::Frame::none().stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 45, 45))).show(ui, |ui| {
                egui::Grid::new("ri_matrix_grid").striped(true).spacing(egui::vec2(12.0, 8.0)).show(ui, |ui| {
                    render_horizontal_grid_header(ui, &years, "METRICS / STATIONS");
                    render_editable_row(ui, &years, "Total Equity (Book Value)", "eq", |r| r.total_equity.to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Net Profit After Tax (PAT)", "pat", |r| r.net_profit_after_tax.to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Outstanding Shares", "shares", |r| r.outstanding_shares.to_string(), &analysis_map);

                    ui.separator(); for _ in &years { ui.separator(); } ui.end_row();

                    render_editable_row(ui, &years, "Risk Free Rate (Rf)", "rf", |_| "7.0".to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Market Return (Rm)", "rm", |_| "12.0".to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Income Growth Forecast (g)", "g", |_| "8.0".to_string(), &analysis_map);

                    ui.separator(); for _ in &years { ui.separator(); } ui.end_row();

                    ui.label(egui::RichText::new("RIM Intrinsic Share Price").strong().color(Color32::from_rgb(50, 220, 120)));
                    for yr in &years {
                        let eq_base = access_cell_state(*yr, "eq", "0".to_string()).parse::<f64>().unwrap_or(0.0);
                        let pat_base = access_cell_state(*yr, "pat", "0".to_string()).parse::<f64>().unwrap_or(0.0);
                        let shares = access_cell_state(*yr, "shares", "0".to_string()).parse::<f64>().unwrap_or(0.0);

                        let rf = access_cell_state(*yr, "rf", "7.0".to_string()).parse::<f64>().unwrap_or(7.0) / 100.0;
                        let rm = access_cell_state(*yr, "rm", "12.0".to_string()).parse::<f64>().unwrap_or(12.0) / 100.0;
                        let growth = access_cell_state(*yr, "g", "8.0".to_string()).parse::<f64>().unwrap_or(8.0) / 100.0;

                        let ke = rf + calculated_beta * (rm - rf);

                        if ke > 0.0 && eq_base > 0.0 && shares > 0.0 {
                            let mut pv_residual_income = 0.0;
                            let mut projected_equity = eq_base;
                            let mut projected_pat = pat_base;

                            for step in 1..=5 {
                                let equity_charge = projected_equity * ke;
                                let residual_income = projected_pat - equity_charge;
                                pv_residual_income += residual_income / (1.0 + ke).powi(step);

                                projected_pat *= 1.0 + growth;
                                projected_equity += residual_income;
                            }

                            let intrinsic_value = (eq_base + pv_residual_income) / shares;
                            ui.label(egui::RichText::new(format!("₹ {:.2}", intrinsic_value)).strong().color(Color32::GREEN));
                        } else if shares == 0.0 {
                            ui.label(egui::RichText::new("Missing Shares").weak().color(Color32::LIGHT_RED));
                        } else {
                            ui.label(egui::RichText::new("Value N/A").weak());
                        }
                    }
                    ui.end_row();
                });
            });
        });
    }
}

// =========================================================================
// PIPELINE ROUTING CANVAS ORCHESTRATOR
// =========================================================================
pub fn draw_analysis_panel(ui: &mut Ui, active_ticker: &str) {
    DataManager::ensure_analysis_data(active_ticker);
    ACTIVE_PANEL_TICKER.with(|ticker| { *ticker.borrow_mut() = active_ticker.to_string(); });

    let tabs: &[&dyn AbstractSubTab<Vec<AnalysisMetadataRow>>] = &[
        &DcfTab,
        &DdmTab,
        &ResidualIncomeTab,
    ];

    draw_nav_canvas_orchestrator(
        ui, active_ticker, "analysis_metadata", "VALUATION WORKSPACE", "analysis_active_tab_id", tabs
    );
}