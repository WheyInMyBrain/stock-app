pub mod add_ticker;
pub mod overview;
pub mod analysis;
pub mod financials;
pub mod ai;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceTab {
    Overview,
    Financials,
    Analysis,
    Ai,
}

impl WorkspaceTab {
    pub const ALL: &'static [WorkspaceTab] = &[
        WorkspaceTab::Overview,
        WorkspaceTab::Financials,
        WorkspaceTab::Analysis,
        WorkspaceTab::Ai,
    ];

    pub fn label(&self) -> String {
        let variant_str = format!("{:?}", self);
        let mut result = String::new();
        for (i, c) in variant_str.chars().enumerate() {
            if i > 0 && c.is_uppercase() {
                result.push(' ');
            }
            result.push(c);
        }
        result
    }

    pub fn render(&self, ui: &mut egui::Ui, active_ticker: &str) {
        match self {
            WorkspaceTab::Overview => {
                overview::draw_overview_panel(ui, active_ticker);
            }
            WorkspaceTab::Financials => {
                financials::draw_financials_panel(ui, active_ticker);
            }
            WorkspaceTab::Analysis => {
                analysis::draw_analysis_panel(ui, active_ticker);
            }
            WorkspaceTab::Ai => {
                ai::draw_ai_panel(ui, active_ticker);
            }
        }
    }
}