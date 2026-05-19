// Shared structural model for all NSE reporting variations
#[derive(Debug, Clone)]
pub struct NseRecord {
    pub source_file: String,
    pub tag_name: String,
    pub context_id: String,
    pub date_bounds: String,
    pub raw_value: String,
}