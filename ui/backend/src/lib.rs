// stock-app/ui/backend/src/lib.rs
pub mod commands;
pub mod pipeline;

/// 🚀 ARCHITECTURE UNIFIED INITIALIZER FOR PURE RUST DESKTOP NATIVE CORE
/// Boots up all background worker engines seamlessly under a single process execution frame.
pub fn initialize_backend() {
    // 1. Fire up your Go sidecar persistent downloader daemon natively
    commands::downloader::initialize_go_daemon();
    
    // 2. Fire up your global interval clock ticker tracking thread
    commands::ticker::spawn_global_ticker_daemon();
    
    println!("📡 [BACKEND SYSTEM CORE]: All headless background pipelines are fully engaged.");
}