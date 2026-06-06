use serde_json::{json, Value};
use crate::commands::pipeline::CatalogItem;
use crate::pipeline::{WorkspaceModule, WorkspaceDataContext};

pub struct StockChartCard;

// 📊 Pure, dependency-free calendar calculation helper mapping epoch milliseconds to readable labels
fn format_timestamp(ms: i64, is_intraday: bool) -> String {
    let seconds = ms / 1000;
    
    if is_intraday {
        let secs_in_day = seconds.rem_euclid(86400);
        let hours = secs_in_day / 3600;
        let minutes = (secs_in_day / 60) % 60;
        return format!("{:02}:{:02}", hours, minutes);
    }

    let days = seconds / 86400;
    let mut year = 1970;
    let mut days_left = days;

    if days_left >= 0 {
        loop {
            let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
            let days_in_year = if is_leap { 366 } else { 365 };
            if days_left < days_in_year {
                break;
            }
            days_left -= days_in_year;
            year += 1;
        }
        
        let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
        let month_days = if is_leap {
            [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        } else {
            [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        };
        
        let mut month = 1;
        for &days_in_month in &month_days {
            if days_left < days_in_month {
                break;
            }
            days_left -= days_in_month;
            month += 1;
        }
        let day = days_left + 1;
        format!("{:04}-{:02}-{:02}", year, month, day)
    } else {
        "Historic".to_string()
    }
}

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
        let mut price_high = 0.0;
        let mut price_low = 0.0;
        let mut delta_string = "No session change data".to_string();
        let mut chart_stream_data = Vec::new();

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

                    // 🎯 STEP 4: TRANSLATE TIME SERIES TO THE COMPACT INTERACTIVE PLOT MATRIX
                    let is_intraday = active_tf == "1D";
                    chart_stream_data.reserve(points.len());
                    
                    for p in points {
                        let formatted_label = format_timestamp(p.0, is_intraday);
                        chart_stream_data.push(json!({
                            "time_horizon": formatted_label,
                            "market_value": p.1
                        }));
                    }
                }
            }
        }

        // ==============================================================================
        // 📊 STEP 5: OUTPUT CONSOLIDATED CARDS SPECIFICATION UNIFIED MESH
        // ==============================================================================
        Ok(json!({
            "type": "card",
            "title": current_price_str,
            "subtitle": format!("// PERFORMANCE CANVAS: {} [{}]", ticker_upper, active_tf),
            "footer": format!("Interval Low Boundary: ₹{:.2} | High Boundary: ₹{:.2}", price_low, price_high),
            "children": [
                {
                    "type": "container",
                    "className": "flex flex-row justify-between items-center w-full mt-1 mb-2 pointer-events-auto",
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
                    "type": "interactive_chart",
                    "xAxisKey": "time_horizon",
                    "series": [
                        { "key": "market_value", "label": "Spot Value", "stroke": "#38bdf8", "strokeWidth": 1.75 }
                    ],
                    "data": chart_stream_data
                }
            ]
        }))
    }
}