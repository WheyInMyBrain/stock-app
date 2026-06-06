// stock-app/ui/backend/src/commands/memory_pool.rs
use std::collections::HashMap;
use std::sync::{RwLock, LazyLock};

/// 📊 THE CENTRAL IN-MEMORY BLOB CORE
/// A flat, completely abstract Key-Value store sitting inside shared RAM.
pub static CENTRAL_MEMORY_POOL: LazyLock<RwLock<HashMap<String, String>>> = LazyLock::new(|| {
    RwLock::new(HashMap::new())
});

/// 📥 Ingestion Link: Overwrites or seeds a data bucket in RAM instantly
pub fn update_memory_cache(ticker: &str, api_endpoint: &str, payload: String) {
    let key = format!("{}__{}", ticker.to_uppercase(), api_endpoint);
    let mut pool = CENTRAL_MEMORY_POOL.write().unwrap();
    pool.insert(key, payload);
}

/// 📤 Frame-Pulling Link: UI threads borrow direct data from RAM continuously at 60+ FPS
pub fn read_memory_cache(ticker: &str, api_endpoint: &str) -> Option<String> {
    let key = format!("{}__{}", ticker.to_uppercase(), api_endpoint);
    let pool = CENTRAL_MEMORY_POOL.read().unwrap();
    pool.get(&key).cloned()
}