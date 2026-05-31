// stock-app/ui/backend/src/pipeline/stock_chart.rs

use serde_json::{json, Value};
use crate::commands::pipeline::CatalogItem;
use crate::pipeline::{WorkspaceModule, WorkspaceDataContext};

pub struct StockChartCard;

impl WorkspaceModule for StockChartCard {
    fn catalog_definition(&self) -> CatalogItem {
        CatalogItem {
            id: "stock_chart".to_string(), 
            name: "Historical Performance Chart".to_string(),
            description: "Minimalist vector tracking canvas displaying chronological market execution boundaries and custom interval loops.".to_string(),
        }
    }

    fn compile(&self, ticker: &str, timeframe: &str, data: &WorkspaceDataContext) -> Result<Value, String> {
        let ticker_upper = ticker.to_uppercase();
        
        // 1. Fallback / Default configuration timeline selection
        let mut active_tf = timeframe.trim().to_string();
        if active_tf.is_empty() {
            active_tf = "1D".to_string();
        }

        // 2. Discover available files inside the folder dynamically to populate dropdown options
        let target_dir_selector = "nse_historical-chart-data";
        let folder_data = data.get_dataset(target_dir_selector);
        
        let mut available_timeframes = vec!["1D".to_string(), "1W".to_string(), "1M".to_string(), "1Y".to_string()];
        
        if let Some(obj) = folder_data.as_object() {
            let mut found_keys: Vec<String> = obj.keys()
                .map(|k| k.replace(".json", ""))
                .collect();
            if !found_keys.is_empty() {
                found_keys.sort_by_key(|a| a.len());
                available_timeframes = found_keys;
            }
        }

        // 3. Load the specific targeted file dataset
        let file_selector = format!("{}/{}", target_dir_selector, active_tf);
        let chart_json = data.get_dataset(&file_selector);

        // Fallback definitions
        let mut current_price_str = "₹N/A".to_string();
        let mut path_string = "".to_string();
        let mut price_high = 0.0;
        let mut price_low = 0.0;
        let mut delta_string = "No session change data".to_string();

        // Parse coordinates from nested "grapthData" block matrix frame
        if let Some(graph_data) = chart_json["grapthData"].as_array() {
            if !graph_data.is_empty() {
                let mut points: Vec<(i64, f64)> = Vec::new();
                for item in graph_data {
                    if let Some(arr) = item.as_array() {
                        if arr.len() >= 2 {
                            let ts = arr[0].as_i64().unwrap_or(0);
                            let price = arr[1].as_f64().unwrap_or(0.0);
                            if price > 0.0 {
                                points.push((ts, price));
                            }
                        }
                    }
                }

                // Sort chronologically by timestamp (ascending) so line draws left-to-right cleanly
                points.sort_by_key(|p| p.0);

                if !points.is_empty() {
                    let first_price = points.first().unwrap().1;
                    let last_price = points.last().unwrap().1;
                    current_price_str = format!("₹{:.2}", last_price);

                    price_high = points.iter().map(|p| p.1).fold(f64::MIN, f64::max);
                    price_low = points.iter().map(|p| p.1).fold(f64::MAX, f64::min);

                    let net_change = last_price - first_price;
                    let pct_change = (net_change / first_price) * 100.0;

                    if net_change > 0.0 {
                        delta_string = format!("+₹{:.2} (+{:.2}%)", net_change, pct_change.abs());
                    } else if net_change < 0.0 {
                        delta_string = format!("-₹{:.2} (-{:.2}%)", net_change.abs(), pct_change.abs());
                    } else {
                        delta_string = "₹0.00 (0.00%)".to_string();
                    }

                    // EDGE-TO-EDGE CANVAS TRANSLATION (Width: 500, Height: 180)
                    let canvas_w = 500.0;
                    let canvas_h = 180.0;
                    let padding_y = 10.0; 
                    let usable_h = canvas_h - (padding_y * 2.0);

                    let total_points = points.len();
                    let price_range = if price_high == price_low { 1.0 } else { price_high - price_low };

                    let mut segments = Vec::new();
                    for (i, p) in points.iter().enumerate() {
                        let x = if total_points > 1 {
                            (i as f64 / (total_points - 1) as f64) * canvas_w
                        } else {
                            canvas_w / 2.0
                        };

                        let y = padding_y + (usable_h - (((p.1 - price_low) / price_range) * usable_h));
                        
                        if i == 0 {
                            segments.push(format!("M {:.1} {:.1}", x, y));
                        } else {
                            segments.push(format!("L {:.1} {:.1}", x, y));
                        }
                    }
                    path_string = segments.join(" ");
                }
            }
        }

        Ok(json!({
            "type": "card",
            "title": current_price_str,
            "subtitle": format!("// PERFORMANCE CANVAS: {} [{}]", ticker_upper, active_tf),
            "footer": format!("Interval Low Boundary: ₹{:.2} | High Boundary: ₹{:.2}", price_low, price_high),
            "children": [
                {
                    "type": "container",
                    "className": "flex flex-row justify-between items-center w-full mt-1 mb-2 pointer-events-auto",
                    // 🚀 RESTORED STYLE DICTIONARY: This locks the items on opposite sides and stops select stretching!
                    "style": { "display": "flex", "flexDirection": "row", "justifyContent": "between" },
                    "children": [
                        {
                            "type": "text",
                            "className": "text-sm font-semibold font-mono opacity-80", 
                            "value": delta_string
                        },
                        {
                            "type": "select",
                            "action_target": "stock_chart", 
                            "default_value": active_tf,
                            "options": available_timeframes
                        }
                    ]
                },
                {
                    "type": "vector_canvas",
                    "className": "w-full flex-1",
                    "style": { "padding": "0px", "margin": "0px" },
                    "children": [
                        {
                            "type": "vector_path",
                            "d": path_string,
                            "stroke": "currentColor", 
                            "stroke_width": 1.5,
                            "className": "opacity-90 transition-all duration-300"
                        }
                    ]
                }
            ]
        }))
    }
}