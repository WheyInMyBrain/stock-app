// stock-app/ui/backend/src/pipeline/data_loader.rs

use std::path::PathBuf;
use std::fs;
use std::sync::RwLock;
use std::collections::HashMap;
use serde_json::{json, Value};
use tauri::AppHandle;
use crate::commands::data_dir::get_active_data_directory;

// 🎯 GLOBAL MEMORY CACHE CELL: Holds parsed JSON trees across active threads
// This stops rapid concurrent UI card reads from spamming the hard drive.
static CORE_DATA_CACHE: RwLock<Option<HashMap<String, Value>>> = RwLock::new(None);

pub struct WorkspaceDataContext {
    // This maps your specific file data into memory
    pub endpoint_metadata: Value,
}

impl WorkspaceDataContext {
    /// UNIFIED LOADER ENTRYPOINT
    pub fn load(app_handle: &AppHandle, ticker: &str) -> Self {
        let ticker_upper = ticker.to_uppercase();

        // 🎯 1. Fast Path: Check our memory cache container first before hitting the disk
        {
            let cache_read = CORE_DATA_CACHE.read().unwrap();
            if let Some(ref cache_map) = *cache_read {
                if let Some(cached_json) = cache_map.get(&ticker_upper) {
                    return Self {
                        endpoint_metadata: cached_json.clone(),
                    };
                }
            }
        }

        // 🎯 2. Slow Path (Cache Miss): Run your abstract function to load it from disk
        let metadata_payload = Self::load_json_component(
            app_handle,
            &ticker_upper, // Use upper-cased ticker argument directly
            "nse_symbol-core-data",
            "endpoint-metadata.json"
        );

        // 🎯 3. Commit parsed payload frame straight into memory cache map allocation
        {
            let mut cache_write = CORE_DATA_CACHE.write().unwrap();
            if cache_write.is_none() {
                *cache_write = Some(HashMap::new());
            }
            if let Some(ref mut cache_map) = *cache_write {
                cache_map.insert(ticker_upper, metadata_payload.clone());
            }
        }

        Self {
            endpoint_metadata: metadata_payload,
        }
    }

    /// 🎯 FLUSH SPECIFIC ASSET CACHE LOOP
    /// Call this when your background update daemon successfully flushes fresh ticks to disk!
    pub fn invalidate_ticker(ticker: &str) {
        let ticker_upper = ticker.to_uppercase();
        let mut cache_write = CORE_DATA_CACHE.write().unwrap();
        if let Some(ref mut cache_map) = *cache_write {
            cache_map.remove(&ticker_upper);
            println!("🔄 [DATA DAEMON]: Dropped memory cache allocation for: {}", ticker_upper);
        }
    }

    pub fn load_json_component(
        app_handle: &AppHandle,
        sub_folder: &str,
        inner_folder: &str,
        file_name: &str
    ) -> Value {
        // 1. Core dynamic root resolution straight from your data_dir.rs configuration!
        let active_root = get_active_data_directory(app_handle.clone());
        
        // 2. Build the exact physical path on disk cleanly
        let mut target_path = PathBuf::from(active_root);
        target_path.push(sub_folder);    // e.g., "IMFA"
        target_path.push(inner_folder);  // e.g., "nse_symbol-core-data"
        target_path.push(file_name);     // e.g., "endpoint-metadata.json"

        // 3. Read and parse the document safely
        if !target_path.exists() {
            return json!({}); // Return clean empty fallback object if file doesn't exist yet
        }

        fs::read_to_string(&target_path)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_else(|| json!({}))
    }
}