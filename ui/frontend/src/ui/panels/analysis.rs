use egui::{Ui, Color32};
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use crate::core::data_manager::DataManager;
use crate::ui::layouts::canvas::{AbstractSubTab, draw_nav_canvas_orchestrator, paint_abstract_chart_canvas, GenericChartLine, GenericChartPoint, paint_abstract_bar_canvas, GenericBarGroup, GenericBarChartSeries};
use backend::database::analysis::{AnalysisMetadataRow, ValuationResultRow};

// =========================================================================
// THREAD-SAFE CELLS CACHE CONTROL LAYER
// =========================================================================
#[derive(Clone)]
struct DynamicCellCache {
    inputs: HashMap<(i32, String), String>,
    last_edit_time: f64,
    pending_recalc: bool,
    pending_dcf_update: bool,
    pending_ddm_update: bool,
    pending_rem_update: bool,
    pending_epv_update: bool,
    pending_bgvm_update: bool,
    pending_eva_update: bool,
    pending_mc_update: bool,
}

impl Default for DynamicCellCache {
    fn default() -> Self {
        Self { 
            inputs: HashMap::new(),
            last_edit_time: 0.0,
            pending_recalc: false,
            pending_dcf_update: false,
            pending_ddm_update: false,
            pending_rem_update: false,
            pending_epv_update: false,
            pending_bgvm_update: false,
            pending_eva_update: false,
            pending_mc_update: false,
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
                if val.trim().is_empty() { return false; }
            } else { return false; }
        }
        true
    })
}

// =========================================================================
// BACKEND COORDINATION INGESTION WRITER & RETRIEVAL HELPERS
// =========================================================================
pub fn push_interactive_state_to_pool(years: &[i32], tab_metrics: &[&str], storage_slot_key: &str) {
    let mut master_rows = Vec::with_capacity(years.len());
    
    let prefix = match storage_slot_key {
        "dcf_metadata" => "dcf",
        "ddm_metadata" => "ddm",
        "epv_metadata" => "epv",
        "bgvm_metadata" => "bgvm",
        "eva_metadata"  => "eva",
        "monte_carlo_metadata" => "mc",
        _ => "rem",
    };

    let mut base_rows: Vec<AnalysisMetadataRow> = Vec::new();
    backend::commands::memory_pool::with_active_table::<Vec<AnalysisMetadataRow>, _, _>("analysis_metadata", |table| {
        base_rows = table.clone();
    });
    let base_map: HashMap<i32, AnalysisMetadataRow> = base_rows.into_iter().map(|r| (r.year, r)).collect();
    
    for &year in years {
        if storage_slot_key == "monte_carlo_metadata" {
            let days = access_cell_state(year, "mc_days", String::new());
            let sims = access_cell_state(year, "mc_sims", String::new());
            let conf = access_cell_state(year, "mc_conf", String::new());
            let date = access_cell_state(year, "mc_date", String::new());
            let lookback = access_cell_state(year, "mc_lookback", String::new());

            backend::commands::memory_pool::store_parsed_table(&format!("{}_mc_days", storage_slot_key), vec![days]);
            backend::commands::memory_pool::store_parsed_table(&format!("{}_mc_sims", storage_slot_key), vec![sims]);
            backend::commands::memory_pool::store_parsed_table(&format!("{}_mc_conf", storage_slot_key), vec![conf]);
            backend::commands::memory_pool::store_parsed_table(&format!("{}_mc_date", storage_slot_key), vec![date]);
            backend::commands::memory_pool::store_parsed_table(&format!("{}_mc_lookback", storage_slot_key), vec![lookback]);
            continue; 
        }

        if !check_column_filled(year, tab_metrics) {
            continue;
        }
        
        let base_row = match base_map.get(&year) {
            Some(row) => row,
            None => continue, 
        };

        let baseline_g_str = match prefix {
            "dcf" => base_row.dcf_g.to_string(),
            "ddm" => base_row.ddm_g.to_string(),
            _ => base_row.rem_g.to_string(),
        };

        let rf = access_cell_state(year, &format!("{}_rf", prefix), base_row.dynamic_rf.to_string());
        let rm = access_cell_state(year, &format!("{}_rm", prefix), base_row.dynamic_rm.to_string());
        let gn = access_cell_state(year, &format!("{}_gn", prefix), base_row.dcf_gn.to_string());
        let g  = access_cell_state(year, &format!("{}_g", prefix), baseline_g_str);

        backend::commands::memory_pool::store_parsed_table(&format!("{}_{}_rf", storage_slot_key, year), vec![rf.clone()]);
        backend::commands::memory_pool::store_parsed_table(&format!("{}_{}_rm", storage_slot_key, year), vec![rm.clone()]);
        backend::commands::memory_pool::store_parsed_table(&format!("{}_{}_g", storage_slot_key, year), vec![g.clone()]);
        backend::commands::memory_pool::store_parsed_table(&format!("{}_{}_gn", storage_slot_key, year), vec![gn.clone()]);

        let ext_ocf = access_cell_state(year, "ocf", base_row.operating_cash_flow.to_string());
        let ext_capex_out = access_cell_state(year, "capex_out", base_row.capex_outflow.to_string());
        let ext_debt = access_cell_state(year, "debt", base_row.total_debt.to_string());
        let ext_eq = access_cell_state(year, "eq", base_row.total_equity.to_string());
        let ext_shares = access_cell_state(year, "shares", base_row.outstanding_shares.to_string());
        let ext_pbt = access_cell_state(year, "pbt", base_row.profit_before_tax.to_string());
        let ext_pat = access_cell_state(year, "pat", base_row.net_profit_after_tax.to_string());
        let ext_interest = access_cell_state(year, "interest", base_row.finance_interest_expense.to_string());
        let ext_div = access_cell_state(year, "div", base_row.dividend_paid.to_string());

        let operating_cash_flow = ext_ocf.parse::<i64>().unwrap_or(base_row.operating_cash_flow);
        let capex_outflow = ext_capex_out.parse::<i64>().unwrap_or(base_row.capex_outflow);
        let total_debt = ext_debt.parse::<i64>().unwrap_or(base_row.total_debt);
        let total_equity = ext_eq.parse::<i64>().unwrap_or(base_row.total_equity);
        let outstanding_shares = ext_shares.parse::<i64>().unwrap_or(base_row.outstanding_shares);
        let profit_before_tax = ext_pbt.parse::<i64>().unwrap_or(base_row.profit_before_tax);
        let net_profit_after_tax = ext_pat.parse::<i64>().unwrap_or(base_row.net_profit_after_tax);
        let finance_interest_expense = ext_interest.parse::<i64>().unwrap_or(base_row.finance_interest_expense);
        let dividend_paid = ext_div.parse::<i64>().unwrap_or(base_row.dividend_paid);
        let parsed_input_g = g.parse::<f64>().unwrap_or(base_row.dcf_g);

        let net_capex = capex_outflow + base_row.capex_inflow;
        let free_cash_flow = operating_cash_flow + net_capex;

        master_rows.push(AnalysisMetadataRow {
            year, 
            dividend_paid, 
            basic_eps: base_row.basic_eps, 
            net_profit_after_tax, 
            total_equity, 
            total_debt,
            operating_cash_flow, 
            capex_outflow, 
            capex_inflow: base_row.capex_inflow, 
            net_capex, 
            free_cash_flow,
            outstanding_shares, 
            profit_before_tax, 
            finance_interest_expense, 
            effective_tax_rate: base_row.effective_tax_rate,
            nse_beta: base_row.nse_beta, 
            bse_beta: base_row.bse_beta, 
            average_beta: base_row.average_beta,
            
            dynamic_rf: rf.parse::<f64>().unwrap_or(base_row.dynamic_rf),
            dynamic_rm: rm.parse::<f64>().unwrap_or(base_row.dynamic_rm),
            dcf_g: if prefix == "dcf" { parsed_input_g } else { base_row.dcf_g },
            ddm_g: if prefix == "ddm" { parsed_input_g } else { base_row.ddm_g },
            rem_g: if prefix == "rem" { parsed_input_g } else { base_row.rem_g },
            bgvm_g: if prefix == "bgvm" { parsed_input_g } else { base_row.bgvm_g },
            dcf_gn: gn.parse::<f64>().unwrap_or(base_row.dcf_gn),
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

    let mut val_rows: Vec<ValuationResultRow> = Vec::new();
    backend::commands::memory_pool::with_active_table::<Vec<ValuationResultRow>, _, _>(result_slot_key, |table| {
        val_rows = table.clone();
    });

    val_rows.sort_by(|a, b| a.year.cmp(&b.year));
    let mut val_points = Vec::with_capacity(val_rows.len());
    for res in val_rows {
        if res.status_ok && res.intrinsic_value > 0.0 {
            val_points.push(GenericChartPoint { date: format!("{}-03-31", res.year), value: res.intrinsic_value });
        }
    }

    let mut chart_lines = vec![
        GenericChartLine { label: "NSE", color: Color32::from_rgb(250, 210, 50), stroke_width: 1.5, points: nse_points },
        GenericChartLine { label: "BSE", color: Color32::from_rgb(50, 150, 250), stroke_width: 1.5, points: bse_points },
    ];
    if !val_points.is_empty() {
        chart_lines.push(GenericChartLine { label: value_label, color: Color32::from_rgb(50, 220, 120), stroke_width: 2.0, points: val_points });
    }
    paint_abstract_chart_canvas(ui, &chart_lines);
}

/// Generic workspace router that routes your imported canvas structs straight to the painter
fn render_workspace_bar_chart(ui: &mut Ui, series: &GenericBarChartSeries) {

    if series.groups.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.weak("No historical bar metrics mapped to active canvas frame context.");
        });
        return;
    }

    // Pass the pre-built canvas series directly down to the zero-intercept renderer
    paint_abstract_bar_canvas(ui, series);
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
        let fallback = analysis_map.get(yr).map(&extract_fallback).unwrap_or_else(|| "".to_string());
        let mut value_buffer = access_cell_state(*yr, metric_id, fallback);
        
        if ui.add(egui::TextEdit::singleline(&mut value_buffer).desired_width(80.0)).changed() {
            update_cell_state(*yr, metric_id, value_buffer);
            
            INTERACTIVE_CELL_CACHE.with(|cache| {
                let mut c = cache.borrow_mut();
                c.last_edit_time = ui.input(|i| i.time);
                c.pending_recalc = true;
                match storage_slot_key {
                    "dcf_metadata" => c.pending_dcf_update = true,
                    "ddm_metadata" => c.pending_ddm_update = true,
                    "epv_metadata" => c.pending_epv_update = true,
                    "bgvm_metadata" => c.pending_bgvm_update = true,
                    "eva_metadata" => c.pending_eva_update  = true,
                    "monte_carlo_metadata"  => c.pending_mc_update  = true,
                    _ => c.pending_rem_update = true,
                }
            });
        }
    }
    ui.end_row();
}

/// Generic grid-rendering abstraction shared symmetrically by all subtabs
fn render_valuation_matrix_subtab(
    ui: &mut Ui,
    title: &'static str,
    scroll_id: &'static str,
    grid_id: &'static str,
    metadata_slot: &'static str,
    results_slot: &'static str,
    price_row_label: &'static str,
    metrics: Vec<(&'static str, &'static str, Box<dyn Fn(&AnalysisMetadataRow) -> String>)>,
    assumptions: Vec<(&'static str, &'static str, Box<dyn Fn(&AnalysisMetadataRow) -> String>)>,
) {
    let tab_metrics: Vec<&str> = metrics.iter().map(|m| m.1).chain(assumptions.iter().map(|a| a.1)).collect();
    let (years, analysis_map) = get_valuation_maps(&tab_metrics, metadata_slot);
    if years.is_empty() { return; }

    let mut results_rows: Vec<ValuationResultRow> = Vec::new();
    backend::commands::memory_pool::with_active_table::<Vec<ValuationResultRow>, _, _>(results_slot, |table| {
        results_rows = table.clone();
    });
    let results_map: HashMap<i32, ValuationResultRow> = results_rows.into_iter().map(|r| (r.year, r)).collect();

    ui.label(egui::RichText::new(title).strong().size(14.0));
    ui.add_space(4.0);

    egui::ScrollArea::both().id_source(scroll_id).show(ui, |ui| {
        egui::Frame::none().stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 45, 45))).show(ui, |ui| {
            egui::Grid::new(grid_id).striped(true).spacing(egui::vec2(12.0, 8.0)).show(ui, |ui| {
                render_horizontal_grid_header(ui, &years, "METRICS FROM INTEGRATED PARQUETS");
                for (label, id, fallback_extractor) in &metrics {
                    render_editable_row(ui, &years, label, id, metadata_slot, fallback_extractor, &analysis_map);
                }

                ui.separator(); for _ in &years { ui.separator(); } ui.end_row();
                
                render_horizontal_grid_header(ui, &years, "USER FORECAST ASSUMPTIONS");
                for (label, id, fallback_extractor) in &assumptions {
                    render_editable_row(ui, &years, label, id, metadata_slot, fallback_extractor, &analysis_map);
                }

                ui.separator(); for _ in &years { ui.separator(); } ui.end_row();
                ui.label(egui::RichText::new(price_row_label).strong().color(Color32::from_rgb(50, 220, 120)));
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

// =========================================================================
// TAB IMPLEMENTATIONS
// =========================================================================
struct DcfTab;
impl AbstractSubTab<Vec<AnalysisMetadataRow>> for DcfTab {
    fn id(&self) -> usize { 0 }
    fn label(&self) -> &'static str { "Discounted Cash Flow" }
    fn render_main(&self, ui: &mut Ui, _data: &Vec<AnalysisMetadataRow>) { render_workspace_chart(ui, "dcf_calculated_results", "DCF Value"); }
    fn render_bottom(&self, ui: &mut Ui, _data: &Vec<AnalysisMetadataRow>) {
        render_valuation_matrix_subtab(
            ui, "Discounted Cash Flow (TABULAR CALCULATOR)", "dcf_matrix_scroll_area", "dcf_matrix_grid", "dcf_metadata", "dcf_calculated_results", "DCF INTRINSIC VALUE",
            vec![
                ("Operating Cash Flow (OCF)", "ocf", Box::new(|r| r.operating_cash_flow.to_string())),
                ("Capital Expenditure (Capex)", "capex_out", Box::new(|r| r.capex_outflow.to_string())),
                ("Total Debt (Short + Long Term)", "debt", Box::new(|r| r.total_debt.to_string())),
                ("Total Shareholder Equity", "eq", Box::new(|r| r.total_equity.to_string())),
                ("Outstanding Shares Count", "shares", Box::new(|r| r.outstanding_shares.to_string())),
                ("Profit Before Tax (PBT)", "pbt", Box::new(|r| r.profit_before_tax.to_string())),
                ("Net Profit After Tax (PAT)", "pat", Box::new(|r| r.net_profit_after_tax.to_string())),
                ("Finance Interest Expenses", "interest", Box::new(|r| r.finance_interest_expense.to_string())),
            ],
            vec![
                ("Risk Free Rate (Rf)", "dcf_rf", Box::new(|r| r.dynamic_rf.to_string())),
                ("Expected Market Return (Rm)", "dcf_rm", Box::new(|r| r.dynamic_rm.to_string())),
                ("Stage 1 Forecast Growth (g)", "dcf_g", Box::new(|r| r.dcf_g.to_string())),
                ("Terminal Perpetuity Growth (gn)", "dcf_gn", Box::new(|r| r.dcf_gn.to_string())),
            ]
        );
    }
}

struct DdmTab;
impl AbstractSubTab<Vec<AnalysisMetadataRow>> for DdmTab {
    fn id(&self) -> usize { 1 }
    fn label(&self) -> &'static str { "Dividend Discount Model" }
    fn render_main(&self, ui: &mut Ui, _data: &Vec<AnalysisMetadataRow>) { render_workspace_chart(ui, "ddm_calculated_results", "DDM Value"); }
    fn render_bottom(&self, ui: &mut Ui, _data: &Vec<AnalysisMetadataRow>) {
        render_valuation_matrix_subtab(
            ui, "Dividend Discount Model (Gordon Growth Grid)", "ddm_matrix_scroll_area", "ddm_matrix_grid", "ddm_metadata", "ddm_calculated_results", "DDM Intrinsic Share Price",
            vec![
                ("Aggregate Dividend Paid", "div", Box::new(|r| r.dividend_paid.to_string())),
                ("Outstanding Shares", "shares", Box::new(|r| r.outstanding_shares.to_string())),
            ],
            vec![
                ("Risk Free Rate (Rf)", "ddm_rf", Box::new(|r| r.dynamic_rf.to_string())),
                ("Market Premium (Rm)", "ddm_rm", Box::new(|r| r.dynamic_rm.to_string())),
                ("Dividend Growth Rate (g)", "ddm_g", Box::new(|r| r.ddm_g.to_string())),
            ]
        );
    }
}

struct ResidualIncomeTab;
impl AbstractSubTab<Vec<AnalysisMetadataRow>> for ResidualIncomeTab {
    fn id(&self) -> usize { 2 }
    fn label(&self) -> &'static str { "Residual Income Model" }
    fn render_main(&self, ui: &mut Ui, _data: &Vec<AnalysisMetadataRow>) { render_workspace_chart(ui, "rem_calculated_results", "RIM Value"); }
    fn render_bottom(&self, ui: &mut Ui, _data: &Vec<AnalysisMetadataRow>) {
        render_valuation_matrix_subtab(
            ui, "Residual Income Multi-Stage Capital Table", "ri_matrix_scroll_area", "ri_matrix_grid", "rem_metadata", "rem_calculated_results", "RIM Intrinsic Share Price",
            vec![
                ("Total Equity (Book Value)", "eq", Box::new(|r| r.total_equity.to_string())),
                ("Net Profit After Tax (PAT)", "pat", Box::new(|r| r.net_profit_after_tax.to_string())),
                ("Outstanding Shares", "shares", Box::new(|r| r.outstanding_shares.to_string())),
            ],
            vec![
                ("Risk Free Rate (Rf)", "rem_rf", Box::new(|r| r.dynamic_rf.to_string())),
                ("Market Return (Rm)", "rem_rm", Box::new(|r| r.dynamic_rm.to_string())),
                ("Income Growth Forecast (g)", "rem_g", Box::new(|r| r.rem_g.to_string())),
            ]
        );
    }
}

struct EpvTab;
impl AbstractSubTab<Vec<AnalysisMetadataRow>> for EpvTab {
    fn id(&self) -> usize { 3 }
    fn label(&self) -> &'static str { "Earnings Power Value" }
    fn render_main(&self, ui: &mut Ui, _data: &Vec<AnalysisMetadataRow>) { 
        render_workspace_chart(ui, "epv_calculated_results", "EPV Zero-Growth Floor"); 
    }
    fn render_bottom(&self, ui: &mut Ui, _data: &Vec<AnalysisMetadataRow>) {
        render_valuation_matrix_subtab(
            ui, "Earnings Power Value (Bruce Greenwald Matrix)", "epv_matrix_scroll_area", "epv_matrix_grid", "epv_metadata", "epv_calculated_results", "EPV Intrinsic Value",
            vec![
                ("Net Profit After Tax (PAT)", "pat", Box::new(|r| r.net_profit_after_tax.to_string())),
                ("Total Debt (Short + Long Term)", "debt", Box::new(|r| r.total_debt.to_string())),
                ("Total Shareholder Equity", "eq", Box::new(|r| r.total_equity.to_string())),
                ("Outstanding Shares Count", "shares", Box::new(|r| r.outstanding_shares.to_string())),
                ("Profit Before Tax (PBT)", "pbt", Box::new(|r| r.profit_before_tax.to_string())),
                ("Finance Interest Expenses", "interest", Box::new(|r| r.finance_interest_expense.to_string())),
            ],
            vec![
                ("Risk Free Rate (Rf)", "epv_rf", Box::new(|r| r.dynamic_rf.to_string())),
                ("Expected Market Return (Rm)", "epv_rm", Box::new(|r| r.dynamic_rm.to_string())),
            ]
        );
    }
}

struct GrahamTab;
impl AbstractSubTab<Vec<AnalysisMetadataRow>> for GrahamTab {
    fn id(&self) -> usize { 4 }
    fn label(&self) -> &'static str { "Graham Classic Model" }
    fn render_main(&self, ui: &mut Ui, _data: &Vec<AnalysisMetadataRow>) { 
        render_workspace_chart(ui, "bgvm_calculated_results", "Graham Intrinsic Value"); 
    }
    fn render_bottom(&self, ui: &mut Ui, _data: &Vec<AnalysisMetadataRow>) {
        render_valuation_matrix_subtab(
            ui, "Benjamin Graham Formulas Checklist", "bgvm_matrix_scroll_area", "bgvm_matrix_grid", "bgvm_metadata", "bgvm_calculated_results", "Graham Intrinsic Price",
            vec![
                ("Net Profit After Tax (PAT)", "pat", Box::new(|r| r.net_profit_after_tax.to_string())),
                ("Total Equity (Book Value)", "eq", Box::new(|r| r.total_equity.to_string())),
                ("Outstanding Shares Count", "shares", Box::new(|r| r.outstanding_shares.to_string())),
            ],
            vec![
                ("Risk Free Rate (Rf)", "bgvm_rf", Box::new(|r| r.dynamic_rf.to_string())),
                ("Expected Long-Term Growth (g)", "bgvm_g", Box::new(|r| r.rem_g.to_string())),
            ]
        );
    }
}

struct EvaTab;
impl AbstractSubTab<Vec<AnalysisMetadataRow>> for EvaTab {
    fn id(&self) -> usize { 5 }
    fn label(&self) -> &'static str { "Economic Value Added (EVA)" }
    fn render_main(&self, ui: &mut Ui, _data: &Vec<AnalysisMetadataRow>) {
        let mut val_rows: Vec<ValuationResultRow> = Vec::new();
        backend::commands::memory_pool::with_active_table::<Vec<ValuationResultRow>, _, _>("eva_calculated_results", |table| {
            val_rows = table.clone();
        });
        val_rows.sort_by(|a, b| a.year.cmp(&b.year));

        let mut groups = Vec::with_capacity(val_rows.len());
        for res in val_rows {
            groups.push(GenericBarGroup {
                date: format!("{}-03-31", res.year),
                value: res.intrinsic_value,
                label: if res.status_ok {
                    if res.intrinsic_value >= 0.0 { "Wealth Generated".to_string() } else { "Capital Destroyed".to_string() }
                } else {
                    format!("Error: {}", res.error_msg)
                },
            });
        }

        let bar_series = GenericBarChartSeries {
            series_name: "EVA Per Share",
            positive_color: Color32::from_rgb(50, 220, 120),  // Green
            negative_color: Color32::from_rgb(230, 75, 75),   // Red
            groups,
        };

        render_workspace_bar_chart(ui, &bar_series);
    }
    fn render_bottom(&self, ui: &mut Ui, _data: &Vec<AnalysisMetadataRow>) {
        render_valuation_matrix_subtab(
            ui, "Economic Value Added (Capital Allocation Performance Matrix)", "eva_matrix_scroll_area", "eva_matrix_grid", "eva_metadata", "eva_calculated_results", "EVA Per Share",
            vec![
                ("Profit Before Tax (PBT)", "pbt", Box::new(|r| r.profit_before_tax.to_string())),
                ("Net Profit After Tax (PAT)", "pat", Box::new(|r| r.net_profit_after_tax.to_string())),
                ("Total Shareholder Equity", "eq", Box::new(|r| r.total_equity.to_string())),
                ("Total Debt (Short + Long Term)", "debt", Box::new(|r| r.total_debt.to_string())),
                ("Finance Interest Expenses", "interest", Box::new(|r| r.finance_interest_expense.to_string())),
                ("Outstanding Shares Count", "shares", Box::new(|r| r.outstanding_shares.to_string())),
            ],
            vec![
                ("Risk Free Rate (Rf)", "eva_rf", Box::new(|r| r.dynamic_rf.to_string())),
                ("Expected Market Return (Rm)", "eva_rm", Box::new(|r| r.dynamic_rm.to_string())),
            ]
        );
    }
}

struct MonteCarloTab;
impl AbstractSubTab<Vec<AnalysisMetadataRow>> for MonteCarloTab {
    fn id(&self) -> usize { 6 }
    fn label(&self) -> &'static str { "Monte Carlo Simulation" }
    
    fn render_main(&self, ui: &mut Ui, _data: &Vec<AnalysisMetadataRow>) { 
        // 1. Fetch historical data points
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

        let mut chart_lines = vec![
            GenericChartLine { label: "NSE", color: Color32::from_rgb(250, 210, 50), stroke_width: 1.5, points: nse_points },
            GenericChartLine { label: "BSE", color: Color32::from_rgb(50, 150, 250), stroke_width: 1.5, points: bse_points },
        ];

        // 2. Fetch calculated simulation paths from pool
        let mut path_points: Vec<backend::database::analysis::MonteCarloPathPoint> = Vec::new();
        backend::commands::memory_pool::with_active_table::<Vec<backend::database::analysis::MonteCarloPathPoint>, _, _>("monte_carlo_path_results", |table| {
            path_points = table.clone();
        });

        if !path_points.is_empty() {
            // Group step points into vector tracks
            let mut paths_map: std::collections::HashMap<u32, Vec<backend::database::analysis::MonteCarloPathPoint>> = std::collections::HashMap::new();
            for pt in path_points {
                paths_map.entry(pt.path_index).or_default().push(pt);
            }

            // Gather the final terminal prices to establish a distribution scaling frame
            let mut terminal_values = Vec::new();
            for path in paths_map.values() {
                if !path.is_empty() {
                    terminal_values.push(path.last().unwrap().simulated_price);
                }
            }

            let total_paths = terminal_values.len() as f64;
            let avg_terminal_price = if total_paths > 0.0 { terminal_values.iter().sum::<f64>() / total_paths } else { 0.0 };

            for (idx, steps) in paths_map {
                let mut path_points_rendered = Vec::with_capacity(steps.len());
                for step in &steps {
                    path_points_rendered.push(GenericChartPoint {
                        date: step.step_date.clone(), // Uses matched string alignment identifiers directly
                        value: step.simulated_price,
                    });
                }

                // 3. Distribution Gradient Coloring Mechanics
                // Scales path divergence relative to the sample average terminal expected value
                let final_price = steps.last().map(|s| s.simulated_price).unwrap_or(avg_terminal_price);
                
                let path_color = if final_price >= avg_terminal_price {
                    // Trajectory upward: Blend pure green scaling saturation intensity based on divergence depth
                    let divergence_ratio = if avg_terminal_price > 0.0 { ((final_price - avg_terminal_price) / avg_terminal_price).min(1.0) } else { 0.5 };
                    let green_component = (140.0 + (115.0 * divergence_ratio)) as u8; 
                    let alpha_component = (35.0 + (50.0 * divergence_ratio)) as u8;
                    Color32::from_rgba_unmultiplied(40, green_component, 110, alpha_component)
                } else {
                    // Trajectory downward: Blend pure red scaling saturation intensity based on divergence depth
                    let divergence_ratio = if final_price > 0.0 { ((avg_terminal_price - final_price) / final_price).min(1.0) } else { 0.5 };
                    let red_component = (140.0 + (115.0 * divergence_ratio)) as u8;
                    let alpha_component = (35.0 + (50.0 * divergence_ratio)) as u8;
                    Color32::from_rgba_unmultiplied(red_component, 65, 65, alpha_component)
                };

                chart_lines.push(GenericChartLine {
                    label: if idx == 0 { "Simulated Trajectories" } else { "" },
                    color: path_color,
                    stroke_width: 1.2,
                    points: path_points_rendered,
                });
            }
        }

        paint_abstract_chart_canvas(ui, &chart_lines);
    }
    
    fn render_bottom(&self, ui: &mut Ui, _data: &Vec<AnalysisMetadataRow>) {
        let target_year = match _data.last() {
            Some(row) => row.year,
            None => return, 
        };

        let mut chart_rows: Vec<backend::database::analysis::HistoricalChartRow> = Vec::new();
        backend::commands::memory_pool::with_active_table::<Vec<backend::database::analysis::HistoricalChartRow>, _, _>("historical_chart_data", |table| {
            chart_rows = table.clone();
        });

        let absolute_latest_date = match chart_rows.last() {
            Some(row) => row.date.clone(),
            None => return, 
        };

        let is_initialized = INTERACTIVE_CELL_CACHE.with(|cache| {
            cache.borrow().inputs.contains_key(&(target_year, "mc_days".to_string()))
        });

        if !is_initialized {
            update_cell_state(target_year, "mc_date", absolute_latest_date.clone());
            update_cell_state(target_year, "mc_days", "252".to_string());
            update_cell_state(target_year, "mc_sims", "5000".to_string());
            update_cell_state(target_year, "mc_conf", "95".to_string());
            update_cell_state(target_year, "mc_lookback", "252".to_string());
        }

        egui::ScrollArea::vertical()
            .auto_shrink([true; 2]) 
            .show(ui, |ui| {
                // Force a maximum width limit to safeguard the workspace layout from expanding
                ui.allocate_ui(ui.available_size(), |ui| {
                    ui.vertical(|ui| {
                        ui.heading("Stochastic Model Settings");
                        ui.add_space(10.0);

                        egui::Grid::new("mc_interactive_grid")
                            .num_columns(3)
                            .spacing([20.0, 14.0])
                            .show(ui, |ui| {
                                ui.label("Simulation Anchor Date:");
                                let mut current_date = access_cell_state(target_year, "mc_date", String::new());
                                let res_date = ui.add(egui::TextEdit::singleline(&mut current_date).desired_width(85.0));
                                if res_date.changed() {
                                    update_cell_state(target_year, "mc_date", current_date.clone());
                                    INTERACTIVE_CELL_CACHE.with(|cache| {
                                        let mut c = cache.borrow_mut();
                                        c.pending_mc_update = true;
                                        c.pending_recalc = true;
                                        c.last_edit_time = ui.input(|i| i.time);
                                    });
                                }
                                // Fix: Wrap description text cleanly to prevent horizontal layout bloating
                                ui.add(egui::Label::new("Historical cutoff boundary (YYYY-MM-DD)").wrap(true));
                                ui.end_row();

                                ui.label("Historical Lookback Window:");
                                let mut current_lookback = access_cell_state(target_year, "mc_lookback", String::new());
                                let res_lookback = ui.add(egui::TextEdit::singleline(&mut current_lookback).desired_width(60.0));
                                if res_lookback.changed() {
                                    update_cell_state(target_year, "mc_lookback", current_lookback.clone());
                                    INTERACTIVE_CELL_CACHE.with(|cache| {
                                        let mut c = cache.borrow_mut();
                                        c.pending_mc_update = true;
                                        c.pending_recalc = true;
                                        c.last_edit_time = ui.input(|i| i.time);
                                    });
                                }
                                ui.add(egui::Label::new("Trading days context to harvest parameters (e.g., 252, 756)").wrap(true));
                                ui.end_row();

                                ui.label("Forecast Horizon (Days):");
                                let mut current_days = access_cell_state(target_year, "mc_days", String::new());
                                let res_days = ui.add(egui::TextEdit::singleline(&mut current_days).desired_width(60.0));
                                if res_days.changed() {
                                    update_cell_state(target_year, "mc_days", current_days.clone());
                                    INTERACTIVE_CELL_CACHE.with(|cache| {
                                        let mut c = cache.borrow_mut();
                                        c.pending_mc_update = true;
                                        c.pending_recalc = true;
                                        c.last_edit_time = ui.input(|i| i.time);
                                    });
                                }
                                ui.add(egui::Label::new("Days forward to project (e.g., 30, 90, 252)").wrap(true));
                                ui.end_row();

                                ui.label("Total Paths to Simulate:");
                                let mut current_sims = access_cell_state(target_year, "mc_sims", String::new());
                                let res_sims = ui.add(egui::TextEdit::singleline(&mut current_sims).desired_width(60.0));
                                if res_sims.changed() {
                                    update_cell_state(target_year, "mc_sims", current_sims.clone());
                                    INTERACTIVE_CELL_CACHE.with(|cache| {
                                        let mut c = cache.borrow_mut();
                                        c.pending_mc_update = true;
                                        c.pending_recalc = true;
                                        c.last_edit_time = ui.input(|i| i.time);
                                    });
                                }
                                ui.add(egui::Label::new("Iteration count (e.g., 1000, 5000, 10000)").wrap(true));
                                ui.end_row();

                                ui.label("Confidence Percentile (%):");
                                let mut current_conf = access_cell_state(target_year, "mc_conf", String::new());
                                let res_conf = ui.add(egui::TextEdit::singleline(&mut current_conf).desired_width(60.0));
                                if res_conf.changed() {
                                    update_cell_state(target_year, "mc_conf", current_conf.clone());
                                    INTERACTIVE_CELL_CACHE.with(|cache| {
                                        let mut c = cache.borrow_mut();
                                        c.pending_mc_update = true;
                                        c.pending_recalc = true;
                                        c.last_edit_time = ui.input(|i| i.time);
                                    });
                                }
                                ui.add(egui::Label::new("Statistical threshold tail cutoff (e.g., 95, 99)").wrap(true));
                                ui.end_row();
                            });

                        ui.add_space(16.0);
                        ui.separator();
                        ui.add_space(8.0);

                        let mut is_dirty = false;
                        let mut is_awaiting_debounce = false;
                        INTERACTIVE_CELL_CACHE.with(|cache| {
                            let c = cache.borrow();
                            is_dirty = c.pending_mc_update;
                            is_awaiting_debounce = c.pending_recalc;
                        });

                        if is_dirty && !is_awaiting_debounce {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.weak("Waiting for typing to settle...");
                            });
                        } else if is_awaiting_debounce {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.colored_label(Color32::from_rgb(50, 150, 250), "Spawning simulation engines...");
                            });
                            ui.ctx().request_repaint(); 
                        } else {
                            let mut summary_rows: Vec<backend::database::analysis::MonteCarloResultSummary> = Vec::new();
                            backend::commands::memory_pool::with_active_table::<Vec<backend::database::analysis::MonteCarloResultSummary>, _, _>("monte_carlo_summary_results", |table| {
                                summary_rows = table.clone();
                            });

                            if let Some(summary) = summary_rows.first() {
                                if summary.status_ok {
                                    ui.colored_label(Color32::from_rgb(50, 220, 120), "✅ STATUS: Calculation complete. Summary results:");
                                    ui.indent("mc_summary_stats", |ui| {
                                        ui.label(format!("• Expected Terminal Price: {:.2}", summary.expected_value));
                                        ui.label(format!("• Upper Target Boundary: {:.2}", summary.upper_bound));
                                        ui.label(format!("• Lower Support Boundary: {:.2}", summary.lower_bound));
                                    });
                                } else {
                                    ui.colored_label(Color32::from_rgb(230, 75, 75), format!("❌ ENGINE ERROR: {}", summary.error_msg));
                                }
                            } else {
                                INTERACTIVE_CELL_CACHE.with(|cache| {
                                    let mut c = cache.borrow_mut();
                                    c.pending_mc_update = true;
                                    c.pending_recalc = true;
                                });
                                ui.ctx().request_repaint();
                            }
                        }
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
        INTERACTIVE_CELL_CACHE.with(|cache| {
            let mut c = cache.borrow_mut();
            c.inputs.clear();
            c.pending_dcf_update = false;
            c.pending_ddm_update = false;
            c.pending_rem_update = false;
            c.pending_epv_update = false;
            c.pending_bgvm_update = false;
            c.pending_eva_update = false;
            c.pending_mc_update = false;
            c.pending_recalc = false;
        });

        let mut base_data: Vec<AnalysisMetadataRow> = Vec::new();
        backend::commands::memory_pool::with_active_table::<Vec<AnalysisMetadataRow>, _, _>("analysis_metadata", |table| {
            base_data = table.clone();
        });

        if !base_data.is_empty() {
            backend::commands::memory_pool::store_parsed_table("dcf_metadata", base_data.clone());
            backend::commands::memory_pool::store_parsed_table("ddm_metadata", base_data.clone());
            backend::commands::memory_pool::store_parsed_table("rem_metadata", base_data.clone());
            backend::commands::memory_pool::store_parsed_table("epv_metadata", base_data.clone()); 
            backend::commands::memory_pool::store_parsed_table("bgvm_metadata", base_data.clone());
            backend::commands::memory_pool::store_parsed_table("eva_metadata", base_data.clone());
            backend::commands::memory_pool::store_parsed_table("monte_carlo_metadata", base_data.clone());

            backend::commands::analysis_engine::compute_on_fly_valuation(active_ticker, "DCF");
            backend::commands::analysis_engine::compute_on_fly_valuation(active_ticker, "DDM");
            backend::commands::analysis_engine::compute_on_fly_valuation(active_ticker, "REM");
            backend::commands::analysis_engine::compute_on_fly_valuation(active_ticker, "EPV"); 
            backend::commands::analysis_engine::compute_on_fly_valuation(active_ticker, "BGVM");
            backend::commands::analysis_engine::compute_on_fly_valuation(active_ticker, "EVA");
        }
    }

    let tabs: &[&dyn AbstractSubTab<Vec<AnalysisMetadataRow>>] = &[
        &DcfTab,
        &DdmTab,
        &ResidualIncomeTab,
        &EpvTab,
        &GrahamTab,
        &EvaTab,
        &MonteCarloTab,
    ];

    draw_nav_canvas_orchestrator(
        ui, active_ticker, "analysis_metadata", "VALUATION WORKSPACE", "analysis_active_tab_id", tabs
    );

    let mut trigger_debounced_recalc = false;
    let mut run_dcf = false;
    let mut run_ddm = false;
    let mut run_rem = false;
    let mut run_epv = false;
    let mut run_bgvm = false;
    let mut run_eva = false;
    let mut run_mc = false;

    INTERACTIVE_CELL_CACHE.with(|cache| {
        let mut c = cache.borrow_mut();
        if c.pending_recalc {
            if ui.input(|i| i.time) - c.last_edit_time > 0.5 { // 500 ms wait
                c.pending_recalc = false;
                trigger_debounced_recalc = true;
                
                run_dcf = c.pending_dcf_update;
                run_ddm = c.pending_ddm_update;
                run_rem = c.pending_rem_update;
                run_epv = c.pending_epv_update;
                run_bgvm = c.pending_bgvm_update;
                run_eva = c.pending_eva_update;
                run_mc = c.pending_mc_update;
                
                c.pending_dcf_update = false;
                c.pending_ddm_update = false;
                c.pending_rem_update = false;
                c.pending_epv_update = false;
                c.pending_bgvm_update = false;
                c.pending_eva_update = false;
                c.pending_mc_update = false;
            } else {
                ui.ctx().request_repaint(); // Keep frame updates running until timer finishes
            }
        }
    });

    if trigger_debounced_recalc {
        if run_dcf {
            let metrics = vec!["ocf", "capex_out", "debt", "eq", "shares", "pbt", "pat", "interest", "dcf_rf", "dcf_rm", "dcf_g", "dcf_gn"];
            let (years, _) = get_valuation_maps(&metrics, "dcf_metadata");
            push_interactive_state_to_pool(&years, &metrics, "dcf_metadata");
            backend::commands::analysis_engine::compute_on_fly_valuation(active_ticker, "DCF");
        }
        if run_ddm {
            let metrics = vec!["div", "shares", "ddm_rf", "ddm_rm", "ddm_g"];
            let (years, _) = get_valuation_maps(&metrics, "ddm_metadata");
            push_interactive_state_to_pool(&years, &metrics, "ddm_metadata");
            backend::commands::analysis_engine::compute_on_fly_valuation(active_ticker, "DDM");
        }
        if run_rem {
            let metrics = vec!["eq", "pat", "shares", "rem_rf", "rem_rm", "rem_g"];
            let (years, _) = get_valuation_maps(&metrics, "rem_metadata");
            push_interactive_state_to_pool(&years, &metrics, "rem_metadata");
            backend::commands::analysis_engine::compute_on_fly_valuation(active_ticker, "REM");
        }
        if run_epv {
            let metrics = vec!["pat", "debt", "eq", "shares", "pbt", "interest", "epv_rf", "epv_rm"];
            let (years, _) = get_valuation_maps(&metrics, "epv_metadata");
            push_interactive_state_to_pool(&years, &metrics, "epv_metadata");
            backend::commands::analysis_engine::compute_on_fly_valuation(active_ticker, "EPV");
        }
        if run_bgvm {
            let metrics = vec!["pat", "eq", "shares", "bgvm_rf", "bgvm_g"];
            let (years, _) = get_valuation_maps(&metrics, "bgvm_metadata");
            push_interactive_state_to_pool(&years, &metrics, "bgvm_metadata");
            backend::commands::analysis_engine::compute_on_fly_valuation(active_ticker, "BGVM");
        }
        if run_eva {
            let metrics = vec!["pbt", "pat", "eq", "debt", "interest", "shares", "eva_rf", "eva_rm"];
            let (years, _) = get_valuation_maps(&metrics, "eva_metadata");
            push_interactive_state_to_pool(&years, &metrics, "eva_metadata");
            backend::commands::analysis_engine::compute_on_fly_valuation(active_ticker, "EVA");
        }
        if run_mc {
            let metrics = vec!["mc_days", "mc_sims", "mc_conf", "mc_date", "mc_lookback"];
            let mut metadata_rows: Vec<AnalysisMetadataRow> = Vec::new();
            backend::commands::memory_pool::with_active_table::<Vec<AnalysisMetadataRow>, _, _>("analysis_metadata", |table| {
                metadata_rows = table.clone();
            });
            let target_year = match metadata_rows.last() {
                Some(row) => row.year,
                None => return,
            };
            push_interactive_state_to_pool(&[target_year], &metrics, "monte_carlo_metadata");
            backend::commands::analysis_engine::compute_on_fly_valuation(active_ticker, "MONTE_CARLO");
        }
        
    }
}