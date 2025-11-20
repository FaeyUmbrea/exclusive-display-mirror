// Capture source management for OBS
use libobs_sources::windows::{
    GameCaptureSourceBuilder, MonitorCaptureSourceBuilder, ObsGameCaptureMode,
    ObsGameCaptureRgbaSpace, ObsHookRate, ObsWindowCaptureMethod, ObsWindowPriority,
    WindowCaptureSourceBuilder, WindowSearchMode,
};
use libobs_sources::ObsSourceBuilder;
use libobs_wrapper::context::ObsContext;
use libobs_wrapper::scenes::ObsSceneRef;
use libobs_wrapper::sources::ObsSourceRef;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

lazy_static::lazy_static! {
    static ref ACTIVE_CAPTURE_SOURCE: Arc<Mutex<Option<ObsSourceRef>>> = Arc::new(Mutex::new(None));
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum CaptureSourceType {
    Monitor {
        monitor_id: String,
        monitor_name: String,
        force_sdr: bool,
    },
    Window {
        window_title: String,
        window_class: String,
        executable: String,
        force_sdr: bool,
    },
    Game {
        window_title: String,
        window_class: String,
        executable: String,
        color_space: String, // "srgb" or "2100pq"
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureSourceInfo {
    pub id: String,
    pub name: String,
    pub source_type: CaptureSourceType,
}

/// List available monitors for capture
#[tauri::command]
pub async fn list_monitors() -> Result<Vec<CaptureSourceInfo>, String> {
    let monitors = MonitorCaptureSourceBuilder::get_monitors()
        .map_err(|e| format!("Failed to get monitors: {}", e))?;

    Ok(monitors
        .iter()
        .enumerate()
        .map(|(i, monitor)| {
            let monitor_name = monitor.0.friendly_name.clone();
            CaptureSourceInfo {
                id: format!("monitor_{}", i),
                name: monitor_name.clone(),
                source_type: CaptureSourceType::Monitor {
                    monitor_id: i.to_string(),
                    monitor_name,
                    force_sdr: true,
                },
            }
        })
        .collect())
}

/// List available windows for capture
#[tauri::command]
pub async fn list_windows() -> Result<Vec<CaptureSourceInfo>, String> {
    // Get list of capturable windows from OBS
    // Use IncludeMinimized to get all windows including minimized ones
    let windows = WindowCaptureSourceBuilder::get_windows(WindowSearchMode::IncludeMinimized)
        .map_err(|e| format!("Failed to get windows: {}", e))?;

    Ok(windows
        .iter()
        .enumerate()
        .filter_map(|(i, window)| {
            let window_str = window.0.title.clone().unwrap_or(window.0.obs_id.clone());

            // Try to extract useful information from the debug string
            // WindowInfo typically contains: title, class, executable
            // Skip empty or system windows
            if window_str.contains("\"\"") || window_str.is_empty() {
                return None;
            }

            // Create a user-friendly name from the window info
            let name = window_str
                .trim_start_matches("Sendable { inner: WindowInfo { ")
                .trim_end_matches(" } }")
                .to_string();

            Some(CaptureSourceInfo {
                id: format!("window_{}", i),
                name: name.clone(),
                source_type: CaptureSourceType::Window {
                    window_title: name.clone(),
                    window_class: String::new(),
                    executable: String::new(),
                    force_sdr: true,
                },
            })
        })
        .collect())
}

/// Set the active capture source
#[tauri::command]
pub async fn set_capture_source(source_info: CaptureSourceInfo) -> Result<String, String> {
    use crate::state::CURR_STATE;

    // Get a cloned ObsContext from state (avoid holding a mutable borrow across scene access)
    let state_read = CURR_STATE.read().await;
    let mut context = state_read
        .obs_context
        .as_ref()
        .ok_or("OBS not initialized")?
        .clone();
    drop(state_read);

    // Acquire state write lock to get mutable scene reference and update obs_source later
    let mut state = CURR_STATE.write().await;
    let scene_ref = state
        .obs_scene
        .as_mut()
        .ok_or("OBS scene not initialized")?;

    {
        let guard = &mut ACTIVE_CAPTURE_SOURCE.lock().map_err(|e| format!("{}", e))?;
        let source_option = guard.take();

        if let Some(source) = source_option {
            log::info!("Removing current source");
            scene_ref
                .remove_source(&source)
                .map_err(|e| format!("{}", e))?;
            log::info!("Removed current source");
        }
    }

    // Create new source based on type
    let new_source = match &source_info.source_type {
        CaptureSourceType::Monitor {
            monitor_name,
            force_sdr,
            ..
        } => {
            log::info!(
                "Creating monitor capture source: {} (Force SDR: {})",
                monitor_name,
                force_sdr
            );
            create_monitor_source(&mut context, scene_ref, monitor_name, *force_sdr)?
        }
        CaptureSourceType::Window {
            window_title,
            window_class,
            executable,
            force_sdr,
        } => {
            log::info!(
                "Creating window capture source: {} ({}) (Force SDR: {})",
                window_title,
                executable,
                force_sdr
            );
            create_window_source(
                &mut context,
                scene_ref,
                window_title,
                window_class,
                executable,
                *force_sdr,
            )?
        }
        CaptureSourceType::Game {
            window_title,
            window_class,
            executable,
            color_space,
        } => {
            log::info!(
                "Creating game capture source: {} ({}) with color space: {}",
                window_title,
                executable,
                color_space
            );
            create_game_source(
                &mut context,
                scene_ref,
                window_title,
                window_class,
                executable,
                color_space,
            )?
        }
    };

    // Store the new source
    {
        let mut active_source = ACTIVE_CAPTURE_SOURCE
            .lock()
            .map_err(|e| format!("Failed to lock active source: {}", e))?;
        *active_source = Some(new_source.clone());
    }

    // Update state
    state.obs_source = Some(new_source);

    Ok(format!("✓ Capture source set to: {}", source_info.name))
}

fn create_monitor_source(
    context: &mut ObsContext,
    scene: &mut ObsSceneRef,
    monitor_name: &str,
    force_sdr: bool,
) -> Result<ObsSourceRef, String> {
    let monitors = MonitorCaptureSourceBuilder::get_monitors()
        .map_err(|e| format!("Failed to get monitors: {}", e))?;

    let monitor_index = monitors
        .iter()
        .position(|m| m.0.friendly_name == monitor_name)
        .ok_or_else(|| format!("Monitor not found: {}", monitor_name))?;

    let source = context
        .source_builder::<MonitorCaptureSourceBuilder, _>("Monitor Capture")
        .map_err(|e| format!("Failed to create source builder: {}", e))?
        .set_monitor(&monitors[monitor_index])
        .set_force_sdr(force_sdr)
        .add_to_scene(scene)
        .map_err(|e| format!("Failed to add source to scene: {}", e))?;

    Ok(source)
}

fn create_window_source(
    context: &mut ObsContext,
    scene: &mut ObsSceneRef,
    window_title: &str,
    _window_class: &str,
    _executable: &str,
    force_sdr: bool,
) -> Result<ObsSourceRef, String> {
    let windows = WindowCaptureSourceBuilder::get_windows(WindowSearchMode::IncludeMinimized)
        .map_err(|e| format!("Failed to get windows: {}", e))?;

    // Find the window by matching the title in the debug string
    let window_index = windows
        .iter()
        .position(|w| {
            let window_str = w.0.title.clone().unwrap_or(w.0.obs_id.clone());
            window_str.contains(window_title)
        })
        .ok_or_else(|| format!("Window not found: {}", window_title))?;

    // Create window capture source with the selected window
    let source = context
        .source_builder::<WindowCaptureSourceBuilder, _>("Window Capture")
        .map_err(|e| format!("Failed to create source builder: {}", e))?
        .set_window(&windows[window_index])
        .set_priority(ObsWindowPriority::Title)
        .set_capture_method(ObsWindowCaptureMethod::MethodWgc)
        .set_force_sdr(force_sdr)
        .add_to_scene(scene)
        .map_err(|e| format!("Failed to add source to scene: {}", e))?;

    Ok(source)
}

fn create_game_source(
    context: &mut ObsContext,
    scene: &mut ObsSceneRef,
    window_title: &str,
    _window_class: &str,
    _executable: &str,
    color_space: &str,
) -> Result<ObsSourceRef, String> {
    let windows = WindowCaptureSourceBuilder::get_windows(WindowSearchMode::IncludeMinimized)
        .map_err(|e| format!("Failed to get windows: {}", e))?;

    // Find the window by matching the title
    let window_index = windows
        .iter()
        .position(|w| {
            let window_str = w.0.title.clone().unwrap_or(w.0.obs_id.clone());
            window_str.contains(window_title)
        })
        .ok_or_else(|| format!("Window not found: {}", window_title))?;

    // Use the provided mutable scene reference

    // Parse color space string to enum
    let rgba_space = match color_space {
        "2100pq" => ObsGameCaptureRgbaSpace::RGBA2100pq,
        _ => ObsGameCaptureRgbaSpace::SRgb, // Default to sRGB
    };

    // Create game capture source with the selected window and color space
    let source = context
        .source_builder::<GameCaptureSourceBuilder, _>("Game Capture")
        .map_err(|e| format!("Failed to create source builder: {}", e))?
        .set_capture_mode(ObsGameCaptureMode::CaptureSpecificWindow)
        .set_window(&windows[window_index].0)
        .set_capture_cursor(true)
        .set_anti_cheat_hook(true)
        .set_hook_rate(ObsHookRate::Normal)
        .set_priority(ObsWindowPriority::Executable)
        .set_rgb10a2_space(rgba_space)
        .add_to_scene(scene)
        .map_err(|e| format!("Failed to add source to scene: {}", e))?;

    log::info!(
        "Game capture source created with color space: {:?}",
        rgba_space
    );

    Ok(source)
}

/// Get current capture source info
#[tauri::command]
pub async fn get_current_source() -> Option<String> {
    let active_source = ACTIVE_CAPTURE_SOURCE.lock().ok()?;
    active_source.as_ref().map(|_source| {
        // Get source name or type info
        "Active source".to_string() // TODO: Store more metadata
    })
}
