use egui::{Ui, Color32, RichText};
use backend::commands::memory_pool::CENTRAL_ACTIVE_SLOT;
use crate::core::data_manager::DataManager;
use crate::ui::layouts::canvas::draw_three_zone_canvas;
use backend::database::analysis::{AnalysisMetadataRow, CashFlowMetadataRow};

pub fn draw_analysis_panel(ui: &mut Ui, active_ticker: &str) {
    DataManager::ensure_analysis_data(active_ticker);

    let mut income_rows: Vec<AnalysisMetadataRow> = Vec::new();
    let mut cash_rows: Vec<CashFlowMetadataRow> = Vec::new();

    if let Ok(slot_guard) = CENTRAL_ACTIVE_SLOT.read() {
        if let Some(slot) = slot_guard.as_ref() {
            if slot.ticker == active_ticker.to_uppercase() {
                // Table 1 Downcast
                if let Some(any_ptr) = slot.parsed_tables.get("analysis_metadata") {
                    if let Some(timeline) = any_ptr.downcast_ref::<Vec<AnalysisMetadataRow>>() {
                        income_rows = timeline.clone();
                    }
                }
                // Table 2 Downcast
                if let Some(any_ptr) = slot.parsed_tables.get("cashflow_metadata") {
                    if let Some(timeline) = any_ptr.downcast_ref::<Vec<CashFlowMetadataRow>>() {
                        cash_rows = timeline.clone();
                    }
                }
            }
        }
    }

    let render_main_ledger = |ui: &mut Ui| {
        ui.heading(RichText::new("📊 RECONCILED FINANCIAL DATA MATRIX (SIDE-BY-SIDE)").strong().color(Color32::from_rgb(220, 220, 220)));
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        if income_rows.is_empty() && cash_rows.is_empty() {
            ui.label("No financial records mapped for this ticker.");
        } else {
            egui::ScrollArea::both().id_source("side_by_side_scroll_pane").show(ui, |ui| {
                ui.horizontal(|ui| {
                    
                    // =========================================================================
                    // LEFT COLUMN: INCOME STATEMENT & EQUITY MAPPINGS
                    // =========================================================================
                    ui.vertical(|ui| {
                        ui.label(RichText::new("📂 INCOME & EQUITY MATRIX (DDM CORE)").strong().color(Color32::from_rgb(180, 180, 180)));
                        ui.add_space(4.0);
                        
                        egui::Grid::new("income_matrix_grid")
                            .striped(true)
                            .min_col_width(90.0)
                            .spacing(egui::vec2(16.0, 10.0))
                            .show(ui, |ui| {
                                ui.label(RichText::new("YEAR").strong().color(Color32::from_rgb(140, 140, 140)));
                                ui.label(RichText::new("DIVIDEND PAID").strong().color(Color32::from_rgb(140, 140, 140)));
                                ui.label(RichText::new("BASIC EPS").strong().color(Color32::from_rgb(140, 140, 140)));
                                ui.label(RichText::new("NET PROFIT (AT)").strong().color(Color32::from_rgb(140, 140, 140)));
                                ui.label(RichText::new("TOTAL EQUITY").strong().color(Color32::from_rgb(140, 140, 140)));
                                ui.end_row();

                                for row in &income_rows {
                                    ui.label(format!("{}", row.year));
                                    ui.label(RichText::new(format!("₹ {}", row.dividend_paid)).color(Color32::from_rgb(100, 210, 100)));
                                    ui.label(RichText::new(format!("{:.2}", row.basic_eps)).color(Color32::from_rgb(200, 200, 200)));
                                    ui.label(RichText::new(format!("₹ {}", row.net_profit_after_tax)).color(Color32::from_rgb(100, 180, 240)));
                                    ui.label(RichText::new(format!("₹ {}", row.total_equity)).color(Color32::from_rgb(240, 180, 100)));
                                    ui.end_row();
                                }
                            });
                    });

                    ui.add_space(40.0); // Visual gap spacing dividing the tables
                    ui.separator();
                    ui.add_space(40.0);

                    // =========================================================================
                    // RIGHT COLUMN: CASH FLOW & METRIC-BY-METRIC CAPEX EXTRACTIONS
                    // =========================================================================
                    ui.vertical(|ui| {
                        ui.label(RichText::new("📂 STRUCTURAL CASH FLOWS (DCF CORE)").strong().color(Color32::from_rgb(180, 180, 180)));
                        ui.add_space(4.0);

                        egui::Grid::new("cashflow_matrix_grid")
                            .striped(true)
                            .min_col_width(90.0)
                            .spacing(egui::vec2(16.0, 10.0))
                            .show(ui, |ui| {
                                ui.label(RichText::new("YEAR").strong().color(Color32::from_rgb(140, 140, 140)));
                                ui.label(RichText::new("OPERATING CF").strong().color(Color32::from_rgb(140, 140, 140)));
                                ui.label(RichText::new("CAPEX OUTFLOW").strong().color(Color32::from_rgb(140, 140, 140)));
                                ui.label(RichText::new("CAPEX INFLOW").strong().color(Color32::from_rgb(140, 140, 140)));
                                ui.label(RichText::new("NET CAPEX").strong().color(Color32::from_rgb(140, 140, 140)));
                                ui.label(RichText::new("FREE CASH FLOW").strong().color(Color32::from_rgb(140, 140, 140)));
                                ui.end_row();

                                for row in &cash_rows {
                                    ui.label(format!("{}", row.year));
                                    ui.label(RichText::new(format!("₹ {}", row.operating_cash_flow)).color(Color32::from_rgb(100, 210, 100)));
                                    ui.label(RichText::new(format!("₹ {}", row.capex_outflow)).color(Color32::from_rgb(230, 90, 90)));
                                    ui.label(RichText::new(format!("₹ {}", row.capex_inflow)).color(Color32::from_rgb(100, 180, 240)));
                                    ui.label(RichText::new(format!("₹ {}", row.net_capex)).color(Color32::from_rgb(200, 200, 200)));
                                    ui.label(RichText::new(format!("₹ {}", row.free_cash_flow)).strong().color(Color32::from_rgb(240, 180, 100)));
                                    ui.end_row();
                                }
                            });
                    });

                });
            });
        }
    };

    let render_empty_slot = |_: &mut Ui| {};

    draw_three_zone_canvas(ui, render_main_ledger, render_empty_slot, render_empty_slot);
}