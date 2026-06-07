// stock-app/ui/frontend_native/src/ui/panels/add_ticker.rs
use egui::Ui;

pub fn draw_add_ticker_panel(ui: &mut Ui) {
    // A. Safely clone a short-lived snapshot of the active ingestion telemetry from the backend state
    let active_ingestion_snapshot = {
        let guard = backend::commands::downloader::ACTIVE_INGESTION.lock().unwrap();
        guard.clone()
    };
    
    let is_running = active_ingestion_snapshot.as_ref().map_or(false, |p| !p.is_done);
    let is_done = active_ingestion_snapshot.as_ref().map_or(false, |p| p.is_done);

    if is_running {
        // ============================================================================
        // 🚀 ACTIVE VIEW: Replaces the full screen view to track ingestion progress
        // ============================================================================
        ui.vertical_centered(|ui| {
            ui.add_space(60.0);
            
            ui.allocate_ui(egui::vec2(480.0, 0.0), |ui| {
                ui.vertical(|ui| {
                    
                    // Header layout: active ticker detail row on left, red STOP button aligned cleanly right
                    ui.horizontal(|ui| {
                        if let Some(ref progress) = active_ingestion_snapshot {
                            ui.label(egui::RichText::new(format!("⚡ WORKING ON TICKER: {}", progress.ticker)).heading());
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let stop_btn = egui::Button::new(egui::RichText::new("🛑 STOP").color(egui::Color32::WHITE))
                                .fill(egui::Color32::from_rgb(200, 40, 40));
                            if ui.add(stop_btn).clicked() {
                                let mut guard = backend::commands::downloader::ACTIVE_INGESTION.lock().unwrap();
                                *guard = None; // Instantly drops state context to flag thread cancellation loops
                            }
                        });
                    });

                    ui.add_space(35.0);
                    ui.label("WORKFLOW PIPELINE EXECUTION STEPS:");
                    ui.add_space(16.0);

                    if let Some(ref progress) = active_ingestion_snapshot {
                        // 1. Download Market Data Category Row Anchor
                        ui.label("1. Download Market Data");
                        ui.add_space(12.0);

                        // 📦 NSE PIPELINE NESTED DOWNLOAD LANES
                        if progress.nse_active {
                            ui.horizontal(|ui| {
                                ui.add_space(24.0);
                                ui.label(egui::RichText::new("NSE").strong().small());
                            });
                            ui.add_space(6.0);

                            // Auto-filter out 100% complete tasks to prevent screen congestion
                            let active_nse: Vec<&_> = progress.nse_downloads.iter()
                                .filter(|t| t.percentage < 100.0)
                                .collect();

                            if active_nse.is_empty() {
                                ui.horizontal(|ui| {
                                    ui.add_space(40.0);
                                    ui.label(egui::RichText::new("Waiting in queue / Synchronized").small().weak());
                                });
                                ui.add_space(14.0);
                            } else {
                                for track in active_nse {
                                    ui.horizontal(|ui| {
                                        ui.add_space(40.0);
                                        let display_name = if track.filename.is_empty() {
                                            track.current_api.clone()
                                        } else {
                                            format!("{} - {}", track.current_api, track.filename)
                                        };
                                        ui.label(egui::RichText::new(display_name).small().weak());
                                    });
                                    ui.add_space(4.0);

                                    ui.horizontal(|ui| {
                                        ui.add_space(40.0);
                                        let fraction = (track.percentage / 100.0).clamp(0.0, 1.0);
                                        let bar_width = ui.available_width() - 55.0;
                                        
                                        let (rect, _) = ui.allocate_at_least(egui::vec2(bar_width, 5.0), egui::Sense::hover());
                                        ui.painter().rect_filled(rect, 0.0, egui::Color32::from_rgb(25, 25, 25)); // Solid Black track
                                        
                                        if fraction > 0.0 {
                                            let mut progress_rect = rect;
                                            progress_rect.set_width(rect.width() * fraction);
                                            ui.painter().rect_filled(progress_rect, 0.0, egui::Color32::WHITE); // Solid White fill bar
                                        }

                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            ui.label(egui::RichText::new(format!("{:.1}%", track.percentage)).small().weak());
                                        });
                                    });
                                    ui.add_space(16.0); // Clean vertical breathing margins between items
                                }
                            }
                        }

                        // 📦 BSE PIPELINE NESTED DOWNLOAD LANES
                        if progress.bse_active {
                            ui.horizontal(|ui| {
                                ui.add_space(24.0);
                                ui.label(egui::RichText::new("BSE").strong().small());
                            });
                            ui.add_space(6.0);

                            // Auto-filter out 100% complete tasks to prevent screen congestion
                            let active_bse: Vec<&_> = progress.bse_downloads.iter()
                                .filter(|t| t.percentage < 100.0)
                                .collect();

                            if active_bse.is_empty() {
                                ui.horizontal(|ui| {
                                    ui.add_space(40.0);
                                    ui.label(egui::RichText::new("Waiting in queue / Synchronized").small().weak());
                                });
                                ui.add_space(14.0);
                            } else {
                                for track in active_bse {
                                    ui.horizontal(|ui| {
                                        ui.add_space(40.0);
                                        let display_name = if track.filename.is_empty() {
                                            track.current_api.clone()
                                        } else {
                                            format!("{} - {}", track.current_api, track.filename)
                                        };
                                        ui.label(egui::RichText::new(display_name).small().weak());
                                    });
                                    ui.add_space(4.0);

                                    ui.horizontal(|ui| {
                                        ui.add_space(40.0);
                                        let fraction = (track.percentage / 100.0).clamp(0.0, 1.0);
                                        let bar_width = ui.available_width() - 55.0;
                                        
                                        let (rect, _) = ui.allocate_at_least(egui::vec2(bar_width, 5.0), egui::Sense::hover());
                                        ui.painter().rect_filled(rect, 0.0, egui::Color32::from_rgb(25, 25, 25)); // Solid Black track
                                        
                                        if fraction > 0.0 {
                                            let mut progress_rect = rect;
                                            progress_rect.set_width(rect.width() * fraction);
                                            ui.painter().rect_filled(progress_rect, 0.0, egui::Color32::WHITE); // Solid White fill bar
                                        }

                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            ui.label(egui::RichText::new(format!("{:.1}%", track.percentage)).small().weak());
                                        });
                                    });
                                    ui.add_space(16.0);
                                }
                            }
                        }
                    }

                    // Standalone inactive pipeline mockups padded cleanly below download tracks
                    ui.add_space(14.0);
                    ui.add_enabled_ui(false, |ui| {
                        ui.checkbox(&mut false, "2. Parse Market Structures");
                    });

                    ui.add_space(24.0);
                    ui.add_enabled_ui(false, |ui| {
                        ui.checkbox(&mut false, "3. Core Financial Analysis");
                    });
                });
            });
        });
    } else if is_done {
        // ============================================================================
        // 🎉 COMPLETION VIEW: Success feedback step
        // ============================================================================
        ui.vertical_centered(|ui| {
            ui.add_space(80.0);
            if let Some(ref progress) = active_ingestion_snapshot {
                ui.label(egui::RichText::new(format!("🎉 INGESTION COMPLETE FOR {}", progress.ticker)).heading());
            }
            ui.add_space(20.0);
            ui.label("All requested exchange datasets have been parsed, cached, and saved cleanly.");
            ui.add_space(40.0);
            
            if ui.button(" 📑 CLEAR AND FINALIZE WORKFLOW ").clicked() {
                let mut guard = backend::commands::downloader::ACTIVE_INGESTION.lock().unwrap();
                *guard = None;
            }
        });
    } else {
        // ============================================================================
        // ➕ ENTRY VIEW: Standard ticker selector input form layout
        // ============================================================================
        ui.vertical_centered(|ui| {
            ui.add_space(60.0);
            ui.heading("➕ ADD NEW TICKER");
            ui.add_space(25.0);

            let input_id = egui::Id::new("add_ticker_input_buffer_field");
            let step1_id = egui::Id::new("add_ticker_step1_download");
            let nse_id = egui::Id::new("add_ticker_nse_checked");
            let bse_id = egui::Id::new("add_ticker_bse_checked");

            let mut input_buffer = ui.data_mut(|d| d.get_temp::<String>(input_id).unwrap_or_default());

            let text_edit = egui::TextEdit::singleline(&mut input_buffer)
                .hint_text("Enter Symbol (e.g. OIL, RELIANCE, TATA)")
                .desired_width(320.0);
            
            if ui.add(text_edit).changed() {
                ui.data_mut(|d| d.insert_temp(input_id, input_buffer.clone()));
            }

            ui.add_space(35.0);

            ui.allocate_ui(egui::vec2(480.0, 0.0), |ui| {
                ui.vertical(|ui| {
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
                            if ui.checkbox(&mut nse_checked, "NSE").changed() {
                                ui.data_mut(|d| d.insert_temp(nse_id, nse_checked));
                            }
                            ui.add_space(24.0);
                            if ui.checkbox(&mut bse_checked, "BSE").changed() {
                                ui.data_mut(|d| d.insert_temp(bse_id, bse_checked));
                            }
                        });
                    }

                    ui.add_space(28.0); 
                    ui.add_enabled_ui(false, |ui| {
                        ui.checkbox(&mut false, "2. Parse Market Structures");
                    });

                    ui.add_space(28.0);
                    ui.add_enabled_ui(false, |ui| {
                        ui.checkbox(&mut false, "3. Core Financial Analysis");
                    });
                });
            });

            ui.add_space(45.0);
            
            if ui.button(" ⚡ INGEST TICKER ").clicked() {
                let ticker_symbol = input_buffer.trim().to_uppercase();
                if !ticker_symbol.is_empty() {
                    let step1_download = ui.data_mut(|d| d.get_temp::<bool>(step1_id).unwrap_or(true));
                    
                    if step1_download {
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
                        
                        tokio::spawn(async move {
                            let _ = crate::core::downloader::dispatch_download(args).await;
                        });
                    }
                    
                    input_buffer.clear();
                    ui.data_mut(|d| d.insert_temp(input_id, input_buffer.clone()));
                }
            }
        });
    }
}