pub mod download;
pub mod api;

use std::path::Path;
use crate::client::BseClient;
use download::build_save_directory;
use api::BseEndpoint;
use futures::stream::{self, StreamExt};
use reqwest::header::{ACCEPT, ORIGIN, REFERER, USER_AGENT};

const DESKTOP_USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";
const REFERER_VAL: &str = "https://www.bseindia.com/";
const ORIGIN_VAL: &str = "https://www.bseindia.com";
const ACCEPT_JSON_VAL: &str = "application/json, text/plain, */*";

pub async fn execute_bse_strategy(
    client: &BseClient,
    symbol: &str,
    bse_code: &str,
    endpoint: BseEndpoint,
    global_data_dir: &Path,
    only_json: bool,
) -> Result<(), String> {
    let output_dir = build_save_directory(global_data_dir, symbol, endpoint.name())?;
    let api_url = endpoint.build_url(symbol, bse_code, global_data_dir);
    
    println!("\x1b[96m[downloader] [BSE-Orchestrator] 📡 Ingesting: {}...\x1b[0m", endpoint.name());

    let response = client.http_client.get(&api_url)
        .header(USER_AGENT, DESKTOP_USER_AGENT)
        .header(REFERER, REFERER_VAL)
        .header(ORIGIN, ORIGIN_VAL)
        .header(ACCEPT, ACCEPT_JSON_VAL)
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

    if only_json {
        println!("\x1b[96m[downloader] [BSE-Orchestrator] 🟢 JSON-Only mode active for '{}'. Bypassing file attachments.\x1b[0m", endpoint.name());
        return Ok(());
    }

    let attachments = endpoint.parse_attachments(symbol, global_data_dir, &raw_bytes);
    if !attachments.is_empty() {
        println!("\x1b[96m[downloader] [BSE-Orchestrator] 📂 Found {} child file targets linked inside '{}'. Downloading sequentially...\x1b[0m", attachments.len(), endpoint.name());
        
        for (period, download_url) in attachments {
            let extension = std::path::Path::new(&download_url)
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("xml");

            let file_name = format!("{}.{}", period, extension);
            let destination = output_dir.join(&file_name);

            if destination.exists() {
                println!("\x1b[96m[downloader]   [file-worker] skip (exists): {}\x1b[0m", file_name);
                continue;
            }

            println!("\x1b[96m[downloader]   [file-worker] ⏳ Fetching: {}\x1b[0m", file_name);
            let file_req = client.http_client.get(&download_url)
                .header(USER_AGENT, DESKTOP_USER_AGENT)
                .header(REFERER, REFERER_VAL)
                .send()
                .await;

            match file_req {
                Ok(file_resp) => {
                    let status = file_resp.status();
                    if status == reqwest::StatusCode::OK {
                        match file_resp.bytes().await {
                            Ok(file_bytes) => {
                                match std::fs::write(&destination, file_bytes) {
                                    Ok(_) => println!("\x1b[96m[downloader]   [file-worker] ✅ Saved: {}\x1b[0m", file_name),
                                    Err(e) => println!("\x1b[96m[downloader]   [file-worker] ❌ Disk write error for {}: {}\x1b[0m", file_name, e),
                                }
                            }
                            Err(e) => println!("\x1b[96m[downloader]   [file-worker] ❌ Failed reading response bytes for {}: {}\x1b[0m", file_name, e),
                        }
                    } else {
                        println!("\x1b[96m[downloader]   [file-worker] ❌ Server rejected request for {}: HTTP status {}\x1b[0m", file_name, status);
                    }
                }
                Err(e) => println!("\x1b[96m[downloader]   [file-worker] ❌ Connection error for {}: {}\x1b[0m", file_name, e),
            }
            
            tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
        }
    }

    Ok(())
}

pub async fn execute_bse_batch(
    client: &BseClient,
    symbol: &str,
    bse_code: &str,
    endpoints: Vec<BseEndpoint>,
    global_data_dir: &Path,
    max_concurrency: usize,
    only_json: bool,
) {
    println!("\x1b[96m[downloader] [BSE-Batch-Orchestrator] ⚙️ Spawning collection loop (Max Concurrency Capacity: {})\x1b[0m", max_concurrency);

    stream::iter(endpoints)
        .for_each_concurrent(max_concurrency, |endpoint| async move {
            match execute_bse_strategy(client, symbol, bse_code, endpoint, global_data_dir, only_json).await {
                Ok(_) => println!("\x1b[96m[downloader] [BSE-Batch-Orchestrator]  Success: {}\x1b[0m", endpoint.name()),
                Err(e) => println!("\x1b[96m[downloader] [BSE-Batch-Orchestrator] ❌ Failure on Strategy [{}]: {}\x1b[0m", endpoint.name(), e),
            }
        })
        .await;
}