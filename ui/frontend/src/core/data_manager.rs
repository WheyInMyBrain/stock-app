use backend::commands::history::get_history_tickers_headless;
use backend::commands::memory_pool::CENTRAL_ACTIVE_SLOT;
use backend::database::overview::hydrate_overview_metadata;

pub struct DataManager;

impl DataManager {
    pub fn load_active_tickers() -> Vec<String> {
        get_history_tickers_headless()
    }

    pub fn ensure_overview_data(ticker: &str) {
        let needs_hydration = if let Ok(slot_guard) = CENTRAL_ACTIVE_SLOT.read() {
            slot_guard.as_ref().map_or(true, |slot| {
                slot.ticker != ticker.to_uppercase() || !slot.parsed_tables.contains_key("overview_metadata")
            })
        } else {
            false
        };

        if needs_hydration {
            if let Err(e) = hydrate_overview_metadata(ticker) {
                println!("\x1b[96m[downloader] ❌ Ingestion hydration failed for '{}': {}\x1b[0m", ticker, e);
            }
        }
    }
}