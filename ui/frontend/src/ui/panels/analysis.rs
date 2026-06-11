use egui::{Ui, Color32, Pos2, Stroke, Vec2};
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use crate::core::data_manager::DataManager;
use crate::ui::layouts::canvas::{AbstractSubTab, draw_nav_canvas_orchestrator};
use backend::database::analysis::AnalysisMetadataRow;

// =========================================================================
// THREAD-SAFE GLOBAL INTERACTIVE STATE ENGINE
// =========================================================================
#[derive(Clone)]
struct DynamicCellCache {
    inputs: HashMap<(i32, String), String>,
    chart_zoom: f32,
    chart_offset: f32,
}

impl Default for DynamicCellCache {
    fn default() -> Self {
        Self { 
            inputs: HashMap::new(),
            chart_zoom: 1.0,
            chart_offset: 0.0,
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

// =========================================================================
// INTERACTIVE ENGINE VALUE-STITCHED POOL SYNC BACKEND
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
// HIGH-PERFORMANCE NATIVE CANVAS DUAL PRICE CHART ENGINE
// =========================================================================
fn render_horizontal_grid_header(ui: &mut Ui, years: &[i32], title: &str) {
    ui.label(egui::RichText::new(title).strong());
    for year in years {
        ui.label(egui::RichText::new(format!("{}", year)).strong());
    }
    ui.end_row();
}

fn draw_historical_chart_canvas(ui: &mut Ui) {
    let mut entries: Vec<backend::database::analysis::HistoricalChartRow> = Vec::new();
    
    backend::commands::memory_pool::with_active_table::<Vec<backend::database::analysis::HistoricalChartRow>, _, _>(
        "historical_chart_data", 
        |table| {
            entries = table.clone();
        }
    );

    if entries.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.weak("No historical price ticks compiled in memory slot for tracking.");
        });
        return;
    }

    // Allocate basic static non-interactive painter real estate
    let desired_size = Vec2::new(ui.available_width(), ui.available_height() - 10.0);
    let (response, painter) = ui.allocate_painter(desired_size, egui::Sense::hover());
    let rect = response.rect;

    // Draw frame canvas card borders
    painter.rect_filled(rect, 4.0, Color32::from_rgb(10, 10, 10));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0, Color32::from_rgb(25, 25, 25)));

    let total_points = entries.len();
    if total_points < 2 { return; }

    // =========================================================================
    // MATRIX PLANE BOUNDARY EVALUATION
    // =========================================================================
    let mut min_price = f64::MAX;
    let mut max_price = f64::MIN;

    for row in &entries {
        if let Some(p) = row.nse_close {
            if p < min_price { min_price = p; }
            if p > max_price { max_price = p; }
        }
        if let Some(p) = row.bse_close {
            if p < min_price { min_price = p; }
            if p > max_price { max_price = p; }
        }
    }

    if min_price >= max_price { min_price = 0.0; max_price = 100.0; }
    let price_range = max_price - min_price;
    min_price -= price_range * 0.05;
    max_price += price_range * 0.05;

    // Flat fully-fitted coordinate projection mapping closures (Zero Zoom/Pan modifiers)
    let map_to_screen = |index: usize, price: f64| -> Pos2 {
        let pct_x = (index as f32) / ((total_points - 1) as f32);
        let screen_x = rect.left() + pct_x * rect.width();

        let pct_y = ((price - min_price) / (max_price - min_price)) as f32;
        let screen_y = rect.bottom() - (pct_y * rect.height());
        Pos2::new(screen_x, screen_y)
    };

    let map_to_index = |screen_x: f32| -> i32 {
        let pct_x = (screen_x - rect.left()) / rect.width();
        (pct_x * (total_points - 1) as f32).round() as i32
    };

    // =========================================================================
    // HORIZONTAL PRICE LEVEL GRID LINES
    // =========================================================================
    let grid_stroke = Stroke::new(1.0, Color32::from_rgb(20, 20, 20));
    for i in 1..4 {
        let y_pos = rect.top() + (rect.height() * 0.25 * (i as f32));
        painter.line_segment([Pos2::new(rect.left(), y_pos), Pos2::new(rect.right(), y_pos)], grid_stroke);
        
        let label_price = max_price - ((max_price - min_price) * 0.25 * (i as f64));
        painter.text(
            Pos2::new(rect.left() + 8.0, y_pos - 6.0),
            egui::Align2::LEFT_TOP,
            format!("₹ {:.1}", label_price),
            egui::FontId::proportional(11.0),
            Color32::from_rgb(100, 100, 100)
        );
    }

    // =========================================================================
    // TIMELINE YEAR-START ANCHORED MARKERS (1st JAN CHRONOLOGY DIVIDERS)
    // =========================================================================
    let mut tracked_years: HashSet<String> = HashSet::new();

    for (idx, row) in entries.iter().enumerate() {
        if row.date.len() >= 4 {
            let year_string = row.date[0..4].to_string();
            // Isolate the earliest transaction entry encountering a new year boundary block
            if !tracked_years.contains(&year_string) {
                tracked_years.insert(year_string.clone());
                
                let marker_pos = map_to_screen(idx, min_price);
                
                // Draw vertical layout boundary slice lines
                painter.line_segment(
                    [Pos2::new(marker_pos.x, rect.top()), Pos2::new(marker_pos.x, rect.bottom() - 6.0)],
                    Stroke::new(0.5, Color32::from_rgb(18, 18, 18))
                );
                
                painter.line_segment(
                    [Pos2::new(marker_pos.x, rect.bottom() - 6.0), Pos2::new(marker_pos.x, rect.bottom())],
                    Stroke::new(1.0, Color32::from_rgb(34, 34, 34))
                );

                // Print clean year digits text exclusively
                painter.text(
                    Pos2::new(marker_pos.x, rect.bottom() - 14.0),
                    egui::Align2::CENTER_BOTTOM,
                    &year_string,
                    egui::FontId::proportional(10.0),
                    Color32::from_rgb(70, 70, 70)
                );
            }
        }
    }

    let clip_rect = rect;

    // Track 1: Draw NSE Core Financial String Vectors (Yellow)
    let nse_stroke = Stroke::new(1.5, Color32::from_rgb(250, 210, 50));
    let mut last_nse_point: Option<Pos2> = None;
    for (idx, row) in entries.iter().enumerate() {
        if let Some(price) = row.nse_close {
            let current_pos = map_to_screen(idx, price);
            if let Some(last_pos) = last_nse_point {
                painter.with_clip_rect(clip_rect).line_segment([last_pos, current_pos], nse_stroke);
            }
            last_nse_point = Some(current_pos);
        }
    }

    // Track 2: Draw BSE Core Financial String Vectors (Blue)
    let bse_stroke = Stroke::new(1.5, Color32::from_rgb(50, 150, 250));
    let mut last_bse_point: Option<Pos2> = None;
    for (idx, row) in entries.iter().enumerate() {
        if let Some(price) = row.bse_close {
            let current_pos = map_to_screen(idx, price);
            if let Some(last_pos) = last_bse_point {
                painter.with_clip_rect(clip_rect).line_segment([last_pos, current_pos], bse_stroke);
            }
            last_bse_point = Some(current_pos);
        }
    }

    // =========================================================================
    // HOVER INSIGHT CROSSHAIR RADAR LAYER (SNAP TO AXIS LAYOUTS)
    // =========================================================================
    if let Some(pointer_pos) = ui.ctx().pointer_latest_pos() {
        if rect.contains(pointer_pos) {
            let target_idx = map_to_index(pointer_pos.x);
            
            if target_idx >= 0 && target_idx < total_points as i32 {
                let row = &entries[target_idx as usize];
                
                painter.line_segment(
                    [Pos2::new(pointer_pos.x, rect.top()), Pos2::new(pointer_pos.x, rect.bottom())],
                    Stroke::new(1.0, Color32::from_rgb(50, 50, 50))
                );

                // 1. Snap Date Badge over Bottom Horizontal Timeline Axis
                let date_badge_rect = egui::Rect::from_center_size(
                    Pos2::new(pointer_pos.x, rect.bottom() - 14.0),
                    Vec2::new(75.0, 18.0)
                );
                painter.rect_filled(date_badge_rect, 2.0, Color32::from_rgb(30, 30, 30));
                painter.text(
                    date_badge_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    &row.date,
                    egui::FontId::proportional(11.0),
                    Color32::from_rgb(255, 255, 255)
                );

                // 2. Left Pricing Vertical Axis Snap Callouts
                let mut label_offset_y = 0.0;
                
                if let Some(nse_p) = row.nse_close {
                    let screen_pos = map_to_screen(target_idx as usize, nse_p);
                    painter.circle_filled(screen_pos, 4.0, Color32::from_rgb(250, 210, 50));
                    
                    let axis_tag_rect = egui::Rect::from_min_size(
                        Pos2::new(rect.left() + 4.0, screen_pos.y - 9.0),
                        Vec2::new(85.0, 18.0)
                    );
                    painter.rect_filled(axis_tag_rect, 2.0, Color32::from_rgb(250, 210, 50));
                    painter.text(
                        axis_tag_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        format!("NSE: ₹{:.1}", nse_p),
                        egui::FontId::proportional(10.0),
                        Color32::from_rgb(0, 0, 0)
                    );
                    label_offset_y += 22.0;
                }

                if let Some(bse_p) = row.bse_close {
                    let screen_pos = map_to_screen(target_idx as usize, bse_p);
                    painter.circle_filled(screen_pos, 4.0, Color32::from_rgb(50, 150, 250));
                    
                    let axis_tag_rect = egui::Rect::from_min_size(
                        Pos2::new(rect.left() + 4.0, screen_pos.y - 9.0 + label_offset_y),
                        Vec2::new(85.0, 18.0)
                    );
                    painter.rect_filled(axis_tag_rect, 2.0, Color32::from_rgb(50, 150, 250));
                    painter.text(
                        axis_tag_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        format!("BSE: ₹{:.1}", bse_p),
                        egui::FontId::proportional(10.0),
                        Color32::from_rgb(255, 255, 255)
                    );
                }
            }
        }
    }
}

// =========================================================================
// REUSABLE MODERN MODULAR FIELD PRIMITIVES (DEDUPLICATES ALL ROWS)
// =========================================================================
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

// =========================================================================
// ABSTRACT IMPLEMENTATIONS: INTRINSIC ESTIMATION CANVAS HOOKS
// =========================================================================
struct DcfTab;
impl AbstractSubTab<Vec<AnalysisMetadataRow>> for DcfTab {
    fn id(&self) -> usize { 0 }
    fn label(&self) -> &'static str { "Discounted Cash Flow (DCF)" }
    
    fn render_main(&self, ui: &mut Ui, _data: &Vec<AnalysisMetadataRow>) {
        draw_historical_chart_canvas(ui);
    }
    
    fn render_bottom(&self, ui: &mut Ui, _data: &Vec<AnalysisMetadataRow>) {
        let metrics = vec!["ocf", "capex_out", "capex_in", "debt", "shares"];
        let (years, analysis_map) = get_valuation_maps(&metrics);
        if years.is_empty() { return; }

        egui::ScrollArea::horizontal().id_source("dcf_scroll_node").show(ui, |ui| {
            egui::Frame::none().stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 45, 45))).show(ui, |ui| {
                egui::Grid::new("dcf_node_grid").striped(true).min_col_width(110.0).spacing(egui::vec2(16.0, 10.0)).show(ui, |ui| {
                    render_horizontal_grid_header(ui, &years, "METRICS / YEARS");
                    render_editable_row(ui, &years, "Operating Cash Flow", "ocf", |r| r.operating_cash_flow.to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Capex Outflow", "capex_out", |r| r.capex_outflow.to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Capex Inflow", "capex_in", |r| r.capex_inflow.to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Total Outstanding Debt", "debt", |r| r.total_debt.to_string(), &analysis_map);
                    render_editable_row(ui, &years, "Outstanding Shares", "shares", |r| r.outstanding_shares.to_string(), &analysis_map);
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
        draw_historical_chart_canvas(ui);
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
        draw_historical_chart_canvas(ui);
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
        "analysis_metadata",       // Backend shared cache table lookup key string
        "VALUATION ENGINE",        // Header structural canvas view context prefix
        "analysis_active_tab_id",  // Unique token driving UI index storage
        tabs,
    );
}