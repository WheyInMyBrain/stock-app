// downloader-rust/src/main.rs
use std::path::PathBuf;
use downloader::client::BseClient;
use downloader::bse::{execute_bse_strategy, api::BseEndpoint};

#[tokio::main]
async fn main() {
    println!("--- RUNNING NATIVE RUST SCRAPER INITIALIZATION TESTS ---");
    let test_data_dir = PathBuf::from("./data");

    let bse_client = match BseClient::new().await {
        Ok(client) => client,
        Err(e) => {
            println!("❌ NSE CLIENT INIT FAILED: {}", e);
            return;
        }
    };

    let symbol = "IMFA";

    // In your backend command: Just runs ONE isolated endpoint
    let _ = execute_bse_strategy(&bse_client, symbol, "533047", BseEndpoint::CorporateDetails, &test_data_dir, false).await;
}