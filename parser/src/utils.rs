// parser/src/utils.rs
use polars::prelude::*;
use std::fs;
use std::fs::File;
use std::path::Path;

/// ⚡ Zero-Copy Columnar Dataset Serializer for Standard Exchange Filings (XML)
pub fn save_to_parquet<P: AsRef<Path>>(
    data_dir_base: P,
    ticker: &str,
    folder_name: &str,
    mut files: Vec<String>,
    mut tags: Vec<String>,
    mut contexts: Vec<String>,
    mut dates: Vec<String>,
    mut values: Vec<String>,
) -> PolarsResult<String> {
    // 🎯 FIXED: Removed unnecessary 'mut' keyword to comply with Cargo analysis rules
    let output_dir = data_dir_base.as_ref().join(ticker).join("parquets");
    if let Err(e) = fs::create_dir_all(&output_dir) {
        return Err(PolarsError::ComputeError(
            format!("Failed to create directory structure {:?}: {}", output_dir, e).into()
        ));
    }

    let output_file_path = output_dir.join(format!("{}.parquet", folder_name));

    let s_files = Series::new("source_file".into(), std::mem::take(&mut files));
    let s_tags = Series::new("tag_name".into(), std::mem::take(&mut tags));
    let s_contexts = Series::new("context_id".into(), std::mem::take(&mut contexts));
    let s_dates = Series::new("date_bounds".into(), std::mem::take(&mut dates));
    let s_values = Series::new("raw_value".into(), std::mem::take(&mut values));

    let mut df = DataFrame::new(vec![s_files, s_tags, s_contexts, s_dates, s_values])?;

    let output_file = File::create(&output_file_path).map_err(|e| {
        PolarsError::ComputeError(format!("Failed to build physical output track at {:?}: {}", output_file_path, e).into())
    })?;
    
    ParquetWriter::new(output_file).finish(&mut df)?;

    Ok(output_file_path.to_string_lossy().into_owned())
}

/// ⚡ Zero-Copy Columnar Dataset Serializer for Annual OCR Data (Markdown)
pub fn save_ocr_to_parquet<P: AsRef<Path>>(
    data_dir_base: P,
    ticker: &str,
    statement_type: &str,
    mut files: Vec<String>,
    mut statements: Vec<String>,
    mut contexts: Vec<String>,
    mut particulars: Vec<String>,
    mut notes: Vec<String>,
    mut curr_years: Vec<String>,
    mut prev_years: Vec<String>,
) -> PolarsResult<String> {
    // 🎯 FIXED: Removed unnecessary 'mut' keyword to comply with Cargo analysis rules
    let output_dir = data_dir_base.as_ref().join(ticker).join("parquets").join("annual_report");
    if let Err(e) = fs::create_dir_all(&output_dir) {
        return Err(PolarsError::ComputeError(
            format!("Failed to create directory structure {:?}: {}", output_dir, e).into()
        ));
    }

    let output_file_path = output_dir.join(format!("{}.parquet", statement_type));

    let s_files = Series::new("file_name".into(), std::mem::take(&mut files));
    let s_statement = Series::new("statement_type".into(), std::mem::take(&mut statements));
    let s_contexts = Series::new("context_id".into(), std::mem::take(&mut contexts));
    let s_particulars = Series::new("particulars".into(), std::mem::take(&mut particulars));
    let s_notes = Series::new("notes".into(), std::mem::take(&mut notes));
    let s_curr = Series::new("curr_year".into(), std::mem::take(&mut curr_years));
    let s_prev = Series::new("prev_year".into(), std::mem::take(&mut prev_years));

    let mut df = DataFrame::new(vec![s_files, s_statement, s_contexts, s_particulars, s_notes, s_curr, s_prev])?;

    let output_file = File::create(&output_file_path).map_err(|e| {
        PolarsError::ComputeError(format!("Failed to build physical output track at {:?}: {}", output_file_path, e).into())
    })?;
    
    ParquetWriter::new(output_file).finish(&mut df)?;

    Ok(output_file_path.to_string_lossy().into_owned())
}