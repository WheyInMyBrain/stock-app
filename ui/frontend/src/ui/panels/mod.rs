// stock-app/ui/frontend_native/src/ui/panels/mod.rs
pub mod add_ticker;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceTab {
    Overview,
}

impl WorkspaceTab {
    pub const ALL: &'static [WorkspaceTab] = &[
        WorkspaceTab::Overview,
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
                // Employs your abstract responsive 3-zone canvas cleanly
                crate::ui::layouts::canvas::draw_three_zone_canvas(
                    ui,
                    |ui| {
                        ui.heading(format!("Main Content Sheet: {}", active_ticker));
                        ui.weak("Responsive laptop aspect-ratio frame viewport canvas area.");
                    },
                    |ui| {
                        ui.label("Bottom Panel Row Matrix Spreadsheet Area Placeholder");
                    },
                    |ui| {
                        ui.label("Right Panel Slender Control Strip Cockpit");
                    },
                );
            }
        }
    }
}