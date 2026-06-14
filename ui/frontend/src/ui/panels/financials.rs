// stock-app/ui/frontend/src/ui/panels/financials.rs

use egui::{Ui, Color32, Vec2, Stroke};
use crate::core::data_manager::DataManager;
use crate::ui::layouts::canvas::{AbstractSubTab, draw_nav_canvas_orchestrator};
use backend::database::financial::{FinancialStatementCollection, FinancialStatementLineItem};

/// Shared engine core logic to cleanly draw standard 3-column rows packed tightly from edge to edge
fn execute_statement_main_grid(ui: &mut Ui, id_tag: usize, collection: &FinancialStatementCollection, lines: &[FinancialStatementLineItem]) {
    if lines.is_empty() {
        ui.weak("No entries recorded for this statement category.");
        return;
    }

    let mut selected_file = collection.available_files.last().cloned().unwrap_or_default();
    let state_id = ui.make_persistent_id(format!("selected_financial_year_file_{}", id_tag));
    
    ui.data_mut(|d| {
        if let Some(cached) = d.get_persisted::<String>(state_id) {
            if collection.available_files.contains(&cached) {
                selected_file = cached.clone();
            }
        }
    });

    let current_year_numeric = selected_file.parse::<i32>().unwrap_or(0);
    let previous_year_numeric = if current_year_numeric > 0 { current_year_numeric - 1 } else { 0 };

    let current_year_header = if current_year_numeric > 0 { current_year_numeric.to_string() } else { "Current Year".to_string() };
    let previous_year_header = if previous_year_numeric > 0 { previous_year_numeric.to_string() } else { "Previous Year".to_string() };

    let filtered_lines: Vec<&FinancialStatementLineItem> = lines
        .iter()
        .filter(|l| l.file_name == selected_file)
        .collect();

    let val_col_width = 160.0;
    let col_spacing = 24.0;

    // 1. Edge-to-Edge Floating Header Layout Pass
    ui.horizontal(|ui| {
        let total_width = ui.available_width();
        
        // Render from right-to-left so numbers stick to the absolute right border margin
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_sized(Vec2::new(val_col_width, 24.0), egui::Label::new(egui::RichText::new(previous_year_header).strong().heading()));
            ui.add_space(col_spacing);
            ui.add_sized(Vec2::new(val_col_width, 24.0), egui::Label::new(egui::RichText::new(current_year_header).strong().heading()));
            
            // The remaining leftmost container bounds are claimed entirely by Particulars
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                let particulars_width = (total_width - (val_col_width * 2.0) - col_spacing).max(100.0);
                ui.add_sized(Vec2::new(particulars_width, 24.0), egui::Label::new(egui::RichText::new("PARTICULARS").strong().heading()));
            });
        });
    });
        
    ui.separator();
    ui.add_space(4.0);

    // 2. Scrollable Rows using the same Layout Pattern
    egui::ScrollArea::vertical()
        .id_source(format!("financial_grid_scroll_{}", id_tag))
        .show(ui, |ui| {
            let row_height = 28.0;
            
            for (idx, item) in filtered_lines.into_iter().enumerate() {
                // Alternating row background shading
                let bg_color = if idx % 2 == 0 { Color32::from_rgb(18, 18, 18) } else { Color32::from_rgb(14, 14, 14) };
                
                let frame = egui::Frame::none()
                    .fill(bg_color)
                    .inner_margin(egui::Margin::symmetric(0.0, 4.0));

                frame.show(ui, |ui| {
                    let total_width = ui.available_width();
                    
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // Previous Year pinned to the absolute right edge
                            ui.add_sized(Vec2::new(val_col_width, row_height), egui::Label::new(egui::RichText::new(&item.previous_year_value).strong()));
                            ui.add_space(col_spacing);
                            
                            // Current Year pinned directly next to it
                            ui.add_sized(Vec2::new(val_col_width, row_height), egui::Label::new(egui::RichText::new(&item.current_year_value).strong()));
                            
                            // Particulars stretches dynamically across all remaining space on the left
                            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                let particulars_width = (total_width - (val_col_width * 2.0) - col_spacing).max(100.0);
                                ui.allocate_ui(Vec2::new(particulars_width, row_height), |ui| {
                                    ui.add(egui::Label::new(egui::RichText::new(&item.particulars).strong()).wrap(true));
                                });
                            });
                        });
                    });
                });
                ui.separator();
            }
        });
}

fn execute_statement_bottom_deck(ui: &mut Ui, id_tag: usize, collection: &FinancialStatementCollection) {
    let mut selected_file = collection.available_files.last().cloned().unwrap_or_default();
    let state_id = ui.make_persistent_id(format!("selected_financial_year_file_{}", id_tag));
    
    ui.data_mut(|d| {
        if let Some(cached) = d.get_persisted::<String>(state_id) {
            if collection.available_files.contains(&cached) {
                selected_file = cached.clone();
            }
        }
    });

    let mut chronological_reversed_years = collection.available_files.clone();
    chronological_reversed_years.reverse();

    ui.vertical(|ui| {
        ui.label(egui::RichText::new("FINANCIAL REPORTING PERIOD INTERACTIVE DECK").strong());
        ui.add_space(8.0);
        
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(10.0, 10.0);
            
            for file in &chronological_reversed_years {
                let is_selected = *file == selected_file;
                
                let btn_text = egui::RichText::new(format!("{}", file)).strong();
                let button_widget = egui::Button::new(btn_text)
                    .min_size(Vec2::new(88.0, 38.0))
                    .stroke(if is_selected {
                        Stroke::new(2.0, Color32::from_rgb(34, 139, 34))
                    } else {
                        Stroke::new(1.0, Color32::from_rgb(50, 50, 50))
                    });

                if ui.add(button_widget).clicked() {
                    selected_file = file.clone();
                    ui.data_mut(|d| d.insert_persisted(state_id, selected_file.clone()));
                }
            }
        });
    });
}

struct IncomeStatementTab;
impl AbstractSubTab<FinancialStatementCollection> for IncomeStatementTab {
    fn id(&self) -> usize { 0 }
    fn label(&self) -> &'static str { "Income Statement" }
    
    fn render_main(&self, ui: &mut Ui, collection: &FinancialStatementCollection) {
        execute_statement_main_grid(ui, self.id(), collection, &collection.income_statement);
    }
    
    fn render_bottom(&self, ui: &mut Ui, collection: &FinancialStatementCollection) {
        execute_statement_bottom_deck(ui, self.id(), collection);
    }
}

struct BalanceSheetTab;
impl AbstractSubTab<FinancialStatementCollection> for BalanceSheetTab {
    fn id(&self) -> usize { 1 }
    fn label(&self) -> &'static str { "Balance Sheet" }
    
    fn render_main(&self, ui: &mut Ui, collection: &FinancialStatementCollection) {
        execute_statement_main_grid(ui, self.id(), collection, &collection.balance_sheet);
    }
    
    fn render_bottom(&self, ui: &mut Ui, collection: &FinancialStatementCollection) {
        execute_statement_bottom_deck(ui, self.id(), collection);
    }
}

struct CashFlowTab;
impl AbstractSubTab<FinancialStatementCollection> for CashFlowTab {
    fn id(&self) -> usize { 2 }
    fn label(&self) -> &'static str { "Cash Flow Statement" }
    
    fn render_main(&self, ui: &mut Ui, collection: &FinancialStatementCollection) {
        execute_statement_main_grid(ui, self.id(), collection, &collection.cash_flow);
    }
    
    fn render_bottom(&self, ui: &mut Ui, collection: &FinancialStatementCollection) {
        execute_statement_bottom_deck(ui, self.id(), collection);
    }
}

pub fn draw_financials_panel(ui: &mut Ui, active_ticker: &str) {
    DataManager::ensure_financials_data(active_ticker);

    let tabs: &[&dyn AbstractSubTab<FinancialStatementCollection>] = &[
        &IncomeStatementTab,
        &BalanceSheetTab,
        &CashFlowTab,
    ];

    draw_nav_canvas_orchestrator(
        ui,
        active_ticker,
        "financial_metadata", 
        "FINANCIAL REPORTING STATEMENTS",
        "financials_active_sub_tab_id",
        tabs,
    );
}