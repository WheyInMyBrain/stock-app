use regex::Regex;

#[derive(Debug, Clone)]
pub struct UnifiedOcrOutput {
    pub source_file: String,
    pub statement_type: String,
    pub context: String,
    pub tag: String,
    pub date_bounds: String,
    pub payload_value: String,
}

pub trait OcrStatementExtractor: Send + Sync {
    fn statement_type(&self) -> &'static str;
    fn section_heading_regex(&self) -> &Regex;
    fn parse_table(&self, file_name: &str, header_text: &str, table_str: &str) -> Vec<UnifiedOcrOutput>;
}