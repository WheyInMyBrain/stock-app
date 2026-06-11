use egui::{Ui, Color32, RichText};
use backend::commands::memory_pool::CENTRAL_ACTIVE_SLOT;
use crate::core::data_manager::DataManager;
use crate::ui::layouts::canvas::draw_three_zone_canvas;
use backend::database::analysis::AnalysisMetadataRow;

pub fn draw_analysis_panel(ui: &mut Ui, active_ticker: &str) {
    DataManager::ensure_analysis_data(active_ticker);

    let mut data_rows: Vec<AnalysisMetadataRow> = Vec::new();
    if let Ok(slot_guard) = CENTRAL_ACTIVE_SLOT.read() {
        if let Some(slot) = slot_guard.as_ref() {
            if slot.ticker == active_ticker.to_uppercase() {
                if let Some(any_ptr) = slot.parsed_tables.get("analysis_metadata") {
                    if let Some(timeline) = any_ptr.downcast_ref::<Vec<AnalysisMetadataRow>>() {
                        data_rows = timeline.clone();
                    }
                }
            }
        }
    }

    let render_main_ledger = |ui: &mut Ui| {
        ui.heading(RichText::new("📊 RECONCILED HISTORICAL FINANCIAL DATA LEDGER").strong().color(Color32::from_rgb(220, 220, 220)));
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        if data_rows.is_empty() {
            ui.label("No financial records mapped for this ticker.");
        } else {
            egui::ScrollArea::vertical().id_source("analysis_scroll_pane").show(ui, |ui| {
                egui::Grid::new("clean_analysis_grid")
                    .striped(true)
                    .min_col_width(100.0)
                    .spacing(egui::vec2(24.0, 12.0))
                    .show(ui, |ui| {
                        // Headers
                        ui.label(RichText::new("YEAR").strong().color(Color32::from_rgb(140, 140, 140)));
                        ui.label(RichText::new("DIVIDEND PAID").strong().color(Color32::from_rgb(140, 140, 140)));
                        ui.label(RichText::new("BASIC EPS").strong().color(Color32::from_rgb(140, 140, 140)));
                        ui.label(RichText::new("NET PROFIT (AT)").strong().color(Color32::from_rgb(140, 140, 140)));
                        ui.label(RichText::new("TOTAL EQUITY").strong().color(Color32::from_rgb(140, 140, 140)));
                        ui.end_row();

                        // Rows
                        for row in &data_rows {
                            ui.label(format!("{}", row.year));
                            ui.label(RichText::new(format!("₹ {}", row.dividend_paid)).color(Color32::from_rgb(100, 210, 100)));
                            ui.label(RichText::new(format!("{:.2}", row.basic_eps)).color(Color32::from_rgb(200, 200, 200)));
                            ui.label(RichText::new(format!("₹ {}", row.net_profit_after_tax)).color(Color32::from_rgb(100, 180, 240)));
                            ui.label(RichText::new(format!("₹ {}", row.total_equity)).color(Color32::from_rgb(240, 180, 100)));
                            ui.end_row();
                        }
                    });
            });
        }
    };

    let render_empty_slot = |_: &mut Ui| {};

    draw_three_zone_canvas(ui, render_main_ledger, render_empty_slot, render_empty_slot);
}