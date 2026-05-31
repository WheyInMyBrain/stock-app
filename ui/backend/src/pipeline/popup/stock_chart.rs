// stock-app/ui/backend/src/pipeline/popup/stock_chart.rs

use crate::pipeline::popup::WorkspacePopup;
use serde_json::{json, Value};

pub struct StockChartPopup;

impl WorkspacePopup for StockChartPopup {
    fn window_title(&self, ticker: &str) -> String {
        format!("CORE ANALYSIS DESK // {}", ticker.to_uppercase())
    }

    fn initial_size(&self) -> (f64, f64) {
        (950.0, 600.0)
    }

    fn compile(&self, ticker: &str) -> Result<Value, String> {
        let symbol = ticker.to_uppercase();

        Ok(json!({
            "type": "popup_workspace",
            "children": [
                // 1. Sleek Header Card Frame
                {
                    "type": "card",
                    "title": format!("SYSTEM PERFORMANCE PROTOCOL: {}", symbol),
                    "subtitle": "// ANALYTICS SUBSYSTEM MATRIX",
                    "className": "border-neutral-800/60",
                    "children": [
                        {
                            "type": "text",
                            "className": "text-xs text-neutral-400 font-mono mt-2",
                            "value": format!("Isolated execution terminal running live data nodes for asset stream identifier {}.", symbol)
                        }
                    ]
                },
                // 2. High-Density Multi-Column Metrics Grid
                {
                    "type": "container",
                    "className": "grid grid-cols-3 mt-4",
                    "children": [
                        {
                            "type": "card",
                            "children": [
                                { "type": "metric", "title": "REALTIME VELOCITY S1", "value": "249.42 M/s" }
                            ]
                        },
                        {
                            "type": "card",
                            "children": [
                                { "type": "metric", "title": "INTEGRATION VOLTAGE", "value": "0.942 ms" }
                            ]
                        },
                        {
                            "type": "card",
                            "children": [
                                { "type": "metric", "title": "BUFFER FREQUENCY", "value": "120.04 Hz" }
                            ]
                        }
                    ]
                }
            ]
        }))
    }
}