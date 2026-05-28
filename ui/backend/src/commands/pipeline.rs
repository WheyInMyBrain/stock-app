use serde::{Serialize, Deserialize};
use crate::pipeline;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UiModulePayload {
    pub id: String,
    pub title: String,
    pub root_node: serde_json::Value,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CatalogItem {
    pub id: String,
    pub name: String,
    pub description: String,
}

// 🎯 ENSURE THIS ATTRIBUTE WRAPPER SITS DIRECTLY ABOVE THIS FUNCTION:
#[tauri::command]
pub fn fetch_component_telemetry(ticker: String, module_id: String) -> Result<UiModulePayload, String> {
    match module_id.as_str() {
        "overview_metrics" => {
            let layout_tree = pipeline::overview_metrics::compile_metrics_card(&ticker);
            Ok(UiModulePayload {
                id: "overview_metrics".to_string(),
                title: "Data Matrix Processing Core".to_string(),
                root_node: layout_tree,
            })
        },
        "performance_chart" => {
            let layout_tree = pipeline::overview_metrics::compile_chart_card(&ticker);
            Ok(UiModulePayload {
                id: "performance_chart".to_string(),
                title: "Volatility Velocity Stream Matrix".to_string(),
                root_node: layout_tree,
            })
        },
        _ => Err(format!("Unrecognized module request identifier '{}'", module_id)),
    }
}

// 🎯 ENSURE THIS ATTRIBUTE WRAPPER SITS DIRECTLY ABOVE THIS FUNCTION:
#[tauri::command]
pub fn fetch_component_catalog() -> Result<Vec<CatalogItem>, String> {
    Ok(vec![
        CatalogItem {
            id: "overview_metrics".to_string(),
            name: "Metrics Grid Matrix".to_string(),
            description: "Live summary monitoring stream covering market yield totals and parquet pool streams.".to_string(),
        },
        CatalogItem {
            id: "performance_chart".to_string(),
            name: "Volatility Velocity Stream".to_string(),
            description: "Asynchronous bar telemetry scaling smoothly via backend calculations.".to_string(),
        },
    ])
}