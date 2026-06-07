// stock-app/ui/frontend_native/src/ui/workspace.rs
use egui::Ui;
use crate::ui::panels::add_ticker::draw_add_ticker_panel; // 🚀 Link the isolated component

pub fn draw_main_workspace_viewport(ui: &mut Ui, sidebar_open: &mut bool, active_ticker: &str) {
    let spacing_backup = ui.spacing().clone();
    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
    ui.spacing_mut().window_margin = egui::Margin::same(0.0);

    ui.vertical(|ui| {
        // Control Toolbar Header Row
        ui.horizontal(|ui| {
            if !*sidebar_open {
                if ui.button(" ☰ MENU ").clicked() {
                    *sidebar_open = true;
                }
                ui.add_space(10.0);
            }
            ui.heading(" TERMINAL VIEWPORT ENGINE CORE ");
        });
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(15.0);

        // 🎯 THE CONDITIONAL VIEWPORT ROUTER
        if active_ticker.is_empty() {
            // Render input layout when nothing is selected
            draw_add_ticker_panel(ui);
        } else {
            // Render active tracking workspace once focused
            ui.centered_and_justified(|ui| {
                ui.label(format!("ACTIVE SYMBOL MONITOR TARGET Focus: {}", active_ticker));
            });
        }
    });

    *ui.spacing_mut() = spacing_backup;
}