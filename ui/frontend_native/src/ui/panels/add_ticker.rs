// stock-app/ui/frontend_native/src/ui/panels/add_ticker.rs
use egui::Ui;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

// 🎯 THREAD-SAFE STATE CHANNEL BOUNDARIES:
// Tracks parsing pipelines out-of-band to prevent UI freezing without external Tauri overrides
static PARSER_IS_RUNNING: AtomicBool = AtomicBool::new(false);
static PARSER_TICKER: Mutex<String> = Mutex::new(String::new());
static ACTIVE_PARSING_LOGS: Mutex<Option<String>> = Mutex::new(None);

pub fn draw_add_ticker_panel(ui: &mut Ui) {
    // A. Safely clone short-lived snapshots of the ongoing execution layers
    let active_download_snapshot = {
        let guard = backend::commands::downloader::ACTIVE_INGESTION.lock().unwrap();
        guard.clone()
    };
    
    let is_download_running = active_download_snapshot.as_ref().map_or(false, |p| !p.is_done);
    let is_download_done = active_download_snapshot.as_ref().map_or(false, |p| p.is_done);
    
    let is_parser_running = PARSER_IS_RUNNING.load(Ordering::Relaxed);
    let has_parsing_logs = ACTIVE_PARSING_LOGS.lock().unwrap().is_some();

    if is_download_running {
        // ============================================================================
        // 🚀 VIEW MODE A: ACTIVE DOWNLOADS VIEWPORT
        // ============================================================================
        ui.vertical_centered(|ui| {
            ui.add_space(60.0);
            
            ui.allocate_ui(egui::vec2(480.0, 0.0), |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        if let Some(ref progress) = active_download_snapshot {
                            ui.label(egui::RichText::new(format!("⚡ DOWNLOADING TICKER: {}", progress.ticker)).heading());
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let stop_btn = egui::Button::new(egui::RichText::new("🛑 STOP").color(egui::Color32::WHITE))
                                .fill(egui::Color32::from_rgb(200, 40, 40));
                            if ui.add(stop_btn).clicked() {
                                let mut guard = backend::commands::downloader::ACTIVE_INGESTION.lock().unwrap();
                                *guard = None; // Flushes active network handles instantly
                            }
                        });
                    });

                    ui.add_space(35.0);
                    ui.label("WORKFLOW PIPELINE EXECUTION STEPS:");
                    ui.add_space(16.0);

                    ui.label("1. Download Market Data");
                    ui.add_space(12.0);

                    if let Some(ref progress) = active_download_snapshot {
                        if progress.nse_active {
                            ui.horizontal(|ui| { ui.add_space(24.0); ui.label(egui::RichText::new("NSE").strong().small()); });
                            ui.add_space(6.0);

                            let active_nse: Vec<&_> = progress.nse_downloads.iter().filter(|t| t.percentage < 100.0).collect();
                            if active_nse.is_empty() {
                                ui.horizontal(|ui| { ui.add_space(40.0); ui.label(egui::RichText::new("Waiting in queue / Synchronized").small().weak()); });
                                ui.add_space(14.0);
                            } else {
                                for track in active_nse {
                                    ui.horizontal(|ui| {
                                        ui.add_space(40.0);
                                        let name = if track.filename.is_empty() { track.current_api.clone() } else { format!("{} - {}", track.current_api, track.filename) };
                                        ui.label(egui::RichText::new(name).small().weak());
                                    });
                                    ui.add_space(4.0);
                                    ui.horizontal(|ui| {
                                        ui.add_space(40.0);
                                        let fraction = (track.percentage / 100.0).clamp(0.0, 1.0);
                                        let bar_width = ui.available_width() - 55.0;
                                        let (rect, _) = ui.allocate_at_least(egui::vec2(bar_width, 5.0), egui::Sense::hover());
                                        ui.painter().rect_filled(rect, 0.0, egui::Color32::from_rgb(25, 25, 25));
                                        if fraction > 0.0 {
                                            let mut pr = rect; pr.set_width(rect.width() * fraction);
                                            ui.painter().rect_filled(pr, 0.0, egui::Color32::WHITE);
                                        }
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { ui.label(egui::RichText::new(format!("{:.1}%", track.percentage)).small().weak()); });
                                    });
                                    ui.add_space(16.0);
                                }
                            }
                        }

                        if progress.bse_active {
                            ui.horizontal(|ui| { ui.add_space(24.0); ui.label(egui::RichText::new("BSE").strong().small()); });
                            ui.add_space(6.0);

                            let active_bse: Vec<&_> = progress.bse_downloads.iter().filter(|t| t.percentage < 100.0).collect();
                            if active_bse.is_empty() {
                                ui.horizontal(|ui| { ui.add_space(40.0); ui.label(egui::RichText::new("Waiting in queue / Synchronized").small().weak()); });
                                ui.add_space(14.0);
                            } else {
                                for track in active_bse {
                                    ui.horizontal(|ui| {
                                        ui.add_space(40.0);
                                        let name = if track.filename.is_empty() { track.current_api.clone() } else { format!("{} - {}", track.current_api, track.filename) };
                                        ui.label(egui::RichText::new(name).small().weak());
                                    });
                                    ui.add_space(4.0);
                                    ui.horizontal(|ui| {
                                        ui.add_space(40.0);
                                        let fraction = (track.percentage / 100.0).clamp(0.0, 1.0);
                                        let bar_width = ui.available_width() - 55.0;
                                        let (rect, _) = ui.allocate_at_least(egui::vec2(bar_width, 5.0), egui::Sense::hover());
                                        ui.painter().rect_filled(rect, 0.0, egui::Color32::from_rgb(25, 25, 25));
                                        if fraction > 0.0 {
                                            let mut pr = rect; pr.set_width(rect.width() * fraction);
                                            ui.painter().rect_filled(pr, 0.0, egui::Color32::WHITE);
                                        }
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { ui.label(egui::RichText::new(format!("{:.1}%", track.percentage)).small().weak()); });
                                    });
                                    ui.add_space(16.0);
                                }
                            }
                        }
                    }

                    ui.add_space(14.0);
                    ui.add_enabled_ui(false, |ui| { ui.checkbox(&mut true, "2. Parse Market Structures (Queued...)"); });
                    ui.add_space(24.0);
                    ui.add_enabled_ui(false, |ui| { ui.checkbox(&mut false, "3. Core Financial Analysis"); });
                });
            });
        });
    } else if is_parser_running {
        // ============================================================================
        // ⚡ VIEW MODE B: ACTIVE PARSER VIEWPORT (RAYON / POLARS PIPELINE)
        // ============================================================================
        ui.vertical_centered(|ui| {
            ui.add_space(60.0);
            ui.allocate_ui(egui::vec2(480.0, 0.0), |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        let ticker = PARSER_TICKER.lock().unwrap().clone();
                        ui.label(egui::RichText::new(format!("⚡ PARSING DATASTRUCTURES: {}", ticker)).heading());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let abort_btn = egui::Button::new(egui::RichText::new("🛑 STOP").color(egui::Color32::WHITE))
                                .fill(egui::Color32::from_rgb(200, 40, 40));
                            if ui.add(abort_btn).clicked() {
                                PARSER_IS_RUNNING.store(false, Ordering::Relaxed); // Force flush tracking variables
                            }
                        });
                    });

                    ui.add_space(45.0);
                    ui.label("WORKFLOW PIPELINE EXECUTION STEPS:");
                    ui.add_space(16.0);

                    ui.add_enabled_ui(false, |ui| { ui.checkbox(&mut false, "1. Download Market Data (Skipped/Complete)"); });
                    ui.add_space(24.0);

                    ui.label("2. Parse Market Structures");
                    ui.add_space(12.0);

                    ui.horizontal(|ui| {
                        ui.add_space(24.0);
                        ui.label(egui::RichText::new("Running multi-threaded Polars data frame serialization loops...").small().weak());
                    });
                    ui.add_space(6.0);

                    ui.horizontal(|ui| {
                        ui.add_space(24.0);
                        let bar_width = ui.available_width();
                        let (rect, _) = ui.allocate_at_least(egui::vec2(bar_width, 6.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 0.0, egui::Color32::from_rgb(25, 25, 25));
                        ui.painter().rect_filled(rect, 0.0, egui::Color32::WHITE); // Solid elegant white loader indicator
                    });

                    ui.add_space(34.0);
                    ui.add_enabled_ui(false, |ui| { ui.checkbox(&mut false, "3. Core Financial Analysis"); });
                });
            });
        });
    } else if is_download_done || has_parsing_logs {
        // ============================================================================
        // 🎉 VIEW MODE C: COMPLETION & REPORT VIEWPORT
        // ============================================================================
        ui.vertical_centered(|ui| {
            ui.add_space(60.0);
            
            let ticker = if let Some(ref progress) = active_download_snapshot {
                progress.ticker.clone()
            } else {
                PARSER_TICKER.lock().unwrap().clone()
            };

            ui.label(egui::RichText::new(format!("🎉 INGESTION COMPLETE FOR {}", ticker)).heading());
            ui.add_space(15.0);
            ui.label("All active operations have finished. Below is the compiled dataset report matrix:");
            ui.add_space(20.0);

            // Render high-contrast Polars log statistics safely using precise monospaced layouts
            if let Some(ref logs) = *ACTIVE_PARSING_LOGS.lock().unwrap() {
                ui.allocate_ui(egui::vec2(520.0, 0.0), |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(320.0)
                        .id_source("parser_report_logs_viewport")
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new(logs).font(egui::FontId::new(11.5, egui::FontFamily::Monospace)));
                        });
                });
                ui.add_space(25.0);
            } else {
                ui.label("All physical exchange files downloaded and archived locally.");
                ui.add_space(35.0);
            }
            
            if ui.button(" 📑 CLEAR AND FINALIZE WORKFLOW ").clicked() {
                let mut download_guard = backend::commands::downloader::ACTIVE_INGESTION.lock().unwrap();
                *download_guard = None;
                let mut logs_guard = ACTIVE_PARSING_LOGS.lock().unwrap();
                *logs_guard = None;
            }
        });
    } else {
        // ============================================================================
        // ➕ VIEW MODE D: FORM CONFIGURATION & LAUNCH ENTRY
        // ============================================================================
        ui.vertical_centered(|ui| {
            ui.add_space(60.0);
            ui.heading("➕ ADD NEW TICKER");
            ui.add_space(25.0);

            let input_id = egui::Id::new("add_ticker_input_buffer_field");
            let step1_id = egui::Id::new("add_ticker_step1_download");
            let step2_id = egui::Id::new("add_ticker_step2_parse");
            let nse_id = egui::Id::new("add_ticker_nse_checked");
            let bse_id = egui::Id::new("add_ticker_bse_checked");

            let mut input_buffer = ui.data_mut(|d| d.get_temp::<String>(input_id).unwrap_or_default());

            let text_edit = egui::TextEdit::singleline(&mut input_buffer)
                .hint_text("Enter Symbol (e.g. IMFA, OIL, RELIANCE)")
                .desired_width(320.0);
            
            if ui.add(text_edit).changed() {
                ui.data_mut(|d| d.insert_temp(input_id, input_buffer.clone()));
            }

            ui.add_space(35.0);

            ui.allocate_ui(egui::vec2(480.0, 0.0), |ui| {
                ui.vertical(|ui| {
                    // 📦 STEP 1 CHECKBOX CONFIG
                    let mut step1_download = ui.data_mut(|d| d.get_temp::<bool>(step1_id).unwrap_or(true));
                    if ui.checkbox(&mut step1_download, "1. Download Market Data").changed() {
                        ui.data_mut(|d| d.insert_temp(step1_id, step1_download));
                    }

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

                    // 📦 STEP 2 CHECKBOX CONFIG (NOW FULLY INTERACTIVE & INDEPENDENT!)
                    let mut step2_parse = ui.data_mut(|d| d.get_temp::<bool>(step2_id).unwrap_or(true));
                    if ui.checkbox(&mut step2_parse, "2. Parse Market Structures").changed() {
                        ui.data_mut(|d| d.insert_temp(step2_id, step2_parse));
                    }

                    ui.add_space(28.0);
                    ui.add_enabled_ui(false, |ui| { ui.checkbox(&mut false, "3. Core Financial Analysis"); });
                });
            });

            ui.add_space(45.0);
            
            if ui.button(" ⚡ INGEST TICKER ").clicked() {
                let ticker_symbol = input_buffer.trim().to_uppercase();
                if !ticker_symbol.is_empty() {
                    let run_download = ui.data_mut(|d| d.get_temp::<bool>(step1_id).unwrap_or(true));
                    let run_parse = ui.data_mut(|d| d.get_temp::<bool>(step2_id).unwrap_or(true));

                    let nse_checked = ui.data_mut(|d| d.get_temp::<bool>(nse_id).unwrap_or(false));
                    let bse_checked = ui.data_mut(|d| d.get_temp::<bool>(bse_id).unwrap_or(false));

                    let mut args = vec![ticker_symbol.clone()];
                    if nse_checked && !bse_checked {
                        args.push("--mode=nse".to_string());
                    } else if bse_checked && !nse_checked {
                        args.push("--mode=bse".to_string());
                    } else {
                        args.push("--mode=both".to_string());
                    }

                    // Flush any stale memory blocks before launching new execution thread loops
                    if let Ok(mut logs) = ACTIVE_PARSING_LOGS.lock() { *logs = None; }

                    tokio::spawn(async move {
                        if run_download {
                            // Blocks background task until IPC network socket triggers final completion signals
                            let _ = crate::core::downloader::dispatch_download(args).await;
                        }

                        if run_parse {
                            PARSER_IS_RUNNING.store(true, Ordering::Relaxed);
                            if let Ok(mut t) = PARSER_TICKER.lock() { *t = ticker_symbol.clone(); }

                            // Fire your pure zero-copy native backend command function
                            let parse_result = backend::commands::parser::run_pipeline_parser(&ticker_symbol, None);

                            PARSER_IS_RUNNING.store(false, Ordering::Relaxed);
                            if let Ok(mut logs) = ACTIVE_PARSING_LOGS.lock() {
                                match parse_result {
                                    Ok(report) => *logs = Some(report),
                                    Err(err) => *logs = Some(format!("❌ Ingestion Parser Error:\n{}", err)),
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