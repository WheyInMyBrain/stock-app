use egui::Ui;
use crate::core::data_manager::DataManager;
use crate::ui::layouts::canvas::{AbstractSubTab, draw_nav_canvas_orchestrator};
use backend::database::overview::OverviewMetadata;

struct DetailsSubTab;
impl AbstractSubTab<OverviewMetadata> for DetailsSubTab {
    fn id(&self) -> usize { 2 }
    fn label(&self) -> &'static str { "Details" }
    
    fn render_main(&self, ui: &mut Ui, meta: &OverviewMetadata) {
        ui.label(format!("ISIN: {}", meta.isin));
        ui.label(format!("NSE Code: {}", meta.nse_code));
        ui.label(format!("BSE Code: {}", meta.bse_code));
        ui.label(format!("Face Value: {}", meta.face_value));
        if !meta.nse_listing_date.is_empty() { ui.label(format!("NSE Listed Date: {}", meta.nse_listing_date)); }
        if !meta.bse_listing_date.is_empty() { ui.label(format!("BSE Listed Date: {}", meta.bse_listing_date)); }
        ui.add_space(10.0);
        ui.label(format!("Macro Category: {}", meta.macro_category));
        ui.label(format!("Sector: {}", meta.sector));
        ui.label(format!("Industry: {}", meta.industry));
        ui.add_space(10.0);
        if !meta.indexes.is_empty() {
            ui.label(egui::RichText::new("Tracked Market Indexes:").strong());
            ui.horizontal_wrapped(|ui| {
                ui.label(meta.indexes.join(", "));
            });
            ui.add_space(10.0);
        }
        if !meta.address.is_empty() { ui.label(format!("Address: {}", meta.address)); }
        if !meta.telephone.is_empty() { ui.label(format!("Telephone: {}", meta.telephone)); }
        if !meta.fax.is_empty() { ui.label(format!("Fax: {}", meta.fax)); }
        if !meta.email.is_empty() { ui.label(format!("Email: {}", meta.email)); }
        if !meta.website.is_empty() { ui.label(format!("Website: {}", meta.website)); }
    }
}

struct BoardSubTab;
impl AbstractSubTab<OverviewMetadata> for BoardSubTab {
    fn id(&self) -> usize { 1 }
    fn label(&self) -> &'static str { "Board of Directors" }
    
    fn render_main(&self, ui: &mut Ui, meta: &OverviewMetadata) {
        ui.label(egui::RichText::new("BOARD OF DIRECTORS").strong());
        ui.add_space(10.0);
        for dir in &meta.directors {
            ui.label(format!("• {} ({})", dir.name, dir.designation));
        }
    }
}

pub fn draw_overview_panel(ui: &mut Ui, active_ticker: &str) {
    DataManager::ensure_overview_data(active_ticker);

    let tabs: &[&dyn AbstractSubTab<OverviewMetadata>] = &[
        &DetailsSubTab,
        &BoardSubTab,
    ];

    // Passes our concrete type data parameters out into the abstract layout framework
    draw_nav_canvas_orchestrator(
        ui,
        active_ticker,
        "overview_metadata",        // Target memory pool lookup table key
        "OVERVIEW",                 // Title heading contextual identifier 
        "overview_active_sub_tab",  // Unique token string driving temporary frame view state storage keys
        tabs,
    );
}