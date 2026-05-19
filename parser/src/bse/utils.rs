// Shared model for all BSE reporting types
#[derive(Debug, Clone)]
pub struct BseRecord {
    pub source_file: String,
    pub tag_name: String,
    pub context_id: String,
    pub date_bounds: String,
    pub raw_value: String,
}