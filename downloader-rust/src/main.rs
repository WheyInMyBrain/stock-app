// downloader-rust/src/main.rs
use std::path::PathBuf;
use downloader::client::NseClient;
use downloader::nse::{execute_nse_strategy, api::NseEndpoint};

#[tokio::main]
async fn main() {
    println!("--- RUNNING NATIVE RUST SCRAPER INITIALIZATION TESTS ---");
    let test_data_dir = PathBuf::from("./data");

    let nse_client = match NseClient::new().await {
        Ok(client) => client,
        Err(e) => {
            println!("❌ NSE CLIENT INIT FAILED: {}", e);
            return;
        }
    };

    let symbol = "IMFA";

    // In your backend command: Just runs ONE isolated endpoint
    execute_nse_strategy(&nse_client, symbol, "1", NseEndpoint::CorporateDetails, &test_data_dir, false).await;
}