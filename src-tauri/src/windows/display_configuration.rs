// display_configuration.rs
//
// Module for managing the multi-step display configuration flow.
// This allows the UI to guide users through selecting output device, resolution,
// and refresh rate before activating the display.

use super::display_iterable::DisplayTargetIterable;
use crate::monitor::MonitorInfo;
use std::sync::Mutex;
use windows::Devices::Display::Core::*;
use windows_collections::IIterable;

lazy_static::lazy_static! {
    /// Stores the configuration state between UI steps
    static ref CONFIG_STATE: Mutex<Option<ConfigurationState>> = Mutex::new(None);
}

/// Represents the state of a display configuration in progress
pub struct ConfigurationState {
    pub modifiable_state: DisplayState,
    pub path: DisplayPath,
    pub available_modes: Vec<DisplayModeInfo>,
}

/// Information about a display mode for the UI
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct DisplayModeInfo {
    pub width: u32,
    pub height: u32,
    pub refresh_rate: u32,
    pub index: usize,
}

/// Step 1: Initialize display configuration up to the point of mode selection
#[tauri::command]
pub fn initialize_display_configuration(
    monitor: MonitorInfo,
) -> std::result::Result<Vec<DisplayModeInfo>, String> {
    log::info!(
        "display config: Initializing configuration for monitor: {}",
        monitor.name
    );

    // Create DisplayManager
    let display_manager = DisplayManager::Create(DisplayManagerOptions::None)
        .map_err(|e| format!("Failed to create DisplayManager: {}", e))?;

    log::info!("display config: DisplayManager created");

    // Find matching DisplayTarget
    let targets = display_manager
        .GetCurrentTargets()
        .map_err(|e| format!("Failed to get display targets: {}", e))?;

    let count = targets
        .Size()
        .map_err(|e| format!("Failed to get target count: {}", e))?;

    log::info!("display config: Found {} display targets", count);

    let mut display_target: Option<DisplayTarget> = None;
    for i in 0..count {
        let target = targets
            .GetAt(i)
            .map_err(|e| format!("Failed to get target at index {}: {}", i, e))?;

        let stable_id = target
            .StableMonitorId()
            .map_err(|e| format!("Failed to get stable monitor ID: {}", e))?;

        // Match by stable_id
        if let Some(ref target_stable_id) = monitor.stable_id {
            if stable_id == target_stable_id {
                log::info!(
                    "display config: Matched target by stable_id: {}",
                    target_stable_id
                );
                display_target = Some(target);
                break;
            }
        }
    }

    let display_target =
        display_target.ok_or_else(|| "No matching display target found".to_string())?;

    // Check the usage kind
    let usage_kind = display_target
        .UsageKind()
        .map_err(|e| format!("Failed to get usage kind: {}", e))?;

    log::info!("display config: Target UsageKind={:?}", usage_kind);

    // Create IIterable for the target
    let targets_iterable: IIterable<DisplayTarget> =
        DisplayTargetIterable::from_single(display_target.clone()).into();

    // Acquire targets and create empty modifiable state
    log::info!("display config: Acquiring target and creating modifiable state...");
    let acquire_result = display_manager
        .TryAcquireTargetsAndCreateEmptyState(&targets_iterable)
        .map_err(|e| format!("Failed to acquire targets: {}", e))?;

    if acquire_result
        .ErrorCode()
        .map_err(|e| format!("Failed to get error code: {}", e))?
        != DisplayManagerResult::Success
    {
        let err_code = acquire_result
            .ErrorCode()
            .map_err(|e| format!("Failed to get error code: {}", e))?;
        log::error!("display config: Failed to acquire target: {:?}", err_code);
        return Err(format!("Failed to acquire target: {:?}", err_code));
    }

    let modifiable_state = acquire_result
        .State()
        .map_err(|e| format!("Failed to get modifiable state: {}", e))?;

    log::info!("display config: [OK] Created modifiable state");

    // Connect the target to create an active display path
    log::info!("display config: Connecting target...");
    let path = modifiable_state
        .ConnectTarget(&display_target)
        .map_err(|e| format!("Failed to connect target: {}", e))?;

    log::info!("display config: [OK] Connected display path");

    // Find available display modes
    log::info!("display config: Finding display modes...");
    let modes = path
        .FindModes(DisplayModeQueryOptions::None)
        .map_err(|e| format!("Failed to find display modes: {}", e))?;

    let mode_count = modes
        .Size()
        .map_err(|e| format!("Failed to get mode count: {}", e))?;

    if mode_count == 0 {
        log::error!("display config: No display modes available for target");
        return Err("No display modes available".to_string());
    }

    log::info!(
        "display config: Found {} available display modes",
        mode_count
    );

    // Extract mode information for the UI
    let mut available_modes = Vec::new();
    for i in 0..mode_count {
        let mode = modes
            .GetAt(i)
            .map_err(|e| format!("Failed to get mode at index {}: {}", i, e))?;

        let source_res = mode
            .SourceResolution()
            .map_err(|e| format!("Failed to get source resolution: {}", e))?;

        let present_rate = mode
            .PresentationRate()
            .map_err(|e| format!("Failed to get presentation rate: {}", e))?;

        let refresh_rate = if present_rate.VerticalSyncRate.Denominator > 0 {
            present_rate.VerticalSyncRate.Numerator / present_rate.VerticalSyncRate.Denominator
        } else {
            60
        };

        available_modes.push(DisplayModeInfo {
            width: source_res.Width as u32,
            height: source_res.Height as u32,
            refresh_rate,
            index: i as usize,
        });
    }

    // Store the state for the next step
    let state = ConfigurationState {
        modifiable_state,
        path,
        available_modes: available_modes.clone(),
    };

    if let Ok(mut guard) = CONFIG_STATE.lock() {
        *guard = Some(state);
    } else {
        return Err("Failed to store configuration state".to_string());
    }

    log::info!(
        "display config: Configuration initialized, returning {} modes to UI",
        available_modes.len()
    );
    Ok(available_modes)
}

/// Step 2: Apply the selected mode and activate the display
#[tauri::command]
pub fn apply_display_configuration(mode_index: usize) -> std::result::Result<String, String> {
    log::info!(
        "display config: Applying configuration with mode index: {}",
        mode_index
    );

    let mut guard = CONFIG_STATE
        .lock()
        .map_err(|e| format!("Failed to lock configuration state: {}", e))?;

    let state = guard.as_mut().ok_or_else(|| {
        "No configuration state found. Call initialize_display_configuration first.".to_string()
    })?;

    // Validate mode index
    if mode_index >= state.available_modes.len() {
        return Err(format!(
            "Invalid mode index: {}. Available modes: {}",
            mode_index,
            state.available_modes.len()
        ));
    }

    let selected_mode_info = &state.available_modes[mode_index];
    log::info!(
        "display config: Selected mode: {}x{} @ {}Hz",
        selected_mode_info.width,
        selected_mode_info.height,
        selected_mode_info.refresh_rate
    );

    // Get the actual DisplayMode object
    let modes = state
        .path
        .FindModes(DisplayModeQueryOptions::None)
        .map_err(|e| format!("Failed to find display modes: {}", e))?;

    let selected_mode = modes
        .GetAt(mode_index as u32)
        .map_err(|e| format!("Failed to get mode at index {}: {}", mode_index, e))?;

    // Apply the selected mode
    state
        .path
        .ApplyPropertiesFromMode(&selected_mode)
        .map_err(|e| format!("Failed to apply mode properties: {}", e))?;

    log::info!("display config: [OK] Applied mode properties");

    // Apply the display state to activate the display
    log::info!("display config: Applying display state...");
    let apply_result = state
        .modifiable_state
        .TryApply(DisplayStateApplyOptions::None)
        .map_err(|e| format!("Failed to apply display state: {}", e))?;

    if apply_result
        .Status()
        .map_err(|e| format!("Failed to get status: {}", e))?
        != DisplayStateOperationStatus::Success
    {
        let status = apply_result
            .Status()
            .map_err(|e| format!("Failed to get status: {}", e))?;
        log::error!(
            "display config: Failed to apply display state: {:?}",
            status
        );
        return Err(format!("Failed to apply display state: {:?}", status));
    }

    log::info!("display config: [OK][OK][OK] Display activated successfully!");

    // Save the mode info before releasing the state
    let result_message = format!(
        "Display activated at {}x{} @ {}Hz",
        selected_mode_info.width, selected_mode_info.height, selected_mode_info.refresh_rate
    );

    // Give the display a moment to stabilize
    std::thread::sleep(std::time::Duration::from_millis(500));

    // CRITICAL: Clear the state to release the target
    // The renderer needs to acquire it for DisplayCore
    drop(guard);
    if let Ok(mut clear_guard) = CONFIG_STATE.lock() {
        *clear_guard = None;
        log::info!("display config: [OK] Released display target for renderer");
    }

    Ok(result_message)
}

/// Cancel the configuration process and clean up
#[tauri::command]
pub fn cancel_display_configuration() -> std::result::Result<String, String> {
    log::info!("display config: Cancelling configuration");

    if let Ok(mut guard) = CONFIG_STATE.lock() {
        *guard = None;
    }

    Ok("Configuration cancelled".to_string())
}
