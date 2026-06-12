use egui::{Ui, Color32};
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use crate::core::data_manager::DataManager;
use crate::ui::layouts::canvas::{AbstractSubTab, draw_nav_canvas_orchestrator, paint_abstract_chart_canvas, GenericChartLine, GenericChartPoint};
use backend::database::analysis::{AnalysisMetadataRow, ValuationResultRow};

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
pub fn push_interactive_state_to_pool(years: &[i32], storage_slot_key: &str) {
    let mut master_rows = Vec::with_capacity(years.len());

    let mut extracted_values = Vec::with_capacity(years.len());
    {
        for &year in years {
            extracted_values.push((
                year,
                access_cell_state(year, "eps", "0".to_string()),
                access_cell_state(year, "pat", "0".to_string()),
                access_cell_state(year, "div", "0".to_string()),
                access_cell_state(year, "eq", "0".to_string()),
                access_cell_state(year, "debt", "0".to_string()),    
                access_cell_state(year, "ocf", "0".to_string()),
                access_cell_state(year, "capex_out", "0".to_string()),
                access_cell_state(year, "capex_in", "0".to_string()),
                access_cell_state(year, "shares", "0".to_string()),
                access_cell_state(year, "pbt", "0".to_string()),
                access_cell_state(year, "interest", "0".to_string()),
                access_cell_state(year, "tax_rate", "0.25".to_string()),
                access_cell_state(year, "beta", "1.0".to_string()),
            ));
        }
    }

    for (year, eps, pat, div, eq, debt, ocf, capex_out, capex_in, shares, pbt, interest, tax, beta) in extracted_values {
        let basic_eps = eps.parse::<f64>().unwrap_or(0.0);
        let net_profit_after_tax = pat.parse::<i64>().unwrap_or(0);
        let dividend_paid = div.parse::<i64>().unwrap_or(0);
        let total_equity = eq.parse::<i64>().unwrap_or(0);
        let total_debt = debt.parse::<i64>().unwrap_or(0);    
        let operating_cash_flow = ocf.parse::<i64>().unwrap_or(0);
        let capex_outflow = capex_out.parse::<i64>().unwrap_or(0);
        let capex_inflow = capex_in.parse::<i64>().unwrap_or(0);
        let outstanding_shares = shares.parse::<i64>().unwrap_or(0);
        let profit_before_tax = pbt.parse::<i64>().unwrap_or(0);
        let finance_interest_expense = interest.parse::<i64>().unwrap_or(0);
        let effective_tax_rate = tax.parse::<f64>().unwrap_or(0.25);
        let user_beta = beta.parse::<f64>().unwrap_or(1.0);
        
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

    backend::commands::memory_pool::store_parsed_table(storage_slot_key, master_rows);
}

fn get_valuation_maps(tab_metrics: &[&str], storage_slot_key: &str) -> (Vec<i32>, HashMap<i32, AnalysisMetadataRow>) {
    let active_ticker = ACTIVE_PANEL_TICKER.with(|ticker| ticker.borrow().clone());
    if active_ticker.is_empty() { return (Vec::new(), HashMap::new()); }

    let mut analysis_rows: Vec<AnalysisMetadataRow> = Vec::new();
    backend::commands::memory_pool::with_active_table::<Vec<AnalysisMetadataRow>, _, _>(storage_slot_key, |table| {
        analysis_rows = table.clone();
    });

    if analysis_rows.is_empty() {
        backend::commands::memory_pool::with_active_table::<Vec<AnalysisMetadataRow>, _, _>("analysis_metadata", |table| {
            analysis_rows = table.clone();
        });
    }

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

fn render_workspace_chart(ui: &mut Ui, result_slot_key: &str, value_label: &'static str) {
    // 1. Fetch historical pricing points
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

    // 2. Fetch computed intrinsic values specifically for the active tab context
    let mut val_rows: Vec<ValuationResultRow> = Vec::new();
    backend::commands::memory_pool::with_active_table::<Vec<ValuationResultRow>, _, _>(result_slot_key, |table| {
        val_rows = table.clone();
    });

    // Ensure intrinsic points render left-to-right
    val_rows.sort_by(|a, b| a.year.cmp(&b.year));

    let mut val_points = Vec::with_capacity(val_rows.len());
    for res in val_rows {
        if res.status_ok && res.intrinsic_value > 0.0 {
            // Append explicit March 31st financial year-end to map to generic string dates correctly
            val_points.push(GenericChartPoint { 
                date: format!("{}-03-31", res.year), 
                value: res.intrinsic_value 
            });
        }
    }

    let mut chart_lines = vec![
        GenericChartLine { label: "NSE", color: Color32::from_rgb(250, 210, 50), stroke_width: 1.5, points: nse_points },
        GenericChartLine { label: "BSE", color: Color32::from_rgb(50, 150, 250), stroke_width: 1.5, points: bse_points },
    ];

    // Overlay dynamically calculated intrinsic value line if data exists for this specific model
    if !val_points.is_empty() {
        chart_lines.push(GenericChartLine { 
            label: value_label, 
            color: Color32::from_rgb(50, 220, 120), // Standout Green
            stroke_width: 2.0, 
            points: val_points 
        });
    }

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
    storage_slot_key: &'static str,
    extract_fallback: impl Fn(&AnalysisMetadataRow) -> String,
    analysis_map: &HashMap<i32, AnalysisMetadataRow>,
) {
    ui.label(label);
    for yr in years {
        let fallback = analysis_map.get(yr).map(&extract_fallback).unwrap_or_else(|| "0".to_string());
        let mut value_buffer = access_cell_state(*yr, metric_id, fallback);
        if ui.add(egui::TextEdit::singleline(&mut value_buffer).desired_width(80.0)).changed() {
            update_cell_state(*yr, metric_id, value_buffer);
            push_interactive_state_to_pool(years, storage_slot_key);

            let active_ticker = ACTIVE_PANEL_TICKER.with(|ticker| ticker.borrow().clone());
            let calculation_tag = match storage_slot_key {
                "dcf_metadata" => "DCF",
                "ddm_metadata" => "DDM",
                _ => "REM",
            };
            backend::commands::analysis_engine::compute_on_fly_valuation(&active_ticker, calculation_tag);
        }
    }
    ui.end_row();
}

// =========================================================================
// DISCOUNTED CASH FLOW SUBTAB - ZERO COPY DIRECT RAM DISPLAY ONLY
// =========================================================================
struct DcfTab;
impl AbstractSubTab<Vec<AnalysisMetadataRow>> for DcfTab {
    fn id(&self) -> usize { 0 }
    fn label(&self) -> &'static str { "Discounted Cash Flow (DCF)" }
    fn render_main(&self, ui: &mut Ui, _data: &Vec<AnalysisMetadataRow>) { 
        render_workspace_chart(ui, "dcf_calculated_results", "DCF Value"); 
    }
    
    fn render_bottom(&self, ui: &mut Ui, _data: &Vec<AnalysisMetadataRow>) {
        let metrics = vec!["ocf", "capex_out", "debt", "eq", "shares", "pbt", "pat", "interest", "rf", "rm", "g", "gn"];
        let (years, analysis_map) = get_valuation_maps(&metrics, "dcf_metadata");
        if years.is_empty() { return; }

        let mut results_rows: Vec<ValuationResultRow> = Vec::new();
        backend::commands::memory_pool::with_active_table::<Vec<ValuationResultRow>, _, _>("dcf_calculated_results", |table| {
            results_rows = table.clone();
        });
        let results_map: HashMap<i32, ValuationResultRow> = results_rows.into_iter().map(|r| (r.year, r)).collect();

        ui.label(egui::RichText::new("Discounted Cash Flow (TABULAR CALCULATOR)").strong().size(14.0));
        ui.add_space(4.0);

        egui::ScrollArea::both().id_source("dcf_matrix_scroll_area").show(ui, |ui| {
            egui::Frame::none().stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 45, 45))).show(ui, |ui| {
                egui::Grid::new("dcf_matrix_grid").striped(true).spacing(egui::vec2(12.0, 8.0)).show(ui, |ui| {
                    render_horizontal_grid_header(ui, &years, "METRICS FROM INTEGRATED PARQUETS");
                    
                    render_editable_row(ui, &years, "Operating Cash Flow (OCF)", "ocf", "dcf_metadata", |r| r.operating_cash_flow.to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Capital Expenditure (Capex)", "capex_out", "dcf_metadata", |r| r.capex_outflow.to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Total Debt (Short + Long Term)", "debt", "dcf_metadata", |r| r.total_debt.to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Total Shareholder Equity", "eq", "dcf_metadata", |r| r.total_equity.to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Outstanding Shares Count", "shares", "dcf_metadata", |r| r.outstanding_shares.to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Profit Before Tax (PBT)", "pbt", "dcf_metadata", |r| r.profit_before_tax.to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Net Profit After Tax (PAT)", "pat", "dcf_metadata", |r| r.net_profit_after_tax.to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Finance Interest Expenses", "interest", "dcf_metadata", |r| r.finance_interest_expense.to_string(), &analysis_map);

                    ui.separator(); for _ in &years { ui.separator(); } ui.end_row();
                    render_horizontal_grid_header(ui, &years, "USER FORECAST ASSUMPTIONS");
                    
                    render_editable_row(ui, &years, "Risk Free Rate (Rf)", "rf", "dcf_metadata", |_| "7.0".to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Expected Market Return (Rm)", "rm", "dcf_metadata", |_| "12.0".to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Stage 1 Forecast Growth (g)", "g", "dcf_metadata", |_| "10.0".to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Terminal Perpetuity Growth (gn)", "gn", "dcf_metadata", |_| "4.5".to_string(), &analysis_map);

                    ui.separator(); for _ in &years { ui.separator(); } ui.end_row();
                    render_horizontal_grid_header(ui, &years, "DERIVED DATA ATTRIBUTES ENGINE");

                    ui.label("Calculated Marginal Tax Rate");
                    for yr in &years {
                        if let Some(res) = results_map.get(yr) {
                            ui.label(format!("{:.1}%", res.calculated_tax_rate * 100.0));
                        } else {
                            ui.label(egui::RichText::new("0").weak());
                        }
                    }
                    ui.end_row();

                    ui.label("Calculated Pre-tax Cost of Debt (Kd)");
                    for yr in &years {
                        if let Some(res) = results_map.get(yr) {
                            ui.label(format!("{:.2}%", res.calculated_kd * 100.0));
                        } else {
                            ui.label(egui::RichText::new("0").weak());
                        }
                    }
                    ui.end_row();

                    ui.label("Calculated Cost of Equity (Ke)");
                    for yr in &years {
                        if let Some(res) = results_map.get(yr) {
                            ui.label(format!("{:.2}%", res.calculated_ke * 100.0));
                        } else {
                            ui.label(egui::RichText::new("0").weak());
                        }
                    }
                    ui.end_row();

                    ui.label(egui::RichText::new("Calculated WACC").strong().color(Color32::from_rgb(50, 160, 240)));
                    for yr in &years {
                        if let Some(res) = results_map.get(yr) {
                            ui.label(format!("{:.2}%", res.calculated_wacc * 100.0));
                        } else {
                            ui.label(egui::RichText::new("0").weak());
                        }
                    }
                    ui.end_row();

                    ui.label(egui::RichText::new("DCF INTRINSIC VALUE").strong().color(Color32::from_rgb(50, 220, 120)));
                    for yr in &years {
                        if let Some(res) = results_map.get(yr) {
                            if res.status_ok {
                                ui.label(egui::RichText::new(format!("₹ {:.2}", res.intrinsic_value)).strong().color(Color32::GREEN));
                            } else {
                                ui.label(egui::RichText::new(&res.error_msg).weak().color(Color32::LIGHT_RED));
                            }
                        } else {
                            ui.label(egui::RichText::new("0").weak());
                        }
                    }
                    ui.end_row();
                });
            });
        });
    }
}

// =========================================================================
// DIVIDEND DISCOUNT MODEL SUBTAB - ZERO COPY DIRECT RAM DISPLAY ONLY
// =========================================================================
struct DdmTab;
impl AbstractSubTab<Vec<AnalysisMetadataRow>> for DdmTab {
    fn id(&self) -> usize { 1 }
    fn label(&self) -> &'static str { "Dividend Discount Model (DDM)" }
    fn render_main(&self, ui: &mut Ui, _data: &Vec<AnalysisMetadataRow>) { 
        render_workspace_chart(ui, "ddm_calculated_results", "DDM Value"); 
    }

    fn render_bottom(&self, ui: &mut Ui, _data: &Vec<AnalysisMetadataRow>) {
        let metrics = vec!["div", "shares", "rf", "rm", "g"];
        let (years, analysis_map) = get_valuation_maps(&metrics, "ddm_metadata");
        if years.is_empty() { return; }

        let mut results_rows: Vec<ValuationResultRow> = Vec::new();
        backend::commands::memory_pool::with_active_table::<Vec<ValuationResultRow>, _, _>("ddm_calculated_results", |table| {
            results_rows = table.clone();
        });
        let results_map: HashMap<i32, ValuationResultRow> = results_rows.into_iter().map(|r| (r.year, r)).collect();

        ui.label(egui::RichText::new("Dividend Discount Model (Gordon Growth Grid)").strong().size(14.0));
        ui.add_space(4.0);

        egui::ScrollArea::both().id_source("ddm_matrix_scroll_area").show(ui, |ui| {
            egui::Frame::none().stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 45, 45))).show(ui, |ui| {
                egui::Grid::new("ddm_matrix_grid").striped(true).spacing(egui::vec2(12.0, 8.0)).show(ui, |ui| {
                    render_horizontal_grid_header(ui, &years, "METRICS / STATIONS");
                    render_editable_row(ui, &years, "Aggregate Dividend Paid", "div", "ddm_metadata", |r| r.dividend_paid.to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Outstanding Shares", "shares", "ddm_metadata", |r| r.outstanding_shares.to_string(), &analysis_map);
                    
                    ui.separator(); for _ in &years { ui.separator(); } ui.end_row();

                    render_editable_row(ui, &years, "Risk Free Rate (Rf)", "rf", "ddm_metadata", |_| "7.0".to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Market Premium (Rm)", "rm", "ddm_metadata", |_| "12.0".to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Dividend Growth Rate (g)", "g", "ddm_metadata", |_| "5.0".to_string(), &analysis_map);

                    ui.separator(); for _ in &years { ui.separator(); } ui.end_row();

                    ui.label(egui::RichText::new("DDM Intrinsic Share Price").strong().color(Color32::from_rgb(50, 220, 120)));
                    for yr in &years {
                        if let Some(res) = results_map.get(yr) {
                            if res.status_ok {
                                ui.label(egui::RichText::new(format!("₹ {:.2}", res.intrinsic_value)).strong().color(Color32::GREEN));
                            } else {
                                ui.label(egui::RichText::new(&res.error_msg).weak().color(Color32::LIGHT_RED));
                            }
                        } else {
                            ui.label(egui::RichText::new("0").weak());
                        }
                    }
                    ui.end_row();
                });
            });
        });
    }
}

// =========================================================================
// RESIDUAL INCOME MODEL SUBTAB - ZERO COPY DIRECT RAM DISPLAY ONLY
// =========================================================================
struct ResidualIncomeTab;
impl AbstractSubTab<Vec<AnalysisMetadataRow>> for ResidualIncomeTab {
    fn id(&self) -> usize { 2 }
    fn label(&self) -> &'static str { "Residual Income" }
    fn render_main(&self, ui: &mut Ui, _data: &Vec<AnalysisMetadataRow>) { 
        render_workspace_chart(ui, "rem_calculated_results", "RIM Value"); 
    }

    fn render_bottom(&self, ui: &mut Ui, _data: &Vec<AnalysisMetadataRow>) {
        let metrics = vec!["eq", "pat", "shares", "rf", "rm", "g"];
        let (years, analysis_map) = get_valuation_maps(&metrics, "rem_metadata");
        if years.is_empty() { return; }

        let mut results_rows: Vec<ValuationResultRow> = Vec::new();
        backend::commands::memory_pool::with_active_table::<Vec<ValuationResultRow>, _, _>("rem_calculated_results", |table| {
            results_rows = table.clone();
        });
        let results_map: HashMap<i32, ValuationResultRow> = results_rows.into_iter().map(|r| (r.year, r)).collect();

        ui.label(egui::RichText::new("Residual Income Multi-Stage Capital Table").strong().size(14.0));
        ui.add_space(4.0);

        egui::ScrollArea::both().id_source("ri_matrix_scroll_area").show(ui, |ui| {
            egui::Frame::none().stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 45, 45))).show(ui, |ui| {
                egui::Grid::new("ri_matrix_grid").striped(true).spacing(egui::vec2(12.0, 8.0)).show(ui, |ui| {
                    render_horizontal_grid_header(ui, &years, "METRICS / STATIONS");
                    render_editable_row(ui, &years, "Total Equity (Book Value)", "eq", "rem_metadata", |r| r.total_equity.to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Net Profit After Tax (PAT)", "pat", "rem_metadata", |r| r.net_profit_after_tax.to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Outstanding Shares", "shares", "rem_metadata", |r| r.outstanding_shares.to_string(), &analysis_map);

                    ui.separator(); for _ in &years { ui.separator(); } ui.end_row();

                    render_editable_row(ui, &years, "Risk Free Rate (Rf)", "rf", "rem_metadata", |_| "7.0".to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Market Return (Rm)", "rm", "rem_metadata", |_| "12.0".to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Income Growth Forecast (g)", "g", "rem_metadata", |_| "8.0".to_string(), &analysis_map);

                    ui.separator(); for _ in &years { ui.separator(); } ui.end_row();

                    ui.label(egui::RichText::new("RIM Intrinsic Share Price").strong().color(Color32::from_rgb(50, 220, 120)));
                    for yr in &years {
                        if let Some(res) = results_map.get(yr) {
                            if res.status_ok {
                                ui.label(egui::RichText::new(format!("₹ {:.2}", res.intrinsic_value)).strong().color(Color32::GREEN));
                            } else {
                                ui.label(egui::RichText::new(&res.error_msg).weak().color(Color32::LIGHT_RED));
                            }
                        } else {
                            ui.label(egui::RichText::new("0").weak());
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

    let mut initial_sync_triggered = false;
    ACTIVE_PANEL_TICKER.with(|ticker| {
        let mut t = ticker.borrow_mut();
        if *t != active_ticker {
            *t = active_ticker.to_string();
            initial_sync_triggered = true;
        }
    });

    if initial_sync_triggered {
        INTERACTIVE_CELL_CACHE.with(|cache| cache.borrow_mut().inputs.clear());

        let mut base_data: Vec<AnalysisMetadataRow> = Vec::new();
        backend::commands::memory_pool::with_active_table::<Vec<AnalysisMetadataRow>, _, _>("analysis_metadata", |table| {
            base_data = table.clone();
        });

        if !base_data.is_empty() {
            backend::commands::memory_pool::store_parsed_table("dcf_metadata", base_data.clone());
            backend::commands::memory_pool::store_parsed_table("ddm_metadata", base_data.clone());
            backend::commands::memory_pool::store_parsed_table("rem_metadata", base_data.clone());

            backend::commands::analysis_engine::compute_on_fly_valuation(active_ticker, "DCF");
            backend::commands::analysis_engine::compute_on_fly_valuation(active_ticker, "DDM");
            backend::commands::analysis_engine::compute_on_fly_valuation(active_ticker, "REM");
        }
    }

    let tabs: &[&dyn AbstractSubTab<Vec<AnalysisMetadataRow>>] = &[
        &DcfTab,
        &DdmTab,
        &ResidualIncomeTab,
    ];

    draw_nav_canvas_orchestrator(
        ui, active_ticker, "analysis_metadata", "VALUATION WORKSPACE", "analysis_active_tab_id", tabs
    );
}