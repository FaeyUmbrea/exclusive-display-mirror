use crate::state::CURR_STATE;
use libobs_sources::windows::{MonitorCaptureSourceBuilder, MonitorCaptureSourceUpdater};
use libobs_sources::ObsObjectUpdater;
use libobs_wrapper::utils::traits::ObsUpdatable;

#[tauri::command]
pub fn switch_monitor() -> String {
    tauri::async_runtime::block_on(async move {
        let state = CURR_STATE.read().await;
        if state.obs_source.is_none() {
            return "Source is not initialized".to_string();
        }

        let ctx = state.obs_context.clone();
        if ctx.is_none() {
            return "OBS runtime is not initialized".to_string();
        }

        drop(state);
        let mut state = CURR_STATE.write().await;
        let monitors = MonitorCaptureSourceBuilder::get_monitors().unwrap();

        state.monitor_index = (state.monitor_index + 1) % monitors.len();
        let monitor = &monitors[state.monitor_index];

        let mut source = state.obs_source.as_ref().unwrap().clone();
        source
            .create_updater::<MonitorCaptureSourceUpdater>()
            .unwrap()
            .set_monitor(monitor)
            .update()
            .unwrap();

        format!("Monitor switched to {:?}", monitor)
    })
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct MonitorInfo {
    pub name: String,
    pub device_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stable_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter_id: Option<u32>,
    pub is_connected: bool,
    pub is_stale: bool,
}
