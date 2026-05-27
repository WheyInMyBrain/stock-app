use polars::prelude::*;
use std::fs::{self, File};
use std::path::Path;

pub fn save_to_parquet(
    input_folder_path: &str,
    sub_hierarchy: &[&str], 
    files: &Vec<String>,
    tags: &Vec<String>,
    contexts: &Vec<String>,
    dates: &Vec<String>,
    values: &Vec<String>,
) -> PolarsResult<String> {
    
    let path_obj = Path::new(input_folder_path);
    
    // Non-destructive parsing logic fallback 
    let (ticker, file_base_name) = if sub_hierarchy.is_empty() {
        let legacy_folder_name = path_obj.file_name().unwrap().to_string_lossy().into_owned();
        let legacy_ticker = path_obj
            .parent()
            .unwrap()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        (legacy_ticker, legacy_folder_name)
    } else {
        // For OCR paths, main.rs passed custom target path format metadata tokens down
        let extracted_statement_filename = path_obj.file_name().unwrap().to_string_lossy().into_owned();
        
        let mut check_ancestor = path_obj;
        while let Some(parent) = check_ancestor.parent() {
            if parent.file_name().map(|f| f.to_string_lossy()) == Some("data".into()) {
                break;
            }
            check_ancestor = parent;
        }
        let dynamic_ticker = check_ancestor.file_name().unwrap().to_string_lossy().into_owned();
        (dynamic_ticker, extracted_statement_filename)
    };

    let mut output_dir = format!("../data/{}/parquets", ticker);
    for subfolder in sub_hierarchy {
        output_dir.push_str(&format!("/{}", subfolder));
    }

    let output_file_path = format!("{}/{}.parquet", output_dir, file_base_name);

    if let Err(e) = fs::create_dir_all(&output_dir) {
        return Err(PolarsError::ComputeError(
            format!("Failed to create directory structure {}: {}", output_dir, e).into()
        ));
    }

    let s_files = Series::new("source_file".into(), files);
    let s_tags = Series::new("tag_name".into(), tags);
    let s_contexts = Series::new("context_id".into(), contexts);
    let s_dates = Series::new("date_bounds".into(), dates);
    let s_values = Series::new("raw_value".into(), values);

    let mut df = DataFrame::new(vec![s_files, s_tags, s_contexts, s_dates, s_values])?;

    let output_file = File::create(Path::new(&output_file_path)).map_err(|e| {
        PolarsError::ComputeError(format!("Failed to build physical output track: {}", e).into())
    })?;
    
    ParquetWriter::new(output_file).finish(&mut df)?;

    Ok(output_file_path)
}

pub fn save_ocr_to_parquet(
    input_folder_path: &str,
    sub_hierarchy: &[&str],
    files: &Vec<String>,
    statements: &Vec<String>,
    contexts: &Vec<String>,
    particulars: &Vec<String>,
    notes: &Vec<String>,
    curr_years: &Vec<String>,
    prev_years: &Vec<String>,
) -> PolarsResult<String> {
    let path_obj = Path::new(input_folder_path);
    let extracted_statement_filename = path_obj.file_name().unwrap().to_string_lossy().into_owned();
    
    let mut check_ancestor = path_obj;
    while let Some(parent) = check_ancestor.parent() {
        if parent.file_name().map(|f| f.to_string_lossy()) == Some("data".into()) {
            break;
        }
        check_ancestor = parent;
    }
    let ticker = check_ancestor.file_name().unwrap().to_string_lossy().into_owned();

    let mut output_dir = format!("../data/{}/parquets", ticker);
    for subfolder in sub_hierarchy {
        output_dir.push_str(&format!("/{}", subfolder));
    }
    let output_file_path = format!("{}/{}.parquet", output_dir, extracted_statement_filename);

    if let Err(e) = fs::create_dir_all(&output_dir) {
        return Err(PolarsError::ComputeError(
            format!("Failed to create directory structure {}: {}", output_dir, e).into()
        ));
    }

    // Natively structure your explicit 7-column database fields
    let s_files = Series::new("file_name".into(), files);
    let s_statement = Series::new("statement_type".into(), statements);
    let s_contexts = Series::new("context_id".into(), contexts);
    let s_particulars = Series::new("particulars".into(), particulars);
    let s_notes = Series::new("notes".into(), notes);
    let s_curr = Series::new("curr_year".into(), curr_years);
    let s_prev = Series::new("prev_year".into(), prev_years);

    let mut df = DataFrame::new(vec![s_files, s_statement, s_contexts, s_particulars, s_notes, s_curr, s_prev])?;

    let output_file = File::create(Path::new(&output_file_path)).map_err(|e| {
        PolarsError::ComputeError(format!("Failed to build physical output track: {}", e).into())
    })?;
    
    ParquetWriter::new(output_file).finish(&mut df)?;

    Ok(output_file_path)
}