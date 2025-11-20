use crate::monitor::MonitorInfo;

// Windows-only implementation to enumerate specialized monitors (removed from desktop)
// Based on DisplayCoreCustomCompositor.cpp reference implementation
#[cfg(target_os = "windows")]
#[tauri::command]
pub fn get_detached_monitors() -> Result<Vec<MonitorInfo>, String> {
    tauri::async_runtime::block_on(async move {
        use windows::Devices::Display::Core::{DisplayManager, DisplayManagerOptions};

        let mut results: Vec<MonitorInfo> = Vec::new();

        // Create the DisplayManager
        let display_manager = DisplayManager::Create(DisplayManagerOptions::None)
            .map_err(|e| format!("Failed to create DisplayManager: {}", e))?;

        // Get ALL display targets
        let targets = display_manager
            .GetCurrentTargets()
            .map_err(|e| format!("Failed to get display targets: {}", e))?;

        let count = targets
            .Size()
            .map_err(|e| format!("Failed to get target count: {}", e))?;

        // Filter for specialized displays (UsageKind == 2, SpecialPurpose/Removed from Desktop)
        // This matches the C++ reference implementation which checks for HeadMounted or SpecialPurpose
        for i in 0..count {
            let target = targets
                .GetAt(i)
                .map_err(|e| format!("Failed to get target at index {}: {}", i, e))?;

            // Check UsageKind - we only want SpecialPurpose displays
            let usage_kind = target
                .UsageKind()
                .map_err(|e| format!("Failed to get usage kind: {}", e))?;

            // DisplayMonitorUsageKind values:
            // 0 = Standard (normal desktop display)
            // 1 = HeadMounted (VR headset)
            // 2 = SpecialPurpose (removed from desktop, specialized display)
            if usage_kind.0 != 2 {
                // Skip non-specialized displays
                continue;
            }

            // Get the stable monitor ID
            let stable_id = target
                .StableMonitorId()
                .map_err(|e| format!("Failed to get stable monitor ID: {}", e))?;

            // Get adapter information
            let adapter = target
                .Adapter()
                .map_err(|e| format!("Failed to get adapter: {}", e))?;

            let adapter_id = adapter
                .Id()
                .map_err(|e| format!("Failed to get adapter ID: {}", e))?;

            // Get connection state
            let is_connected = target.IsConnected().unwrap_or(false);
            let is_stale = target.IsStale().unwrap_or(false);

            // Only include connected, non-stale displays
            if !is_connected || is_stale {
                continue;
            }

            // Try to get friendly name from EDID
            let name = if let Ok(monitor) = target.TryGetMonitor() {
                if let Ok(display_name) = monitor.DisplayName() {
                    let name_str = display_name.to_string();
                    if !name_str.is_empty() {
                        name_str
                    } else {
                        format!("Specialized Display {}", i)
                    }
                } else {
                    format!("Specialized Display {}", i)
                }
            } else {
                format!("Specialized Display {}", i)
            };

            // Format device name to match what the renderer expects
            let device_name = format!("DISPLAY_CORE_{:#x}_{}", adapter_id.LowPart, stable_id);

            results.push(MonitorInfo {
                name,
                device_name,
                stable_id: Some(stable_id.to_string()),
                adapter_id: Some(adapter_id.LowPart),
                is_connected,
                is_stale,
            });
        }

        Ok(results)
    })
}
