#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Import our isolated sub-modules from the commands folder
mod commands;
pub mod pipeline;

fn main() {
    tauri::Builder::default()
        // 🎯 FIXED: Initialized the shell plugin state completely! 
        // This lets your dynamic background update loops spawn Go sidecars without crashing.
        .plugin(tauri_plugin_shell::init()) 
        .invoke_handler(tauri::generate_handler![
            // Data
            commands::data_dir::set_custom_data_directory,
            
            // History Commands
            commands::history::get_history_tickers,
            
            // Layout Management Commands
            commands::layout::save_workspace_layout,
            commands::layout::load_workspace_layout,
            
            // Server-Driven Data Pipeline Engine
            commands::ticker::set_active_workspace,
            commands::pipeline::fetch_component_telemetry,
            commands::pipeline::fetch_component_catalog
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();
            
            // 🎯 FIXED: Initialized through the consolidated downloader module file natively!
            commands::downloader::initialize_persistent_go_daemon(app_handle.clone());
            
            // Start your standard interval clock tracker
            commands::ticker::spawn_global_ticker_daemon(app_handle);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}