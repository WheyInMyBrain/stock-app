// stock-app/ui/frontend_native/src/main.rs
use eframe::egui;
use std::time::Duration;
use tokio::sync::mpsc;

mod ui;
mod core; 

struct TerminalApplicationCore {
    sidebar_open: bool,
    active_ticker: String,
    data_dir: Option<String>,
    data_dir_input: String,
    available_tickers: Vec<String>,
    daemon_spawned: bool,
    ticker_update_rx: mpsc::Receiver<Vec<String>>,
}

impl TerminalApplicationCore {
    pub fn new(cc: &eframe::CreationContext<'_>, ticker_update_rx: mpsc::Receiver<Vec<String>>) -> Self {
        // Enforce reactive UI context theme modifications immediately on initialization
        ui::theme::apply_high_contrast_theme(&cc.egui_ctx);

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
            ticker_update_rx,
        }
    }
}

impl eframe::App for TerminalApplicationCore {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 1. ASYNC UPDATE RECEIVER REACTION GATES
        if let Ok(fresh_tickers) = self.ticker_update_rx.try_recv() {
            if self.available_tickers != fresh_tickers {
                self.available_tickers = fresh_tickers;
                ctx.request_repaint();
            }
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
    }
}

fn main() -> eframe::Result<()> {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _guard = runtime.enter();

    // Setup an asynchronous multi-producer single-consumer thread communication channel
    let (tx, rx) = mpsc::channel::<Vec<String>>(4);
    let egui_ctx_framer = egui::Context::default();
    let thread_ctx = egui_ctx_framer.clone();

    // =========================================================================
    // HIGH-PERFORMANCE BACKGROUND SCHEDULER (NON-BLOCKING IO WORKER)
    // =========================================================================
    runtime.spawn(async move {
        let mut loop_timer = tokio::time::interval(Duration::from_secs(2));
        
        let mut last_known_tickers: Vec<String> = Vec::new();
        
        loop {
            loop_timer.tick().await;
            
            let fresh_tickers = core::data_manager::DataManager::load_active_tickers();
            
            if fresh_tickers != last_known_tickers {
                last_known_tickers = fresh_tickers.clone();
                if tx.send(fresh_tickers).await.is_err() {
                    break; 
                }
                thread_ctx.request_repaint();
            }
        }
    });

    let mut native_options = eframe::NativeOptions::default();
    native_options.viewport = egui::ViewportBuilder::default()
        .with_title("Stock App")
        .with_inner_size([1100.0, 700.0]);

    native_options.renderer = eframe::Renderer::Wgpu;

    eframe::run_native(
        "NativeTradingTerminal",
        native_options,
        Box::new(move |cc| {
            let app_context = TerminalApplicationCore::new(cc, rx);
            Box::new(app_context)
        }),
    )
}