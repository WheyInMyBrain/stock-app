use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use serde_json::{json, Value};
use crate::commands::pipeline::CatalogItem;
use crate::pipeline::WorkspaceModule;

pub struct OverviewMetricsCard;

fn get_shared_data_directory() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); path.pop();
    path.push("data");
    path
}

impl WorkspaceModule for OverviewMetricsCard {
    fn catalog_definition(&self) -> CatalogItem {
        CatalogItem {
            id: "overview_metrics".to_string(),
            name: "Metrics Grid Matrix".to_string(),
            description: "Live summary monitoring stream covering market yield totals and pool configurations.".to_string(),
        }
    }

    fn compile(&self, ticker: &str, timeframe: &str) -> Result<Value, String> {
        let mut close_price = "84.20B".to_string();
        let mut pool_stream = "1.4M".to_string();

        // Safely parse the historical JSON file inline to extract live metrics
        let mut path = get_shared_data_directory();
        path.push(ticker);
        path.push("bse_historical-chart-data");
        path.push(format!("{}.json", timeframe));

        if path.exists() {
            if let Ok(mut file) = File::open(&path) {
                let mut content = String::new();
                if file.read_to_string(&mut content).is_ok() {
                    if let Ok(parsed) = serde_json::from_str::<Value>(&content) {
                        if let Some(arr) = parsed["grapthData"].as_array() {
                            if let Some(latest) = arr.last() {
                                if let (Some(p), Some(v)) = (latest[1].as_f64(), latest[2].as_f64()) {
                                    close_price = format!("{:.2}", p);
                                    pool_stream = format!("{:.1}M", v / 1_000_000.0);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(json!({
            "type": "card",
            "title": format!("{} STABLE", ticker.to_uppercase()),
            "subtitle": "// CAPTURE GRID",
            "footer": "Status: Parquet Pool Active",
            "children": [
                {
                    "type": "container",
                    "className": "flex flex-row gap-8 justify-start py-4 px-2",
                    "children": [
                        { "type": "metric", "value": close_price, "title": "Market Yield" },
                        { "type": "metric", "value": pool_stream, "title": "Pool Stream" }
                    ]
                }
            ]
        }))
    }
}