use egui::{Ui, Color32};
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use crate::core::data_manager::DataManager;
use crate::ui::layouts::canvas::{AbstractSubTab, draw_nav_canvas_orchestrator, paint_abstract_chart_canvas, GenericChartLine, GenericChartPoint};
use backend::database::analysis::AnalysisMetadataRow;

// =========================================================================
// THREAD-SAFE GLOBAL INTERACTIVE STATE ENGINE
// =========================================================================
#[derive(Clone)]
struct DynamicCellCache {
    inputs: HashMap<(i32, String), String>,
    global_inputs: HashMap<String, String>,
}

impl Default for DynamicCellCache {
    fn default() -> Self {
        let mut global_inputs = HashMap::new();
        global_inputs.insert("risk_free_rate".to_string(), "7.0".to_string());   // e.g., 7% Govt Bond yield
        global_inputs.insert("market_return".to_string(), "12.0".to_string());   // e.g., 12% Nifty long-term return
        global_inputs.insert("growth_rate".to_string(), "10.0".to_string());     // Stage 1 Growth assumption
        global_inputs.insert("terminal_growth".to_string(), "5.0".to_string());  // Perpetual Terminal Growth rate
        
        Self { 
            inputs: HashMap::new(),
            global_inputs,
        }
    }
}

std::thread_local! {
    static ACTIVE_PANEL_TICKER: RefCell<String> = RefCell::new(String::new());
    static INTERACTIVE_CELL_CACHE: RefCell<DynamicCellCache> = RefCell::new(DynamicCellCache::default());
}

fn access_cell_state(year: i32, metric: &str, fallback_val: String) -> String {
    INTERACTIVE_CELL_CACHE.with(|cache| {
        let mut cache_ref = cache.borrow_mut();
        let key = (year, metric.to_string());
        cache_ref.inputs.entry(key).or_insert(fallback_val).clone()
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
                if trimmed.is_empty() || trimmed == "0" || trimmed == "0.0" || trimmed == "0.00" {
                    return false;
                }
            } else {
                return false;
            }
        }
        true
    })
}

fn access_global_state(metric_id: &str, fallback_val: String) -> String {
    INTERACTIVE_CELL_CACHE.with(|cache| {
        let mut cache_ref = cache.borrow_mut();
        cache_ref.global_inputs.entry(metric_id.to_string()).or_insert(fallback_val).clone()
    })
}

fn update_global_state(metric_id: &str, current_text: String) {
    INTERACTIVE_CELL_CACHE.with(|cache| {
        let mut cache_ref = cache.borrow_mut();
        cache_ref.global_inputs.insert(metric_id.to_string(), current_text);
    });
}

// =========================================================================
// INTERACTIVE ENGINE VALUE-STITCHED POOL SYNC BACKEND
// =========================================================================
pub fn push_interactive_state_to_pool(years: &[i32]) {
    let mut master_rows = Vec::with_capacity(years.len());
    let user_beta = access_global_state("beta", "1.0".to_string()).parse::<f64>().unwrap_or(1.0);

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
        let effective_tax_rate = access_cell_state(year, "tax_rate", "0.0".to_string()).parse::<f64>().unwrap_or(0.0);
        
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

    backend::commands::memory_pool::store_parsed_table("analysis_metadata", master_rows.clone());
    backend::commands::memory_pool::store_parsed_table("dcf_metadata", master_rows.clone());
    backend::commands::memory_pool::store_parsed_table("ddm_metadata", master_rows.clone());
    backend::commands::memory_pool::store_parsed_table("rem_metadata", master_rows);
}

fn get_valuation_maps(tab_metrics: &[&str]) -> (Vec<i32>, HashMap<i32, AnalysisMetadataRow>) {
    let active_ticker = ACTIVE_PANEL_TICKER.with(|ticker| ticker.borrow().clone());
    if active_ticker.is_empty() {
        return (Vec::new(), HashMap::new());
    }

    let mut analysis_rows: Vec<AnalysisMetadataRow> = Vec::new();
    let mut unique_years = BTreeSet::new();

    backend::commands::memory_pool::with_active_table::<Vec<AnalysisMetadataRow>, _, _>("analysis_metadata", |table| {
        analysis_rows = table.clone();
    });

    let mut analysis_map = HashMap::new();
    for row in analysis_rows {
        unique_years.insert(row.year);
        analysis_map.insert(row.year, row);
    }

    let mut years_vector: Vec<i32> = unique_years.into_iter().collect();

    if !years_vector.is_empty() {
        let mut current_min = years_vector[0];
        while check_column_filled(current_min - 1, tab_metrics) {
            current_min -= 1;
        }
        let absolute_start = current_min - 1;

        let mut current_max = years_vector[years_vector.len() - 1];
        while check_column_filled(current_max + 1, tab_metrics) {
            current_max += 1;
        }
        let absolute_end = current_max + 1;

        years_vector = (absolute_start..=absolute_end).collect();
    }

    (years_vector, analysis_map)
}

// =========================================================================
// ROUTING LAYER: MAP CACHE TABLES DIRECTLY TO ABSTRACT CANVAS PLOTTERS
// =========================================================================
fn render_workspace_chart(ui: &mut Ui) {
    let mut entries: Vec<backend::database::analysis::HistoricalChartRow> = Vec::new();
    backend::commands::memory_pool::with_active_table::<Vec<backend::database::analysis::HistoricalChartRow>, _, _>(
        "historical_chart_data", 
        |table| { entries = table.clone(); }
    );

    let mut nse_points = Vec::with_capacity(entries.len());
    let mut bse_points = Vec::with_capacity(entries.len());

    for row in entries {
        if let Some(val) = row.nse_close {
            nse_points.push(GenericChartPoint { date: row.date.clone(), value: val });
        }
        if let Some(val) = row.bse_close {
            bse_points.push(GenericChartPoint { date: row.date, value: val });
        }
    }

    let chart_lines = vec![
        GenericChartLine {
            label: "NSE",
            color: Color32::from_rgb(250, 210, 50),
            stroke_width: 1.5,
            points: nse_points,
        },
        GenericChartLine {
            label: "BSE",
            color: Color32::from_rgb(50, 150, 250),
            stroke_width: 1.5,
            points: bse_points,
        },
    ];

    // Delegate presentation directly to your central layout engine
    paint_abstract_chart_canvas(ui, &chart_lines);
}

// =========================================================================
// REUSABLE MODERN MODULAR FIELD PRIMITIVES (DEDUPLICATES ALL ROWS)
// =========================================================================
fn render_horizontal_grid_header(ui: &mut Ui, years: &[i32], title: &str) {
    ui.label(egui::RichText::new(title).strong());
    for year in years {
        ui.label(egui::RichText::new(format!("{}", year)).strong());
    }
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
        if ui.text_edit_singleline(&mut value_buffer).changed() {
            update_cell_state(*yr, metric_id, value_buffer);
            push_interactive_state_to_pool(years);
        }
    }
    ui.end_row();
}

fn render_global_input_field(ui: &mut Ui, label: &str, metric_id: &str, fallback: &str, suffix: &str) -> bool {
    let mut changed = false;
    ui.label(label);
    let mut value_buffer = access_global_state(metric_id, fallback.to_string());
    ui.horizontal(|ui| {
        if ui.add(egui::TextEdit::singleline(&mut value_buffer).desired_width(70.0)).changed() {
            update_global_state(metric_id, value_buffer);
            changed = true;
        }
        if !suffix.is_empty() {
            ui.weak(suffix);
        }
    });
    ui.end_row();
    changed
}

// =========================================================================
// ABSTRACT IMPLEMENTATIONS: INTRINSIC ESTIMATION CANVAS HOOKS
// =========================================================================
struct DcfTab;
impl AbstractSubTab<Vec<AnalysisMetadataRow>> for DcfTab {
    fn id(&self) -> usize { 0 }
    fn label(&self) -> &'static str { "Discounted Cash Flow (DCF)" }
    
    fn render_main(&self, ui: &mut Ui, _data: &Vec<AnalysisMetadataRow>) {
        render_workspace_chart(ui);
    }
    
    fn render_bottom(&self, ui: &mut Ui, data: &Vec<AnalysisMetadataRow>) {
        let metrics = vec!["ocf", "capex_out", "capex_in", "debt", "shares", "pbt", "interest", "tax_rate"];
        let (years, analysis_map) = get_valuation_maps(&metrics);
        if years.is_empty() { return; }

        let fallback_beta = data.first().map(|r| format!("{:.2}", r.nse_beta)).unwrap_or_else(|| "1.00".to_string());

        ui.columns(2, |columns| {
            // COLUMN 1: TIME SERIES GRID INPUTS
            columns[0].vertical(|ui| {
                ui.heading("Historical Statement Parameters");
                ui.add_space(4.0);
                egui::ScrollArea::horizontal().id_source("dcf_scroll_node").show(ui, |ui| {
                    egui::Frame::none().stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 45, 45))).show(ui, |ui| {
                        egui::Grid::new("dcf_node_grid").striped(true).min_col_width(110.0).spacing(egui::vec2(16.0, 10.0)).show(ui, |ui| {
                            render_horizontal_grid_header(ui, &years, "METRICS / YEARS");
                            render_editable_row(ui, &years, "Operating Cash Flow", "ocf", |r| r.operating_cash_flow.to_string(), &analysis_map);
                            render_editable_row(ui, &years, "Capex Outflow", "capex_out", |r| r.capex_outflow.to_string(), &analysis_map);
                            render_editable_row(ui, &years, "Capex Inflow", "capex_in", |r| r.capex_inflow.to_string(), &analysis_map);
                            render_editable_row(ui, &years, "Total Outstanding Debt", "debt", |r| r.total_debt.to_string(), &analysis_map);
                            render_editable_row(ui, &years, "Outstanding Shares", "shares", |r| r.outstanding_shares.to_string(), &analysis_map);
                            render_editable_row(ui, &years, "Profit Before Tax", "pbt", |r| r.profit_before_tax.to_string(), &analysis_map);
                            render_editable_row(ui, &years, "Finance Interest Costs", "interest", |r| r.finance_interest_expense.to_string(), &analysis_map);
                            render_editable_row(ui, &years, "Effective Tax Rate", "tax_rate", |r| format!("{:.4}", r.effective_tax_rate), &analysis_map);
                        });
                    });
                });
            });

            // COLUMN 2: GLOBAL VALUATION ASSUMPTIONS BLOCK
            columns[1].vertical(|ui| {
                ui.heading("Valuation Assumptions Model");
                ui.add_space(4.0);
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(15, 15, 15))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(35, 35, 35)))
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        egui::Grid::new("dcf_assumptions_grid").spacing(egui::vec2(10.0, 12.0)).show(ui, |ui| {
                            let mut trigger_sync = false;
                            
                            trigger_sync |= render_global_input_field(ui, "Empirical Systematic Risk (Beta)", "beta", &fallback_beta, "x");
                            trigger_sync |= render_global_input_field(ui, "Risk-Free Rate (Rf)", "risk_free_rate", "7.0", "%");
                            trigger_sync |= render_global_input_field(ui, "Expected Market Return (Rm)", "market_return", "12.0", "%");
                            trigger_sync |= render_global_input_field(ui, "Stage 1 Growth Rate (g)", "growth_rate", "10.0", "%");
                            trigger_sync |= render_global_input_field(ui, "Terminal Growth Rate (g_n)", "terminal_growth", "5.0", "%");

                            if trigger_sync {
                                push_interactive_state_to_pool(&years);
                            }

                            ui.end_row();
                            ui.separator(); ui.end_row();

                            let rf = access_global_state("risk_free_rate", "7.0".to_string()).parse::<f64>().unwrap_or(7.0) / 100.0;
                            let rm = access_global_state("market_return", "12.0".to_string()).parse::<f64>().unwrap_or(12.0) / 100.0;
                            let beta = access_global_state("beta", "1.0".to_string()).parse::<f64>().unwrap_or(1.0);
                            
                            let cost_of_equity = rf + beta * (rm - rf);

                            ui.label(egui::RichText::new("Calculated Cost of Equity (Ke):").strong().color(egui::Color32::from_rgb(50, 200, 120)));
                            ui.label(egui::RichText::new(format!("{:.2}%", cost_of_equity * 100.0)).strong().color(egui::Color32::from_rgb(255, 255, 255)));
                            ui.end_row();
                        });
                    });
            });
        });
    }
}

struct DdmTab;
impl AbstractSubTab<Vec<AnalysisMetadataRow>> for DdmTab {
    fn id(&self) -> usize { 1 }
    fn label(&self) -> &'static str { "Dividend Discount Model (DDM)" }
    
    fn render_main(&self, ui: &mut Ui, _data: &Vec<AnalysisMetadataRow>) {
        render_workspace_chart(ui);
    }

    fn render_bottom(&self, ui: &mut Ui, _data: &Vec<AnalysisMetadataRow>) {
        let metrics = vec!["eps", "pat", "div", "debt", "shares"];
        let (years, analysis_map) = get_valuation_maps(&metrics);
        if years.is_empty() { return; }

        egui::ScrollArea::horizontal().id_source("ddm_scroll_node").show(ui, |ui| {
            egui::Frame::none().stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 45, 45))).show(ui, |ui| {
                egui::Grid::new("ddm_node_grid").striped(true).min_col_width(110.0).spacing(egui::vec2(16.0, 10.0)).show(ui, |ui| {
                    render_horizontal_grid_header(ui, &years, "METRICS / YEARS");
                    render_editable_row(ui, &years, "Basic EPS", "eps", |r| format!("{:.2}", r.basic_eps), &analysis_map);
                    render_editable_row(ui, &years, "Net Profit (AT)", "pat", |r| r.net_profit_after_tax.to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Dividend Paid", "div", |r| r.dividend_paid.to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Outstanding Shares", "shares", |r| r.outstanding_shares.to_string(), &analysis_map);
                });
            });
        });
    }
}

struct ResidualIncomeTab;
impl AbstractSubTab<Vec<AnalysisMetadataRow>> for ResidualIncomeTab {
    fn id(&self) -> usize { 2 }
    fn label(&self) -> &'static str { "Residual Income" }
    
    fn render_main(&self, ui: &mut Ui, _data: &Vec<AnalysisMetadataRow>) {
        render_workspace_chart(ui);
    }

    fn render_bottom(&self, ui: &mut Ui, _data: &Vec<AnalysisMetadataRow>) {
        let metrics = vec!["eq", "pat", "debt", "shares"];
        let (years, analysis_map) = get_valuation_maps(&metrics);
        if years.is_empty() { return; }

        egui::ScrollArea::horizontal().id_source("ri_scroll_node").show(ui, |ui| {
            egui::Frame::none().stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 45, 45))).show(ui, |ui| {
                egui::Grid::new("ri_node_grid").striped(true).min_col_width(110.0).spacing(egui::vec2(16.0, 10.0)).show(ui, |ui| {
                    render_horizontal_grid_header(ui, &years, "METRICS / YEARS");
                    render_editable_row(ui, &years, "Total Equity", "eq", |r| r.total_equity.to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Net Profit After Tax", "pat", |r| r.net_profit_after_tax.to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Outstanding Shares", "shares", |r| r.outstanding_shares.to_string(), &analysis_map);
                });
            });
        });
    }
}

// =========================================================================
// MAIN ENTRYPOINT ORCHESTRATION PIPELINE 
// =========================================================================
pub fn draw_analysis_panel(ui: &mut Ui, active_ticker: &str) {
    DataManager::ensure_analysis_data(active_ticker);

    ACTIVE_PANEL_TICKER.with(|ticker| {
        *ticker.borrow_mut() = active_ticker.to_string();
    });

    let tabs: &[&dyn AbstractSubTab<Vec<AnalysisMetadataRow>>] = &[
        &DcfTab,
        &DdmTab,
        &ResidualIncomeTab,
    ];

    draw_nav_canvas_orchestrator(
        ui,
        active_ticker,
        "analysis_metadata",
        "VALUATION ENGINE",
        "analysis_active_tab_id",
        tabs,
    );
}