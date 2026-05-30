// stock-app/ui/backend/src/commands/ticker.rs

use tauri::{AppHandle, Emitter, Manager, command};
use tokio::time::{sleep, Duration};
use std::sync::RwLock;
use crate::commands::downloader::run_sidecar_downloader;
use crate::pipeline::WorkspaceDataContext;
use crate::commands::data_dir::get_active_data_directory;

// Global Source of Truth for the current open workspace view
static ACTIVE_TICKER: RwLock<Option<String>> = RwLock::new(None);

/// 🎯 THE ONLY BACKEND COMMAND YOU NEED TO CALL
/// Updates the active asset target instantly across the entire application thread layout
#[command]
pub fn set_active_workspace(ticker: Option<String>) {
    let mut active = ACTIVE_TICKER.write().unwrap();
    *active = ticker.clone();
    println!("💼 [WORKSPACE CHANGED]: Active target focus tracking updated to: {:?}", active);
}

/// Helper to replicate your centralized module IDs list safely
fn get_registered_module_ids() -> Vec<(String, u64, Vec<String>)> {
    vec![
        // (Module ID, Interval in seconds, Custom API Extra Args)
        (
            "stock_stats".to_string(), 
            30, 
            vec!["--mode=nse".to_string(), "--api=symbol-core-data".to_string()]
        ),
        // Easily add a new abstract module here next week in 1 line:
        // ("company_profile".to_string(), 60, vec!["--mode=profile".to_string()])
    ]
}

/// 🎯 AUTOMATED TRACKING CLOCK ENGINE
pub fn spawn_global_ticker_daemon(app_handle: AppHandle) {
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        runtime.block_on(async {
            println!("🚀 [GLOBAL TICKER ENGINE]: Master tracking engine initialized.");
            let mut total_elapsed_seconds: u64 = 0;

            loop {
                sleep(Duration::from_secs(1)).await;
                total_elapsed_seconds += 1;

                // 1. Fetch whatever asset the user currently has open on screen right now
                let active_ticker = {
                    let current = ACTIVE_TICKER.read().unwrap();
                    current.clone()
                };

                // If no ticker is open, do absolutely nothing (Dormant State)
                if let Some(ticker) = active_ticker {
                    let modules = get_registered_module_ids();

                    for (module_id, interval, extra_flags) in modules {
                        // Check if this module's custom window threshold has hit
                        if total_elapsed_seconds % interval == 0 {
                            let current_app = app_handle.clone();
                            let target_ticker = ticker.clone();
                            let current_module = module_id.clone();

                            // Reconstruct the dynamic execution arguments array
                            let mut execution_flags = vec![target_ticker.clone()];
                            execution_flags.extend(extra_flags);

                            tokio::spawn(async move {
                                let active_dir = get_active_data_directory(current_app.clone());
                                execution_flags.push(format!("--data-dir={}", active_dir));

                                let result = run_sidecar_downloader(
                                    current_app.clone(),
                                    Some(active_dir),
                                    Some(execution_flags)
                                ).await;

                                if result.is_ok() {
                                    // Flash cache mapping layers
                                    WorkspaceDataContext::invalidate_ticker(&target_ticker);

                                    // Push update straight out to the window canvas layer
                                    if let Some(window) = current_app.get_webview_window("main") {
                                        #[derive(Clone, serde::Serialize)]
                                        struct GenericPayload { module_id: String, ticker: String }

                                        let _ = window.emit(
                                            "pipeline-invalidated",
                                            GenericPayload { module_id: current_module, ticker: target_ticker }
                                        );
                                    }
                                }
                            });
                        }
                    }
                }
            }
        });
    });
}