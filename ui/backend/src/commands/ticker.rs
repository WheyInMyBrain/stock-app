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
#[command]
pub fn set_active_workspace(ticker: Option<String>) {
    let mut active = ACTIVE_TICKER.write().unwrap();
    *active = ticker.clone();
    println!("💼 [WORKSPACE CHANGED]: Active target focus tracking updated to: {:?}", active);
}

/// Helper to replicate your centralized module IDs list safely
fn get_registered_module_ids() -> Vec<(String, u64, Vec<String>)> {
    vec![
        (
            "stock_stats".to_string(), 
            30, 
            vec!["--mode=nse".to_string(), "--api=symbol-core-data".to_string()]
        ),
        (
            "company_profile".to_string(), 
            30, 
            vec!["--mode=nse".to_string(), "--api=symbol-core-data".to_string()]
        ),
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

                let active_ticker = {
                    let current = ACTIVE_TICKER.read().unwrap();
                    current.clone()
                };

                if let Some(ticker) = active_ticker {
                    let modules = get_registered_module_ids();
                    
                    // 🎯 STEP 1: Deduplicate identical execution requests hitting on the same second
                    let mut unique_execution_groups: Vec<(Vec<String>, Vec<String>)> = Vec::new();

                    for (module_id, interval, extra_flags) in modules {
                        if total_elapsed_seconds % interval == 0 {
                            // 🎯 FIXED: Clean execution flags containing ONLY ticker and parameters.
                            // No data-dir is hardcoded here to prevent duplicate flag injection!
                            let mut flags = vec![ticker.clone()];
                            flags.extend(extra_flags);

                            if let Some(position) = unique_execution_groups.iter().position(|(f, _)| f == &flags) {
                                unique_execution_groups[position].1.push(module_id);
                            } else {
                                unique_execution_groups.push((flags, vec![module_id]));
                            }
                        }
                    }

                    // 🎯 STEP 2: Fire precisely 1 downloader pass per unique flag group
                    for (execution_flags, dependent_modules) in unique_execution_groups {
                        let current_app = app_handle.clone();
                        let target_ticker = ticker.clone();

                        tokio::spawn(async move {
                            let active_dir = get_active_data_directory(current_app.clone());
                            
                            // Passes single directory path + clean deduplicated execution flag arrays
                            let result = run_sidecar_downloader(
                                current_app.clone(),
                                Some(active_dir),
                                Some(execution_flags)
                            ).await;

                            if result.is_ok() {
                                WorkspaceDataContext::invalidate_ticker(&target_ticker);

                                if let Some(window) = current_app.get_webview_window("main") {
                                    #[derive(Clone, serde::Serialize)]
                                    struct GenericPayload { module_id: String, ticker: String }

                                    for current_module in dependent_modules {
                                        let _ = window.emit(
                                            "pipeline-invalidated",
                                            GenericPayload { module_id: current_module, ticker: target_ticker.clone() }
                                        );
                                    }
                                }
                            }
                        });
                    }
                }
            }
        });
    });
}