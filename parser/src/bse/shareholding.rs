use hashbrown::HashMap;
use quick_xml::events::Event;
use quick_xml::Reader;
use std::fs::File;
use std::io::{BufReader, Seek, SeekFrom};
use std::path::Path;
use crate::bse::utils::BseRecord;

#[derive(Debug, Clone, Default)]
struct ContextDates {
    start_date: String,
    end_date: String,
    instant: String,
}

pub fn parse(path: &Path) -> Result<Vec<BseRecord>, String> {
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
    let mut date_map: HashMap<String, ContextDates> = HashMap::new();
    let mut records = Vec::new();

    // ============================================================================
    // PASS 1: MAP ALL DATES FOR ALL CONTEXT TYPES DYNAMICALLY
    // ============================================================================
    let mut active_context_id = String::new();
    let mut inside_period = false;
    let mut temp_dates = ContextDates::default();
    let mut current_element = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let local_name = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();
                current_element = local_name.clone();

                if local_name == "context" || local_name.ends_with(":context") {
                    for attr in e.attributes().flatten() {
                        if attr.key.local_name().as_ref() == b"id" {
                            active_context_id = String::from_utf8_lossy(&attr.value).into_owned();
                            temp_dates = ContextDates::default();
                            break;
                        }
                    }
                } else if local_name == "period" || local_name.ends_with(":period") {
                    inside_period = true;
                }
            }
            Ok(Event::Text(e)) => {
                if !active_context_id.is_empty() {
                    let text = e.unescape().unwrap().into_owned();
                    if inside_period {
                        if current_element == "startDate" || current_element.ends_with(":startDate") {
                            temp_dates.start_date = text;
                        } else if current_element == "endDate" || current_element.ends_with(":endDate") {
                            temp_dates.end_date = text;
                        } else if current_element == "instant" || current_element.ends_with(":instant") {
                            temp_dates.instant = text;
                        }
                    }
                }
            }
            Ok(Event::End(e)) => {
                let local_name = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();
                if local_name == "context" || local_name.ends_with(":context") {
                    if !active_context_id.is_empty() {
                        date_map.insert(active_context_id.clone(), temp_dates.clone());
                    }
                    active_context_id.clear();
                } else if local_name == "period" || local_name.ends_with(":period") {
                    inside_period = false;
                }
                current_element.clear();
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    // ============================================================================
    // PASS 2: EXTRACT EVERY DATA FACT RECONCILED WITH ITS RELEVANT CONTEXT
    // ============================================================================
    let inner_stream = reader.into_inner();
    inner_stream.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
    
    let mut data_reader = Reader::from_reader(inner_stream);
    data_reader.config_mut().trim_text(true);
    
    let mut open_tag = String::new();
    let mut open_context = String::new();

    loop {
        match data_reader.read_event_into(&mut buf) {
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
                    
                    // Reconcile dates depending on whether it's an Instant or a Duration
                    let date_string = match date_map.get(&open_context) {
                        Some(d) => {
                            if !d.instant.is_empty() {
                                d.instant.clone()
                            } else if !d.start_date.is_empty() && !d.end_date.is_empty() {
                                format!("{} to {}", d.start_date, d.end_date)
                            } else {
                                "As-of Reporting Date".to_string()
                            }
                        }
                        None => "No explicit period".to_string(),
                    };

                    records.push(BseRecord {
                        source_file: file_name.clone(),
                        tag_name: open_tag.clone(),
                        context_id: open_context.clone(),
                        date_bounds: date_string,
                        raw_value: text_value,
                    });
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

    Ok(records)
}