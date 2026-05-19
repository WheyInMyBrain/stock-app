use std::path::Path;

// Re-export all your distinct record structures so main.rs can easily bundle them
pub use crate::bse::financial_report::BseRecord;
// pub use crate::bse::shareholding::ShareholdingRecord; // When you add it later!

pub fn parse_bse_file(path: &Path, folder_name: &str) -> Result<Vec<BseRecord>, String> {
    match folder_name {
        "bse_financial-results-docs" => {
            // Pipes straight into your existing financial_report module
            crate::bse::financial_report::parse(path)
        }
        
        // 🚀 PLUG-AND-PLAY ADDITIONS HERE:
        // "bse_shareholding_pattern" => {
        //     crate::bse::shareholding::parse(path)
        // }
        
        _ => Err(format!("No custom parser registered for folder schema: '{}'", folder_name)),
    }
}