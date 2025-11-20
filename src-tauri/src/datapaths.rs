use libobs_wrapper::utils::{ObsPath, StartupPaths};
use log::{error, info, warn};
use std::path::Path;

const PATHS: &[&str; 2] = &["resources", "resources\\obs"];

pub fn get_lib_obs_root_path() -> Option<String> {
    // Try to locate a bundled resources directory next to the executable; fall back to the exe parent.
    match std::env::current_exe() {
        Ok(exe_path) => {
            if let Some(parent) = exe_path.parent() {
                for path in PATHS {
                    let candidate = parent.join(path);
                    if candidate.exists()
                        && candidate.is_dir()
                        && candidate.join("obs.dll").exists()
                    {
                        info!("libobs root: {}", candidate.display());
                        return Some(candidate.to_string_lossy().to_string());
                    }
                }
                // Not found in resource subdirs; use the executable parent directory.
                info!("libobs root (fallback): {}", parent.display());
                return Some(parent.to_string_lossy().to_string());
            } else {
                warn!("executable path has no parent");
            }
        }
        Err(e) => {
            error!("failed to get current executable path: {}", e);
        }
    }

    None
}

fn get_lib_obs_data_path() -> Option<ObsPath> {
    if let Some(root_path) = get_lib_obs_root_path() {
        let data_path = Path::new(&root_path).join("data").join("libobs");
        if data_path.exists() {
            info!("data path: {}", data_path.display());
            return Some(ObsPath::new(data_path.to_str()?));
        } else {
            warn!("data path not found: {}", data_path.display());
        }
    } else {
        warn!("root path not found when looking for data");
    }
    None
}

fn get_lib_obs_plugin_path() -> Option<ObsPath> {
    if let Some(root_path) = get_lib_obs_root_path() {
        let plugin_path = Path::new(&root_path).join("obs-plugins").join("64bit");
        if plugin_path.exists() {
            info!("plugin path: {}", plugin_path.display());
            return Some(ObsPath::new(plugin_path.to_str()?));
        } else {
            warn!("plugin path not found: {}", plugin_path.display());
        }
    } else {
        warn!("root path not found when looking for plugins");
    }
    None
}

fn get_lib_obs_plugin_data_path() -> Option<ObsPath> {
    if let Some(root_path) = get_lib_obs_root_path() {
        let plugin_data_path = Path::new(&root_path).join("data").join("obs-plugins");
        if plugin_data_path.exists() {
            info!("plugin data path: {}", plugin_data_path.display());
            return Some(ObsPath::new(plugin_data_path.to_str()?));
        } else {
            warn!("plugin data path not found: {}", plugin_data_path.display());
        }
    } else {
        warn!("root path not found when looking for plugin data");
    }
    None
}

pub fn get_startup_paths() -> Result<StartupPaths, String> {
    let data_path = get_lib_obs_data_path().ok_or_else(|| {
        let msg = "OBS data path not found".to_string();
        error!("{}", msg);
        msg
    })?;

    let plugin_path = get_lib_obs_plugin_path().ok_or_else(|| {
        let msg = "OBS plugin path not found".to_string();
        error!("{}", msg);
        msg
    })?;

    let plugin_data_path = get_lib_obs_plugin_data_path().ok_or_else(|| {
        let msg = "OBS plugin data path not found".to_string();
        error!("{}", msg);
        msg
    })?;

    info!("startup paths located");
    Ok(StartupPaths::new(data_path, plugin_path, plugin_data_path))
}
