#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Import our isolated sub-modules from the commands folder
mod commands;
pub mod pipeline;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            // Data
            commands::data_dir::set_custom_data_directory,
            
            // History Commands
            commands::history::get_history_tickers,
            
            // Layout Management Commands
            commands::layout::save_workspace_layout,
            commands::layout::load_workspace_layout,
            
            // Server-Driven Data Pipeline Engine
            commands::pipeline::fetch_component_telemetry,
            commands::pipeline::fetch_component_catalog
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}