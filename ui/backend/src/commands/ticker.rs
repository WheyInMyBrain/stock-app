// stock-app/ui/backend/src/commands/ticker.rs

use tauri::{AppHandle, Emitter, Manager, command};
use tokio::time::{sleep, Duration};
use std::sync::RwLock;
use crate::commands::downloader::run_sidecar_downloader;
use crate::commands::data_loader::WorkspaceDataContext;
use crate::commands::data_dir::get_active_data_directory;

static ACTIVE_TICKER: RwLock<Option<String>> = RwLock::new(None);

pub static POPUP_FROM_TIMESTAMP: RwLock<i64> = RwLock::new(0);

#[command]
pub fn set_active_workspace(ticker: Option<String>) {
    let mut active = ACTIVE_TICKER.write().unwrap();
    *active = ticker.clone();
    println!("💼 [WORKSPACE CHANGED]: Active target focus tracking updated to: {:?}", active);
}

fn get_registered_module_ids() -> Vec<(String, u64, Vec<String>)> {
    vec![
        (
            "stock_stats".to_string(), 
            30, 
            vec![
                "--mode=nse".to_string(), 
                "--api=symbol-core-data".to_string(),
                "--stream".to_string()
            ]
        ),
        // 🚀 DEDICATED POPUP REGISTRY SLOT
        (
            "stock_chart".to_string(), 
            10, 
            vec![
                "--mode=nse".to_string(), 
                "--api=real-time-chart-delta".to_string(),
                "--stream".to_string(),
                format!("--from={}", *POPUP_FROM_TIMESTAMP.read().unwrap())
            ]
        ),
    ]
}

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
                    let mut unique_execution_groups: Vec<(Vec<String>, Vec<String>)> = Vec::new();

                    for (module_id, interval, extra_flags) in modules {
                        if total_elapsed_seconds % interval == 0 {
                            let mut flags = vec![ticker.clone()];
                            flags.extend(extra_flags);

                            if let Some(position) = unique_execution_groups.iter().position(|(f, _)| f == &flags) {
                                unique_execution_groups[position].1.push(module_id);
                            } else {
                                unique_execution_groups.push((flags, vec![module_id]));
                            }
                        }
                    }

                    for (execution_flags, dependent_modules) in unique_execution_groups {
                        let current_app = app_handle.clone();
                        let target_ticker = ticker.clone();

                        tokio::spawn(async move {
                            let active_dir = get_active_data_directory(current_app.clone());
                            
                            let result = run_sidecar_downloader(
                                current_app.clone(),
                                Some(active_dir),
                                Some(execution_flags)
                            ).await;

                            if result.is_ok() {
                                WorkspaceDataContext::invalidate_ticker(&target_ticker);

                                #[derive(Clone, serde::Serialize)]
                                struct GenericPayload { module_id: String, ticker: String }

                                // 🚀 BROADCAST DISPATCH: Send the event to all active windows
                                for (_, webview_window) in current_app.webview_windows() {
                                    for current_module in &dependent_modules {
                                        let _ = webview_window.emit(
                                            "pipeline-invalidated",
                                            GenericPayload { 
                                                module_id: current_module.clone(), 
                                                ticker: target_ticker.clone() 
                                            }
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