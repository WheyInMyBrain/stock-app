// stock-app/ui/backend/src/lib.rs

// 🎯 DUAL-WORKING FIX: Expose your command and data pipeline modules publicly 
// so a future native frontend can reference their logic from memory.
pub mod commands;
pub mod pipeline;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Initialize shell plugin state completely
        .plugin(tauri_plugin_shell::init()) 
        .setup(|app| {
            // Retain logging capabilities for debug sessions
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            
            let app_handle = app.handle().clone();
            
            // Initialize your Go sidecar persistent downloader daemon natively
            commands::downloader::initialize_persistent_go_daemon(app_handle.clone());
            
            // Fire up your global interval clock ticker tracking thread
            commands::ticker::spawn_global_ticker_daemon(app_handle);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Data Directory Setup
            commands::data_dir::set_custom_data_directory,
            
            // History Tracking
            commands::history::get_history_tickers,
            
            // Layout Framework Workspace Hooks
            commands::layout::save_workspace_layout,
            commands::layout::load_workspace_layout,
            
            // Active Ticker & Components Pipelines Telemetry Command Handlers
            commands::ticker::set_active_workspace,
            commands::pipeline::fetch_component_telemetry,
            commands::pipeline::fetch_component_catalog,

            // Native Popups Management Channels
            commands::popup_manager::spawn_native_popup,
            commands::popup_manager::compile_popup_telemetry,

            // Core Math & Data Analysis Triggers
            commands::analysis::trigger_core_analysis
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}