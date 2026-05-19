use polars::prelude::*;
use std::fs::{self, File};
use std::path::Path;

/// Automatically derives the ticker and folder name from the input path to structure the Parquet file.
pub fn save_to_parquet(
    input_folder_path: &str, // E.g., "../data/IMFA/bse_financial-results-docs"
    files: &Vec<String>,
    tags: &Vec<String>,
    contexts: &Vec<String>,
    dates: &Vec<String>,
    values: &Vec<String>,
) -> PolarsResult<String> {
    
    // Parse the path to isolate the ticker and the folder name
    let path_obj = Path::new(input_folder_path);
    let folder_name = path_obj.file_name().unwrap().to_string_lossy().into_owned(); // "bse_financial-results-docs"
    
    let ticker = path_obj
        .parent()
        .unwrap()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned(); // "IMFA"

    // Construct the output directory and file path
    let output_dir = format!("../data/{}/parquets", ticker);
    let output_file_path = format!("{}/{}.parquet", output_dir, folder_name);

    // Create the output directory if it doesn't exist
    if let Err(e) = fs::create_dir_all(&output_dir) {
        return Err(PolarsError::ComputeError(
            format!("Failed to create directory structure {}: {}", output_dir, e).into()
        ));
    }

    // Build Polars Series structures directly from the vectors
    let s_files = Series::new("source_file".into(), files);
    let s_tags = Series::new("tag_name".into(), tags);
    let s_contexts = Series::new("context_id".into(), contexts);
    let s_dates = Series::new("date_bounds".into(), dates);
    let s_values = Series::new("raw_value".into(), values);

    // Assemble the DataFrame
    let mut df = DataFrame::new(vec![s_files, s_tags, s_contexts, s_dates, s_values])?;

    // Write compressed data chunks to disk
    let output_file = File::create(Path::new(&output_file_path)).map_err(|e| {
        PolarsError::ComputeError(format!("Failed to build physical output track: {}", e).into())
    })?;
    
    ParquetWriter::new(output_file).finish(&mut df)?;

    Ok(output_file_path)
}