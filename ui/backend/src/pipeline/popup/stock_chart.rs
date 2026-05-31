use crate::pipeline::popup::WorkspacePopup;
use serde_json::{json, Value};
use crate::commands::data_loader::WorkspaceDataContext;
use std::collections::BTreeMap;

pub struct StockChartPopup;

impl WorkspacePopup for StockChartPopup {
    fn window_title(&self, ticker: &str) -> String {
        format!("CORE TERMINAL ANALYTICS LAYER // SYSTEM: {}", ticker.to_uppercase())
    }

    fn initial_size(&self) -> (f64, f64) {
        (1100.0, 700.0) 
    }

    fn compile(&self, _ticker: &str, data: &WorkspaceDataContext) -> Result<Value, String> {
        let nse_base_json = data.get_dataset("nse_real-time-chart/endpoint-metadata.json");
        let nse_delta_json = data.get_dataset("nse_real-time-chart-delta/endpoint-metadata.json");

        let extract_candles = |json_data: &Value| -> Vec<Value> {
            match json_data {
                Value::Object(map) => {
                    if let Some(Value::Array(arr)) = map.get("data") {
                        arr.clone()
                    } else {
                        vec![]
                    }
                },
                Value::Array(arr) => arr.clone(),
                _ => vec![]
            }
        };

        let base_candles = extract_candles(&nse_base_json);
        let delta_candles = extract_candles(&nse_delta_json);

        let mut base_timestamps: Vec<i64> = base_candles
            .iter()
            .filter_map(|c| c.get("time").and_then(|t| t.as_i64()))
            .collect();
        
        base_timestamps.sort();
        let from_timestamp_value = base_timestamps.last().cloned().unwrap_or(0);

        if let Ok(mut lock) = crate::commands::ticker::POPUP_FROM_TIMESTAMP.write() {
            *lock = from_timestamp_value;
        }

        let mut chronological_candle_map: BTreeMap<i64, Value> = BTreeMap::new();

        for candle in base_candles {
            if let Some(time_val) = candle.get("time").and_then(|t| t.as_i64()) {
                chronological_candle_map.insert(time_val, candle);
            }
        }

        for candle in delta_candles {
            if let Some(time_val) = candle.get("time").and_then(|t| t.as_i64()) {
                chronological_candle_map.insert(time_val, candle);
            }
        }

        let merged_candles: Vec<Value> = chronological_candle_map.into_values().collect();
        let mut cleaned_candles = Vec::new();

        for mut item in merged_candles {
            if let Some(obj) = item.as_object_mut() {
                if let Some(time_val) = obj.get("time") {
                    if let Some(padded_ts) = time_val.as_i64() {
                        let mut seconds_ts = if padded_ts > 9_999_999_999 {
                            padded_ts / 1000
                        } else {
                            padded_ts
                        };

                        seconds_ts -= 5 * 3600 + 30 * 60;
                        obj.insert("time".to_string(), json!(seconds_ts * 1000));
                    }
                }
                cleaned_candles.push(Value::Object(obj.clone()));
            }
        }

        Ok(json!({
            "id": "stock_chart_popup",
            "type": "popup_workspace",
            "className": "flex flex-col w-full h-full gap-2",
            "from_timestamp": from_timestamp_value,
            "children": [
                {
                    "type": "container",
                    "className": "flex flex-col flex-1 h-full w-full",
                    "children": [
                        {
                            "type": "chart_viewer",
                            "seriesData": cleaned_candles
                        }
                    ]
                }
            ]
        }))
    }
}