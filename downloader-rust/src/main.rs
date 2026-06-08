use std::path::Path;
use downloader_rust::client::{BseClient, NseClient};
use downloader_rust::nse::{execute_nse_strategy, api::NseEndpoint};
use downloader_rust::bse::{execute_bse_batch, api::BseEndpoint};
use downloader_rust::search::load_stock_metadata;

#[tokio::main]
async fn main() {
    let data_directory = Path::new("./data");
    let symbol = "IMFA";

    let nse_client = match NseClient::new().await {
        Ok(client) => client,
        Err(e) => {
            println!("[Main] ❌ Failed to warm up NSE cookie jars: {}", e);
            return;
        }
    };

    let bse_client = match BseClient::new().await {
        Ok(client) => client,
        Err(e) => {
            println!("[Main] ❌ Failed to warm up BSE cookie jars: {}", e);
            return;
        }
    };

    let meta = match load_stock_metadata(symbol, data_directory) {
        Ok(metadata) => metadata,
        Err(e) => {
            println!("[Main] ❌ Metadata missing from search cache block: {}", e);
            return;
        }
    };

    println!("[Main] 🔄 Running NSE delta timeline sync for {}...", symbol);
    match execute_nse_strategy(
        &nse_client,
        symbol,
        &meta.nse_code,
        NseEndpoint::RealTimeChartDelta(None),
        data_directory,
        false,
    )
    .await
    {
        Ok(_) => println!("[Main] ✅ NSE Delta ingestion pass finalized completely!"),
        Err(e) => println!("[Main] ❌ NSE Strategy runner failed: {}", e),
    }

    println!("[Main] 🔄 Launching BSE strategy batch collection suite for {}...", symbol);
    let bse_suite = vec![
        BseEndpoint::BulkDeals,
        BseEndpoint::BlockDeals,
    ];

    execute_bse_batch(
        &bse_client,
        symbol,
        &meta.bse_code,
        bse_suite,
        data_directory,
        1,
        false,
    )
    .await;

    println!("[Main] 🎉 All cross-exchange operational strategy tasks executed successfully!");
}