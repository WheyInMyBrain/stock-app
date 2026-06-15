// stock-app/ui/backend/src/database/mod.rs

pub mod overview;
pub mod analysis;
pub mod financial;

use std::fs;
use crate::commands::data_dir::resolve_data_directory_headless;
use crate::commands::memory_pool::{CENTRAL_ACTIVE_SLOT, ActiveTickerData};

pub struct WorkspaceDataLoader {
    ticker: String,
}

impl WorkspaceDataLoader {
    pub fn bind(ticker: &str) -> Self {
        let ticker_upper = ticker.to_uppercase();

        let mut slot = CENTRAL_ACTIVE_SLOT.write().unwrap();
        if slot.is_none() || slot.as_ref().unwrap().ticker != ticker_upper {
            *slot = Some(ActiveTickerData {
                ticker: ticker_upper.clone(),
                parsed_tables: std::collections::HashMap::new(),
                raw_endpoints: std::collections::HashMap::new(), 
            });
        }

        Self { ticker: ticker_upper }
    }

    pub fn load_raw_bytes(&self, target_path: &str) -> Result<Vec<u8>, String> {
        let ticker_upper = self.ticker.to_uppercase();

        // 🎯 STEP 1 AND 3 CACHE LOGIC REMOVED
        let parts: Vec<&str> = target_path.split('/').filter(|s| !s.is_empty()).collect();
        if parts.is_empty() {
            return Err("Empty target file path selector".to_string());
        }

        let data_root = resolve_data_directory_headless();
        let filename = parts.last().unwrap_or(&"");

        let mut folder_variants = vec![parts[0].to_string()];
        if parts[0].starts_with("nse_") || parts[0].starts_with("bse_") {
            if parts[0].len() > 4 {
                folder_variants.push(parts[0][4..].to_string());
            }
        }

        let mut final_path = None;
        let mut tried_paths = Vec::new();

        for folder in folder_variants {
            let mut sub_elements = Vec::new();
            if parts.len() > 2 {
                for i in 1..(parts.len() - 1) {
                    sub_elements.push(parts[i]);
                }
            }

            let mut p1 = data_root.join(&ticker_upper).join(&folder);
            for sub in &sub_elements { p1.push(sub); }
            p1.push(filename);
            if !p1.exists() && p1.extension().is_none() { p1.set_extension("json"); }
            tried_paths.push(p1.to_string_lossy().to_string());
            if p1.is_file() {
                final_path = Some(p1);
                break;
            }

            let mut p2 = data_root.join(&folder).join(&ticker_upper);
            for sub in &sub_elements { p2.push(sub); }
            p2.push(filename);
            if !p2.exists() && p2.extension().is_none() { p2.set_extension("json"); }
            tried_paths.push(p2.to_string_lossy().to_string());
            if p2.is_file() {
                final_path = Some(p2);
                break;
            }
        }

        let valid_path = match final_path {
            Some(p) => p,
            None => return Err(format!("Filing nodes missing. Scanned variants: [{}]", tried_paths.join(", "))),
        };

        // 🎯 OS FILE READ (Stateless)
        fs::read(&valid_path).map_err(|e| format!("IO failure on {}: {}", valid_path.display(), e))
    }

    pub fn load_directory_filenames(&self, target_folder_path: &str) -> Result<Vec<String>, String> {
        let ticker_upper = self.ticker.to_uppercase();
        let parts: Vec<&str> = target_folder_path.split('/').filter(|s| !s.is_empty()).collect();
        if parts.is_empty() {
            return Err("Empty target folder path selector".to_string());
        }

        let data_root = resolve_data_directory_headless();
        let mut folder_variants = vec![parts[0].to_string()];
        if parts[0].starts_with("nse_") || parts[0].starts_with("bse_") {
            if parts[0].len() > 4 {
                folder_variants.push(parts[0][4..].to_string());
            }
        }

        let mut final_dir_path = None;
        for folder in folder_variants {
            let mut sub_elements = Vec::new();
            if parts.len() > 1 {
                for i in 1..parts.len() {
                    sub_elements.push(parts[i]);
                }
            }

            // Variant A: data/TICKER/folder/sub-elements
            let mut p1 = data_root.join(&ticker_upper).join(&folder);
            for sub in &sub_elements { p1.push(sub); }
            if p1.is_dir() { final_dir_path = Some(p1); break; }

            // Variant B: data/folder/TICKER/sub-elements
            let mut p2 = data_root.join(&folder).join(&ticker_upper);
            for sub in &sub_elements { p2.push(sub); }
            if p2.is_dir() { final_dir_path = Some(p2); break; }
        }

        let valid_dir = match final_dir_path {
            Some(d) => d,
            None => return Err(format!("Target directory path not resolved: {}", target_folder_path)),
        };

        let mut entries_list = Vec::new();
        if let Ok(read_dir) = fs::read_dir(valid_dir) {
            for entry in read_dir.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        let fname = entry.file_name().to_string_lossy().to_string();
                        entries_list.push(fname);
                    }
                }
            }
        }

        Ok(entries_list)
    }

    pub fn load_json_struct<T: serde::de::DeserializeOwned>(&self, target_path: &str) -> Result<T, String> {
        let bytes = self.load_raw_bytes(target_path)?;
        serde_json::from_slice(&bytes).map_err(|e| format!("JSON syntax error: {}", e))
    }

    pub fn load_text_string(&self, target_path: &str) -> Result<String, String> {
        let bytes = self.load_raw_bytes(target_path)?;
        String::from_utf8(bytes).map_err(|e| format!("String format validation error: {}", e))
    }
}