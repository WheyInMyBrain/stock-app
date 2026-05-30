// stock-app/ui/backend/src/commands/downloader.rs

use tauri::{AppHandle, command, Manager, Emitter};
use tauri_plugin_shell::ShellExt; 
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use std::sync::Mutex;
use crate::pipeline::WorkspaceDataContext;
use crate::commands::data_dir::get_active_data_directory;

// 🎯 GLOBAL PRIVATE HOLDER FOR THE PERSISTENT BACKGROUND GO SERVICE WRITER PIPE
static GO_PROCESS_WRITER: Mutex<Option<CommandChild>> = Mutex::new(None);

/// 🎯 APP INITIALIZATION GATEWAY: Boots the persistent Go binary and primes network session cookies
pub fn initialize_persistent_go_daemon(app_handle: AppHandle) {
    let current_app = app_handle.clone();
    
    // 🎯 FIXED: Uses Tauri's core async runtime manager to bind onto the active Tokio reactor safely
    tauri::async_runtime::spawn(async move {
        let active_dir = get_active_data_directory(current_app.clone());
        println!("🔥 [TAURI CORE DAEMON]: Spawning persistent Go Sidecar. Target dir: {}", active_dir);

        // Spin up the sidecar with our custom --daemon switch flag explicitly attached
        let sidecar_setup = current_app
            .shell()
            .sidecar("downloader")
            .unwrap()
            .args(vec!["--daemon".to_string(), format!("--data-dir={}", active_dir)]);

        match sidecar_setup.spawn() {
            Ok((mut rx, child_control)) => {
                // Save the running process handle globally so ticker.rs can write commands to it
                {
                    let mut writer_guard = GO_PROCESS_WRITER.lock().unwrap();
                    *writer_guard = Some(child_control);
                }

                // 🚀 LIVE STDOUT STREAM MONITORING: Reads logs and catches execution completions
                while let Some(event) = rx.recv().await {
                    match event {
                        CommandEvent::Stdout(bytes) => {
                            let line = String::from_utf8_lossy(&bytes).to_string();
                            
                            // Relay native Go console logs back to your backend window screen instantly
                            print!("{}", line);

                            // Intercept the final payload write confirmation emitted from Go's daemon.go
                            if line.starts_with("SIGNAL_COMPLETED:") {
                                let parts: Vec<&str> = line.trim().split(':').collect();
                                if parts.len() >= 3 {
                                    let ticker = parts[1];
                                    let api_name = parts[2];

                                    println!("⚡ [DAEMON RECEPTION]: Go wrote fresh JSON for [{}] via hot pipe.", ticker);

                                    // Clear memory storage cache frames instantly
                                    WorkspaceDataContext::invalidate_ticker(ticker);

                                    // Emit layout updates directly to active frontend framework viewport windows
                                    if let Some(window) = current_app.get_webview_window("main") {
                                        #[derive(Clone, serde::Serialize)]
                                        struct GenericPayload { module_id: String, ticker: String }

                                        // Route notifications dynamically back to the layout cards
                                        let target_module = if api_name == "symbol-core-data" {
                                            "stock_stats"
                                        } else {
                                            "company_profile"
                                        };

                                        let _ = window.emit(
                                            "pipeline-invalidated",
                                            GenericPayload { module_id: target_module.to_string(), ticker: ticker.to_string() }
                                        );
                                        
                                        // Broadcast a fallback flash to make sure company profile refreshes synchronously
                                        let _ = window.emit(
                                            "pipeline-invalidated",
                                            GenericPayload { module_id: "company_profile".to_string(), ticker: ticker.to_string() }
                                        );
                                    }
                                }
                            }
                        }
                        CommandEvent::Stderr(bytes) => {
                            print!("🚨 [GO DAEMON ERROR]: {}", String::from_utf8_lossy(&bytes));
                        }
                        CommandEvent::Terminated(payload) => {
                            println!("🏁 [GO DAEMON ALERT]: Background sidecar closed with status: {:?}", payload.code);
                        }
                        _ => {}
                    }
                }
            }
            Err(err) => println!("❌ [CRITICAL DAEMON FAULT]: System failed to map persistent sidecar channels: {}", err),
        }
    });
}

/// 🎯 INTERCEPTOR INTERFACE LAYER
/// This transparently hooks your unmodified ticker.rs code execution requests straight into the live memory daemon.
#[command]
pub async fn run_sidecar_downloader(
    _app_handle: AppHandle,
    _data_dir_override: Option<String>,
    extra_args: Option<Vec<String>>, 
) -> Result<String, String> {
    
    // 1. Safely pull execution arguments forwarded natively from your ticker loop array
    let flags = extra_args.ok_or_else(|| "Missing execution flags matrix wrapper payload".to_string())?;
    if flags.is_empty() {
        return Err("Execution flags array size parsed empty".to_string());
    }

    let ticker = &flags[0];
    let mut mode = "both".to_string();
    let mut api = "".to_string();

    // Sift configuration strings out smoothly
    for flag in &flags[1..] {
        if flag.starts_with("--mode=") {
            mode = flag.split('=').nth(1).unwrap_or("both").to_string();
        } else if flag.starts_with("--api=") {
            api = flag.split('=').nth(1).unwrap_or("").to_string();
        }
    }

    // 2. Access the active long-running writer context
    let mut writer_guard = GO_PROCESS_WRITER.lock().unwrap();
    if let Some(ref mut child) = *writer_guard {
        // Construct line format string expected by your Go daemon.go listener: "RUN TICKER MODE API\n"
        let command_payload = format!("RUN {} {} {}\n", ticker, mode, api);
        
        println!("📡 [INTERCEPTOR -> GO STDIN]: Transparently routing instruction payload down warm pipe: {}", command_payload.trim());
        
        child.write(command_payload.as_bytes())
            .map_err(|e| format!("Failed forwarding intercept payload text sequence down memory pipe stream: {}", e))?;
            
        // Return instantly! Ticker daemon frees its thread immediately without waiting on disk operations.
        Ok("Signal intercepted and dispatched to hot connection pool successfully".to_string())
    } else {
        Err("Persistent background Go service infrastructure layer is uninitialized or offline.".to_string())
    }
}