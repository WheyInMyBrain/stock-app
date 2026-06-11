use egui::Ui;
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use crate::core::data_manager::DataManager;
use crate::ui::layouts::canvas::{OverviewSubTab, draw_nav_canvas_orchestrator};
use backend::database::analysis::AnalysisMetadataRow;
use backend::database::overview::OverviewMetadata;

// =========================================================================
// THREAD-SAFE GLOBAL INTERACTIVE STATE ENGINE
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
                if val.trim().is_empty() || val.trim() == "0" || val.trim() == "0.0" {
                    return false;
                }
            } else {
                return false;
            }
        }
        true
    })
}

// =========================================================================
// BACKEND SLOTS INTEGRATION WRITER (SAVES MERGED SIMULATION STATE VECTOR)
// =========================================================================
pub fn push_interactive_state_to_pool(years: &[i32]) {
    let mut master_rows = Vec::with_capacity(years.len());

    for &year in years {
        let basic_eps = access_cell_state(year, "eps", "0".to_string()).parse::<f64>().unwrap_or(0.0);
        let net_profit_after_tax = access_cell_state(year, "pat", "0".to_string()).parse::<i64>().unwrap_or(0);
        let dividend_paid = access_cell_state(year, "div", "0".to_string()).parse::<i64>().unwrap_or(0);
        let total_equity = access_cell_state(year, "eq", "0".to_string()).parse::<i64>().unwrap_or(0);

        let operating_cash_flow = access_cell_state(year, "ocf", "0".to_string()).parse::<i64>().unwrap_or(0);
        let capex_outflow = access_cell_state(year, "capex_out", "0".to_string()).parse::<i64>().unwrap_or(0);
        let capex_inflow = access_cell_state(year, "capex_in", "0".to_string()).parse::<i64>().unwrap_or(0);
        
        let net_capex = capex_outflow + capex_inflow;
        let free_cash_flow = operating_cash_flow + net_capex;

        master_rows.push(AnalysisMetadataRow {
            year,
            dividend_paid,
            basic_eps,
            net_profit_after_tax,
            total_equity,
            operating_cash_flow,
            capex_outflow,
            capex_inflow,
            net_capex,
            free_cash_flow,
        });
    }

    // Split-clone allocations into individual model targets requested
    backend::commands::memory_pool::store_parsed_table("analysis_metadata", master_rows.clone());
    backend::commands::memory_pool::store_parsed_table("dcf_metadata", master_rows.clone());
    backend::commands::memory_pool::store_parsed_table("ddm_metadata", master_rows.clone());
    backend::commands::memory_pool::store_parsed_table("rem_metadata", master_rows);
}

// =========================================================================
// PIPELINE HYDRATION DELEGATE
// =========================================================================
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

fn sync_frontend_to_backend(years: &[i32]) {
    push_interactive_state_to_pool(years);
}

fn render_horizontal_grid_header(ui: &mut Ui, years: &[i32], title: &str) {
    ui.label(egui::RichText::new(title).strong());
    for year in years {
        ui.label(egui::RichText::new(format!("{}", year)).strong());
    }
    ui.end_row();
}

// =========================================================================
// SUB-TAB 1: DISCOUNTED CASH FLOW INTERACTIVE ENGINE
// =========================================================================
struct DcfTab;
impl OverviewSubTab for DcfTab {
    fn id(&self) -> usize { 0 }
    fn label(&self) -> &'static str { "Discounted Cash Flow (DCF)" }
    fn render_main(&self, _ui: &mut Ui, _meta: &OverviewMetadata) {}
    
    fn render_bottom(&self, ui: &mut Ui, _meta: &OverviewMetadata) {
        let metrics = vec!["ocf", "capex_out", "capex_in"];
        let (years, analysis_map) = get_valuation_maps(&metrics);
        if years.is_empty() { return; }

        egui::ScrollArea::horizontal().id_source("dcf_bottom_scroll").show(ui, |ui| {
            egui::Frame::none()
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 45, 45)))
                .show(ui, |ui| {
                    egui::Grid::new("dcf_interactive_clean_grid")
                        .striped(true)
                        .min_col_width(110.0)
                        .spacing(egui::vec2(16.0, 10.0))
                        .show(ui, |ui| {
                            render_horizontal_grid_header(ui, &years, "METRICS / YEARS");

                            ui.label("Operating Cash Flow");
                            for &yr in &years {
                                let fallback = analysis_map.get(&yr).map(|r| r.operating_cash_flow.to_string()).unwrap_or_else(|| "0".to_string());
                                let mut val_str = access_cell_state(yr, "ocf", fallback);
                                if ui.text_edit_singleline(&mut val_str).changed() {
                                    update_cell_state(yr, "ocf", val_str);
                                    sync_frontend_to_backend(&years);
                                }
                            }
                            ui.end_row();

                            ui.label("Capex Outflow");
                            for &yr in &years {
                                let fallback = analysis_map.get(&yr).map(|r| r.capex_outflow.to_string()).unwrap_or_else(|| "0".to_string());
                                let mut val_str = access_cell_state(yr, "capex_out", fallback);
                                if ui.text_edit_singleline(&mut val_str).changed() {
                                    update_cell_state(yr, "capex_out", val_str);
                                    sync_frontend_to_backend(&years);
                                }
                            }
                            ui.end_row();

                            ui.label("Capex Inflow");
                            for &yr in &years {
                                let fallback = analysis_map.get(&yr).map(|r| r.capex_inflow.to_string()).unwrap_or_else(|| "0".to_string());
                                let mut val_str = access_cell_state(yr, "capex_in", fallback);
                                if ui.text_edit_singleline(&mut val_str).changed() {
                                    update_cell_state(yr, "capex_in", val_str);
                                    sync_frontend_to_backend(&years);
                                }
                            }
                            ui.end_row();
                        });
                });
        });
    }
}

// =========================================================================
// SUB-TAB 2: DIVIDEND DISCOUNT MODEL INTERACTIVE ENGINE
// =========================================================================
struct DdmTab;
impl OverviewSubTab for DdmTab {
    fn id(&self) -> usize { 1 }
    fn label(&self) -> &'static str { "Dividend Discount Model (DDM)" }
    fn render_main(&self, _ui: &mut Ui, _meta: &OverviewMetadata) {}

    fn render_bottom(&self, ui: &mut Ui, _meta: &OverviewMetadata) {
        let metrics = vec!["eps", "pat", "div"];
        let (years, analysis_map) = get_valuation_maps(&metrics);
        if years.is_empty() { return; }

        egui::ScrollArea::horizontal().id_source("ddm_bottom_scroll").show(ui, |ui| {
            egui::Frame::none()
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 45, 45)))
                .show(ui, |ui| {
                    egui::Grid::new("ddm_interactive_clean_grid")
                        .striped(true)
                        .min_col_width(110.0)
                        .spacing(egui::vec2(16.0, 10.0))
                        .show(ui, |ui| {
                            render_horizontal_grid_header(ui, &years, "METRICS / YEARS");

                            ui.label("Basic EPS");
                            for &yr in &years {
                                let fallback = analysis_map.get(&yr).map(|r| format!("{:.2}", r.basic_eps)).unwrap_or_else(|| "0.00".to_string());
                                let mut val_str = access_cell_state(yr, "eps", fallback);
                                if ui.text_edit_singleline(&mut val_str).changed() {
                                    update_cell_state(yr, "eps", val_str);
                                    sync_frontend_to_backend(&years);
                                }
                            }
                            ui.end_row();

                            ui.label("Net Profit (AT)");
                            for &yr in &years {
                                let fallback = analysis_map.get(&yr).map(|r| r.net_profit_after_tax.to_string()).unwrap_or_else(|| "0".to_string());
                                let mut val_str = access_cell_state(yr, "pat", fallback);
                                if ui.text_edit_singleline(&mut val_str).changed() {
                                    update_cell_state(yr, "pat", val_str);
                                    sync_frontend_to_backend(&years);
                                }
                            }
                            ui.end_row();

                            ui.label("Dividend Paid");
                            for &yr in &years {
                                let fallback = analysis_map.get(&yr).map(|r| r.dividend_paid.to_string()).unwrap_or_else(|| "0".to_string());
                                let mut val_str = access_cell_state(yr, "div", fallback);
                                if ui.text_edit_singleline(&mut val_str).changed() {
                                    update_cell_state(yr, "div", val_str);
                                    sync_frontend_to_backend(&years);
                                }
                            }
                            ui.end_row();
                        });
                });
        });
    }
}

// =========================================================================
// SUB-TAB 3: RESIDUAL INCOME MODEL INTERACTIVE ENGINE
// =========================================================================
struct ResidualIncomeTab;
impl OverviewSubTab for ResidualIncomeTab {
    fn id(&self) -> usize { 2 }
    fn label(&self) -> &'static str { "Residual Income" }
    fn render_main(&self, _ui: &mut Ui, _meta: &OverviewMetadata) {}

    fn render_bottom(&self, ui: &mut Ui, _meta: &OverviewMetadata) {
        let metrics = vec!["eq", "pat"];
        let (years, analysis_map) = get_valuation_maps(&metrics);
        if years.is_empty() { return; }

        egui::ScrollArea::horizontal().id_source("ri_bottom_scroll").show(ui, |ui| {
            egui::Frame::none()
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 45, 45)))
                .show(ui, |ui| {
                    egui::Grid::new("ri_interactive_clean_grid")
                        .striped(true)
                        .min_col_width(110.0)
                        .spacing(egui::vec2(16.0, 10.0))
                        .show(ui, |ui| {
                            render_horizontal_grid_header(ui, &years, "METRICS / YEARS");

                            ui.label("Total Equity");
                            for &yr in &years {
                                let fallback = analysis_map.get(&yr).map(|r| r.total_equity.to_string()).unwrap_or_else(|| "0".to_string());
                                let mut val_str = access_cell_state(yr, "eq", fallback);
                                if ui.text_edit_singleline(&mut val_str).changed() {
                                    update_cell_state(yr, "eq", val_str);
                                    sync_frontend_to_backend(&years);
                                }
                            }
                            ui.end_row();

                            ui.label("Net Profit After Tax");
                            for &yr in &years {
                                let fallback = analysis_map.get(&yr).map(|r| r.net_profit_after_tax.to_string()).unwrap_or_else(|| "0".to_string());
                                let mut val_str = access_cell_state(yr, "pat", fallback);
                                if ui.text_edit_singleline(&mut val_str).changed() {
                                    update_cell_state(yr, "pat", val_str);
                                    sync_frontend_to_backend(&years);
                                }
                            }
                            ui.end_row();
                        });
                });
        });
    }
}

// =========================================================================
// MAIN ANALYSIS ENTRYPOINT
// =========================================================================
pub fn draw_analysis_panel(ui: &mut Ui, active_ticker: &str) {
    DataManager::ensure_analysis_data(active_ticker);

    ACTIVE_PANEL_TICKER.with(|ticker| {
        *ticker.borrow_mut() = active_ticker.to_string();
    });

    let tabs: &[&dyn OverviewSubTab] = &[
        &DcfTab,
        &DdmTab,
        &ResidualIncomeTab,
    ];

    draw_nav_canvas_orchestrator(ui, active_ticker, tabs);
}