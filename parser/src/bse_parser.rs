use std::path::Path;
// Re-export the inner structure out so main.rs can read it easily
pub use crate::bse::financial_report::BseRecord;

pub fn parse_bse_file(path: &Path) -> Result<Vec<BseRecord>, String> {
    // This dispatcher pipes execution down into the bse/ folder module cleanly
    crate::bse::financial_report::parse(path)
}