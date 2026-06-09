use quick_xml::events::Event;
use quick_xml::Reader;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use crate::bse::utils::BseRecord;

pub fn parse(path: &Path) -> Result<Vec<BseRecord>, String> {
    let file_name = path.file_name().unwrap().to_string_lossy().into_owned();

    let metadata = std::fs::metadata(path).map_err(|e| e.to_string())?;
    if metadata.len() == 0 {
        return Err("Empty file".to_string());
    }

    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut raw_content = String::new();
    file.read_to_string(&mut raw_content).map_err(|e| e.to_string())?;

    let clean_html = if let Some(start_idx) = raw_content.find('<') {
        &raw_content[start_idx..]
    } else {
        &raw_content
    };

    let mut reader = Reader::from_str(clean_html);
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
                // Strip any namespace prefix (e.g., "ix:nonNumeric" -> "nonNumeric")
                let tag_local = if let Some(idx) = full_tag_name.find(':') {
                    &full_tag_name[(idx + 1)..]
                } else {
                    &full_tag_name
                };

                if tag_local == "nonNumeric" || tag_local == "nonFraction" {
                    let mut found_name = String::new();
                    let mut found_context = String::new();

                    for attr in e.attributes().flatten() {
                        let attr_key_raw = String::from_utf8_lossy(attr.key.as_ref()).into_owned();
                        // Strip namespace from attribute key if present (e.g., "ix:name" or "name")
                        let attr_key = if let Some(idx) = attr_key_raw.find(':') {
                            &attr_key_raw[(idx + 1)..]
                        } else {
                            &attr_key_raw
                        };

                        if attr_key == "name" {
                            let full_name = String::from_utf8_lossy(&attr.value).into_owned();
                            found_name = if let Some(split_idx) = full_name.find(':') {
                                full_name[(split_idx + 1)..].to_string()
                            } else {
                                full_name
                            };
                        } else if attr_key == "contextRef" {
                            found_context = String::from_utf8_lossy(&attr.value).into_owned();
                        }
                    }

                    if !found_name.is_empty() && !found_context.is_empty() {
                        open_tag = found_name;
                        open_context = found_context;
                    }
                }
            }
            Ok(Event::Text(e)) => {
                if !open_tag.is_empty() && !open_context.is_empty() {
                    let text_value = e.unescape().unwrap_or_default().into_owned();
                    if !text_value.is_empty() {
                        records.push(BseRecord {
                            source_file: file_name.clone(),
                            tag_name: open_tag.clone(),
                            context_id: open_context.clone(),
                            date_bounds: String::new(),
                            raw_value: text_value,
                        });
                    }
                }
            }
            Ok(Event::End(e)) => {
                let full_tag_name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                let tag_local = if let Some(idx) = full_tag_name.find(':') {
                    &full_tag_name[(idx + 1)..]
                } else {
                    &full_tag_name
                };

                if tag_local == "nonNumeric" || tag_local == "nonFraction" {
                    open_tag.clear();
                    open_context.clear();
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(records)
}