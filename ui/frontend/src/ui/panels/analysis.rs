use egui::{Ui, Color32};
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap, HashSet};
use crate::core::data_manager::DataManager;
use crate::ui::layouts::canvas::{AbstractSubTab, draw_nav_canvas_orchestrator, paint_abstract_chart_canvas, GenericChartLine, GenericChartPoint, paint_abstract_bar_canvas, GenericBarGroup, GenericBarChartSeries};
use backend::database::analysis::{AnalysisMetadataRow, ValuationResultRow, DcfMcPercentileSummary};

// =========================================================================
// THREAD-SAFE CELLS CACHE CONTROL LAYER
// =========================================================================
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModelType {
    Dcf,
    Ddm,
    Rem,
    Epv,
    Bgvm,
    Eva,
    MonteCarlo,
    DcfMonteCarlo,
}

impl ModelType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Dcf => "DCF",
            Self::Ddm => "DDM",
            Self::Rem => "REM",
            Self::Epv => "EPV",
            Self::Bgvm => "BGVM",
            Self::Eva => "EVA",
            Self::MonteCarlo => "MONTE_CARLO",
            Self::DcfMonteCarlo => "DCF_MONTE_CARLO",
        }
    }

    pub fn metadata_key(&self) -> &'static str {
        match self {
            Self::Dcf => "dcf_metadata",
            Self::Ddm => "ddm_metadata",
            Self::Rem => "rem_metadata",
            Self::Epv => "epv_metadata",
            Self::Bgvm => "bgvm_metadata",
            Self::Eva => "eva_metadata",
            Self::MonteCarlo => "monte_carlo_metadata",
            Self::DcfMonteCarlo => "dcfmc_metadata",
        }
    }

    pub fn metrics(&self) -> &'static [&'static str] {
        match self {
            Self::Dcf => &[
                "dcf_ocf", "dcf_capex_out", "dcf_debt", "dcf_eq", 
                "dcf_shares", "dcf_pbt", "dcf_pat", "dcf_interest", 
                "dcf_rf", "dcf_rm", "dcf_g", "dcf_gn"
            ],
            Self::Ddm => &[
                "ddm_div", "ddm_shares", "ddm_rf", "ddm_rm", "ddm_g"
            ],
            Self::Rem => &[
                "rem_eq", "rem_pat", "rem_shares", "rem_rf", "rem_rm", "rem_g"
            ],
            Self::Epv => &[
                "epv_pat", "epv_debt", "epv_eq", "epv_shares", 
                "epv_pbt", "epv_interest", "epv_rf", "epv_rm"
            ],
            Self::Bgvm => &[
                "bgvm_pat", "bgvm_eq", "bgvm_shares", "bgvm_rf", "bgvm_g"
            ],
            Self::Eva => &[
                "eva_pbt", "eva_pat", "eva_eq", "eva_debt", 
                "eva_interest", "eva_shares", "eva_rf", "eva_rm"
            ],
            Self::MonteCarlo => &[
                "mc_days", "mc_sims", "mc_conf", "mc_date", "mc_lookback"
            ],
            Self::DcfMonteCarlo => &[
                "dcfmc_ocf", "dcfmc_capex_out", "dcfmc_debt", "dcfmc_eq", "dcfmc_shares", 
                "dcfmc_pbt", "dcfmc_pat", "dcfmc_interest", "dcfmc_rf", "dcfmc_rm", 
                "dcfmc_gn", "dcfmc_sims"
            ],
        }
    }
}

// A much cleaner, unified cache state representation
#[derive(Clone, Default)]
struct DynamicCellCache {
    inputs: HashMap<(i32, String), String>,
    last_edit_time: f64,
    pending_recalc: bool,
    pending_updates: HashSet<ModelType>, 
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
    // 1. Calculate clean string prefixes ONCE outside the loop to avoid heavy heap churn
    let prefix = storage_slot_key.replace("_metadata", "");

    if storage_slot_key == "monte_carlo_metadata" {
        for &year in years {
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
        }
        return;
    }

    // Single lock allocation strategy - reserve bounds safely
    let mut base_map: HashMap<i32, AnalysisMetadataRow> = HashMap::new();
    backend::commands::memory_pool::with_active_table::<Vec<AnalysisMetadataRow>, _, _>("analysis_metadata", |table| {
        base_map.reserve(table.len());
        for row in table {
            base_map.insert(row.year, row.clone());
        }
    });

    let mut master_rows = Vec::with_capacity(years.len());
    let mut key_buf = String::with_capacity(64);
    
    if storage_slot_key == "dcfmc_metadata" {
        for &year in years {
            let ocf = access_cell_state(year, "dcfmc_ocf", base_map.get(&year).map(|r| r.operating_cash_flow.to_string()).unwrap_or_default());
            let capex = access_cell_state(year, "dcfmc_capex_out", base_map.get(&year).map(|r| r.capex_outflow.to_string()).unwrap_or_default());
            let debt = access_cell_state(year, "dcfmc_debt", base_map.get(&year).map(|r| r.total_debt.to_string()).unwrap_or_default());
            let eq = access_cell_state(year, "dcfmc_eq", base_map.get(&year).map(|r| r.total_equity.to_string()).unwrap_or_default());
            let shares = access_cell_state(year, "dcfmc_shares", base_map.get(&year).map(|r| r.outstanding_shares.to_string()).unwrap_or_default());
            let pbt = access_cell_state(year, "dcfmc_pbt", base_map.get(&year).map(|r| r.profit_before_tax.to_string()).unwrap_or_default());
            let pat = access_cell_state(year, "dcfmc_pat", base_map.get(&year).map(|r| r.net_profit_after_tax.to_string()).unwrap_or_default());
            let interest = access_cell_state(year, "dcfmc_interest", base_map.get(&year).map(|r| r.finance_interest_expense.to_string()).unwrap_or_default());
            let div = access_cell_state(year, "dcfmc_div", base_map.get(&year).map(|r| r.dividend_paid.to_string()).unwrap_or_default());
            
            let rf = access_cell_state(year, "dcfmc_rf", base_map.get(&year).map(|r| r.dynamic_rf.to_string()).unwrap_or_default());
            let rm = access_cell_state(year, "dcfmc_rm", base_map.get(&year).map(|r| r.dynamic_rm.to_string()).unwrap_or_default());
            let gn = access_cell_state(year, "dcfmc_gn", base_map.get(&year).map(|r| r.terminal_gn.to_string()).unwrap_or_default());
            let sims = access_cell_state(year, "dcfmc_sims", "5000".to_string());

            backend::commands::memory_pool::store_parsed_table(&format!("{}_{}_rf", storage_slot_key, year), vec![rf.clone()]);
            backend::commands::memory_pool::store_parsed_table(&format!("{}_{}_rm", storage_slot_key, year), vec![rm.clone()]);
            backend::commands::memory_pool::store_parsed_table(&format!("{}_{}_gn", storage_slot_key, year), vec![gn.clone()]);
            backend::commands::memory_pool::store_parsed_table(&format!("{}_{}_sims", storage_slot_key, year), vec![sims.clone()]);

            update_cell_state(year, "dcfmc_ocf", ocf.clone());
            update_cell_state(year, "dcfmc_capex_out", capex.clone());
            update_cell_state(year, "dcfmc_debt", debt.clone());
            update_cell_state(year, "dcfmc_eq", eq.clone());
            update_cell_state(year, "dcfmc_shares", shares.clone());
            update_cell_state(year, "dcfmc_pbt", pbt.clone());
            update_cell_state(year, "dcfmc_pat", pat.clone());
            update_cell_state(year, "dcfmc_interest", interest.clone());
            update_cell_state(year, "dcfmc_div", div.clone());
            update_cell_state(year, "dcfmc_rf", rf.clone());
            update_cell_state(year, "dcfmc_rm", rm.clone());
            update_cell_state(year, "dcfmc_gn", gn.clone());
            update_cell_state(year, "dcfmc_sims", sims.clone());

            let base_row = match base_map.get(&year) { Some(r) => r, None => continue };
            
            let parsed_ocf = ocf.parse::<i64>().unwrap_or(base_row.operating_cash_flow);
            let parsed_capex = capex.parse::<i64>().unwrap_or(base_row.capex_outflow);

            master_rows.push(AnalysisMetadataRow {
                year,
                dividend_paid: div.parse::<i64>().unwrap_or(base_row.dividend_paid),
                basic_eps: base_row.basic_eps,
                net_profit_after_tax: pat.parse::<i64>().unwrap_or(base_row.net_profit_after_tax),
                total_equity: eq.parse::<i64>().unwrap_or(base_row.total_equity),
                total_debt: debt.parse::<i64>().unwrap_or(base_row.total_debt),
                operating_cash_flow: parsed_ocf,
                capex_outflow: parsed_capex,
                capex_inflow: base_row.capex_inflow,
                net_capex: parsed_capex + base_row.capex_inflow,
                free_cash_flow: parsed_ocf + (parsed_capex + base_row.capex_inflow),
                outstanding_shares: shares.parse::<i64>().unwrap_or(base_row.outstanding_shares),
                profit_before_tax: pbt.parse::<i64>().unwrap_or(base_row.profit_before_tax),
                finance_interest_expense: interest.parse::<i64>().unwrap_or(base_row.finance_interest_expense),
                effective_tax_rate: base_row.effective_tax_rate,
                nse_beta: base_row.nse_beta,
                bse_beta: base_row.bse_beta,
                average_beta: base_row.average_beta,
                dynamic_rf: rf.parse::<f64>().unwrap_or(base_row.dynamic_rf),
                dynamic_rm: rm.parse::<f64>().unwrap_or(base_row.dynamic_rm),
                sustainable_g: base_row.sustainable_g,
                terminal_gn: gn.parse::<f64>().unwrap_or(base_row.terminal_gn),
            });
        }
        backend::commands::memory_pool::store_parsed_table(storage_slot_key, master_rows);
        return;
    }

    for &year in years {
        if !check_column_filled(year, tab_metrics) { continue; }
        let base_row = match base_map.get(&year) {
            Some(row) => row,
            None => continue, 
        };

        // Unified helper closure using pre-calculated `prefix` references
        let mut process_cell = |suffix: &str, default: String, store_suffix: Option<&str>| -> String {
            key_buf.clear();
            use std::fmt::Write;
            let _ = write!(key_buf, "{}_{}", prefix, suffix);
            
            let val = access_cell_state(year, &key_buf, default);
            if let Some(st_suffix) = store_suffix {
                key_buf.clear();
                let _ = write!(key_buf, "{}_{}_{}", storage_slot_key, year, st_suffix);
                backend::commands::memory_pool::store_parsed_table(&key_buf, vec![val.clone()]);
            }
            val
        };

        let rf = process_cell("rf", base_row.dynamic_rf.to_string(), Some("rf"));
        let rm = process_cell("rm", base_row.dynamic_rm.to_string(), Some("rm"));
        let g  = process_cell("g",  base_row.sustainable_g.to_string(), Some("g"));
        let gn = process_cell("gn", base_row.terminal_gn.to_string(), Some("gn"));

        let operating_cash_flow = process_cell("ocf", base_row.operating_cash_flow.to_string(), None).parse::<i64>().unwrap_or(base_row.operating_cash_flow);
        let capex_outflow = process_cell("capex_out", base_row.capex_outflow.to_string(), None).parse::<i64>().unwrap_or(base_row.capex_outflow);
        let total_debt = process_cell("debt", base_row.total_debt.to_string(), None).parse::<i64>().unwrap_or(base_row.total_debt);
        let total_equity = process_cell("eq", base_row.total_equity.to_string(), None).parse::<i64>().unwrap_or(base_row.total_equity);
        let outstanding_shares = process_cell("shares", base_row.outstanding_shares.to_string(), None).parse::<i64>().unwrap_or(base_row.outstanding_shares);
        let profit_before_tax = process_cell("pbt", base_row.profit_before_tax.to_string(), None).parse::<i64>().unwrap_or(base_row.profit_before_tax);
        let net_profit_after_tax = process_cell("pat", base_row.net_profit_after_tax.to_string(), None).parse::<i64>().unwrap_or(base_row.net_profit_after_tax);
        let finance_interest_expense = process_cell("interest", base_row.finance_interest_expense.to_string(), None).parse::<i64>().unwrap_or(base_row.finance_interest_expense);
        let dividend_paid = process_cell("div", base_row.dividend_paid.to_string(), None).parse::<i64>().unwrap_or(base_row.dividend_paid);

        let net_capex = capex_outflow + base_row.capex_inflow;

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
            free_cash_flow: operating_cash_flow + net_capex,
            outstanding_shares, 
            profit_before_tax, 
            finance_interest_expense, 
            effective_tax_rate: base_row.effective_tax_rate,
            nse_beta: base_row.nse_beta, 
            bse_beta: base_row.bse_beta, 
            average_beta: base_row.average_beta,
            dynamic_rf: rf.parse::<f64>().unwrap_or(base_row.dynamic_rf),
            dynamic_rm: rm.parse::<f64>().unwrap_or(base_row.dynamic_rm),
            sustainable_g: g.parse::<f64>().unwrap_or(base_row.sustainable_g),
            terminal_gn: gn.parse::<f64>().unwrap_or(base_row.terminal_gn),
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
        // Optimization: Use an empty string literal to avoid unnecessary heap allocations
        let fallback = analysis_map.get(yr).map(&extract_fallback).unwrap_or_default();
        let mut value_buffer = access_cell_state(*yr, metric_id, fallback);
        
        if ui.add(egui::TextEdit::singleline(&mut value_buffer).desired_width(80.0)).changed() {
            update_cell_state(*yr, metric_id, value_buffer);
            
            // Map the storage string down to our new type-safe enum variants
            let target_model = match storage_slot_key {
                "dcf_metadata" => ModelType::Dcf,
                "ddm_metadata" => ModelType::Ddm,
                "epv_metadata" => ModelType::Epv,
                "bgvm_metadata" => ModelType::Bgvm,
                "eva_metadata"  => ModelType::Eva,
                "monte_carlo_metadata" => ModelType::MonteCarlo,
                "dcfmc_metadata" => ModelType::DcfMonteCarlo,
                _ => ModelType::Rem,
            };

            INTERACTIVE_CELL_CACHE.with(|cache| {
                let mut c = cache.borrow_mut();
                c.last_edit_time = ui.input(|i| i.time);
                c.pending_recalc = true;
                c.pending_updates.insert(target_model);
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
                ("Operating Cash Flow (OCF)", "dcf_ocf", Box::new(|r| r.operating_cash_flow.to_string())),
                ("Capital Expenditure (Capex)", "dcf_capex_out", Box::new(|r| r.capex_outflow.to_string())),
                ("Total Debt (Short + Long Term)", "dcf_debt", Box::new(|r| r.total_debt.to_string())),
                ("Total Shareholder Equity", "dcf_eq", Box::new(|r| r.total_equity.to_string())),
                ("Outstanding Shares Count", "dcf_shares", Box::new(|r| r.outstanding_shares.to_string())),
                ("Profit Before Tax (PBT)", "dcf_pbt", Box::new(|r| r.profit_before_tax.to_string())),
                ("Net Profit After Tax (PAT)", "dcf_pat", Box::new(|r| r.net_profit_after_tax.to_string())),
                ("Finance Interest Expenses", "dcf_interest", Box::new(|r| r.finance_interest_expense.to_string())),
            ],
            vec![
                ("Risk Free Rate (Rf)", "dcf_rf", Box::new(|r| r.dynamic_rf.to_string())),
                ("Expected Market Return (Rm)", "dcf_rm", Box::new(|r| r.dynamic_rm.to_string())),
                ("Stage 1 Forecast Growth (g)", "dcf_g", Box::new(|r| r.sustainable_g.to_string())),
                ("Terminal Perpetuity Growth (gn)", "dcf_gn", Box::new(|r| r.terminal_gn.to_string())),
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
                ("Aggregate Dividend Paid", "ddm_div", Box::new(|r| r.dividend_paid.to_string())),
                ("Outstanding Shares", "ddm_shares", Box::new(|r| r.outstanding_shares.to_string())),
            ],
            vec![
                ("Risk Free Rate (Rf)", "ddm_rf", Box::new(|r| r.dynamic_rf.to_string())),
                ("Market Premium (Rm)", "ddm_rm", Box::new(|r| r.dynamic_rm.to_string())),
                ("Dividend Growth Rate (g)", "ddm_g", Box::new(|r| r.sustainable_g.to_string())),
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
                ("Total Equity (Book Value)", "rem_eq", Box::new(|r| r.total_equity.to_string())),
                ("Net Profit After Tax (PAT)", "rem_pat", Box::new(|r| r.net_profit_after_tax.to_string())),
                ("Outstanding Shares", "rem_shares", Box::new(|r| r.outstanding_shares.to_string())),
            ],
            vec![
                ("Risk Free Rate (Rf)", "rem_rf", Box::new(|r| r.dynamic_rf.to_string())),
                ("Market Return (Rm)", "rem_rm", Box::new(|r| r.dynamic_rm.to_string())),
                ("Income Growth Forecast (g)", "rem_g", Box::new(|r| r.sustainable_g.to_string())),
            ]
        );
    }
}

struct EpvTab;
impl AbstractSubTab<Vec<AnalysisMetadataRow>> for EpvTab {
    fn id(&self) -> usize { 3 }
    fn label(&self) -> &'static str { "Earnings Power Value" }
    fn render_main(&self, ui: &mut Ui, _data: &Vec<AnalysisMetadataRow>) { render_workspace_chart(ui, "epv_calculated_results", "EPV Zero-Growth Floor"); }
    fn render_bottom(&self, ui: &mut Ui, _data: &Vec<AnalysisMetadataRow>) {
        render_valuation_matrix_subtab(
            ui, "Earnings Power Value (Bruce Greenwald Matrix)", "epv_matrix_scroll_area", "epv_matrix_grid", "epv_metadata", "epv_calculated_results", "EPV Intrinsic Value",
            vec![
                ("Net Profit After Tax (PAT)", "epv_pat", Box::new(|r| r.net_profit_after_tax.to_string())),
                ("Total Debt (Short + Long Term)", "epv_debt", Box::new(|r| r.total_debt.to_string())),
                ("Total Shareholder Equity", "epv_eq", Box::new(|r| r.total_equity.to_string())),
                ("Outstanding Shares Count", "epv_shares", Box::new(|r| r.outstanding_shares.to_string())),
                ("Profit Before Tax (PBT)", "epv_pbt", Box::new(|r| r.profit_before_tax.to_string())),
                ("Finance Interest Expenses", "epv_interest", Box::new(|r| r.finance_interest_expense.to_string())),
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
    fn render_main(&self, ui: &mut Ui, _data: &Vec<AnalysisMetadataRow>) { render_workspace_chart(ui, "bgvm_calculated_results", "Graham Intrinsic Value"); }
    fn render_bottom(&self, ui: &mut Ui, _data: &Vec<AnalysisMetadataRow>) {
        render_valuation_matrix_subtab(
            ui, "Benjamin Graham Formulas Checklist", "bgvm_matrix_scroll_area", "bgvm_matrix_grid", "bgvm_metadata", "bgvm_calculated_results", "Graham Intrinsic Price",
            vec![
                ("Net Profit After Tax (PAT)", "bgvm_pat", Box::new(|r| r.net_profit_after_tax.to_string())),
                ("Total Equity (Book Value)", "bgvm_eq", Box::new(|r| r.total_equity.to_string())),
                ("Outstanding Shares Count", "bgvm_shares", Box::new(|r| r.outstanding_shares.to_string())),
            ],
            vec![
                ("Risk Free Rate (Rf)", "bgvm_rf", Box::new(|r| r.dynamic_rf.to_string())),
                ("Expected Long-Term Growth (g)", "bgvm_g", Box::new(|r| r.sustainable_g.to_string())),
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
            positive_color: Color32::from_rgb(50, 220, 120),
            negative_color: Color32::from_rgb(230, 75, 75),
            groups,
        };
        render_workspace_bar_chart(ui, &bar_series);
    }
    fn render_bottom(&self, ui: &mut Ui, _data: &Vec<AnalysisMetadataRow>) {
        render_valuation_matrix_subtab(
            ui, "Economic Value Added (Capital Allocation Performance Matrix)", "eva_matrix_scroll_area", "eva_matrix_grid", "eva_metadata", "eva_calculated_results", "EVA Per Share",
            vec![
                ("Profit Before Tax (PBT)", "eva_pbt", Box::new(|r| r.profit_before_tax.to_string())),
                ("Net Profit After Tax (PAT)", "eva_pat", Box::new(|r| r.net_profit_after_tax.to_string())),
                ("Total Shareholder Equity", "eva_eq", Box::new(|r| r.total_equity.to_string())),
                ("Total Debt (Short + Long Term)", "eva_debt", Box::new(|r| r.total_debt.to_string())),
                ("Finance Interest Expenses", "eva_interest", Box::new(|r| r.finance_interest_expense.to_string())),
                ("Outstanding Shares Count", "eva_shares", Box::new(|r| r.outstanding_shares.to_string())),
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
                        date: step.step_date.clone(),
                        value: step.simulated_price,
                    });
                }

                // 3. Distribution Gradient Coloring Mechanics
                let final_price = steps.last().map(|s| s.simulated_price).unwrap_or(avg_terminal_price);
                
                let path_color = if final_price >= avg_terminal_price {
                    let divergence_ratio = if avg_terminal_price > 0.0 { ((final_price - avg_terminal_price) / avg_terminal_price).min(1.0) } else { 0.5 };
                    let green_component = (140.0 + (115.0 * divergence_ratio)) as u8; 
                    let alpha_component = (35.0 + (50.0 * divergence_ratio)) as u8;
                    Color32::from_rgba_unmultiplied(40, green_component, 110, alpha_component)
                } else {
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
                                        c.pending_updates.insert(ModelType::MonteCarlo);
                                        c.pending_recalc = true;
                                        c.last_edit_time = ui.input(|i| i.time);
                                    });
                                }
                                ui.add(egui::Label::new("Historical cutoff boundary (YYYY-MM-DD)").wrap(true));
                                ui.end_row();

                                ui.label("Historical Lookback Window:");
                                let mut current_lookback = access_cell_state(target_year, "mc_lookback", String::new());
                                let res_lookback = ui.add(egui::TextEdit::singleline(&mut current_lookback).desired_width(60.0));
                                if res_lookback.changed() {
                                    update_cell_state(target_year, "mc_lookback", current_lookback.clone());
                                    INTERACTIVE_CELL_CACHE.with(|cache| {
                                        let mut c = cache.borrow_mut();
                                        c.pending_updates.insert(ModelType::MonteCarlo);
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
                                        c.pending_updates.insert(ModelType::MonteCarlo);
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
                                        c.pending_updates.insert(ModelType::MonteCarlo);
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
                                        c.pending_updates.insert(ModelType::MonteCarlo);
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
                            is_dirty = c.pending_updates.contains(&ModelType::MonteCarlo);
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
                                    c.pending_updates.insert(ModelType::MonteCarlo);
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

pub struct DcfMonteCarloTab;
impl AbstractSubTab<Vec<AnalysisMetadataRow>> for DcfMonteCarloTab {
    fn id(&self) -> usize { 7 }
    fn label(&self) -> &'static str { "Stochastic DCF" }

    fn render_main(&self, ui: &mut Ui, _data: &Vec<AnalysisMetadataRow>) { 
        let mut results = Vec::new();
        
        backend::commands::memory_pool::with_active_table::<Vec<DcfMcPercentileSummary>, _, _>(
            "dcfmc_calculated_results", 
            |table| { results = table.clone(); }
        );

        if results.is_empty() {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.weak("Simulating multi-percentile parallel distribution paths...");
            });
            return;
        }

        // 1. Unpack baseline raw historical stock exchange market close trends
        let mut entries: Vec<backend::database::analysis::HistoricalChartRow> = Vec::new();
        backend::commands::memory_pool::with_active_table::<Vec<backend::database::analysis::HistoricalChartRow>, _, _>("historical_chart_data", |table| {
            entries = table.clone();
        });

        let mut nse_points = Vec::with_capacity(entries.len());
        let mut bse_points = Vec::with_capacity(entries.len());
        for row in &entries {
            if let Some(val) = row.nse_close { nse_points.push(GenericChartPoint { date: row.date.clone(), value: val }); }
            if let Some(val) = row.bse_close { bse_points.push(GenericChartPoint { date: row.date.clone(), value: val }); }
        }

        let mut chart_lines = vec![
            GenericChartLine { label: "NSE", color: Color32::from_rgb(250, 210, 50), stroke_width: 1.5, points: nse_points },
            GenericChartLine { label: "BSE", color: Color32::from_rgb(50, 150, 250), stroke_width: 1.5, points: bse_points },
        ];

        // 2. Unpack Monte Carlo statistical summaries mapped chronologically year-by-year
        let mut p100_pts  = Vec::with_capacity(results.len());
        let mut p97_5_pts = Vec::with_capacity(results.len());
        let mut p95_pts   = Vec::with_capacity(results.len());
        let mut p90_pts   = Vec::with_capacity(results.len());
        let mut p75_pts   = Vec::with_capacity(results.len());
        let mut p50_pts   = Vec::with_capacity(results.len());
        let mut p25_pts   = Vec::with_capacity(results.len());
        let mut p10_pts   = Vec::with_capacity(results.len());
        let mut p5_pts    = Vec::with_capacity(results.len());
        let mut p2_5_pts  = Vec::with_capacity(results.len());
        let mut p0_pts    = Vec::with_capacity(results.len());
        let mut avg_pts   = Vec::with_capacity(results.len());

        for step in &results {
            // Match the summary node year against our historical data entries to find its true YYYY-MM-DD signature
            let exact_chart_date = entries
                .iter()
                .find(|row| row.date.starts_with(&step.year.to_string()))
                .map(|row| row.date.clone())
                .unwrap_or_else(|| format!("{}-03-31", step.year));

            p100_pts.push(GenericChartPoint  { date: exact_chart_date.clone(), value: step.p100 });
            p97_5_pts.push(GenericChartPoint { date: exact_chart_date.clone(), value: step.p97_5 });
            p95_pts.push(GenericChartPoint   { date: exact_chart_date.clone(), value: step.p95 });
            p90_pts.push(GenericChartPoint   { date: exact_chart_date.clone(), value: step.p90 });
            p75_pts.push(GenericChartPoint   { date: exact_chart_date.clone(), value: step.p75 });
            p50_pts.push(GenericChartPoint   { date: exact_chart_date.clone(), value: step.p50 });
            p25_pts.push(GenericChartPoint   { date: exact_chart_date.clone(), value: step.p25 });
            p10_pts.push(GenericChartPoint   { date: exact_chart_date.clone(), value: step.p10 });
            p5_pts.push(GenericChartPoint    { date: exact_chart_date.clone(), value: step.p5 });
            p2_5_pts.push(GenericChartPoint  { date: exact_chart_date.clone(), value: step.p2_5 });
            p0_pts.push(GenericChartPoint    { date: exact_chart_date.clone(), value: step.p0 });
            avg_pts.push(GenericChartPoint   { date: exact_chart_date,         value: step.average });
        }

        // 3. Assemble tracks using highly differentiated color paths
        chart_lines.extend(vec![
            GenericChartLine { label: "Max (P100)", color: Color32::from_rgb(34, 139, 34), stroke_width: 1.0, points: p100_pts },
            GenericChartLine { label: "P97.5", color: Color32::from_rgb(46, 139, 87), stroke_width: 1.0, points: p97_5_pts },
            GenericChartLine { label: "P95", color: Color32::from_rgb(60, 179, 113), stroke_width: 1.0, points: p95_pts },
            GenericChartLine { label: "P90", color: Color32::from_rgb(50, 205, 50), stroke_width: 1.2, points: p90_pts },
            GenericChartLine { label: "P75", color: Color32::from_rgb(0, 191, 255), stroke_width: 1.2, points: p75_pts },
            GenericChartLine { label: "Median (P50)", color: Color32::from_rgb(30, 144, 255), stroke_width: 2.2, points: p50_pts },
            GenericChartLine { label: "Average", color: Color32::from_rgb(255, 215, 0), stroke_width: 1.8, points: avg_pts },
            GenericChartLine { label: "P25", color: Color32::from_rgb(138, 43, 226), stroke_width: 1.2, points: p25_pts },
            GenericChartLine { label: "P10", color: Color32::from_rgb(218, 112, 214), stroke_width: 1.2, points: p10_pts },
            GenericChartLine { label: "P5", color: Color32::from_rgb(255, 69, 0), stroke_width: 1.0, points: p5_pts },
            GenericChartLine { label: "P2.5", color: Color32::from_rgb(220, 20, 60), stroke_width: 1.0, points: p2_5_pts },
            GenericChartLine { label: "Min (P0)", color: Color32::from_rgb(139, 0, 0), stroke_width: 1.0, points: p0_pts },
        ]);

        egui::ScrollArea::vertical()
            .id_source("stochastic_dcf_canvas_scroll")
            .auto_shrink([true; 2])
            .show(ui, |ui| {
                paint_abstract_chart_canvas(ui, &chart_lines);
            });
    }
    
    fn render_bottom(&self, ui: &mut Ui, data: &Vec<AnalysisMetadataRow>) {
        if let Some(first_row) = data.first() {
            let is_initialized = INTERACTIVE_CELL_CACHE.with(|cache| {
                cache.borrow().inputs.contains_key(&(first_row.year, "dcfmc_sims".to_string()))
            });
            if !is_initialized {
                for row in data {
                    update_cell_state(row.year, "dcfmc_sims", "5000".to_string());
                }
            }
        }

        render_valuation_matrix_subtab(
            ui, 
            "Stochastic Monte Carlo DCF (PROBABILITY CALCULATOR)", 
            "dcfmc_matrix_scroll_area", 
            "dcfmc_matrix_grid", 
            "dcfmc_metadata", 
            "dcfmc_calculated_results",
            "STOCHASTIC EXPECTED VALUE",
            vec![
                ("Operating Cash Flow (OCF)", "dcfmc_ocf", Box::new(|r| r.operating_cash_flow.to_string())),
                ("Capital Expenditure (Capex)", "dcfmc_capex_out", Box::new(|r| r.capex_outflow.to_string())),
                ("Total Debt (Short + Long Term)", "dcfmc_debt", Box::new(|r| r.total_debt.to_string())),
                ("Total Shareholder Equity", "dcfmc_eq", Box::new(|r| r.total_equity.to_string())),
                ("Outstanding Shares Count", "dcfmc_shares", Box::new(|r| r.outstanding_shares.to_string())),
                ("Profit Before Tax (PBT)", "dcfmc_pbt", Box::new(|r| r.profit_before_tax.to_string())),
                ("Net Profit After Tax (PAT)", "dcfmc_pat", Box::new(|r| r.net_profit_after_tax.to_string())),
                ("Finance Interest Expenses", "dcfmc_interest", Box::new(|r| r.finance_interest_expense.to_string())),
            ],
            vec![
                ("Risk Free Rate (Rf)", "dcfmc_rf", Box::new(|r| r.dynamic_rf.to_string())),
                ("Expected Market Return (Rm)", "dcfmc_rm", Box::new(|r| r.dynamic_rm.to_string())),
                ("Terminal Perpetuity Growth (gn)", "dcfmc_gn", Box::new(|r| r.terminal_gn.to_string())),
                ("Number of Simulations", "dcfmc_sims", Box::new(|_| "5000".to_string())),
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
            active_ticker.clone_into(&mut *t);
            initial_sync_triggered = true;
        }
    });

    const ALL_MODELS: [ModelType; 8] = [
        ModelType::Dcf, ModelType::Ddm, ModelType::Rem, 
        ModelType::Epv, ModelType::Bgvm, ModelType::Eva, ModelType::MonteCarlo,
        ModelType::DcfMonteCarlo
    ];

    if initial_sync_triggered {
        INTERACTIVE_CELL_CACHE.with(|cache| {
            let mut c = cache.borrow_mut();
            c.inputs.clear();
            c.pending_recalc = false;
            c.pending_updates.clear();
        });

        let mut base_data: Vec<AnalysisMetadataRow> = Vec::new();
        backend::commands::memory_pool::with_active_table::<Vec<AnalysisMetadataRow>, _, _>("analysis_metadata", |table| {
            base_data = table.clone();
        });

        if !base_data.is_empty() {
            for model in &ALL_MODELS {
                backend::commands::memory_pool::store_parsed_table(model.metadata_key(), base_data.clone());
                if *model == ModelType::DcfMonteCarlo {
                    let years: Vec<i32> = base_data.iter().map(|r| r.year).collect();
                    push_interactive_state_to_pool(&years, model.metrics(), model.metadata_key());
                    backend::commands::analysis_engine::compute_on_fly_valuation(active_ticker, model.as_str());
                } else if *model != ModelType::MonteCarlo {
                    backend::commands::analysis_engine::compute_on_fly_valuation(active_ticker, model.as_str());
                }
            }
        }
    }

    let tabs: &[&dyn AbstractSubTab<Vec<AnalysisMetadataRow>>] = &[
        &DcfTab, &DdmTab, &ResidualIncomeTab, &EpvTab, &GrahamTab, &EvaTab, &MonteCarloTab, &DcfMonteCarloTab,
    ];

    draw_nav_canvas_orchestrator(
        ui, active_ticker, "analysis_metadata", "VALUATION WORKSPACE", "analysis_active_tab_id", tabs
    );

    let mut models_to_recalc = Vec::new();

    INTERACTIVE_CELL_CACHE.with(|cache| {
        let mut c = cache.borrow_mut();
        if c.pending_recalc {
            if ui.input(|i| i.time) - c.last_edit_time > 0.5 { // 500 ms debounce filter
                c.pending_recalc = false;
                models_to_recalc = c.pending_updates.drain().collect::<Vec<_>>();
            } else {
                ui.ctx().request_repaint(); 
            }
        }
    });

    for model in models_to_recalc {
        if model == ModelType::MonteCarlo {
            let mut metadata_rows: Vec<AnalysisMetadataRow> = Vec::new();
            backend::commands::memory_pool::with_active_table::<Vec<AnalysisMetadataRow>, _, _>("analysis_metadata", |table| {
                metadata_rows = table.clone();
            });
            if let Some(row) = metadata_rows.last() {
                push_interactive_state_to_pool(&[row.year], model.metrics(), model.metadata_key());
                backend::commands::analysis_engine::compute_on_fly_valuation(active_ticker, model.as_str());
            }
        } else if model == ModelType::DcfMonteCarlo {
            let mut metadata_rows: Vec<AnalysisMetadataRow> = Vec::new();
            backend::commands::memory_pool::with_active_table::<Vec<AnalysisMetadataRow>, _, _>(model.metadata_key(), |table| {
                metadata_rows = table.clone();
            });
            let years: Vec<i32> = metadata_rows.iter().map(|r| r.year).collect();
            push_interactive_state_to_pool(&years, model.metrics(), model.metadata_key());
            backend::commands::analysis_engine::compute_on_fly_valuation(active_ticker, model.as_str());
        } else {
            let (years, _) = get_valuation_maps(model.metrics(), model.metadata_key());
            push_interactive_state_to_pool(&years, model.metrics(), model.metadata_key());
            backend::commands::analysis_engine::compute_on_fly_valuation(active_ticker, model.as_str());
        }
    }
}