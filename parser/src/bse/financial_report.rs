use quick_xml::events::Event;
use quick_xml::Reader;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use rayon::prelude::*;
use crate::bse::utils::BseRecord;

// A simple tracking container to hold data gathered sequentially before multi-threaded cleaning
struct RawElement {
    tag_name: String,
    context_id: String,
    raw_text: String,
}

pub fn parse(path: &Path) -> Result<Vec<BseRecord>, String> {
    let file_name = path.file_name().unwrap().to_string_lossy().into_owned();

    let metadata = std::fs::metadata(path).map_err(|e| e.to_string())?;
    if metadata.len() == 0 {
        return Err("Empty file".to_string());
    }

    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut raw_content = String::new();
    file.read_to_string(&mut raw_content).map_err(|e| e.to_string())?;

    let clean_xml = if let Some(start_idx) = raw_content.find('<') {
        &raw_content[start_idx..]
    } else {
        &raw_content
    };

    let mut reader = Reader::from_str(clean_xml);
    reader.config_mut().trim_text(true);
    reader.config_mut().check_end_names = false;

    let mut buf = Vec::new();
    
    let mut intermediate_elements = Vec::new();
    let mut open_tag = String::new();
    let mut open_context = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let local_name = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();
                let mut found_context = String::new();
                
                for attr in e.attributes().flatten() {
                    if attr.key.local_name().as_ref() == b"contextRef" {
                        found_context = String::from_utf8_lossy(&attr.value).into_owned();
                        break;
                    }
                }

                if !found_context.is_empty() {
                    open_tag = local_name;
                    open_context = found_context;
                } else {
                    open_tag.clear();
                    open_context.clear();
                }
            }
            Ok(Event::Text(e)) => {
                if !open_tag.is_empty() && !open_context.is_empty() {
                    let text_value = e.unescape().unwrap_or_default().into_owned();
                    if !text_value.is_empty() {
                        intermediate_elements.push(RawElement {
                            tag_name: open_tag.clone(),
                            context_id: open_context.clone(),
                            raw_text: text_value,
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

    let records: Vec<BseRecord> = intermediate_elements
        .into_par_iter()
        .map(|elem| {
            let clean_value = elem.raw_text
                .replace('\r', "")
                .replace('\n', " ")
                .split_whitespace()
                .collect::<Vec<&str>>()
                .join(" ");

            BseRecord {
                source_file: file_name.clone(),
                tag_name: elem.tag_name,
                context_id: elem.context_id,
                date_bounds: String::new(),
                raw_value: clean_value,
            }
        })
        .collect();

    Ok(records)
}