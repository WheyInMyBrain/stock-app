// stock-app/ui/frontend_native/src/ui/panels/mod.rs
pub mod add_ticker;
pub mod financials;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceTab {

}

impl WorkspaceTab {
    pub const ALL: &'static [WorkspaceTab] = &[
        WorkspaceTab::FinancialSheets,
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
            WorkspaceTab::FinancialSheets => {
                
            }
        }
    }
}