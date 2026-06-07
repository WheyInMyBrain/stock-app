use backend::commands::downloader::run_sidecar_downloader_native;

/// 🛰️ Purified Core Bootstrap: Triggers the main backend library initialization loop
pub fn boot_daemon(_data_dir: String) {
    backend::initialize_backend();
}

/// ⚡ Purified Passthrough Wrapper: Dispatches download tasks down the pure library core channels
pub async fn dispatch_download(extra_args: Vec<String>) -> Result<String, String> {
    run_sidecar_downloader_native(extra_args).await
}