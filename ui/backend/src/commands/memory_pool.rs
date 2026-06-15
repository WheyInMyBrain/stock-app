use std::sync::{RwLock, LazyLock};
use std::collections::HashMap;

pub struct ActiveTickerData {
    pub ticker: String,
    pub parsed_tables: HashMap<String, Box<dyn std::any::Any + Send + Sync>>,
    pub raw_endpoints: HashMap<String, Vec<u8>>,
}

pub static CENTRAL_ACTIVE_SLOT: LazyLock<RwLock<Option<ActiveTickerData>>> = LazyLock::new(|| {
    RwLock::new(None)
});

pub fn initialize_active_ticker(ticker: &str) {
    let mut slot = CENTRAL_ACTIVE_SLOT.write().unwrap();
    *slot = Some(ActiveTickerData {
        ticker: ticker.to_uppercase(),
        parsed_tables: HashMap::new(),
        raw_endpoints: HashMap::new(),
    });
}

pub fn clear_active_ticker() {
    let mut slot = CENTRAL_ACTIVE_SLOT.write().unwrap();
    *slot = None;
}

pub fn store_parsed_table<T: std::any::Any + Send + Sync>(table_key: &str, data: T) {
    let mut slot = CENTRAL_ACTIVE_SLOT.write().unwrap();
    if let Some(ref mut current) = *slot {
        current.parsed_tables.insert(table_key.to_string(), Box::new(data));
    }
}

pub fn with_active_table<T: std::any::Any + Send + Sync, R, F: FnOnce(&T) -> R>(table_key: &str, f: F) -> Option<R> {
    let slot = CENTRAL_ACTIVE_SLOT.read().unwrap();
    if let Some(ref current) = *slot {
        if let Some(table) = current.parsed_tables.get(table_key) {
            return table.as_ref().downcast_ref::<T>().map(f);
        }
    }
    None
}

pub fn update_memory_cache(ticker: &str, api_endpoint: &str, payload: String) {
    let mut slot = CENTRAL_ACTIVE_SLOT.write().unwrap();
    let ticker_upper = ticker.to_uppercase();
    
    if let Some(ref mut current) = *slot {
        if current.ticker == ticker_upper {
            current.raw_endpoints.insert(api_endpoint.to_string(), payload.into_bytes());
            return;
        }
    }
    
    let mut new_data = ActiveTickerData {
        ticker: ticker_upper,
        parsed_tables: HashMap::new(),
        raw_endpoints: HashMap::new(),
    };
    new_data.raw_endpoints.insert(api_endpoint.to_string(), payload.into_bytes());
    *slot = Some(new_data);
}

pub fn read_memory_cache(ticker: &str, api_endpoint: &str) -> Option<String> {
    let slot = CENTRAL_ACTIVE_SLOT.read().unwrap();
    if let Some(ref current) = *slot {
        if current.ticker == ticker.to_uppercase() {
            if let Some(bytes) = current.raw_endpoints.get(api_endpoint) {
                return String::from_utf8(bytes.clone()).ok();
            }
        }
    }
    None
}