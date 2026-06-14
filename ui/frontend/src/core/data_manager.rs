use backend::commands::history::get_history_tickers_headless;
use backend::commands::memory_pool::{CENTRAL_ACTIVE_SLOT};
use backend::database::overview::hydrate_overview_metadata;
use backend::database::analysis::hydrate_analysis_metadata;
use backend::database::financial::hydrate_raw_financial_statements;

pub struct DataManager;

impl DataManager {
    pub fn load_active_tickers() -> Vec<String> {
        get_history_tickers_headless()
    }

    pub fn ensure_overview_data(ticker: &str) {
        let ticker_upper = ticker.to_uppercase();
        let needs_hydration = if let Ok(slot_guard) = CENTRAL_ACTIVE_SLOT.read() {
            slot_guard.as_ref().map_or(true, |slot| {
                slot.ticker != ticker_upper || !slot.parsed_tables.contains_key("overview_metadata")
            })
        } else {
            true
        };

        if needs_hydration {
            if let Err(e) = hydrate_overview_metadata(ticker) {
                println!("[downloader] ❌ Ingestion hydration failed for '{}': {}", ticker, e);
            }
        }
    }

    pub fn ensure_financials_data(ticker: &str) {
        let ticker_upper = ticker.to_uppercase();
        let needs_hydration = if let Ok(slot_guard) = CENTRAL_ACTIVE_SLOT.read() {
            slot_guard.as_ref().map_or(true, |slot| {
                slot.ticker != ticker_upper || !slot.parsed_tables.contains_key("financial_metadata")
            })
        } else {
            true
        };

        if needs_hydration {
            if let Err(e) = hydrate_raw_financial_statements(ticker) {
                println!("[financials] ❌ Ingestion hydration failed for '{}': {}", ticker, e);
            }
        }
    }

    pub fn ensure_analysis_data(ticker: &str) {
        let ticker_upper = ticker.to_uppercase();
        let needs_hydration = if let Ok(slot_guard) = CENTRAL_ACTIVE_SLOT.read() {
            if let Some(ref slot) = *slot_guard {
                if slot.ticker == ticker_upper 
                    && slot.parsed_tables.contains_key("analysis_metadata") 
                    && slot.parsed_tables.contains_key("historical_chart_data") 
                {
                    false 
                } else {
                    true
                }
            } else {
                true
            }
        } else {
            true
        };

        if needs_hydration {
            if let Err(e) = hydrate_analysis_metadata(ticker) {
                println!("[analysis] ❌ Ingestion hydration failed for '{}': {}", ticker, e);
            }
        }
    }
}