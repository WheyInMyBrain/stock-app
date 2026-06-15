// stock-app/ui/frontend_native/src/ui/workspace.rs
use egui::{Ui, Color32, RichText};
use crate::ui::panels::add_ticker::draw_add_ticker_panel;
use crate::ui::panels::WorkspaceTab;

pub fn draw_main_workspace_viewport(ui: &mut Ui, sidebar_open: &mut bool, active_ticker: &str) {
    let spacing_backup = ui.spacing().clone();
    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
    ui.spacing_mut().window_margin = egui::Margin::same(0.0);

    ui.vertical(|ui| {
        if active_ticker.is_empty() {
            egui::Frame::none()
                .fill(Color32::from_rgb(12, 12, 12))
                .inner_margin(egui::Margin::symmetric(16.0, 10.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if !*sidebar_open {
                            if ui.button(" ☰ MENU ").clicked() { *sidebar_open = true; }
                            ui.add_space(10.0);
                        }
                        ui.heading(" STOCK-APP ");
                    });
                });
            
            ui.separator();
            draw_add_ticker_panel(ui);
        } else {
            let tab_id = ui.id().with("workspace_active_sub_tab_pointer");
            
            let mut current_tab = ui.data_mut(|d| {
                d.get_temp::<WorkspaceTab>(tab_id)
                    .unwrap_or(WorkspaceTab::ALL[0])
            });

            egui::Frame::none()
                .fill(Color32::from_rgb(18, 18, 18))
                .inner_margin(egui::Margin::symmetric(16.0, 10.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if !*sidebar_open {
                            if ui.button(" ☰ MENU ").clicked() { *sidebar_open = true; }
                            ui.add_space(14.0);
                        }
                        
                        ui.label(RichText::new(format!("Active Ticker: {} ", active_ticker)).strong().color(Color32::WHITE));
                        ui.add_space(40.0);
                        
                        for tab in WorkspaceTab::ALL {
                            if ui.selectable_value(&mut current_tab, *tab, tab.label()).changed() {
                                ui.data_mut(|d| d.insert_temp(tab_id, current_tab));
                            }
                            ui.add_space(16.0);
                        }
                    });
                });

            ui.separator();

            current_tab.render(ui, active_ticker);
        }
    });

    *ui.spacing_mut() = spacing_backup;
}