// stock-app/ui/backend/src/commands/downloader.rs
use std::process::{Stdio, Command};
use std::io::{BufRead, BufReader as StdBufReader}; 
use std::path::Path;
use std::sync::Mutex;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader as TokioBufReader, AsyncBufReadExt}; 
use crate::commands::data_loader::WorkspaceDataContext;
use crate::commands::data_dir::resolve_data_directory_headless;
use crate::commands::memory_pool::update_memory_cache;

#[derive(Clone, Debug)]
pub struct ActiveDownload {
    pub current_api: String,
    pub filename: String,
    pub percentage: f32,
    pub current_step: usize,
    pub total_steps: usize,
}

#[derive(Clone, Debug)]
pub struct IngestionProgress {
    pub ticker: String,
    pub nse_active: bool,
    pub bse_active: bool,
    pub nse_downloads: Vec<ActiveDownload>,
    pub bse_downloads: Vec<ActiveDownload>,
    pub is_done: bool,
    pub current_phase: String,
    pub last_step: usize,
}

pub static ACTIVE_INGESTION: Mutex<Option<IngestionProgress>> = Mutex::new(None);

pub fn initialize_persistent_go_daemon(_app_handle: tauri::AppHandle) {
    initialize_go_daemon();
}

pub fn initialize_go_daemon() {
    std::thread::spawn(move || {
        let active_dir = resolve_data_directory_headless().to_string_lossy().to_string();
        let mut binaries_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        binaries_dir.push("binaries");
        let mut sidecar_executable = None;

        if let Ok(entries) = std::fs::read_dir(binaries_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("downloader") {
                    sidecar_executable = Some(entry.path());
                    break;
                }
            }
        }

        let binary_path = match sidecar_executable {
            Some(p) => p,
            None => return,
        };

        let mut child = match Command::new(&binary_path)
            .args(&["--daemon", &format!("--data-dir={}", active_dir)])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn() 
        {
            Ok(c) => c,
            Err(_) => return,
        };

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let active_dir_clone = active_dir.clone();

        std::thread::spawn(move || {
            let mut reader = StdBufReader::new(stdout).lines();
            while let Some(Ok(line)) = reader.next() {
                let cleaned_line = strip_ansi_escape_codes(&line);
                println!("\x1b[96m[Go Output] {}\x1b[0m", cleaned_line);

                if line.starts_with("SIGNAL_COMPLETED:") {
                    let parts: Vec<&str> = line.trim().split(':').collect();
                    if parts.len() >= 3 {
                        let ticker = parts[1].to_uppercase();
                        let api = parts[2].to_string();

                        WorkspaceDataContext::invalidate_ticker(&ticker);

                        let file_path = Path::new(&active_dir_clone)
                            .join(&ticker)
                            .join(format!("nse_{}", api))
                            .join("endpoint-metadata.json");

                        if file_path.exists() {
                            if let Ok(raw_json) = std::fs::read_to_string(file_path) {
                                update_memory_cache(&ticker, &api, raw_json);
                            }
                        }
                    }
                }
            }
        });

        std::thread::spawn(move || {
            let mut reader = StdBufReader::new(stderr).lines();
            while let Some(Ok(line)) = reader.next() {
                println!("\x1b[31m🚨 [GO DAEMON ERROR]: {}\x1b[0m", line);
            }
        });

        let _ = child.wait();
    });
}

#[tauri::command]
pub async fn run_sidecar_downloader(
    _app_handle: tauri::AppHandle, 
    _data_dir_override: Option<String>,
    extra_args: Option<Vec<String>>, 
) -> Result<String, String> {
    let flags = extra_args.ok_or_else(|| "Missing execution flags".to_string())?;
    run_sidecar_downloader_native(flags).await
}

pub async fn run_sidecar_downloader_native(extra_args: Vec<String>) -> Result<String, String> {
    if extra_args.is_empty() { return Err("Flags empty".to_string()); }

    let ticker = extra_args.iter().find(|f| !f.starts_with("-")).cloned().unwrap_or_default().to_uppercase();
    let active_dir = resolve_data_directory_headless().to_string_lossy().to_string();
    let socket_path = Path::new(&active_dir).join("downloader_engine.sock");

    let mode = extra_args.iter()
        .find(|f| f.starts_with("--mode=") || f.starts_with("-mode="))
        .map(|f| f.split('=').nth(1).unwrap_or("both"))
        .unwrap_or("both");
    let initial_phase = if mode == "bse" { "BSE" } else { "NSE" };

    {
        let mut guard = ACTIVE_INGESTION.lock().unwrap();
        *guard = Some(IngestionProgress {
            ticker: ticker.clone(),
            nse_active: mode == "both" || mode == "nse",
            bse_active: mode == "both" || mode == "bse",
            nse_downloads: Vec::new(),
            bse_downloads: Vec::new(),
            is_done: false,
            current_phase: initial_phase.to_string(),
            last_step: 0,
        });
    }

    match tokio::net::UnixStream::connect(&socket_path).await {
        Ok(mut socket) => {
            let mut payload_parts = vec!["RUN".to_string()];
            payload_parts.extend(extra_args.clone());
            
            let instruction_payload = payload_parts.join(" ") + "\n";
            
            if socket.write_all(instruction_payload.as_bytes()).await.is_ok() {
                let mut reader = TokioBufReader::new(socket);
                let mut line = String::new();

                loop {
                    // Stop checkpoint guard
                    {
                        if ACTIVE_INGESTION.lock().unwrap().is_none() {
                            return Err("Cancelled by user request".to_string());
                        }
                    }

                    line.clear();
                    match reader.read_line(&mut line).await {
                        Ok(0) => break, 
                        Ok(_) => {
                            let trimmed = line.trim();
                            if trimmed.is_empty() {
                                continue;
                            }
                            if trimmed == "PAYLOAD_START" {
                                break;
                            }

                            if trimmed.starts_with("GO_TELEMETRY|") {
                                let parts: Vec<&str> = trimmed.split('|').collect();
                                let mut exchange = String::new();
                                let mut api_name = String::new();
                                let mut file_name = String::new();
                                let mut pct_val = 0.0f32;
                                let mut step_cur = 0;
                                let mut step_tot = 0;

                                for part in parts {
                                    if part.starts_with("EXCH:") {
                                        exchange = part[5..].to_string();
                                    } else if part.starts_with("API:") {
                                        api_name = part[4..].to_string();
                                    } else if part.starts_with("FILE:") {
                                        file_name = part[5..].to_string();
                                    } else if part.starts_with("PCT:") {
                                        if let Ok(val) = part[4..].parse::<f32>() {
                                            pct_val = val;
                                        }
                                    } else if part.starts_with("STEP:") {
                                        let step_str = &part[5..];
                                        let step_parts: Vec<&str> = step_str.split('/').collect();
                                        if step_parts.len() == 2 {
                                            step_cur = step_parts[0].parse::<usize>().unwrap_or(0);
                                            step_tot = step_parts[1].parse::<usize>().unwrap_or(0);
                                        }
                                    }
                                }

                                if let Some(ref mut progress) = *ACTIVE_INGESTION.lock().unwrap() {
                                    if !api_name.is_empty() {
                                        // 🎯 STATELESS DECOUPLING: Route directly into the targeted vector track based on the exchange tag
                                        let target_vec = if exchange == "NSE" {
                                            &mut progress.nse_downloads
                                        } else {
                                            &mut progress.bse_downloads
                                        };

                                        if let Some(existing) = target_vec.iter_mut().find(|d| d.current_api == api_name) {
                                            if !file_name.is_empty() { existing.filename = file_name; }
                                            if trimmed.contains("|PCT:") { existing.percentage = pct_val; }
                                            if step_cur > 0 { existing.current_step = step_cur; }
                                            if step_tot > 0 { existing.total_steps = step_tot; }
                                        } else {
                                            target_vec.push(ActiveDownload {
                                                current_api: api_name,
                                                filename: file_name,
                                                percentage: pct_val,
                                                current_step: step_cur,
                                                total_steps: step_tot,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                        Err(_) => break,
                    }
                }

                if extra_args.iter().any(|f| f == "--stream" || f == "-stream") {
                    let mut header_buffer = [0u8; 4];
                    if reader.read_exact(&mut header_buffer).await.is_ok() {
                        let payload_size = u32::from_be_bytes(header_buffer) as usize;
                        let mut data_buffer = vec![0u8; payload_size];
                        if reader.read_exact(&mut data_buffer).await.is_ok() {
                            if let Ok(raw_json_string) = String::from_utf8(data_buffer) {
                                let api = extra_args.iter()
                                    .find(|f| f.starts_with("--api="))
                                    .map(|f| f.split('=').nth(1).unwrap_or(""))
                                    .unwrap_or("");
                                
                                if !ticker.is_empty() && !api.is_empty() {
                                    update_memory_cache(&ticker, api, raw_json_string.clone());
                                }

                                if let Some(ref mut progress) = *ACTIVE_INGESTION.lock().unwrap() {
                                    progress.is_done = true;
                                }
                                return Ok(raw_json_string);
                            }
                        }
                    }
                    return Err("Failed reading payload response from socket stream".to_string());
                } 
                
                if let Some(ref mut progress) = *ACTIVE_INGESTION.lock().unwrap() {
                    progress.is_done = true;
                }
                return Ok("Signal accepted over socket layer successfully".to_string());
            }
            Err("Failed packing instructions down socket channel".to_string())
        }
        Err(e) => {
            let mut guard = ACTIVE_INGESTION.lock().unwrap();
            *guard = None; 
            Err(format!("🚨 [IPC CONNECTION FAILED]: {}", e))
        }
    }
}

fn strip_ansi_escape_codes(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            while let Some(&next_c) = chars.peek() {
                chars.next();
                if next_c == 'm' { break; }
            }
        } else {
            result.push(c);
        }
    }
    result
}