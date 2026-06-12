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
    last_edit_time: f64,
    pending_recalc: bool,
    pending_dcf_update: bool,
    pending_ddm_update: bool,
    pending_rem_update: bool,
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
                // "0" is a mathematically valid entry. We only reject literal empty strings.
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
    let mut extracted_values = Vec::with_capacity(years.len());
    
    let prefix = match storage_slot_key {
        "dcf_metadata" => "dcf",
        "ddm_metadata" => "ddm",
        _ => "rem",
    };

    let mut base_rows: Vec<AnalysisMetadataRow> = Vec::new();
    backend::commands::memory_pool::with_active_table::<Vec<AnalysisMetadataRow>, _, _>("analysis_metadata", |table| {
        base_rows = table.clone();
    });
    let base_map: HashMap<i32, AnalysisMetadataRow> = base_rows.into_iter().map(|r| (r.year, r)).collect();
    
    {
        for &year in years {
            if !check_column_filled(year, tab_metrics) {
                continue;
            }

            // Extract the baseline assumptions directly from the backend row
            let baseline_rf = base_map.get(&year).map(|r| r.dynamic_rf.to_string()).unwrap_or_else(|| "7.0".to_string());
            let baseline_rm = base_map.get(&year).map(|r| r.dynamic_rm.to_string()).unwrap_or_else(|| "12.0".to_string());
            let baseline_gn = base_map.get(&year).map(|r| r.dcf_gn.to_string()).unwrap_or_else(|| "4.5".to_string());
            
            // Route the correct growth metric based on the active tab
            let baseline_g = base_map.get(&year).map(|r| {
                match prefix {
                    "dcf" => r.dcf_g.to_string(),
                    "ddm" => r.ddm_g.to_string(),
                    _ => r.rem_g.to_string(),
                }
            }).unwrap_or_else(|| "10.0".to_string());

            let rf = access_cell_state(year, &format!("{}_rf", prefix), baseline_rf);
            let rm = access_cell_state(year, &format!("{}_rm", prefix), baseline_rm);
            let g  = access_cell_state(year, &format!("{}_g", prefix), baseline_g);
            let gn = access_cell_state(year, &format!("{}_gn", prefix), baseline_gn);

            backend::commands::memory_pool::store_parsed_table(&format!("{}_{}_rf", storage_slot_key, year), vec![rf.clone()]);
            backend::commands::memory_pool::store_parsed_table(&format!("{}_{}_rm", storage_slot_key, year), vec![rm.clone()]);
            backend::commands::memory_pool::store_parsed_table(&format!("{}_{}_g", storage_slot_key, year), vec![g.clone()]);
            backend::commands::memory_pool::store_parsed_table(&format!("{}_{}_gn", storage_slot_key, year), vec![gn.clone()]);

            extracted_values.push((
                year,
                access_cell_state(year, "eps", "".to_string()),
                access_cell_state(year, "pat", "".to_string()),
                access_cell_state(year, "div", "".to_string()),
                access_cell_state(year, "eq", "".to_string()),
                access_cell_state(year, "debt", "".to_string()),    
                access_cell_state(year, "ocf", "".to_string()),
                access_cell_state(year, "capex_out", "".to_string()),
                access_cell_state(year, "capex_in", "".to_string()),
                access_cell_state(year, "shares", "".to_string()),
                access_cell_state(year, "pbt", "".to_string()),
                access_cell_state(year, "interest", "".to_string()),
                access_cell_state(year, "tax_rate", "0.25".to_string()),
                access_cell_state(year, "beta", "1.0".to_string()),
                rf, rm, g, gn,
            ));
        }
    }

    for (year, eps, pat, div, eq, debt, ocf, capex_out, capex_in, shares, pbt, interest, tax, beta, rf, rm, g, gn) in extracted_values {
        // Parse numericals safely
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

        // Reconstruct the row maintaining all original and dynamic data
        let base_row = base_map.get(&year);
        
        master_rows.push(AnalysisMetadataRow {
            year, dividend_paid, basic_eps, net_profit_after_tax, total_equity, total_debt,
            operating_cash_flow, capex_outflow, capex_inflow, net_capex, free_cash_flow,
            outstanding_shares, profit_before_tax, finance_interest_expense, effective_tax_rate,
            nse_beta: user_beta, bse_beta: user_beta, 
            
            average_beta: base_row.map(|r| r.average_beta).unwrap_or(1.0),
            dynamic_rf: rf.parse::<f64>().unwrap_or(7.0),
            dynamic_rm: rm.parse::<f64>().unwrap_or(12.0),
            dcf_g: if prefix == "dcf" { g.parse::<f64>().unwrap_or(10.0) } else { base_row.map(|r| r.dcf_g).unwrap_or(10.0) },
            ddm_g: if prefix == "ddm" { g.parse::<f64>().unwrap_or(5.0) } else { base_row.map(|r| r.ddm_g).unwrap_or(5.0) },
            rem_g: if prefix == "rem" { g.parse::<f64>().unwrap_or(8.0) } else { base_row.map(|r| r.rem_g).unwrap_or(8.0) },
            dcf_gn: gn.parse::<f64>().unwrap_or(4.5),
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
    fn label(&self) -> &'static str { "Discounted Cash Flow (DCF)" }
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
    fn label(&self) -> &'static str { "Dividend Discount Model (DDM)" }
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
    fn label(&self) -> &'static str { "Residual Income" }
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

    let mut trigger_debounced_recalc = false;
    let mut run_dcf = false;
    let mut run_ddm = false;
    let mut run_rem = false;

    INTERACTIVE_CELL_CACHE.with(|cache| {
        let mut c = cache.borrow_mut();
        if c.pending_recalc {
            if ui.input(|i| i.time) - c.last_edit_time > 0.5 { // 500 ms wait
                c.pending_recalc = false;
                trigger_debounced_recalc = true;
                
                run_dcf = c.pending_dcf_update;
                run_ddm = c.pending_ddm_update;
                run_rem = c.pending_rem_update;
                
                c.pending_dcf_update = false;
                c.pending_ddm_update = false;
                c.pending_rem_update = false;
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
    }
}