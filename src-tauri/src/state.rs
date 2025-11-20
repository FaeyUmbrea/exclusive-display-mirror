use std::pin::Pin;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use lazy_static::lazy_static;
use tauri::async_runtime::RwLock;

use libobs_wrapper::context::ObsContext;
use libobs_wrapper::display::ObsDisplayRef;
use libobs_wrapper::scenes::ObsSceneRef;
use libobs_wrapper::sources::ObsSourceRef;

pub struct CurrState {
    pub monitor_index: usize,
    pub is_bootstrapping: bool,
    pub obs_context: Option<ObsContext>,
    pub obs_scene: Option<ObsSceneRef>,
    pub obs_source: Option<ObsSourceRef>,
    pub obs_display: Option<Pin<Box<ObsDisplayRef>>>,

    // Renderer thread state: optional abort flag and basic params
    // Note: we avoid storing JoinHandle here to keep CurrState Send+Sync
    pub renderer_abort: Option<Arc<AtomicBool>>,
    pub renderer_device_name: Option<String>,
    pub renderer_vsync: Option<bool>,
}

impl CurrState {
    pub fn new() -> Self {
        CurrState {
            monitor_index: 1,
            is_bootstrapping: false,
            obs_context: None,
            obs_scene: None,
            obs_source: None,
            obs_display: None,

            renderer_abort: None,
            renderer_device_name: None,
            renderer_vsync: None,
        }
    }
}

// Add Default impl so clippy's `new_without_default` warning is addressed
impl Default for CurrState {
    fn default() -> Self {
        Self::new()
    }
}

lazy_static! {
    pub static ref CURR_STATE: Arc<RwLock<CurrState>> = Arc::new(RwLock::new(CurrState::new()));
}
