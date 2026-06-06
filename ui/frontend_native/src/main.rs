use eframe::egui;
use egui::{Color32, Stroke, RichText};

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("QUANTSTATION // Native Terminal App")
            .with_inner_size([1150.0, 720.0]),
        ..Default::default()
    };

    eframe::run_native(
        "quantstation_native_client",
        native_options,
        Box::new(|cc| Box::new(QuantStationNativeApp::new(cc))),
    )
}

struct QuantStationNativeApp {
    selected_ticker: String,
    selected_timeframe: String,
}

impl QuantStationNativeApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // 🎯 FIXED: Removed the macro attribute. This safely reads the render state 
        // option on all platforms, removing both the cfg lint and the unused variable warnings.
        if let Some(render_state) = &cc.wgpu_render_state {
            let adapter_info = render_state.adapter.get_info();
            println!("🔒 GPU HARDWARE VERIFICATION SUCCESSFUL:");
            println!("   -> Graphics Backend API: {:?}", adapter_info.backend); // e.g., Metal on Mac, Dx12 on Windows
            println!("   -> Device Controller: {}", adapter_info.name);       // e.g., Apple M3 Max
            println!("   -> Architecture Type: {:?}", adapter_info.device_type);
        }

        Self {
            selected_ticker: "IMFA".to_string(),
            selected_timeframe: "1D".to_string(),
        }
    }
}

impl eframe::App for QuantStationNativeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut visual_style = (*ctx.style()).clone();
        visual_style.visuals.dark_mode = true;
        visual_style.visuals.panel_fill = Color32::from_rgb(10, 10, 10);
        ctx.set_style(visual_style);

        // ==============================================================================
        // 🏛️ HEADER PANEL
        // ==============================================================================
        egui::TopBottomPanel::top("native_terminal_header")
            .frame(egui::Frame::none().inner_margin(12.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(RichText::new("📊 QUANTSTATION").font(egui::FontId::proportional(16.0)).strong());
                    ui.label(RichText::new("// NATIVE HARDWARE CANVAS //").color(Color32::from_rgb(115, 115, 115)).code());
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        egui::ComboBox::from_id_source("native_tf_select")
                            .selected_text(&self.selected_timeframe)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.selected_timeframe, "1D".to_string(), "1D");
                                ui.selectable_value(&mut self.selected_timeframe, "1W".to_string(), "1W");
                            });
                            
                        ui.add_space(8.0);
                        
                        egui::ComboBox::from_id_source("native_ticker_select")
                            .selected_text(&self.selected_ticker)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.selected_ticker, "IMFA".to_string(), "IMFA");
                            });
                    });
                });
                ui.add_space(6.0);
                ui.separator();
            });

        // ==============================================================================
        // 🏛️ CENTRAL VIEWPORT
        // ==============================================================================
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .id_source("native_viewport_scroller")
                .show(ui, |ui| {
                    
                    egui::Frame::none()
                        .fill(Color32::from_rgb(20, 20, 20))
                        .stroke(Stroke::new(1.0, Color32::from_rgb(38, 38, 38)))
                        .inner_margin(16.0)
                        .rounding(4.0)
                        .show(ui, |ui| {
                            ui.vertical(|ui| {
                                ui.label(RichText::new("Shared Library Ingestion Pipeline Engine").strong().color(Color32::WHITE));
                                ui.label(RichText::new("Direct memory access verification to backend compilation handlers.").size(11.0).color(Color32::GRAY));
                            });
                        });

                    ui.add_space(16.0);

                    ui.label(RichText::new("INTERACTIVE GEOMETRIC VECTOR METRICS MESH").font(egui::FontId::monospace(10.0)).color(Color32::from_rgb(163, 163, 163)));
                    ui.add_space(6.0);

                    egui::Frame::none()
                        .fill(Color32::from_rgb(15, 15, 15))
                        .stroke(Stroke::new(1.0, Color32::from_rgb(28, 28, 28)))
                        .inner_margin(40.0)
                        .rounding(4.0)
                        .show(ui, |ui| {
                            ui.centered_and_justified(|ui| {
                                ui.label(RichText::new("[ Interactive Chart Vector Grid Target Area ]").color(Color32::from_rgb(64, 64, 64)).italics());
                            });
                        });
                });
        });
    }
}