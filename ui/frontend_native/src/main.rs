// stock-app/ui/frontend_native/src/main.rs
use eframe::egui;
use std::time::{Instant, Duration};

mod ui;
mod core; 

struct TerminalApplicationCore {
    sidebar_open: bool,
    active_ticker: String,
    data_dir: Option<String>,
    data_dir_input: String,
    available_tickers: Vec<String>,
    daemon_spawned: bool,
    last_sync_time: Instant,
}

impl Default for TerminalApplicationCore {
    fn default() -> Self {
        let mut data_dir = None;
        let mut available_tickers = Vec::new();
        let mut daemon_spawned = false;

        core::lifecycle::AppLifecycleManager::initialize_system(
            &mut data_dir,
            &mut available_tickers,
            &mut daemon_spawned,
        );

        Self {
            sidebar_open: true,
            active_ticker: String::new(), 
            data_dir,
            data_dir_input: String::new(),
            available_tickers,
            daemon_spawned,
            last_sync_time: Instant::now(), // Initialize timer active
        }
    }
}

impl eframe::App for TerminalApplicationCore {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ui::theme::apply_high_contrast_theme(ctx);

        // 🎯 LIVE SYNC INTERCEPTOR:
        // Periodically scans the data folder every 2 seconds in the background.
        // As soon as the Go downloader finishes writing files, the stock pops up instantly!
        if self.data_dir.is_some() && self.last_sync_time.elapsed() >= Duration::from_secs(2) {
            self.available_tickers = core::data_manager::DataManager::load_active_tickers();
            self.last_sync_time = Instant::now();
        }

        if self.data_dir.is_none() {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui::setup::draw_setup_view(ui, &mut self.data_dir_input, &mut self.data_dir);
            });
            
            core::lifecycle::AppLifecycleManager::complete_first_launch_setup(
                &mut self.data_dir,
                &mut self.available_tickers,
                &mut self.daemon_spawned,
                &mut self.active_ticker,
            );
        } else {
            egui::SidePanel::left("left_navigation_dock")
                .resizable(false)
                .default_width(200.0)
                .show_animated(ctx, self.sidebar_open, |ui| {
                    ui::sidebar::draw_retractable_sidebar(
                        ui, 
                        &mut self.sidebar_open, 
                        &mut self.active_ticker,
                        &self.available_tickers,
                    );
                });

            egui::CentralPanel::default().show(ctx, |ui| {
                ui::workspace::draw_main_workspace_viewport(
                    ui, 
                    &mut self.sidebar_open, 
                    &self.active_ticker,
                );
            });
        }

        ctx.request_repaint();
    }
}

fn main() -> eframe::Result<()> {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _guard = runtime.enter();

    let mut native_options = eframe::NativeOptions::default();
    native_options.viewport = egui::ViewportBuilder::default()
        .with_title("High-Speed Financial Terminal Canvas")
        .with_inner_size([1100.0, 700.0]);

    eframe::run_native(
        "NativeTradingTerminal",
        native_options,
        Box::new(|_cc| Box::new(TerminalApplicationCore::default())),
    )
}