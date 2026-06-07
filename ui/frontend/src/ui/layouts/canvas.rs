// stock-app/ui/frontend_native/src/ui/layouts/canvas.rs
use egui::{Ui, Color32, Frame, Margin, Stroke};

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