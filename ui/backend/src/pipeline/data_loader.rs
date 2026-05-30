// stock-app/ui/backend/src/pipeline/data_loader.rs

use std::path::PathBuf;
use std::fs;
use serde_json::{json, Value};
use tauri::AppHandle;
use crate::commands::data_dir::get_active_data_directory;

pub struct WorkspaceDataContext {
    // This maps your specific file data into memory
    pub endpoint_metadata: Value,
}

impl WorkspaceDataContext {
    /// 🎯 UNIFIED LOADER ENTRYPOINT
    pub fn load(app_handle: &AppHandle, ticker: &str) -> Self {
        // We use your abstract helper function to fetch whatever folder and file we want!
        // This maps exactly to: {active_data_dir}/{ticker}/nse_symbol-core-data/endpoint-metadata.json
        let metadata_payload = Self::load_json_component(
            app_handle,
            ticker,
            "nse_symbol-core-data",
            "endpoint-metadata.json"
        );

        Self {
            endpoint_metadata: metadata_payload,
        }
    }

    /// 🎯 THE ABSTRACT FUNCTION
    /// Pass the folder name and file name, and it loads it cleanly from your active directory state.
    /// If you add new files later, you just call this function again with the new names.
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