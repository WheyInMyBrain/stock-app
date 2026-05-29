use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use serde_json::{json, Value};
use crate::commands::pipeline::CatalogItem;
use crate::pipeline::WorkspaceModule;

pub struct PerformanceChartCard;

fn get_shared_data_directory() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); path.pop();
    path.push("data");
    path
}

impl WorkspaceModule for PerformanceChartCard {
    fn catalog_definition(&self) -> CatalogItem {
        CatalogItem {
            id: "performance_chart".to_string(),
            name: "Volatility Velocity Stream Matrix".to_string(),
            description: "Abstract continuous vector canvas line and volume chart tracking.".to_string(),
        }
    }

    fn compile(&self, ticker: &str, timeframe: &str) -> Result<Value, String> {
        // 1. Scan folders to look for dynamic sub-timeframe configurations instantly
        let mut target_dir = get_shared_data_directory();
        target_dir.push(ticker);
        target_dir.push("bse_historical-chart-data");

        let mut auto_discovered_timeframes = Vec::new();
        if target_dir.exists() {
            if let Ok(entries) = fs::read_dir(&target_dir) {
                for entry in entries.flatten() {
                    if let Ok(file_name) = entry.file_name().into_string() {
                        if file_name.ends_with(".json") {
                            auto_discovered_timeframes.push(file_name.replace(".json", ""));
                        }
                    }
                }
            }
        }
        auto_discovered_timeframes.sort();

        // 2. Read the selected timeframe file directly to build vector geometries
        let mut svg_children = Vec::new();
        let mut path = target_dir.clone();
        path.push(format!("{}.json", timeframe));

        if path.exists() {
            if let Ok(mut file) = File::open(&path) {
                let mut content = String::new();
                if file.read_to_string(&mut content).is_ok() {
                    if let Ok(parsed) = serde_json::from_str::<Value>(&content) {
                        if let Some(arr) = parsed["grapthData"].as_array() {
                            // Extract points into local vector structures safely
                            let mut points = Vec::new();
                            for item in arr {
                                if let (Some(p), Some(v)) = (item[1].as_f64(), item[2].as_f64()) {
                                    points.push((p, v));
                                }
                            }

                            if !points.is_empty() {
                                let width = 500.0;
                                let height = 180.0;
                                let padding = 10.0;
                                let price_height = height * 0.7;
                                let volume_top = height * 0.8;
                                let volume_height = height * 0.2;

                                let min_p = points.iter().map(|(p, _)| *p).fold(f64::INFINITY, f64::min);
                                let max_p = points.iter().map(|(p, _)| *p).fold(f64::NEG_INFINITY, f64::max);
                                let p_range = if max_p - min_p == 0.0 { 1.0 } else { max_p - min_p };

                                let max_v = points.iter().map(|(_, v)| *v).fold(f64::NEG_INFINITY, f64::max);
                                let v_denom = if max_v == 0.0 { 1.0 } else { max_v };

                                let total_pts = points.len();
                                let mut line_commands = String::new();

                                for (i, (price, volume)) in points.iter().enumerate() {
                                    let x = padding + (i as f64 / (total_pts - 1) as f64) * (width - padding * 2.0);
                                    let y = price_height - ((price - min_p) / p_range) * (price_height - padding * 2.0) - padding;

                                    if i == 0 {
                                        line_commands.push_str(&format!("M {} {}", x, y));
                                    } else {
                                        line_commands.push_str(&format!(" L {} {}", x, y));
                                    }

                                    let b_width = (width - padding * 2.0) / total_pts as f64 * 0.7;
                                    let b_height = (volume / v_denom) * volume_height;
                                    
                                    svg_children.push(json!({
                                        "type": "vector_rect",
                                        "x": x - b_width / 2.0,
                                        "y": volume_top + (volume_height - b_height),
                                        "width": b_width.max(1.5),
                                        "height": b_height,
                                        "fill": "currentColor",
                                        "className": "opacity-[0.12]"
                                    }));
                                }

                                let is_up = points.last().map(|(p, _)| *p).unwrap_or(0.0) >= points.first().map(|(p, _)| *p).unwrap_or(0.0);
                                let color = if is_up { "rgb(34, 197, 94)" } else { "rgb(239, 68, 68)" };
                                let fill = if is_up { "rgba(34, 197, 94, 0.04)" } else { "rgba(239, 68, 68, 0.04)" };

                                let first_x = padding;
                                let last_x = width - padding;
                                let area_commands = format!("{} L {} {} L {} {} Z", line_commands, last_x, price_height, first_x, price_height);

                                svg_children.insert(0, json!({ "type": "vector_path", "d": area_commands, "fill": fill }));
                                svg_children.insert(1, json!({ "type": "vector_path", "d": line_commands, "stroke": color, "stroke_width": 2 }));
                            }
                        }
                    }
                }
            }
        }

        // 3. Output standard layout structures wrapping primitives cleanly
        Ok(json!({
            "type": "card",
            "title": "ANALYTICAL GRAPH MATRIX",
            "subtitle": "// DATA GRAPH STREAM",
            "footer": "Engine core: Rust Framework Kernel Engine",
            "children": [
                {
                    "type": "container",
                    "className": "flex-row justify-between items-center w-full mb-2",
                    "children": [
                        { "type": "text", "value": "Interval Tracking Layer:" },
                        {
                            "type": "select",
                            "options": auto_discovered_timeframes,
                            "default_value": timeframe,
                            "action_target": "performance_chart"
                        }
                    ]
                },
                {
                    "type": "vector_canvas",
                    "children": svg_children
                }
            ]
        }))
    }
}