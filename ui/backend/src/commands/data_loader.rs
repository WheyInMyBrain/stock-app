use std::path::{Path, PathBuf};
use std::fs;
use std::sync::RwLock;
use std::collections::HashMap;
use serde_json::{json, Value};
use tauri::AppHandle;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use crate::commands::data_dir::get_active_data_directory;

#[derive(Clone, serde::Serialize)]
pub struct TickerDataPayload {
    pub datasets: HashMap<String, Value>, 
}

static CORE_DATA_CACHE: RwLock<Option<HashMap<String, TickerDataPayload>>> = RwLock::new(None);

pub struct WorkspaceDataContext {
    ticker: String,
    ticker_folder_path: PathBuf,
}

impl WorkspaceDataContext {
    /// 🎯 ZERO I/O INITIALIZATION
    pub fn load(app_handle: &AppHandle, ticker: &str) -> Self {
        let ticker_upper = ticker.to_uppercase();
        let active_root = get_active_data_directory(app_handle.clone());
        let ticker_folder_path = PathBuf::from(active_root).join(&ticker_upper);

        Self {
            ticker: ticker_upper,
            ticker_folder_path,
        }
    }

    /// 🎯 AUTOMATED MULTI-TARGET COMPILATION
    pub fn get_multiple_datasets(&self, targets: &[&str]) -> Value {
        let mut compiled_datasets = serde_json::Map::new();

        for target in targets {
            let dataset_value = self.get_dataset(target);
            
            // 🚀 Transform "nse_symbol-core-data/endpoint-metadata" -> "nse_symbol-core-data__endpoint-metadata"
            let normalized_key = target.replace('/', "__");
            
            compiled_datasets.insert(normalized_key, dataset_value);
        }

        Value::Object(compiled_datasets)
    }

    /// 🎯 UNIVERSAL SURGICAL LOAD
    pub fn get_dataset(&self, target_path_selector: &str) -> Value {
        let ticker_upper = &self.ticker;

        // 1. Check RAM Cache
        {
            let cache_read = CORE_DATA_CACHE.read().unwrap();
            if let Some(cache_map) = &*cache_read {
                if let Some(cached_payload) = cache_map.get(ticker_upper) {
                    if let Some(cached_json) = cached_payload.datasets.get(target_path_selector) {
                        return cached_json.clone();
                    }
                }
            }
        }

        // 2. Cache Miss: Reconstruct disk path dynamically supporting infinite subfolders
        let mut target_disk_path = self.ticker_folder_path.clone();
        for component in target_path_selector.split('/') {
            if !component.is_empty() {
                target_disk_path.push(component);
            }
        }

        let mut loaded_value = json!({});

        // 🎯 FIX 1: Auto-Fallback to .json extension if it doesn't exist as-is and has no extension specified
        if !target_disk_path.exists() && target_disk_path.extension().is_none() {
            let mut json_fallback_path = target_disk_path.clone();
            json_fallback_path.set_extension("json");
            if json_fallback_path.is_file() {
                target_disk_path = json_fallback_path;
            }
        }

        // 🎯 FIX 2: Evaluate physical disk node types safely supporting deep trees
        if target_disk_path.exists() {
            if target_disk_path.is_file() {
                loaded_value = Self::read_and_decode_file(&target_disk_path);
            } else if target_disk_path.is_dir() {
                if let Some(folder_json) = Self::ingest_directory_tree(&target_disk_path) {
                    loaded_value = folder_json;
                }
            }
        }

        // 3. Commit to global memory cache
        {
            let mut cache_write = CORE_DATA_CACHE.write().unwrap();
            if cache_write.is_none() {
                *cache_write = Some(HashMap::new());
            }
            if let Some(cache_map) = &mut *cache_write {
                let payload = cache_map.entry(ticker_upper.clone()).or_insert_with(|| TickerDataPayload {
                    datasets: HashMap::new(),
                });
                payload.datasets.insert(target_path_selector.to_string(), loaded_value.clone());
            }
        }

        loaded_value
    }

    /// 🎯 POLYMORPHIC DECODER ENGINE
    fn read_and_decode_file(file_path: &Path) -> Value {
        let extension = file_path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();

        match extension.as_str() {
            "json" => {
                if let Ok(content) = fs::read_to_string(file_path) {
                    if let Ok(parsed_json) = serde_json::from_str::<Value>(&content) {
                        return parsed_json;
                    }
                }
                json!({})
            }
            "md" | "txt" | "csv" => {
                if let Ok(text_content) = fs::read_to_string(file_path) {
                    json!(text_content)
                } else {
                    json!("")
                }
            }
            "parquet" | "bin" => {
                if let Ok(bytes) = fs::read(file_path) {
                    let b64_encoded = STANDARD.encode(bytes);
                    json!({
                        "file_path": file_path.to_string_lossy().to_string(),
                        "file_format": extension,
                        "bytes_base64": b64_encoded
                    })
                } else {
                    json!({})
                }
            }
            _ => json!({})
        }
    }

    /// Dynamic Directory loop that accommodates all file formats present inside a given subdirectory
    fn ingest_directory_tree(dir_path: &Path) -> Option<Value> {
        let mut nested_map = HashMap::new();
        let mut total_files = 0;

        if let Ok(sub_entries) = fs::read_dir(dir_path) {
            for sub_entry in sub_entries.flatten() {
                let path = sub_entry.path();
                if path.is_file() {
                    total_files += 1;
                    let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();
                    let decoded_file_value = Self::read_and_decode_file(&path);
                    nested_map.insert(file_name, decoded_file_value);
                }
            }
        }

        if total_files == 0 { return None; }
        
        // 🎯 FIX 3: Removed volatile flat collapsing to keep response data structures predictable
        Some(json!(nested_map))
    }

    pub fn invalidate_ticker(ticker: &str) {
        let ticker_upper = ticker.to_uppercase();
        let mut cache_write = CORE_DATA_CACHE.write().unwrap();
        if let Some(cache_map) = &mut *cache_write {
            cache_map.remove(&ticker_upper);
            println!("🔄 [DATA DAEMON]: Dropped memory cache allocation for: {}", ticker_upper);
        }
    }
}