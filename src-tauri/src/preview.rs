use libobs_wrapper::display::{ObsDisplayCreationData, WindowPositionTrait};
use tauri::{AppHandle, Manager};

use crate::state::CURR_STATE;

#[tauri::command]
pub async fn add_preview(handle: AppHandle, x: u32, y: u32, width: u32, height: u32) -> String {
    // First check if we already have a display or if OBS isn't initialized
    let state = CURR_STATE.read().await;

    if state.obs_display.is_some() {
        return "Display already exists".to_string();
    }

    if state.obs_context.is_none() {
        return "OBS runtime is not initialized".to_string();
    }

    // Release read lock before acquiring write lock to avoid deadlock
    drop(state);
    let mut state = CURR_STATE.write().await;

    // Clone the context to avoid holding the lock during async operations
    // This prevents potential deadlocks when awaiting OBS operations
    // clone the Option<ObsContext> directly
    let ctx = state.obs_context.clone();

    // Create the OBS display using the cloned context
    let result = if let Some(mut ctx) = ctx {
        // Get the native window handle from Tauri's webview
        // OBS displays need to be parented to a native window
        let window = handle.get_webview_window("main").unwrap();
        let native_handle = window.hwnd().unwrap().0 as isize;

        // Create display configuration
        // ObsDisplayCreationData contains parameters needed for OBS to create a display
        // in a specified window area
        let display_config =
            ObsDisplayCreationData::new(native_handle, x as i32, y as i32, width, height);

        // Request OBS to create a display with our configuration
        // The display will render the current OBS scene graph at the specified position
        match ctx.display(display_config) {
            Ok(display) => {
                // Store the created display in our state
                state.obs_display = Some(display);
                format!("Display created at ({}, {}, {}, {})", x, y, width, height)
            }
            Err(e) => {
                format!("Failed to create OBS display: {}", e)
            }
        }
    } else {
        "OBS runtime is not initialized".to_string()
    };

    result
}

#[tauri::command]
pub async fn resize_preview(x: i32, y: i32, width: u32, height: u32) -> String {
    let display = CURR_STATE.read().await.obs_display.clone();

    if let Some(display) = display {
        display.set_size(width, height).unwrap();
        display.set_pos(x, y).unwrap();

        format!("Display resized to ({}, {}, {}, {})", x, y, width, height)
    } else {
        "Display is not initialized".to_string()
    }
}

#[tauri::command]
pub async fn close_preview() -> String {
    let state = CURR_STATE.read().await;
    if state.obs_display.is_none() {
        return "Display is not initialized".to_string();
    }

    drop(state);
    let mut state = CURR_STATE.write().await;
    let display = state.obs_display.take().unwrap();
    let ctx = state.obs_context.clone();

    if ctx.is_none() {
        return "OBS runtime is not initialized".to_string();
    }

    let mut ctx = ctx.unwrap();
    ctx.remove_display(&display).unwrap();

    "Display closed".to_string()
}
