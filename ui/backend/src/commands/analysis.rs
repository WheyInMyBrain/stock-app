use tauri::command;

#[command]
pub async fn trigger_core_analysis(
    ticker: String, 
    wacc: Option<f64>, 
    terminal_g: Option<f64>, 
    data_dir: Option<String>, 
    modules: Option<String>
) -> Result<String, String> {
    println!("🚀 [ANALYSIS ENGINE]: Invoked from viewport for ticker [{}]", ticker);

    // Resolve parameter configurations with identical fallbacks to your CLI tool
    let final_wacc = wacc.unwrap_or(0.12);
    let final_g = terminal_g.unwrap_or(0.04);
    let final_dir = data_dir.unwrap_or_else(|| "../data".to_string());
    let final_modules = modules.unwrap_or_else(|| "all".to_string());

    // 🎯 FIX: Clone variables to move into the thread closure, keeping originals alive below
    let ticker_for_thread = ticker.clone();
    let modules_for_thread = final_modules.clone();

    // 🧠 Offload execution to a separate OS background thread to prevent GUI lockups
    tauri::async_runtime::spawn_blocking(move || {
        analysis::runner::run_global_analysis_pipeline(
            &ticker_for_thread,
            final_wacc,
            final_g,
            &final_dir,
            &modules_for_thread,
        );
    })
    .await
    .map_err(|e| format!("Analytical thread engine pool panic error: {}", e))?;

    Ok(format!(
        "Success: Analytical computation pipelines completed for [{}]. Targets compiled: {}", 
        ticker, final_modules
    ))
}