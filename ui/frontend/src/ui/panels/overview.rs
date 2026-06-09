use egui::Ui;
use crate::core::data_manager::DataManager;

pub fn draw_overview_panel(ui: &mut Ui, active_ticker: &str) {
    DataManager::ensure_overview_data(active_ticker);

    let show_details_id = egui::Id::new("overview_details_toggle");
    let show_details = ui.data_mut(|d| d.get_temp::<bool>(show_details_id).unwrap_or(false));

    crate::ui::layouts::canvas::draw_three_zone_canvas(
        ui,
        |ui| {
            ui.heading(format!("OVERVIEW: {}", active_ticker.to_uppercase()));
            ui.add_space(15.0);

            if show_details {
                let mut error_msg = None;
                let _ = backend::commands::memory_pool::with_active_table::<String, _, _>("overview_metadata__error", |err| {
                    if !err.is_empty() {
                        error_msg = Some(err.clone());
                    }
                });

                if let Some(err) = error_msg {
                    ui.colored_label(egui::Color32::LIGHT_RED, format!("❌ Error: {}", err));
                } else {
                    let table_found = backend::commands::memory_pool::with_active_table::<backend::database::overview::OverviewMetadata, _, _>("overview_metadata", |meta| {
                        ui.vertical(|ui| {
                            ui.label(format!("Macro Category: {}", meta.macro_category));
                            ui.label(format!("Sector: {}", meta.sector));
                            ui.label(format!("Industry: {}", meta.industry));
                        });
                    });

                    if table_found.is_none() {
                        ui.weak("Loading data attributes into cache slot...");
                    }
                }
            }
        },
        |_ui| {},
        |ui| {
            ui.vertical(|ui| {
                let button_width = ui.available_width();
                
                if ui.add_sized(egui::vec2(button_width, 28.0), egui::Button::new("Details").selected(show_details)).clicked() {
                    ui.data_mut(|d| d.insert_temp(show_details_id, !show_details));
                }
            });
        },
    );
}