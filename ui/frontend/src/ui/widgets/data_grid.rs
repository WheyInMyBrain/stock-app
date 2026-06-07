// stock-app/ui/frontend/src/ui/widgets/data_grid.rs
use egui::{Ui, Color32, RichText};

#[derive(Clone, Debug)]
pub struct MetricItem {
    pub label: String,
    pub value: String,
    pub value_color: Option<Color32>,
}

impl MetricItem {
    pub fn new(label: &str, value: &str) -> Self {
        Self {
            label: label.to_string(),
            value: value.to_string(),
            value_color: None,
        }
    }

    pub fn color(mut self, color: Color32) -> Self {
        self.value_color = Some(color);
        self
    }
}

pub fn draw_high_density_grid(ui: &mut Ui, tracking_id: &str, title: &str, items: &[MetricItem]) {
    ui.vertical(|ui| {
        if !title.is_empty() {
            ui.label(RichText::new(title).strong().color(Color32::from_rgb(180, 180, 180)));
            ui.add_space(6.0);
        }
        egui::Grid::new(tracking_id)
            .num_columns(2)
            .spacing([30.0, 8.0])
            .striped(true)
            .show(ui, |ui| {
                for item in items {
                    ui.label(&item.label);
                    let mut txt = RichText::new(&item.value);
                    if let Some(c) = item.value_color {
                        txt = txt.color(c);
                    }
                    ui.label(txt);
                    ui.end_row();
                }
            });
    });
}