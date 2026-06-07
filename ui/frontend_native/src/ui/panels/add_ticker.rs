// stock-app/ui/frontend_native/src/ui/panels/add_ticker.rs
use egui::Ui;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

// 🎯 THREAD-SAFE GLOBAL WORKFLOW STATE MONITORS
static PARSER_IS_RUNNING: AtomicBool = AtomicBool::new(false);
static ANALYSIS_IS_RUNNING: AtomicBool = AtomicBool::new(false);
static WORKFLOW_COMPLETE: AtomicBool = AtomicBool::new(false);

// 🎯 HIGH-RESOLUTION METRIC TIMING CHANNELS (IN MILLISECONDS)
static DOWNLOAD_DURATION_MS: AtomicU64 = AtomicU64::new(0);
static PARSER_DURATION_MS: AtomicU64 = AtomicU64::new(0);
static ANALYSIS_DURATION_MS: AtomicU64 = AtomicU64::new(0);

static DOWNLOAD_START_TIME: Mutex<Option<std::time::Instant>> = Mutex::new(None);
static PARSER_START_TIME: Mutex<Option<std::time::Instant>> = Mutex::new(None);
static ANALYSIS_START_TIME: Mutex<Option<std::time::Instant>> = Mutex::new(None);

// 🎯 WORKFLOW ENGINE PARAMETER SNAPSHOT TRACKERS
static REQ_RUN_DOWNLOAD: AtomicBool = AtomicBool::new(false);
static REQ_RUN_PARSE: AtomicBool = AtomicBool::new(false);
static REQ_RUN_ANALYZE: AtomicBool = AtomicBool::new(false);

static PIPELINE_TICKER: Mutex<String> = Mutex::new(String::new());

/// ⚡ Metric Timing Formatter: Presents durations cleanly in milliseconds or decimal seconds
fn format_ms(ms: u64) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else {
        format!("{:.2}s", ms as f32 / 1000.0)
    }
}

/// ⚡ Shared Layout Helper: Renders an ultra-clean solid Black track with a high-contrast solid White progress line and estimation text
fn draw_minimal_progress_line_with_est(ui: &mut Ui, percentage: f32, est_text: &str) {
    ui.horizontal(|ui| {
        ui.add_space(40.0);
        let fraction = (percentage / 100.0).clamp(0.0, 1.0);
        let bar_width = ui.available_width() - 110.0; // Allocates margin for estimate text alignment bounds
        
        let (rect, _) = ui.allocate_at_least(egui::vec2(bar_width, 5.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 0.0, egui::Color32::from_rgb(25, 25, 25)); // Solid background track
        
        if fraction > 0.0 {
            let mut progress_rect = rect;
            progress_rect.set_width(rect.width() * fraction);
            ui.painter().rect_filled(progress_rect, 0.0, egui::Color32::WHITE); // Solid foreground line fill
        }

        ui.label(egui::RichText::new(format!("{:.1}%", percentage)).small().weak());
        if !est_text.is_empty() {
            ui.add_space(6.0);
            ui.label(egui::RichText::new(est_text).small().weak());
        }
    });
}

pub fn draw_add_ticker_panel(ui: &mut Ui) {
    // A. Fetch current asynchronous workflow layer runtime snapshots from memory blocks
    let active_download_snapshot = {
        let guard = backend::commands::downloader::ACTIVE_INGESTION.lock().unwrap();
        guard.clone()
    };
    
    let is_download_running = active_download_snapshot.as_ref().map_or(false, |p| !p.is_done);
    
    let is_parser_running = PARSER_IS_RUNNING.load(Ordering::Relaxed);
    let is_analysis_running = ANALYSIS_IS_RUNNING.load(Ordering::Relaxed);

    // 🧠 CENTRAL ENGINE CHECKPOINTS EVALUATION
    let global_is_running = is_download_running || is_parser_running || is_analysis_running;
    let global_is_done = WORKFLOW_COMPLETE.load(Ordering::Relaxed);

    let req_download = REQ_RUN_DOWNLOAD.load(Ordering::Relaxed);
    let req_parse = REQ_RUN_PARSE.load(Ordering::Relaxed);
    let req_analyze = REQ_RUN_ANALYZE.load(Ordering::Relaxed);

    if global_is_running {
        // ============================================================================
        // ⚡ VIEW MODE 1: ACTIVE LIVE RUNNING METRICS WORKFLOW PIPELINE
        // ============================================================================
        ui.vertical_centered(|ui| {
            ui.add_space(60.0);
            ui.allocate_ui(egui::vec2(480.0, 0.0), |ui| {
                ui.vertical(|ui| {
                    
                    // Unified Header Banner
                    ui.horizontal(|ui| {
                        let ticker = if let Some(ref dl) = active_download_snapshot { dl.ticker.clone() } else { PIPELINE_TICKER.lock().unwrap().clone() };
                        ui.label(egui::RichText::new(format!("⚡ PROCESSING WORKFLOW: {}", ticker)).heading());
                    });

                    ui.add_space(35.0);
                    ui.label("WORKFLOW PIPELINE PROGRESS TIMELINE:");
                    ui.add_space(16.0);

                    // 📦 STAGE 1 TIMELINE NODE: DOWNLOADS METRICS CHANNEL
                    if req_download {
                        if is_download_running {
                            // ⏱️ Query live active timer tick
                            let elapsed_txt = if let Some(start) = *DOWNLOAD_START_TIME.lock().unwrap() {
                                format!(" [{:.1}s]", start.elapsed().as_secs_f32())
                            } else {
                                "".to_string()
                            };
                            
                            ui.label(egui::RichText::new(format!("🔄 Step 1: Downloading Market Data...{}", elapsed_txt)).strong());
                            ui.add_space(12.0);

                            if let Some(ref progress) = active_download_snapshot {
                                // --- NSE PIPELINE INDICATORS ---
                                if progress.nse_active {
                                    ui.horizontal(|ui| { ui.add_space(24.0); ui.label(egui::RichText::new("NSE Exchange Stream").small().weak()); });
                                    ui.add_space(4.0);
                                    
                                    let mut nse_curr = 0; let mut nse_total = 0; let mut nse_file_pct = 100.0;
                                    for track in &progress.nse_downloads {
                                        if track.current_step > nse_curr { nse_curr = track.current_step; nse_total = track.total_steps; }
                                        if track.percentage < 100.0 { nse_file_pct = track.percentage; }
                                    }
                                    
                                    let is_nse_done = nse_total > 0 && !progress.nse_downloads.iter().any(|t| t.percentage < 100.0) && nse_curr == nse_total;
                                    if is_nse_done {
                                        ui.horizontal(|ui| { ui.add_space(40.0); ui.label(egui::RichText::new(format!("✅ NSE Complete ({}/{})", nse_total, nse_total)).small().weak()); });
                                    } else {
                                        let active_api_name = progress.nse_downloads.iter().find(|t| t.percentage < 100.0).map(|t| t.current_api.clone()).unwrap_or_else(|| "Connecting...".to_string());
                                        ui.horizontal(|ui| { ui.add_space(40.0); ui.label(egui::RichText::new(format!("API: {} ({}/{})", active_api_name, nse_curr, nse_total)).small().weak()); });
                                        ui.add_space(4.0);
                                        let nse_pct = if nse_total > 0 { ((nse_curr.saturating_sub(1)) as f32 + (nse_file_pct / 100.0)) / (nse_total as f32) * 100.0 } else { 0.0 };
                                        draw_minimal_progress_line_with_est(ui, nse_pct, "Est: ~12s");
                                    }
                                    ui.add_space(14.0);
                                }

                                // --- BSE PIPELINE INDICATORS ---
                                if progress.bse_active {
                                    ui.horizontal(|ui| { ui.add_space(24.0); ui.label(egui::RichText::new("BSE Exchange Stream").small().weak()); });
                                    ui.add_space(4.0);
                                    
                                    let mut bse_curr = 0; let mut bse_total = 0; let mut bse_file_pct = 100.0;
                                    for track in &progress.bse_downloads {
                                        if track.current_step > bse_curr { bse_curr = track.current_step; bse_total = track.total_steps; }
                                        if track.percentage < 100.0 { bse_file_pct = track.percentage; }
                                    }
                                    
                                    let is_bse_done = bse_total > 0 && !progress.bse_downloads.iter().any(|t| t.percentage < 100.0) && bse_curr == bse_total;
                                    if is_bse_done {
                                        ui.horizontal(|ui| { ui.add_space(40.0); ui.label(egui::RichText::new(format!("✅ BSE Complete ({}/{})", bse_total, bse_total)).small().weak()); });
                                    } else {
                                        let active_api_name = progress.bse_downloads.iter().find(|t| t.percentage < 100.0).map(|t| t.current_api.clone()).unwrap_or_else(|| "Connecting...".to_string());
                                        ui.horizontal(|ui| { ui.add_space(40.0); ui.label(egui::RichText::new(format!("API: {} ({}/{})", active_api_name, bse_curr, bse_total)).small().weak()); });
                                        ui.add_space(4.0);
                                        let bse_pct = if bse_total > 0 { ((bse_curr.saturating_sub(1)) as f32 + (bse_file_pct / 100.0)) / (bse_total as f32) * 100.0 } else { 0.0 };
                                        draw_minimal_progress_line_with_est(ui, bse_pct, "Est: ~10s");
                                    }
                                    ui.add_space(14.0);
                                }
                            }
                        } else {
                            let duration = format_ms(DOWNLOAD_DURATION_MS.load(Ordering::Relaxed));
                            ui.label(egui::RichText::new(format!("✅ Step 1: Download Market Data Complete [{}]", duration)).small().weak());
                            ui.add_space(14.0);
                        }
                    }

                    // 📦 STAGE 2 TIMELINE NODE: DATA INGESTION STRUCTURAL PARSER
                    if req_parse {
                        if is_parser_running {
                            let elapsed_txt = if let Some(start) = *PARSER_START_TIME.lock().unwrap() {
                                format!(" [{:.1}s]", start.elapsed().as_secs_f32())
                            } else {
                                "".to_string()
                            };
                            ui.label(egui::RichText::new(format!("🔄 Step 2: Parsing Market Structures...{}", elapsed_txt)).strong());
                            ui.add_space(8.0);
                            ui.horizontal(|ui| { ui.add_space(24.0); ui.label(egui::RichText::new("Compiling exchange filing chunks into optimized Parquet tables...").small().weak()); });
                            ui.add_space(6.0);
                            draw_minimal_progress_line_with_est(ui, 100.0, "Est: <0.1s");
                            ui.add_space(16.0);
                        } else if is_analysis_running || global_is_done {
                            let duration = format_ms(PARSER_DURATION_MS.load(Ordering::Relaxed));
                            ui.label(egui::RichText::new(format!("✅ Step 2: Parse Market Structures Complete [{}]", duration)).small().weak());
                            ui.add_space(14.0);
                        } else {
                            ui.label(egui::RichText::new("⏳ Step 2: Parse Market Structures (Queued...)").small().weak());
                            ui.add_space(14.0);
                        }
                    }

                    // 📦 STAGE 3 TIMELINE NODE: MULTI-THREADED ANALYSIS MATRICES
                    if req_analyze {
                        if is_analysis_running {
                            let elapsed_txt = if let Some(start) = *ANALYSIS_START_TIME.lock().unwrap() {
                                format!(" [{:.1}s]", start.elapsed().as_secs_f32())
                            } else {
                                "".to_string()
                            };
                            ui.label(egui::RichText::new(format!("🔄 Step 3: Core Financial Analysis Active...{}", elapsed_txt)).strong());
                            ui.add_space(8.0);
                            ui.horizontal(|ui| { ui.add_space(24.0); ui.label(egui::RichText::new("Executing parallel DCF projection models, EPV tracks, and Monte Carlo matrices...").small().weak()); });
                            ui.add_space(6.0);
                            
                            let (comp_tasks, tot_tasks) = backend::commands::analysis::get_analysis_progress();
                            ui.horizontal(|ui| {
                                ui.add_space(24.0);
                                ui.label(egui::RichText::new(format!("Analytical Modules Solved: {} / {}", comp_tasks, tot_tasks)).small().weak());
                            });
                            ui.add_space(4.0);
                            let analysis_pct = if tot_tasks > 0 { (comp_tasks as f32 / tot_tasks as f32) * 100.0 } else { 0.0 };
                            draw_minimal_progress_line_with_est(ui, analysis_pct, "Est: ~3.5s");
                        } else if global_is_done {
                            let duration = format_ms(ANALYSIS_DURATION_MS.load(Ordering::Relaxed));
                            ui.label(egui::RichText::new(format!("✅ Step 3: Core Financial Analysis Complete [{}]", duration)).small().weak());
                            ui.add_space(14.0);
                        } else {
                            ui.label(egui::RichText::new("⏳ Step 3: Core Financial Analysis (Queued...)").small().weak());
                            ui.add_space(14.0);
                        }
                    }

                });
            });
        });
    } else if global_is_done {
        // ============================================================================
        // 🎉 VIEW MODE 2: ACCURATE COMPLETION REPORT VIEWER & TIMING METRICS TABLE
        // ============================================================================
        ui.vertical_centered(|ui| {
            ui.add_space(60.0);
            let ticker = PIPELINE_TICKER.lock().unwrap().clone();
            ui.label(egui::RichText::new(format!("🎉 INGESTION COMPLETE FOR {}", ticker)).heading());
            ui.add_space(15.0);
            
            // 🎯 FIXED: Direct, helpful workflow directive prompt
            ui.label(egui::RichText::new("Select this ticker from the sidebar to load its data and see all the analysis.").strong());
            ui.add_space(25.0);

            // 🎯 FIXED: Execution timing analytics grid display block
            ui.allocate_ui(egui::vec2(440.0, 0.0), |ui| {
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(16, 16, 16))
                    .inner_margin(16.0)
                    .rounding(4.0)
                    .show(ui, |ui| {
                        egui::Grid::new("pipeline_timing_summary_matrix_table")
                            .num_columns(2)
                            .spacing([80.0, 12.0])
                            .striped(true)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Pipeline Execution Step").strong());
                                ui.label(egui::RichText::new("Duration").strong());
                                ui.end_row();

                                let mut overall_total_ms = 0u64;

                                if req_download {
                                    let ms = DOWNLOAD_DURATION_MS.load(Ordering::Relaxed);
                                    overall_total_ms += ms;
                                    ui.label("1. Download Market Data");
                                    ui.label(format_ms(ms));
                                    ui.end_row();
                                }
                                if req_parse {
                                    let ms = PARSER_DURATION_MS.load(Ordering::Relaxed);
                                    overall_total_ms += ms;
                                    ui.label("2. Parse Market Structures");
                                    ui.label(format_ms(ms));
                                    ui.end_row();
                                }
                                if req_analyze {
                                    let ms = ANALYSIS_DURATION_MS.load(Ordering::Relaxed);
                                    overall_total_ms += ms;
                                    ui.label("3. Core Financial Analysis");
                                    ui.label(format_ms(ms));
                                    ui.end_row();
                                }

                                ui.label(egui::RichText::new("Total").strong());
                                ui.label(egui::RichText::new(format_ms(overall_total_ms)).strong());
                                ui.end_row();
                            });
                    });
            });

            ui.add_space(35.0);
            
            if ui.button(" 📑 CLEAR AND FINALIZE WORKFLOW ").clicked() {
                let mut download_guard = backend::commands::downloader::ACTIVE_INGESTION.lock().unwrap();
                *download_guard = None;
                WORKFLOW_COMPLETE.store(false, Ordering::Relaxed);
                REQ_RUN_DOWNLOAD.store(false, Ordering::Relaxed);
                REQ_RUN_PARSE.store(false, Ordering::Relaxed);
                REQ_RUN_ANALYZE.store(false, Ordering::Relaxed);
            }
        });
    } else {
        // ============================================================================
        // ➕ VIEW MODE 3: CONFIGURATION FORM COCKPIT (FORM ENTRY INTERFACE)
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

                    // Seed precise intent selection flags down memory registers right away
                    REQ_RUN_DOWNLOAD.store(run_download, Ordering::Relaxed);
                    REQ_RUN_PARSE.store(run_parse, Ordering::Relaxed);
                    REQ_RUN_ANALYZE.store(run_analyze, Ordering::Relaxed);

                    // 🎯 RESET HIGH-RESOLUTION TIMING AND WORKFLOW CONTAINERS BEFORE THREAD LAUNCH
                    DOWNLOAD_DURATION_MS.store(0, Ordering::Relaxed);
                    PARSER_DURATION_MS.store(0, Ordering::Relaxed);
                    ANALYSIS_DURATION_MS.store(0, Ordering::Relaxed);
                    WORKFLOW_COMPLETE.store(false, Ordering::Relaxed);

                    if let Ok(mut start) = DOWNLOAD_START_TIME.lock() { *start = None; }
                    if let Ok(mut start) = PARSER_START_TIME.lock() { *start = None; }
                    if let Ok(mut start) = ANALYSIS_START_TIME.lock() { *start = None; }

                    if let Ok(mut t) = PIPELINE_TICKER.lock() { *t = ticker_symbol.clone(); }

                    tokio::spawn(async move {
                        if run_download {
                            if let Ok(mut start) = DOWNLOAD_START_TIME.lock() { *start = Some(std::time::Instant::now()); }
                            let _ = crate::core::downloader::dispatch_download(args).await;
                            if let Ok(mut start) = DOWNLOAD_START_TIME.lock() {
                                if let Some(t) = start.take() { DOWNLOAD_DURATION_MS.store(t.elapsed().as_millis() as u64, Ordering::Relaxed); }
                            }
                        }

                        if run_parse {
                            PARSER_IS_RUNNING.store(true, Ordering::Relaxed);
                            if let Ok(mut start) = PARSER_START_TIME.lock() { *start = Some(std::time::Instant::now()); }
                            
                            let parse_config = crate::core::parser::ParserConfig { ticker: ticker_symbol.clone(), data_dir_override: None };
                            let _ = crate::core::parser::dispatch_parse(parse_config);
                            
                            if let Ok(mut start) = PARSER_START_TIME.lock() {
                                if let Some(t) = start.take() { PARSER_DURATION_MS.store(t.elapsed().as_millis() as u64, Ordering::Relaxed); }
                            }
                            PARSER_IS_RUNNING.store(false, Ordering::Relaxed);
                        }

                        if run_analyze {
                            ANALYSIS_IS_RUNNING.store(true, Ordering::Relaxed);
                            if let Ok(mut start) = ANALYSIS_START_TIME.lock() { *start = Some(std::time::Instant::now()); }
                            
                            let analysis_config = crate::core::analysis::AnalysisConfig {
                                ticker: ticker_symbol.clone(), wacc: None, terminal_g: None, data_dir_override: None, modules: None,
                            };
                            let _ = crate::core::analysis::dispatch_analysis(analysis_config);
                            
                            if let Ok(mut start) = ANALYSIS_START_TIME.lock() {
                                if let Some(t) = start.take() { ANALYSIS_DURATION_MS.store(t.elapsed().as_millis() as u64, Ordering::Relaxed); }
                            }
                            ANALYSIS_IS_RUNNING.store(false, Ordering::Relaxed);
                        }
                        
                        WORKFLOW_COMPLETE.store(true, Ordering::Relaxed);
                    });
                    
                    input_buffer.clear();
                    ui.data_mut(|d| d.insert_temp(input_id, input_buffer.clone()));
                }
            }
        });
    }
}