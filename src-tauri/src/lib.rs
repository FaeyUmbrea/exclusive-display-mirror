mod bootstrapper;
mod capture_source;
mod monitor;
mod obs;
mod preview;
mod state;

#[cfg(target_os = "windows")]
pub mod windows;

pub use capture_source::{get_current_source, list_monitors, list_windows, set_capture_source};
pub use monitor::{switch_monitor, MonitorInfo};
pub use obs::initialize_obs;
pub use preview::{add_preview, close_preview, resize_preview};
pub use state::{CurrState, CURR_STATE};

#[cfg(target_os = "windows")]
pub use windows::{
    apply_display_configuration, cancel_display_configuration, get_detached_monitors,
    initialize_display_configuration, start_renderer, stop_renderer,
};

use crate::bootstrapper::{
    check_bootstrap_manifest, launch_bootstrapper_downloader, run_bootstrapper_with_pid,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![
            add_preview,
            initialize_obs,
            resize_preview,
            close_preview,
            switch_monitor,
            list_monitors,
            list_windows,
            set_capture_source,
            run_bootstrapper_with_pid,
            check_bootstrap_manifest,
            launch_bootstrapper_downloader,
            get_current_source,
            // Only register the command on Windows builds; generate_handler will ignore if not present
            #[cfg(target_os = "windows")]
            get_detached_monitors,
            #[cfg(target_os = "windows")]
            start_renderer,
            #[cfg(target_os = "windows")]
            stop_renderer,
            #[cfg(target_os = "windows")]
            initialize_display_configuration,
            #[cfg(target_os = "windows")]
            apply_display_configuration,
            #[cfg(target_os = "windows")]
            cancel_display_configuration,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
