use std::path::Path;

pub use crate::nse::utils::NseRecord;

pub fn parse_nse_file(path: &Path, folder_name: &str) -> Result<Vec<NseRecord>, String> {
    match folder_name {
        "nse_corporates-financial-results" => {
            crate::nse::financial_report::parse(path)
        }


        
        _ => Err(format!("No custom NSE parser registered for folder schema: '{}'", folder_name)),
    }
}