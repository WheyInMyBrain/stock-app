// stock-app/ui/frontend_native/src/core/data_manager.rs
use backend::commands::history::get_history_tickers_headless;

pub struct DataManager;

impl DataManager {
    /// 🔍 Isolated Fetcher: Coordinates with the backend library to pull tickers
    pub fn load_active_tickers() -> Vec<String> {
        get_history_tickers_headless()
    }
}