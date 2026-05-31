// stock-app/ui/backend/src/commands/popup_manager.rs

use tauri::{AppHandle, WebviewWindowBuilder, WebviewUrl, Manager}; 
use crate::pipeline::popup::get_popup_registry;

#[derive(serde::Deserialize)]
pub struct PopupRequest {
    pub module_id: String,
    pub ticker: String,
}

#[tauri::command]
pub fn spawn_native_popup(app_handle: AppHandle, request: PopupRequest) -> Result<(), String> {
    let window_id = format!("win_{}_{}", request.module_id, request.ticker.to_lowercase());
    
    if let Some(existing_window) = app_handle.get_webview_window(&window_id) {
        let _ = existing_window.set_focus();
        return Ok(());
    }

    // 🎯 BLIND RUNTIME QUERY: Search the global dynamic map registry for the component definition
    let registry = get_popup_registry();
    let popup_blueprint = registry.get(request.module_id.as_str())
        .ok_or_else(|| format!("Security Boundary Breach: Unmapped module identifier -> [{}]", request.module_id))?;

    let title = popup_blueprint.window_title(&request.ticker);
    let (width, height) = popup_blueprint.initial_size();

    let route_target = format!("index.html#/popup?module={}&ticker={}", request.module_id, request.ticker);

    WebviewWindowBuilder::new(&app_handle, &window_id, WebviewUrl::App(route_target.into()))
        .title(title)
        .inner_size(width, height)
        .resizable(true)
        .focused(true)
        .build()
        .map_err(|e| format!("Failed to allocate window entity: {}", e))?;

    Ok(())
}

#[tauri::command]
pub fn compile_popup_telemetry(module_id: String, ticker: String) -> Result<serde_json::Value, String> {
    // 🎯 BLIND DATA FETCHING: Pull layouts dynamically using the registry map bounds
    let registry = get_popup_registry();
    let popup_blueprint = registry.get(module_id.as_str())
        .ok_or_else(|| format!("Module matrix registry key unmapped: {}", module_id))?;
        
    popup_blueprint.compile(&ticker)
}