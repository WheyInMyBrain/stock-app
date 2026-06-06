// stock-app/ui/backend/src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // 🎯 Boots the Tauri application directly via your shared library layer!
    backend::run(); 
}