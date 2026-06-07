// stock-app/ui/frontend_native/src/ui/sidebar.rs
use egui::{Ui, Color32, Stroke};

pub fn draw_retractable_sidebar(
    ui: &mut Ui, 
    sidebar_open: &mut bool, 
    active_ticker: &mut String,
    tickers: &[String], // Accept the live dynamic ticker list from system RAM
) {
    ui.vertical(|ui| {
        ui.add_space(10.0);
        ui.heading(" 🖥️ DATA CORE ");
        ui.add_space(5.0);
        
        let rect = ui.max_rect();
        ui.painter().line_segment(
            [egui::pos2(rect.min.x, ui.cursor().min.y), egui::pos2(rect.max.x, ui.cursor().min.y)],
            Stroke::new(1.0, Color32::from_rgb(255, 255, 255)),
        );
        ui.add_space(15.0);

        // 1. Updated Category Section Title
        ui.label("TARGET TICKERS:");
        ui.add_space(6.0);

        // 2. Self-Contained Search Query State Persistence
        let search_id = ui.id().with("ticker_search_query");
        let mut search_query = ui.data_mut(|d| d.get_temp::<String>(search_id).unwrap_or_default());

        // High-contrast flat search text input widget
        let search_response = ui.add(
            egui::TextEdit::singleline(&mut search_query)
                .hint_text("Filter tickers...")
                .desired_width(ui.available_width() - 10.0)
        );
        
        if search_response.changed() {
            ui.data_mut(|d| d.insert_temp(search_id, search_query.clone()));
        }

        ui.add_space(10.0);

        // 4. Case-Insensitive Filter Pass (Maintains original alphabetical sorting from history.rs)
        let search_lower = search_query.to_lowercase();
        let filtered_tickers: Vec<&String> = tickers
            .iter()
            .filter(|t| t.to_lowercase().contains(&search_lower))
            .collect();

        // 5. Scrollable High-Density Viewport List Container
        if filtered_tickers.is_empty() {
            ui.horizontal(|ui| {
                ui.add_space(5.0);
                ui.weak("No matching tickers.");
            });
        } else {
            // Encapsulated inside a ScrollArea so the navigation layout never breaks or overflows
            egui::ScrollArea::vertical()
                .id_source("ticker_list_scroll")
                .max_height(ui.available_height() - 60.0)
                .show(ui, |ui| {
                    for ticker in filtered_tickers {
                        let is_selected = active_ticker == ticker;
                        
                        ui.horizontal(|ui| {
                            ui.add_space(5.0);
                            let label_text = if is_selected { format!("▶ {}", ticker) } else { format!("  {}", ticker) };
                            
                            if ui.selectable_label(is_selected, label_text).clicked() {
                                if is_selected {
                                    // Toggle Selection Behavior: Clicking an already active ticker clears it
                                    active_ticker.clear();
                                } else {
                                    *active_ticker = ticker.clone();
                                }
                            }
                        });
                        ui.add_space(4.0);
                    }
                });
        }

        ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
            ui.add_space(15.0);
            if ui.button(" ◀ COLLAPSE MENU ").clicked() {
                *sidebar_open = false;
            }
        });
    });
}