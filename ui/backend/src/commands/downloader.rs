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
    
    tauri::async_runtime::spawn(async move {
        let active_dir = get_active_data_directory(current_app.clone());
        println!("🔥 [TAURI CORE DAEMON]: Spawning persistent Go Sidecar. Target dir: {}", active_dir);

        let sidecar_setup = current_app
            .shell()
            .sidecar("downloader")
            .unwrap()
            .args(vec!["--daemon".to_string(), format!("--data-dir={}", active_dir)]);

        match sidecar_setup.spawn() {
            Ok((mut rx, child_control)) => {
                {
                    let mut writer_guard = GO_PROCESS_WRITER.lock().unwrap();
                    *writer_guard = Some(child_control);
                }

                // 🎯 IN-MEMORY PAYLOAD CAPTURE STATE
                let mut json_accumulator = String::new();
                let mut is_streaming_payload = false;
                let mut stream_ticker = String::new();
                let mut stream_api = String::new();

                // 🚀 LIVE STDOUT STREAM MONITORING
                while let Some(event) = rx.recv().await {
                    match event {
                        CommandEvent::Stdout(bytes) => {
                            let stdout_chunk = String::from_utf8_lossy(&bytes).to_string();
                            
                            // Process text line-by-line to handle structural envelope markers safely
                            for line in stdout_chunk.lines() {
                                
                                // 🎯 INTERCEPT MEMORY WRAPPER BOUNDARY FROM GO
                                if line.starts_with("PAYLOAD_START:") {
                                    is_streaming_payload = true;
                                    json_accumulator.clear();
                                    
                                    let parts: Vec<&str> = line.split(':').collect();
                                    if parts.len() >= 3 {
                                        stream_ticker = parts[1].to_string();
                                        stream_api = parts[2].to_string();
                                    }
                                    continue;
                                }

                                if line == "PAYLOAD_END" {
                                    is_streaming_payload = false;
                                    println!("\x1b[32m⚡ [RAM PIPELINE SUCCESS]: Captured {} raw JSON stream payload completely in system memory!\x1b[0m", stream_ticker);

                                    // Emit the absolute raw JSON string straight to front-end viewport listeners
                                    if let Some(window) = current_app.get_webview_window("main") {
                                        #[derive(Clone, serde::Serialize)]
                                        struct DirectStreamPayload { module_id: String, ticker: String, raw_json: String }

                                        let target_module = if stream_api == "symbol-core-data" {
                                            "stock_stats"
                                        } else {
                                            "company_profile"
                                        };

                                        let _ = window.emit(
                                            "live-memory-data", 
                                            DirectStreamPayload {
                                                module_id: target_module.to_string(),
                                                ticker: stream_ticker.clone(),
                                                raw_json: json_accumulator.clone(),
                                            }
                                        );
                                    }
                                    continue;
                                }

                                // 🎯 ACCUMULATE OR PRINT LOGS DYNAMICALLY
                                if is_streaming_payload {
                                    json_accumulator.push_str(line);
                                    json_accumulator.push('\n');
                                } else {
                                    // Strip color markers if present, then apply uniform styling 
                                    println!("\x1b[34m{}\x1b[0m", line);

                                    // Intercept the traditional completion signal safely outside JSON blocks
                                    if line.starts_with("SIGNAL_COMPLETED:") {
                                        let parts: Vec<&str> = line.trim().split(':').collect();
                                        if parts.len() >= 3 {
                                            let ticker = parts[1];
                                            let api_name = parts[2];

                                            println!("\x1b[32m⚡ [DAEMON RECEPTION]: Go asset sync pipeline complete for [{}].\x1b[0m", ticker);

                                            WorkspaceDataContext::invalidate_ticker(ticker);

                                            if let Some(window) = current_app.get_webview_window("main") {
                                                #[derive(Clone, serde::Serialize)]
                                                struct GenericPayload { module_id: String, ticker: String }

                                                let target_module = if api_name == "symbol-core-data" { "stock_stats" } else { "company_profile" };

                                                let _ = window.emit(
                                                    "pipeline-invalidated",
                                                    GenericPayload { module_id: target_module.to_string(), ticker: ticker.to_string() }
                                                );
                                                let _ = window.emit(
                                                    "pipeline-invalidated",
                                                    GenericPayload { module_id: "company_profile".to_string(), ticker: ticker.to_string() }
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        CommandEvent::Stderr(bytes) => {
                            print!("\x1b[31m🚨 [GO DAEMON ERROR]: {}\x1b[0m", String::from_utf8_lossy(&bytes));
                        }
                        CommandEvent::Terminated(payload) => {
                            println!("\x1b[31m🏁 [GO DAEMON ALERT]: Background sidecar closed with status: {:?}\x1b[0m", payload.code);
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
/// Transparently handles stream parameter parsing without breaking your standard ticker.rs pipeline mapping arrays.
#[command]
pub async fn run_sidecar_downloader(
    app_handle: AppHandle, // 🎯 ACTIVATED: Removed the prefix underscores so we can utilize this context handle
    _data_dir_override: Option<String>,
    extra_args: Option<Vec<String>>, 
) -> Result<String, String> {
    
    let flags = extra_args.ok_or_else(|| "Missing execution flags matrix wrapper payload".to_string())?;
    if flags.is_empty() {
        return Err("Execution flags array size parsed empty".to_string());
    }

    let ticker = &flags[0];
    let mut mode = "both".to_string();
    let mut api = "".to_string();
    let mut is_stream_requested = false;

    // Sift dynamic parameters out smoothly
    for flag in &flags[1..] {
        if flag.starts_with("--mode=") {
            mode = flag.split('=').nth(1).unwrap_or("both").to_string();
        } else if flag.starts_with("--api=") {
            api = flag.split('=').nth(1).unwrap_or("").to_string();
        } else if flag == "--stream" {
            is_stream_requested = true;
        }
    }

    // ============================================================================
    // 🎯 STEP 1: CACHE-FIRST APPROACH (IMMEDIATE PRE-RENDER SIGNAL DUMP)
    // ============================================================================
    if is_stream_requested && !api.is_empty() {
        let active_dir = get_active_data_directory(app_handle.clone());
        
        // Resolve path layout format: {data_dir}/{ticker}/{exchange}_{api}/endpoint-metadata.json
        let fallback_exchange = if mode == "both" { "nse" } else { &mode };
        let cached_file_path = std::path::Path::new(&active_dir)
            .join(ticker)
            .join(format!("{}_{}", fallback_exchange, api))
            .join("endpoint-metadata.json");

        // If a previously archived historical backup snapshot exists on disk, read it instantly
        if cached_file_path.exists() {
            if let Ok(cached_json_string) = std::fs::read_to_string(cached_file_path) {
                if let Some(window) = app_handle.get_webview_window("main") {
                    #[derive(Clone, serde::Serialize)]
                    struct DirectStreamPayload { module_id: String, ticker: String, raw_json: String }

                    let target_module = if api == "symbol-core-data" { "stock_stats" } else { "company_profile" };

                    println!("\x1b[35m🚀 [CACHE DISPATCH]: Found local snapshots for [{}]. Pre-rendering to UI instantly!\x1b[0m", ticker);

                    // Blast historical string to UI over system RAM. Frontend populates charts instantly.
                    let _ = window.emit(
                        "live-memory-data", 
                        DirectStreamPayload {
                            module_id: target_module.to_string(),
                            ticker: ticker.to_string(),
                            raw_json: cached_json_string,
                        }
                    );
                }
            }
        }
    }

    // ============================================================================
    // 🎯 STEP 2: BACKWARD WARM PIPE FORWARDER (LIVE NETWORK OVERLAP RELOAD)
    // ============================================================================
    let mut writer_guard = GO_PROCESS_WRITER.lock().unwrap();
    if let Some(ref mut child) = *writer_guard {
        
        let command_payload = if is_stream_requested {
            format!("RUN {} {} {} --stream\n", ticker, mode, api)
        } else {
            format!("RUN {} {} {}\n", ticker, mode, api)
        };
        
        println!("\x1b[33m📡 [INTERCEPTOR -> GO STDIN]: Transparently routing instruction payload down warm pipe: {}\x1b[0m", command_payload.trim());
        
        child.write(command_payload.as_bytes())
            .map_err(|e| format!("Failed forwarding intercept payload text sequence down memory pipe stream: {}", e))?;
            
        Ok("Signal intercepted and dispatched to hot connection pool successfully".to_string())
    } else {
        Err("Persistent background Go service infrastructure layer is uninitialized or offline.".to_string())
    }
}