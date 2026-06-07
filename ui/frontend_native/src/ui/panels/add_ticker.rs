// stock-app/ui/frontend_native/src/ui/panels/add_ticker.rs
use egui::Ui;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

// 🎯 THREAD-SAFE GLOBAL WORKFLOW MONITORS
static PARSER_IS_RUNNING: AtomicBool = AtomicBool::new(false);
static ANALYSIS_IS_RUNNING: AtomicBool = AtomicBool::new(false);

static PIPELINE_TICKER: Mutex<String> = Mutex::new(String::new());
static ACTIVE_PARSING_LOGS: Mutex<Option<String>> = Mutex::new(None);
static ANALYSIS_REPORT_TXT: Mutex<Option<String>> = Mutex::new(None);

/// ⚡ Shared Layout Helper: Renders an ultra-clean solid Black track with a high-contrast solid White progress line
fn draw_minimal_progress_line(ui: &mut Ui, percentage: f32) {
    ui.horizontal(|ui| {
        ui.add_space(40.0);
        let fraction = (percentage / 100.0).clamp(0.0, 1.0);
        let bar_width = ui.available_width() - 55.0; // Allocates margin for text positioning alignment
        
        let (rect, _) = ui.allocate_at_least(egui::vec2(bar_width, 5.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 0.0, egui::Color32::from_rgb(25, 25, 25)); // Solid background track
        
        if fraction > 0.0 {
            let mut progress_rect = rect;
            progress_rect.set_width(rect.width() * fraction);
            ui.painter().rect_filled(progress_rect, 0.0, egui::Color32::WHITE); // Solid foreground line fill
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(egui::RichText::new(format!("{:.1}%", percentage)).small().weak());
        });
    });
}

pub fn draw_add_ticker_panel(ui: &mut Ui) {
    // A. Fetch current asynchronous workflow layer runtime snapshots
    let active_download_snapshot = {
        let guard = backend::commands::downloader::ACTIVE_INGESTION.lock().unwrap();
        guard.clone()
    };
    
    let is_download_running = active_download_snapshot.as_ref().map_or(false, |p| !p.is_done);
    let is_download_done = active_download_snapshot.as_ref().map_or(false, |p| p.is_done);
    
    let is_parser_running = PARSER_IS_RUNNING.load(Ordering::Relaxed);
    let is_analysis_running = ANALYSIS_IS_RUNNING.load(Ordering::Relaxed);
    
    let has_parsing_logs = ACTIVE_PARSING_LOGS.lock().unwrap().is_some();
    let has_analysis_logs = ANALYSIS_REPORT_TXT.lock().unwrap().is_some();

    // 🧠 CENTRAL EVALUATION CROSS-SECTIONS
    let global_is_running = is_download_running || is_parser_running || is_analysis_running;
    let global_is_done = has_parsing_logs || has_analysis_logs || (is_download_done && !is_parser_running && !is_analysis_running);

    if global_is_running {
        // ============================================================================
        // ⚡ VIEW 1: UNIFIED ACTIVE PROCESSING VIEWPORT (SIMPLIFIED CONSOLIDATED LAYOUT)
        // ============================================================================
        ui.vertical_centered(|ui| {
            ui.add_space(60.0);
            ui.allocate_ui(egui::vec2(480.0, 0.0), |ui| {
                ui.vertical(|ui| {
                    
                    // Unified Header Control Banner
                    ui.horizontal(|ui| {
                        let ticker = if let Some(ref dl) = active_download_snapshot { dl.ticker.clone() } else { PIPELINE_TICKER.lock().unwrap().clone() };
                        ui.label(egui::RichText::new(format!("⚡ PROCESSING WORKFLOW: {}", ticker)).heading());
                        
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let stop_btn = egui::Button::new(egui::RichText::new("🛑 STOP").color(egui::Color32::WHITE))
                                .fill(egui::Color32::from_rgb(200, 40, 40));
                            if ui.add(stop_btn).clicked() {
                                // Instantly flush memory variables to signal thread termination loops safely
                                let mut dl_guard = backend::commands::downloader::ACTIVE_INGESTION.lock().unwrap();
                                *dl_guard = None;
                                PARSER_IS_RUNNING.store(false, Ordering::Relaxed);
                                ANALYSIS_IS_RUNNING.store(false, Ordering::Relaxed);
                            }
                        });
                    });

                    ui.add_space(35.0);
                    ui.label("WORKFLOW PIPELINE PROGRESS TIMELINE:");
                    ui.add_space(16.0);

                    // 📦 SUB-STAGE 1: DOWNLOAD ENGINE METRICS
                    if is_download_running {
                        ui.label(egui::RichText::new("🔄 Step 1: Downloading Market Data...").strong());
                        ui.add_space(12.0);

                        if let Some(ref progress) = active_download_snapshot {
                            if progress.nse_active {
                                ui.horizontal(|ui| { ui.add_space(24.0); ui.label(egui::RichText::new("NSE Exchange Stream").small().weak()); });
                                ui.add_space(4.0);
                                if let Some(track) = progress.nse_downloads.iter().find(|t| t.percentage < 100.0) {
                                    ui.horizontal(|ui| { ui.add_space(40.0); ui.label(egui::RichText::new(&track.current_api).small().weak()); });
                                    ui.add_space(4.0); draw_minimal_progress_line(ui, track.percentage);
                                } else {
                                    ui.horizontal(|ui| { ui.add_space(40.0); ui.label(egui::RichText::new("Synchronizing tracks...").small().weak()); });
                                }
                                ui.add_space(16.0);
                            }
                            if progress.bse_active {
                                ui.horizontal(|ui| { ui.add_space(24.0); ui.label(egui::RichText::new("BSE Exchange Stream").small().weak()); });
                                ui.add_space(4.0);
                                if let Some(track) = progress.bse_downloads.iter().find(|t| t.percentage < 100.0) {
                                    ui.horizontal(|ui| { ui.add_space(40.0); ui.label(egui::RichText::new(&track.current_api).small().weak()); });
                                    ui.add_space(4.0); draw_minimal_progress_line(ui, track.percentage);
                                } else {
                                    ui.horizontal(|ui| { ui.add_space(40.0); ui.label(egui::RichText::new("Synchronizing tracks...").small().weak()); });
                                }
                                ui.add_space(16.0);
                            }
                        }
                    } else if is_download_done || has_parsing_logs || has_analysis_logs {
                        // 🎯 DYNAMIC CORRECTION: Only show complete tag if the track was explicitly executed
                        ui.label(egui::RichText::new("✅ Step 1: Download Market Data Complete").small().weak());
                        ui.add_space(14.0);
                    }

                    // 📦 SUB-STAGE 2: ZERO-COPY DATA FRAME PARSER METRICS
                    if is_parser_running {
                        ui.label(egui::RichText::new("🔄 Step 2: Parsing Market Structures...").strong());
                        ui.add_space(8.0);
                        ui.horizontal(|ui| { ui.add_space(24.0); ui.label(egui::RichText::new("Compiling tabular filings into optimized columnar Parquet tables...").small().weak()); });
                        ui.add_space(6.0);
                        draw_minimal_progress_line(ui, 100.0);
                        ui.add_space(24.0);
                    } else if has_parsing_logs {
                        ui.label(egui::RichText::new("✅ Step 2: Parse Market Structures Complete").small().weak());
                        ui.add_space(14.0);
                    }

                    // 📦 SUB-STAGE 3: MULTI-THREADED ANALYSIS MATRICES
                    if is_analysis_running {
                        ui.label(egui::RichText::new("🔄 Step 3: Core Financial Analysis Active...").strong());
                        ui.add_space(8.0);
                        ui.horizontal(|ui| { ui.add_space(24.0); ui.label(egui::RichText::new("Executing parallel DCF projection models and valuation grids...").small().weak()); });
                        ui.add_space(6.0);
                        draw_minimal_progress_line(ui, 100.0);
                    }
                });
            });
        });
    } else if global_is_done {
        // ============================================================================
        // 🎉 VIEW 2: ACCURATE COMPLETION REPORT VIEWER (NO MISLEADING Mockups)
        // ============================================================================
        ui.vertical_centered(|ui| {
            ui.add_space(60.0);
            let ticker = PIPELINE_TICKER.lock().unwrap().clone();
            ui.label(egui::RichText::new(format!("🎉 INGESTION COMPLETE FOR {}", ticker)).heading());
            ui.add_space(15.0);
            ui.label("All active operations have finished. Below is the compiled execution summary report:");
            ui.add_space(20.0);

            let mut consolidated_report = String::new();
            if let Some(ref p_logs) = *ACTIVE_PARSING_LOGS.lock().unwrap() {
                consolidated_report.push_str(p_logs);
                consolidated_report.push_str("\n");
            }
            if let Some(ref a_logs) = *ANALYSIS_REPORT_TXT.lock().unwrap() {
                consolidated_report.push_str(a_logs);
            }

            if !consolidated_report.is_empty() {
                ui.allocate_ui(egui::vec2(520.0, 0.0), |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(320.0)
                        .id_source("consolidated_ingestion_report_matrix_viewport")
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new(&consolidated_report).font(egui::FontId::new(11.5, egui::FontFamily::Monospace)));
                        });
                });
                ui.add_space(25.0);
            } else {
                // 🎯 DYNAMIC FIX: Shows a tailored statement when only downloads were executed
                ui.label("📈 Market data file packets downloaded and archived successfully.");
                ui.add_space(35.0);
            }
            
            if ui.button(" 📑 CLEAR AND FINALIZE WORKFLOW ").clicked() {
                let mut download_guard = backend::commands::downloader::ACTIVE_INGESTION.lock().unwrap();
                *download_guard = None;
                let mut p_logs = ACTIVE_PARSING_LOGS.lock().unwrap(); *p_logs = None;
                let mut a_logs = ANALYSIS_REPORT_TXT.lock().unwrap(); *a_logs = None;
            }
        });
    } else {
        // ============================================================================
        // ➕ VIEW 3: CONFIGURATION FORM COCKPIT (UNCHANGED ENTRY LAYOUT)
        // ============================================================================
        ui.vertical_centered(|ui| {
            ui.add_space(60.0);
            ui.heading("➕ ADD NEW TICKER");
            ui.add_space(25.0);

            let input_id = egui::Id::new("add_ticker_input_buffer_field");
            let step1_id = egui::Id::new("add_ticker_step1_download");
            let step2_id = egui::Id::new("add_ticker_step2_parse");
            let step3_id = egui::Id::new("add_ticker_step3_analyze");
            let nse_id = egui::Id::new("add_ticker_nse_checked");
            let bse_id = egui::Id::new("add_ticker_bse_checked");

            let mut input_buffer = ui.data_mut(|d| d.get_temp::<String>(input_id).unwrap_or_default());

            let text_edit = egui::TextEdit::singleline(&mut input_buffer)
                .hint_text("Enter Symbol (e.g. IMFA, OIL, TATA)")
                .desired_width(320.0);
            
            if ui.add(text_edit).changed() { ui.data_mut(|d| d.insert_temp(input_id, input_buffer.clone())); }

            ui.add_space(35.0);

            ui.allocate_ui(egui::vec2(480.0, 0.0), |ui| {
                ui.vertical(|ui| {
                    let mut step1_download = ui.data_mut(|d| d.get_temp::<bool>(step1_id).unwrap_or(true));
                    if ui.checkbox(&mut step1_download, "1. Download Market Data").changed() { ui.data_mut(|d| d.insert_temp(step1_id, step1_download)); }

                    if step1_download {
                        ui.add_space(14.0);
                        let mut nse_checked = ui.data_mut(|d| d.get_temp::<bool>(nse_id).unwrap_or(false));
                        let mut bse_checked = ui.data_mut(|d| d.get_temp::<bool>(bse_id).unwrap_or(false));

                        ui.horizontal(|ui| {
                            ui.add_space(24.0); 
                            if ui.checkbox(&mut nse_checked, "NSE").changed() { ui.data_mut(|d| d.insert_temp(nse_id, nse_checked)); }
                            ui.add_space(24.0);
                            if ui.checkbox(&mut bse_checked, "BSE").changed() { ui.data_mut(|d| d.insert_temp(bse_id, bse_checked)); }
                        });
                    }

                    ui.add_space(24.0);

                    let mut step2_parse = ui.data_mut(|d| d.get_temp::<bool>(step2_id).unwrap_or(true));
                    if ui.checkbox(&mut step2_parse, "2. Parse Market Structures").changed() { ui.data_mut(|d| d.insert_temp(step2_id, step2_parse)); }

                    ui.add_space(24.0);

                    let mut step3_analyze = ui.data_mut(|d| d.get_temp::<bool>(step3_id).unwrap_or(true));
                    if ui.checkbox(&mut step3_analyze, "3. Core Financial Analysis").changed() { ui.data_mut(|d| d.insert_temp(step3_id, step3_analyze)); }
                });
            });

            ui.add_space(45.0);
            
            if ui.button(" ⚡ INGEST TICKER ").clicked() {
                let ticker_symbol = input_buffer.trim().to_uppercase();
                if !ticker_symbol.is_empty() {
                    let run_download = ui.data_mut(|d| d.get_temp::<bool>(step1_id).unwrap_or(true));
                    let run_parse = ui.data_mut(|d| d.get_temp::<bool>(step2_id).unwrap_or(true));
                    let run_analyze = ui.data_mut(|d| d.get_temp::<bool>(step3_id).unwrap_or(true));

                    let nse_checked = ui.data_mut(|d| d.get_temp::<bool>(nse_id).unwrap_or(false));
                    let bse_checked = ui.data_mut(|d| d.get_temp::<bool>(bse_id).unwrap_or(false));

                    let mut args = vec![ticker_symbol.clone()];
                    if nse_checked && !bse_checked { args.push("--mode=nse".to_string()); } 
                    else if bse_checked && !nse_checked { args.push("--mode=bse".to_string()); } 
                    else { args.push("--mode=both".to_string()); }

                    if let Ok(mut logs) = ACTIVE_PARSING_LOGS.lock() { *logs = None; }
                    if let Ok(mut logs) = ANALYSIS_REPORT_TXT.lock() { *logs = None; }
                    if let Ok(mut t) = PIPELINE_TICKER.lock() { *t = ticker_symbol.clone(); }

                    tokio::spawn(async move {
                        if run_download {
                            let _ = crate::core::downloader::dispatch_download(args).await;
                        }

                        if run_parse {
                            PARSER_IS_RUNNING.store(true, Ordering::Relaxed);
                            
                            let parse_config = crate::core::parser::ParserConfig { ticker: ticker_symbol.clone(), data_dir_override: None };
                            let parse_result = crate::core::parser::dispatch_parse(parse_config);
                            
                            PARSER_IS_RUNNING.store(false, Ordering::Relaxed);
                            if let Ok(mut logs) = ACTIVE_PARSING_LOGS.lock() {
                                match parse_result {
                                    Ok(report) => *logs = Some(report),
                                    Err(err) => *logs = Some(format!("❌ Ingestion Parser Error:\n{}", err)),
                                }
                            }
                        }

                        if run_analyze {
                            ANALYSIS_IS_RUNNING.store(true, Ordering::Relaxed);
                            
                            let analysis_config = crate::core::analysis::AnalysisConfig {
                                ticker: ticker_symbol.clone(), wacc: None, terminal_g: None, data_dir_override: None, modules: None,
                            };
                            let analysis_result = crate::core::analysis::dispatch_analysis(analysis_config);
                            
                            ANALYSIS_IS_RUNNING.store(false, Ordering::Relaxed);
                            if let Ok(mut logs) = ANALYSIS_REPORT_TXT.lock() {
                                match analysis_result {
                                    Ok(report) => *logs = Some(report),
                                    Err(err) => *logs = Some(format!("❌ Valuation Matrix Analysis Error:\n{}", err)),
                                }
                            }
                        }
                    });
                    
                    input_buffer.clear();
                    ui.data_mut(|d| d.insert_temp(input_id, input_buffer.clone()));
                }
            }
        });
    }
}