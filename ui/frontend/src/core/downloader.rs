use backend::commands::downloader::{initialize_exchange_clients, run_all};

pub fn boot_daemon(_data_dir: String) {
    tokio::spawn(async {
        let _ = initialize_exchange_clients().await;
    });
}

pub async fn dispatch_download(ticker: String, nse_active: bool, bse_active: bool) -> Result<String, String> {
    if !nse_active && !bse_active {
        return Err("No exchange selected for download track".to_string());
    }

    run_all(&ticker, nse_active, bse_active).await?;
    Ok("Success".to_string())
}