// stock-app/ui/frontend_native/src/ui/setup.rs
use egui::Ui;
use std::fs;

pub fn check_existing_config() -> Option<String> {
    if let Ok(path_str) = fs::read_to_string("terminal_config.txt") {
        let trimmed = path_str.trim().to_string();
        if !trimmed.is_empty() {
            return Some(trimmed);
        }
    }
    None
}

pub fn save_config(path: &str) -> Result<(), std::io::Error> {
    fs::write("terminal_config.txt", path.trim())
}

pub fn draw_setup_view(ui: &mut Ui, input_buffer: &mut String, completed_path: &mut Option<String>) {
    ui.vertical_centered(|ui| {
        ui.add_space(150.0);
        ui.heading(" 🛰️ SYSTEM REPOSITORY INITIALIZATION ");
        ui.add_space(20.0);
        
        ui.label("FIRST LAUNCH CONFIGURATION REQUIRED. PLEASE SPECIFY THE");
        ui.label("ABSOLUTE PATH WHERE YOUR 'data/' SUBFOLDER SHOULD BE LINKED:");
        ui.add_space(30.0);

        ui.horizontal(|ui| {
            let total_width = ui.available_width();
            ui.add_space(total_width * 0.15); 
            
            let text_edit = egui::TextEdit::singleline(input_buffer)
                .hint_text("/Users/username/Project/stock-app/data")
                .desired_width(total_width * 0.70);
            
            ui.add(text_edit);
        });

        ui.add_space(40.0);

        if ui.button(" ⚡ INITIALIZE CORE DATA PATH ").clicked() {
            let targeted_path = input_buffer.trim();
            if !targeted_path.is_empty() {
                if save_config(targeted_path).is_ok() {
                    *completed_path = Some(targeted_path.to_string());
                    println!("📁 [DATA ANCHOR INITIALIZED]: Bound to: {}", targeted_path);
                }
            }
        }
    });
}