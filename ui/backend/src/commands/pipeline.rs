use serde::{Serialize, Deserialize};
use serde_json::Value;
use crate::pipeline::WorkspaceModule;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UiModulePayload {
    pub id: String,
    pub title: String,
    pub root_node: Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CatalogItem {
    pub id: String,
    pub name: String,
    pub description: String,
}

fn get_module_registry() -> Vec<Box<dyn WorkspaceModule>> {
    vec![
        Box::new(crate::pipeline::company_profile::CompanyProfileCard),
        Box::new(crate::pipeline::stock_stats::StockStatsCard),
    ]
}

#[tauri::command]
pub fn fetch_component_telemetry(ticker: String, module_id: String, timeframe: Option<String>) -> Result<UiModulePayload, String> {
    let active_tf = timeframe.unwrap_or_else(|| "10Y".to_string());
    let registry = get_module_registry();

    if let Some(module) = registry.iter().find(|m| m.catalog_definition().id == module_id) {
        let definition = module.catalog_definition();
        let layout_tree = module.compile(&ticker, &active_tf)?;

        return Ok(UiModulePayload {
            id: module_id,
            title: definition.name,
            root_node: layout_tree,
        });
    }

    Err(format!("Unrecognized module request identifier '{}'", module_id))
}

#[tauri::command]
pub fn fetch_component_catalog() -> Result<Vec<CatalogItem>, String> {
    let items = get_module_registry()
        .iter()
        .map(|module| module.catalog_definition())
        .collect();
    Ok(items)
}