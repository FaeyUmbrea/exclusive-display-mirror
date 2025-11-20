// Renderer module: provides DisplayCore backend for rendering to specialized displays

use std::sync::Mutex;

use super::renderer_display_core::DisplayCoreRenderer;
use crate::monitor::MonitorInfo;
use crate::state::CURR_STATE;

lazy_static::lazy_static! {
    static ref ACTIVE_RENDERER: Mutex<Option<DisplayCoreRenderer>> = Mutex::new(None);
}

#[derive(serde::Deserialize)]
pub struct RendererStartParams {
    pub monitor: MonitorInfo,
    #[serde(default = "default_vsync")]
    pub vsync: bool,
    #[serde(default = "default_max_fps")]
    pub max_fps: u32,
}

fn default_vsync() -> bool {
    true
}
fn default_max_fps() -> u32 {
    60
}

#[derive(serde::Serialize)]
pub struct RendererStartResult {
    pub status: String,
    pub monitor_device: String,
}

#[tauri::command]
pub fn start_renderer(params: RendererStartParams) -> Result<RendererStartResult, String> {
    // Stop existing renderer if any
    if let Ok(mut guard) = ACTIVE_RENDERER.lock() {
        if let Some(mut renderer) = guard.take() {
            renderer.stop();
        }
    }

    // Start new DisplayCore renderer
    let device_name = params.monitor.device_name.clone();
    let max_fps = params.max_fps;

    let new_renderer =
        DisplayCoreRenderer::start_for_device(params.monitor.clone(), max_fps, params.vsync);

    if let Ok(mut guard) = ACTIVE_RENDERER.lock() {
        *guard = Some(new_renderer);
    }

    // Update state
    let device_name_clone = device_name.clone();
    tauri::async_runtime::block_on(async move {
        let mut state = CURR_STATE.write().await;
        state.renderer_device_name = Some(device_name_clone);
        state.renderer_vsync = Some(params.vsync);
    });

    Ok(RendererStartResult {
        status: "started".into(),
        monitor_device: device_name,
    })
}

#[tauri::command]
pub fn stop_renderer() -> Result<String, String> {
    if let Ok(mut guard) = ACTIVE_RENDERER.lock() {
        if let Some(mut renderer) = guard.take() {
            renderer.stop();
        }
    }

    tauri::async_runtime::block_on(async move {
        let mut state = CURR_STATE.write().await;
        state.renderer_device_name = None;
        state.renderer_vsync = None;
    });

    Ok("stopped".to_string())
}
