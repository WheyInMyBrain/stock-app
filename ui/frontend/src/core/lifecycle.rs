// stock-app/ui/frontend_native/src/core/lifecycle.rs
use crate::ui::setup::check_existing_config;
use crate::core::data_manager::DataManager;
use crate::core::downloader::boot_daemon;

pub struct AppLifecycleManager;

impl AppLifecycleManager {
    /// 🚀 Handles system state configuration on application startup
    pub fn initialize_system(
        data_dir: &mut Option<String>,
        available_tickers: &mut Vec<String>,
        daemon_spawned: &mut bool,
    ) {
        let existing_path = check_existing_config();
        
        if let Some(path) = existing_path {
            *data_dir = Some(path.clone());
            
            // 🎯 FIXED: Forwarded the path argument to match the signature
            boot_daemon(path);
            *daemon_spawned = true;
            
            // Sync directory layout names into memory
            *available_tickers = DataManager::load_active_tickers();
        }
    }

    /// 📥 Transition Catcher: Boots the daemon instantly the moment first-launch setup finishes
    pub fn complete_first_launch_setup(
        data_dir: &mut Option<String>,
        available_tickers: &mut Vec<String>,
        daemon_spawned: &mut bool,
        active_ticker: &mut String,
    ) {
        if let Some(ref path) = *data_dir {
            if !*daemon_spawned {
                // 🎯 FIXED: Forwarded the path argument to match the signature
                boot_daemon(path.clone());
                *daemon_spawned = true;
                
                *available_tickers = DataManager::load_active_tickers();
                
                // Automatically select the first available folder name
                if let Some(first) = available_tickers.first() {
                    *active_ticker = first.clone();
                }
            }
        }
    }
}