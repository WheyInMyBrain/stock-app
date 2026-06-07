// stock-app/ui/frontend_native/src/ui/panels/add_ticker.rs
use egui::Ui;

pub fn draw_add_ticker_panel(ui: &mut Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(100.0);
        ui.heading(" ➕ ADD NEW TICKER ");
        ui.add_space(15.0);
        
        ui.label("ENTER A NEW STOCK SYMBOL TO DOWNLOAD MARKET PIPELINE DATA:");
        ui.add_space(15.0);

        // 1. Ticker Input Field Buffer
        let input_id = ui.id().with("add_ticker_input_buffer_field");
        let mut input_buffer = ui.data_mut(|d| d.get_temp::<String>(input_id).unwrap_or_default());

        let total_width = ui.available_width();
        ui.horizontal(|ui| {
            ui.add_space(total_width * 0.25); 
            
            let text_edit = egui::TextEdit::singleline(&mut input_buffer)
                .hint_text("e.g. OIL, RELIANCE, TATA")
                .desired_width(total_width * 0.50);
            
            if ui.add(text_edit).changed() {
                ui.data_mut(|d| d.insert_temp(input_id, input_buffer.clone()));
            }
        });

        ui.add_space(25.0);
        ui.label("WORKFLOW PIPELINE EXECUTION STEPS:");
        ui.add_space(8.0);

        // 2. Modular Step Layout System (Self-Contained temporary context slots)
        let step1_id = ui.id().with("add_ticker_step1_download");
        // Defaults to true so the system runs out of the box
        let mut step1_download = ui.data_mut(|d| d.get_temp::<bool>(step1_id).unwrap_or(true));

        ui.horizontal(|ui| {
            let space_width = ui.available_width();
            ui.add_space(space_width * 0.36); // Center the step layout alignment nicely
            
            if ui.checkbox(&mut step1_download, " Step 1: Download Market Data ").changed() {
                ui.data_mut(|d| d.insert_temp(step1_id, step1_download));
            }
        });

        // 3. Exchange Checkboxes (Nested inside Step 1 Download state scope)
        let nse_id = ui.id().with("add_ticker_nse_checked");
        let mut nse_checked = ui.data_mut(|d| d.get_temp::<bool>(nse_id).unwrap_or(false));

        let bse_id = ui.id().with("add_ticker_bse_checked");
        let mut bse_checked = ui.data_mut(|d| d.get_temp::<bool>(bse_id).unwrap_or(false));

        // 🎯 NESTING CONDITION: Only display exchange context checkboxes if Step 1 is actively enabled
        if step1_download {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let space_width = ui.available_width();
                ui.add_space(space_width * 0.42); // Align and center checkboxes cleanly under the step label
                
                if ui.checkbox(&mut nse_checked, " NSE ").changed() {
                    ui.data_mut(|d| d.insert_temp(nse_id, nse_checked));
                }
                ui.add_space(25.0);
                if ui.checkbox(&mut bse_checked, " BSE ").changed() {
                    ui.data_mut(|d| d.insert_temp(bse_id, bse_checked));
                }
            });
        }

        // 💡 FUTURE STEPS ACCORDION DOCK:
        // You can drop in Step 2, Step 3, etc. below by mimicking the setup block above:
        //
        // let step2_id = ui.id().with("add_ticker_step2_analysis");
        // let mut step2_analysis = ui.data_mut(|d| d.get_temp::<bool>(step2_id).unwrap_or(false));
        // ui.checkbox(&mut step2_analysis, " Step 2: Core Financial Analysis ");

        ui.add_space(30.0);
        
        // 4. Ingestion Action Trigger
        if ui.button(" ⚡ INGEST TICKER ").clicked() {
            let ticker_symbol = input_buffer.trim().to_uppercase();
            if !ticker_symbol.is_empty() {
                
                // 🎯 EXECUTION STEPS GATEKEEPER:
                // Only dispatches the network download task if Step 1 is explicitly checked.
                if step1_download {
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

                // 💡 FUTURE ROUTING FOR STEP 2, STEP 3, ETC:
                // if step2_analysis {
                //     let ticker_clone = ticker_symbol.clone();
                //     tokio::spawn(async move { crate::core::analysis::run_calculations(ticker_clone).await; });
                // }
                
                // Reset text buffer upon dispatch completion pass
                input_buffer.clear();
                ui.data_mut(|d| d.insert_temp(input_id, input_buffer.clone()));
            }
        }
    });
}