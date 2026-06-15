pub mod api;
pub mod download;

use std::path::Path;
use download::build_save_directory; 
use api::MiscEndpoint;
use futures::stream::{self, StreamExt};

pub async fn execute_misc_strategy(
    symbol: &str,
    endpoint: MiscEndpoint,
    global_data_dir: &Path,
) -> Result<(), String> {
    let output_dir = build_save_directory(global_data_dir, symbol, endpoint.name())?;
    let api_url = endpoint.build_url();
    
    println!("\x1b[96m[downloader] [Misc-Orchestrator] 📡 Ingesting Outside Source: {}...\x1b[0m", endpoint.name());

    let headers = endpoint.build_headers();

    let standalone_client = reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .map_err(|e| format!("Failed to initialize standalone HTTP engine: {}", e))?;

    let response = standalone_client.get(&api_url)
        .send()
        .await
        .map_err(|e| format!("Network transmission failure: {}", e))?;

    if response.status() != reqwest::StatusCode::OK {
        return Err(format!("Server rejected API call with status: {}", response.status()));
    }

    let raw_bytes = response.bytes().await
        .map_err(|e| format!("Failed to buffer response stream: {}", e))?;

    let metadata_path = output_dir.join("endpoint-metadata.json");
    std::fs::write(&metadata_path, &raw_bytes)
        .map_err(|e| format!("Failed storing layout metadata to disk: {}", e))?;

    println!("\x1b[96m[downloader] [Misc-Orchestrator] ✅ Successfully saved endpoint-metadata.json\x1b[0m");
    Ok(())
}

pub async fn execute_misc_batch(
    symbol: &str,
    endpoints: Vec<MiscEndpoint>,
    global_data_dir: &Path,
    max_concurrency: usize,
) {
    println!("\x1b[96m[downloader] [Misc-Batch-Orchestrator] ⚙️ Spawning collection loop (Max Concurrency Capacity: {})\x1b[0m", max_concurrency);

    stream::iter(endpoints)
        .for_each_concurrent(max_concurrency, |endpoint| async move {
            match execute_misc_strategy(symbol, endpoint, global_data_dir).await {
                Ok(_) => println!("\x1b[96m[downloader] [Misc-Batch-Orchestrator]  Success: {}\x1b[0m", endpoint.name()),
                Err(e) => println!("\x1b[96m[downloader] [Misc-Batch-Orchestrator] ❌ Failure on Strategy [{}]: {}\x1b[0m", endpoint.name(), e),
            }
        })
        .await;
}