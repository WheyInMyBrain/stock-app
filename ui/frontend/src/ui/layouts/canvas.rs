use egui::{Ui, Color32, Frame, Margin, Stroke};

pub trait AbstractSubTab<T> {
    fn id(&self) -> usize;
    fn label(&self) -> &'static str;
    fn render_main(&self, ui: &mut Ui, data: &T);
    fn render_bottom(&self, _ui: &mut Ui, _data: &T) {}
}

pub fn draw_three_zone_canvas<M, B, S>(
    ui: &mut Ui,
    render_main: M,
    render_bottom: B,
    render_side: S,
) where
    M: FnOnce(&mut Ui),
    B: FnOnce(&mut Ui),
    S: FnOnce(&mut Ui),
{
    let screen_width_points = ui.ctx().input(|i| i.screen_rect.width());
    let current_zoom = ui.ctx().zoom_factor();
    let unscaled_raw_width = screen_width_points * current_zoom;

    let target_zoom = (unscaled_raw_width / 1366.0).clamp(0.75, 1.20);
    if (current_zoom - target_zoom).abs() > 0.01 {
        ui.ctx().set_zoom_factor(target_zoom);
    }

    let total_width = ui.available_width();
    let total_height = ui.available_height();
    let spacing = ui.spacing().item_spacing;

    let side_width = (total_width * 0.18).clamp(160.0, 260.0);
    let left_width = total_width - side_width - spacing.x;

    let ideal_main_height = left_width * (9.0 / 16.0);
    let max_allowed_main_height = total_height - 130.0 - spacing.y; 
    let main_height = ideal_main_height.min(max_allowed_main_height).max(160.0);
    let bottom_height = total_height - main_height - spacing.y;

    let slot_frame = Frame::none()
        .fill(Color32::from_rgb(14, 14, 14))
        .inner_margin(Margin::same(12.0))
        .stroke(Stroke::new(1.0, Color32::from_rgb(28, 28, 28)));

    ui.horizontal(|ui| {
        ui.allocate_ui(egui::vec2(left_width, total_height), |ui| {
            ui.vertical(|ui| {
                ui.allocate_ui(egui::vec2(left_width, main_height), |ui| {
                    slot_frame.show(ui, |ui| {
                        ui.set_height(ui.available_height());
                        ui.set_width(ui.available_width());
                        render_main(ui);
                    });
                });

                ui.allocate_ui(egui::vec2(left_width, bottom_height), |ui| {
                    slot_frame.show(ui, |ui| {
                        ui.set_height(ui.available_height());
                        ui.set_width(ui.available_width());
                        render_bottom(ui);
                    });
                });
            });
        });

        ui.allocate_ui(egui::vec2(side_width, total_height), |ui| {
            slot_frame.show(ui, |ui| {
                ui.set_height(ui.available_height());
                ui.set_width(ui.available_width());
                render_side(ui);
            });
        });
    });
}

pub fn draw_nav_canvas_orchestrator<T>(
    ui: &mut Ui, 
    active_ticker: &str, 
    table_key: &str,          // "overview_metadata", "analysis_metadata", etc.
    heading_prefix: &str,     // "OVERVIEW", "VALUATION ENGINE", etc.
    id_source_key: &str,      // Unique string token for temporary UI state storage
    tabs: &[&dyn AbstractSubTab<T>]
) 
where 
    T: std::any::Any + Send + Sync, // Requirements to match backend memory slot contracts
{
    if tabs.is_empty() { return; }
    
    // Generate a uniquely distinct state ID token based on the caller context string
    let active_sub_tab_id = egui::Id::new(id_source_key);
    let current_tab_id = ui.data_mut(|d| d.get_temp::<usize>(active_sub_tab_id).unwrap_or(tabs[0].id()));

    let active_tab = tabs.iter().find(|t| t.id() == current_tab_id).unwrap_or(&tabs[0]);

    // Pull from the memory pool utilizing the exact concrete string key passed by the caller
    let table_found = backend::commands::memory_pool::with_active_table::<T, _, _>(table_key, |data| {
        draw_three_zone_canvas(
            ui,
            |ui| {
                ui.heading(format!("{}: {}", heading_prefix.to_uppercase(), active_ticker.to_uppercase()));
                ui.add_space(15.0);
                active_tab.render_main(ui, data);
            },
            |ui| {
                active_tab.render_bottom(ui, data);
            },
            |ui| {
                ui.vertical(|ui| {
                    let button_width = ui.available_width();
                    for tab in tabs {
                        if ui.add_sized(egui::vec2(button_width, 28.0), egui::Button::new(tab.label()).selected(current_tab_id == tab.id())).clicked() {
                            ui.data_mut(|d| d.insert_temp(active_sub_tab_id, tab.id()));
                        }
                        ui.add_space(4.0);
                    }
                });
            },
        );
    });

    // Uniform clean loading layout card fallback boundary execution
    if table_found.is_none() {
        draw_three_zone_canvas(
            ui,
            |ui| {
                ui.heading(format!("{}: {}", heading_prefix.to_uppercase(), active_ticker.to_uppercase()));
                ui.add_space(15.0);
                ui.weak("Loading data attributes into cache slot...");
            },
            |_ui| {},
            |_ui| {},
        );
    }
}