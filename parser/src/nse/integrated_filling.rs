use quick_xml::events::Event;
use quick_xml::Reader;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use crate::nse::utils::NseRecord;

pub fn parse(path: &Path) -> Result<Vec<NseRecord>, String> {
    let file_name = path.file_name().unwrap().to_string_lossy().into_owned();

    let metadata = std::fs::metadata(path).map_err(|e| e.to_string())?;
    if metadata.len() == 0 {
        return Err("Empty file".to_string());
    }

    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut raw_content = String::new();
    file.read_to_string(&mut raw_content).map_err(|e| e.to_string())?;

    // Align with XML start header boundary cleanly
    let clean_xml = if let Some(start_idx) = raw_content.find('<') {
        &raw_content[start_idx..]
    } else {
        &raw_content
    };

    let mut reader = Reader::from_str(clean_xml);
    reader.config_mut().trim_text(true);
    reader.config_mut().check_end_names = false;

    let mut buf = Vec::new();
    let mut records = Vec::new();
    
    let mut open_tag = String::new();
    let mut open_context = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let full_tag_name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                
                // Strip structural namespace prefix (e.g. "in-capmkt:NoOfInvestorComplaints" -> "NoOfInvestorComplaints")
                let tag_local = if let Some(idx) = full_tag_name.find(':') {
                    full_tag_name[(idx + 1)..].to_string()
                } else {
                    full_tag_name
                };

                let mut found_context = String::new();

                // Extract contextRef attribute mapping
                for attr in e.attributes().flatten() {
                    let attr_key_raw = String::from_utf8_lossy(attr.key.as_ref()).into_owned();
                    let attr_key = if let Some(idx) = attr_key_raw.find(':') {
                        &attr_key_raw[(idx + 1)..]
                    } else {
                        &attr_key_raw
                    };

                    if attr_key == "contextRef" {
                        found_context = String::from_utf8_lossy(&attr.value).into_owned();
                        break;
                    }
                }

                if !found_context.is_empty() {
                    open_tag = tag_local;
                    open_context = found_context;
                } else {
                    open_tag.clear();
                    open_context.clear();
                }
            }
            Ok(Event::Text(e)) => {
                if !open_tag.is_empty() && !open_context.is_empty() {
                    let text_value = e.unescape().unwrap_or_default().into_owned();
                    
                    // Collapse spaces and newlines down into flat continuous row entries
                    let clean_value = text_value
                        .replace('\r', "")
                        .replace('\n', " ")
                        .split_whitespace()
                        .collect::<Vec<&str>>()
                        .join(" ");

                    if !clean_value.is_empty() {
                        records.push(NseRecord {
                            source_file: file_name.clone(),
                            tag_name: open_tag.clone(),
                            context_id: open_context.clone(),
                            date_bounds: String::new(),
                            raw_value: clean_value,
                        });
                    }
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