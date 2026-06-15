use std::process::Stdio;
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;

// Import your global application state systems natively
use crate::commands::data_dir::resolve_data_directory_headless;
use crate::commands::memory_pool::CENTRAL_ACTIVE_SLOT;

/// 🖥️ FRONTEND-FACING ENGINE: Safe, asynchronous, non-blocking process manager.
/// Spawns the centralized C++ binary out-of-process and pipes output streams right to egui.
pub async fn run_ai_analyst_stream(user_query: String) -> Result<mpsc::UnboundedReceiver<String>, String> {
    // 1. Fetch the active stock symbol out of your global memory pool natively
    let active_ticker = {
        let pool_lock = CENTRAL_ACTIVE_SLOT.read().map_err(|_| "Failed to lock memory pool context.")?;
        match *pool_lock {
            Some(ref data) => data.ticker.clone(), 
            None => return Err("No ticker selected in the UI. Please activate a company first.".to_string()),
        }
    };

    // 2. Resolve data directory roots natively
    let resolved_data_dir = resolve_data_directory_headless();
    let data_dir_str = resolved_data_dir.to_str().ok_or("Invalid path encoding for data root folder.")?.to_string();

    // 3. Locate the centralized sidecar binary path safely (mirroring your exact system toolchain)
    #[cfg(target_os = "windows")]
    let ext = ".exe";
    #[cfg(not(target_os = "windows"))]
    let ext = "";

    // Safely deduce target triple string programmatically via conditional compilation to avoid env macro blocks
    let target_triple = if cfg!(target_arch = "aarch64") && cfg!(target_os = "macos") {
        "aarch64-apple-darwin"
    } else if cfg!(target_arch = "x86_64") && cfg!(target_os = "windows") {
        "x86_64-pc-windows-msvc"
    } else {
        "x86_64-unknown-linux-gnu" // Fallback fallback safety string
    };

    // Navigate to target/[profile]/binaries/ai_agent-[target_triple]
    let mut sidecar_executable_path = std::env::current_exe()
        .map_err(|e| format!("Failed to locate running process layout context: {}", e))?;
    sidecar_executable_path.pop(); // Pop current binary file execution context
    sidecar_executable_path.push("binaries"); // Enter binaries playground
    sidecar_executable_path.push(format!("ai_agent-{}{}", target_triple, ext));

    if !sidecar_executable_path.exists() {
        return Err(format!("C++ Sidecar Agent executable missing at expected target folder: {}", sidecar_executable_path.display()));
    }

    // 4. Setup the async channel loop
    let (tx, rx) = mpsc::unbounded_channel();

    // 5. Spawn the central C++ application out-of-band via command line arguments
    let mut child = Command::new(sidecar_executable_path)
        .arg("--query")
        .arg(&user_query)
        .arg("--ticker")
        .arg(&active_ticker)
        .arg("--data-dir")
        .arg(&data_dir_str)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped()) 
        .spawn()
        .map_err(|e| format!("Failed to invoke background C++ processing engine: {}", e))?;

    let stdout = child.stdout.take().ok_or("Failed to attach to C++ standard output channel.")?;

    // 6. Monitored background task tracking standard output changes
    tokio::spawn(async move {
        let mut reader = BufReader::new(stdout).lines();
        
        while let Ok(Some(line)) = reader.next_line().await {
            // We append a trailing newline to match traditional text flow formatting blocks into egui markdown
            let chunk_output = format!("{}\n", line);
            if tx.send(chunk_output).is_err() {
                break; // UI layer receiver dropped out, stop child process thread loop gracefully
            }
        }
        
        let _ = child.wait().await;
    });

    Ok(rx)
}