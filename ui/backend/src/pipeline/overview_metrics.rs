use serde_json::{json, Value};

/// Generates a completely isolated, standalone metrics summary primitive card block
pub fn compile_metrics_card(ticker: &str) -> Value {
    json!({
        "type": "card",
        "title": format!("{} STABLE", ticker.to_uppercase()),
        "subtitle": "// CAPTURE GRID",
        "footer": "Status: Parquet Pool Active",
        "children": [
            {
                "type": "container",
                "className": "flex flex-row gap-8 justify-start py-4 px-2",
                "children": [
                    { "type": "metric", "value": "84.2B", "title": "Market Yield" },
                    { "type": "metric", "value": "1.4M", "title": "Pool Stream" }
                ]
            }
        ]
    })
}

/// Generates a completely isolated, standalone graphical telemetry primitive card block
pub fn compile_chart_card(_ticker: &str) -> Value {
    json!({
        "type": "card",
        "title": "ANALYTICAL GRAPH MATRIX",
        "subtitle": "// DATA GRAPH STREAM",
        "footer": "Engine core: Rust Framework Kernel Engine",
        "children": [
            {
                "type": "text",
                "className": "mb-2",
                "value": "Pipeline telemetry scales seamlessly using raw data parameters calculated by the backend server."
            },
            {
                "type": "bar_graph",
                "children": [
                    { "type": "container", "percentage_height": 40, "color_token": "muted" },
                    { "type": "container", "percentage_height": 75, "color_token": "accent" },
                    { "type": "container", "percentage_height": 95, "color_token": "primary" },
                    { "type": "container", "percentage_height": 55, "color_token": "muted" },
                    { "type": "container", "percentage_height": 85, "color_token": "accent" }
                ]
            }
        ]
    })
}