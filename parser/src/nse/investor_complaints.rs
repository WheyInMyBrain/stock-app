use crate::nse::utils::NseRecord;
use quick_xml::events::Event;
use quick_xml::Reader;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

pub fn parse(path: &Path) -> Result<Vec<NseRecord>, String> {
    let file_name = path.file_name().unwrap().to_string_lossy().into_owned();

    let metadata = std::fs::metadata(path).map_err(|e| e.to_string())?;
    if metadata.len() == 0 {
        return Err("Empty file".to_string());
    }

    let file = File::open(path).map_err(|e| e.to_string())?;
    let mut buf_reader = BufReader::new(file);
    let mut reader = Reader::from_reader(&mut buf_reader);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut raw_records = Vec::new();
    
    // Trackers to catch the flat-file period markers dynamically
    let mut start_date = String::new();
    let mut end_date = String::new();

    let mut open_tag = String::new();
    let mut open_context = String::new();

    // ============================================================================
    // SINGLE PASS SPEED: Gather values and intercept standalone reporting dates
    // ============================================================================
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let local_name = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();
                for attr in e.attributes().flatten() {
                    if attr.key.local_name().as_ref() == b"contextRef" {
                        open_tag = local_name.clone();
                        open_context = String::from_utf8_lossy(&attr.value).into_owned();
                        break;
                    }
                }
            }
            Ok(Event::Text(e)) => {
                if !open_tag.is_empty() && !open_context.is_empty() {
                    let text_value = e.unescape().unwrap().into_owned();

                    // Intercept the inline SEBI date tags as they fly through the stream
                    if open_tag == "DateOfStartOfReportingPeriod" {
                        start_date = text_value.clone();
                    } else if open_tag == "DateOfEndOfReportingPeriod" {
                        end_date = text_value.clone();
                    }

                    // Store a temporary copy of the entry
                    raw_records.push((open_tag.clone(), open_context.clone(), text_value));
                }
            }
            Ok(Event::End(_)) => {
                open_tag.clear();
                open_context.clear();
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    // ============================================================================
    // RECONCILIATION: Build the output rows using the intercepted flat dates
    // ============================================================================
    let date_bounds = if !start_date.is_empty() && !end_date.is_empty() {
        format!("{} to {}", start_date, end_date)
    } else if !end_date.is_empty() {
        end_date // Fallback to instant if only End Date is found
    } else {
        "As-of Reporting".to_string()
    };

    let mut records = Vec::with_capacity(raw_records.len());
    for (tag, context, value) in raw_records {
        records.push(NseRecord {
            source_file: file_name.clone(),
            tag_name: tag,
            context_id: context,
            date_bounds: date_bounds.clone(), // Blends the global flat period into all lines
            raw_value: value,
        });
    }

    Ok(records)
}