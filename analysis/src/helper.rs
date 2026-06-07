use std::fs::File;
use std::io::Write;
use std::time::Instant;

/// System-wide parallel output dumping utility
/// Handles empty validation guards, serialization blocks, and precise runtime logging traces.
pub fn dump_matrix_report_to_disk<T>(
    report_data: &[T],
    target_file_path: &str,
    success_log_tag: &str,
    timer: Instant,
) where
    T: serde::Serialize,
{
    if report_data.is_empty() {
        return;
    }

    if let Ok(json_str) = serde_json::to_string_pretty(report_data) {
        if let Ok(mut file) = File::create(target_file_path) {
            if file.write_all(json_str.as_bytes()).is_ok() {
                println!(
                    "\x1b[38;5;130m[ANALYSIS] 💾 [{success_log_tag}] Runtime: {:?}\x1b[0m",
                    timer.elapsed()
                );
            }
        }
    }
}