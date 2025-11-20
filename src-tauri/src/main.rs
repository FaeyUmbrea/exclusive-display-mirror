// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Configure env_logger with a default filter if RUST_LOG is not set
    // This ensures we see our application logs (info and above) in the console
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("tauri_app=debug,warn"),
    )
    .init();

    tauri_app_lib::run()
}
