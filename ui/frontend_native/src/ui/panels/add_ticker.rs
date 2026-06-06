// stock-app/ui/frontend_native/src/ui/panels/add_ticker.rs
use egui::Ui;

pub fn draw_add_ticker_panel(ui: &mut Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(100.0);
        ui.heading(" ➕ ADD NEW TICKER ");
        ui.add_space(15.0);
        
        ui.label("ENTER A NEW STOCK SYMBOL TO DOWNLOAD MARKET PIPELINE DATA:");
        ui.add_space(15.0);

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

        ui.add_space(18.0);
        ui.label("SELECT EXCHANGE TARGET CONTEXTS (OPTIONAL):");
        ui.add_space(8.0);

        let nse_id = ui.id().with("add_ticker_nse_checked");
        let mut nse_checked = ui.data_mut(|d| d.get_temp::<bool>(nse_id).unwrap_or(false));

        let bse_id = ui.id().with("add_ticker_bse_checked");
        let mut bse_checked = ui.data_mut(|d| d.get_temp::<bool>(bse_id).unwrap_or(false));

        ui.horizontal(|ui| {
            let space_width = ui.available_width();
            ui.add_space(space_width * 0.42); 
            
            if ui.checkbox(&mut nse_checked, " NSE ").changed() {
                ui.data_mut(|d| d.insert_temp(nse_id, nse_checked));
            }
            ui.add_space(25.0);
            if ui.checkbox(&mut bse_checked, " BSE ").changed() {
                ui.data_mut(|d| d.insert_temp(bse_id, bse_checked));
            }
        });

        ui.add_space(30.0);
        
        if ui.button(" ⚡ INGEST TICKER ").clicked() {
            let ticker_symbol = input_buffer.trim().to_uppercase();
            if !ticker_symbol.is_empty() {
                let mut args = vec![ticker_symbol];
                
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
                
                input_buffer.clear();
                ui.data_mut(|d| d.insert_temp(input_id, input_buffer.clone()));
            }
        }
    });
}