use crate::state::CURR_STATE;
use libobs_wrapper::context::ObsContext;
use libobs_wrapper::data::video::ObsVideoInfoBuilder;
use libobs_wrapper::enums::ObsVideoFormat;
use tauri::{AppHandle, Emitter};

// Capture mode chosen from the frontend: GPU or CPU
#[derive(serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CaptureMode {
    Gpu,
    Cpu,
}

/// Initialize OBS context with specific display parameters
/// This should be called after display configuration
#[tauri::command]
pub async fn initialize_obs(
    handle: AppHandle,
    width: u32,
    height: u32,
    fps: u32,
    capture_mode: CaptureMode,
) -> Result<String, String> {
    let state = CURR_STATE.read().await;
    if state.obs_context.is_some() {
        return Ok("OBS is already initialized.".to_string());
    }

    log::info!(
        "Initializing OBS context with resolution: {}x{} @ {}fps",
        width,
        height,
        fps
    );
    drop(state);

    let mut state = CURR_STATE.write().await;

    let video_info = ObsVideoInfoBuilder::new()
        .output_width(width)
        .output_height(height)
        .base_width(width)
        .base_height(height)
        .fps_num(fps)
        .fps_den(1)
        .output_format(ObsVideoFormat::BGRA)
        .build();

    let mut context = ObsContext::builder()
        .set_video_info(video_info)
        .start()
        .map_err(|e| format!("Failed to start OBS context: {}", e))?;

    log::debug!("Creating scene");
    {
        let scene = context
            .scene("Main Scene")
            .map_err(|e| format!("Failed to create scene: {}", e))?;
        scene
            .set_to_channel(0)
            .map_err(|e| format!("Failed to set scene to channel: {}", e))?;
        // Store scene in global state so other modules can borrow it
        state.obs_scene = Some(scene);
    }

    // Create and register a custom output type that triggers rendering
    log::debug!("Creating custom passthrough output");
    create_and_start_passthrough_output(&mut context);

    // Start our frame capture thread
    log::info!("[OK] Starting frame capture thread...");
    start_frame_capture_thread(handle.clone(), context.clone(), capture_mode);
    log::info!("[OK] Frame capture thread started");

    state.obs_context = Some(context);

    log::info!("OBS initialized - ready for source selection");
    handle.emit("obs_initialized", ()).unwrap();

    Ok(format!(
        "OBS initialized at {}x{} @ {}fps",
        width, height, fps
    ))
}

// Thread that captures raw video frames from OBS and sends them to the frame buffer
fn create_and_start_passthrough_output(_context: &mut ObsContext) -> *mut libobs::obs_output {
    use std::ffi::CString;

    unsafe {
        use libobs::*;

        // Define callbacks for our custom output
        extern "C" fn output_get_name(
            _type_data: *mut std::ffi::c_void,
        ) -> *const std::os::raw::c_char {
            static NAME: &[u8] = b"Passthrough Output\0";
            NAME.as_ptr() as *const std::os::raw::c_char
        }

        extern "C" fn output_create(
            _settings: *mut obs_data_t,
            _output: *mut obs_output_t,
        ) -> *mut std::ffi::c_void {
            std::ptr::dangling_mut::<std::ffi::c_void>()
        }

        extern "C" fn output_destroy(_data: *mut std::ffi::c_void) {}

        extern "C" fn output_start(_data: *mut std::ffi::c_void) -> bool {
            true
        }

        extern "C" fn output_stop(_data: *mut std::ffi::c_void, _ts: u64) {}

        extern "C" fn output_raw_video(_data: *mut std::ffi::c_void, _frame: *mut video_data) {}

        // Create and register output type
        let mut info: obs_output_info = std::mem::zeroed();
        let id_cstring = CString::new("passthrough_output").unwrap();
        info.id = id_cstring.as_ptr();
        info.flags = OBS_OUTPUT_VIDEO;
        info.get_name = Some(output_get_name);
        info.create = Some(output_create);
        info.destroy = Some(output_destroy);
        info.start = Some(output_start);
        info.stop = Some(output_stop);
        info.raw_video = Some(output_raw_video);

        obs_register_output_s(&info, std::mem::size_of::<obs_output_info>());

        // Create output instance
        let output_id = CString::new("passthrough_output").unwrap();
        let output_name = CString::new("frame_renderer").unwrap();
        let output = obs_output_create(
            output_id.as_ptr(),
            output_name.as_ptr(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );

        if output.is_null() {
            log::debug!("ERROR: Failed to create passthrough output");
            std::mem::forget(id_cstring);
            std::mem::forget(output_name);
            return std::ptr::null_mut();
        }

        // Set video and audio for the output
        let video = obs_get_video();
        let audio = obs_get_audio();
        obs_output_set_video_encoder(output, std::ptr::null_mut());
        obs_output_set_media(output, video, audio);

        // Start the output and begin data capture
        let start_result = obs_output_start(output);
        if start_result {
            let capture_result = obs_output_begin_data_capture(output, 0);
            if capture_result {
                log::debug!("[OK] OBS output started and capturing frames");
            } else {
                log::debug!("ERROR: Failed to begin data capture");
            }
        } else {
            log::debug!("ERROR: Failed to start passthrough output");
        }

        std::mem::forget(id_cstring);
        std::mem::forget(output_name);
        output
    }
}

// Thread that captures raw video frames from OBS and sends them to the frame buffer
fn start_frame_capture_thread(handle: AppHandle, _context: ObsContext, mode: CaptureMode) {
    use std::thread;

    thread::spawn(move || {
        log::debug!("Frame capture thread started");

        match mode {
            CaptureMode::Cpu => {
                log::debug!("User selected CPU capture - starting CPU capture");
                start_cpu_frame_capture();
            }
            CaptureMode::Gpu => {
                log::debug!("User selected GPU capture - attempting GPU capture");

                // If not on Windows, fall back to CPU
                log::debug!("Non-Windows platform or GPU not supported - starting CPU capture");
                let _ = handle.emit(
                    "gpu_capture_unavailable",
                    "GPU capture unsupported on this platform",
                );
                start_cpu_frame_capture();
            }
        }
    });
}

fn start_cpu_frame_capture() {
    #[cfg(target_os = "windows")]
    use crate::windows::direct_render;
    use std::sync::atomic::{AtomicU64, Ordering};

    unsafe {
        use libobs::*;

        log::debug!("=== CPU FRAME CAPTURE (DIRECT RENDER) STARTING ===");

        let video_output = obs_get_video();
        if video_output.is_null() {
            log::debug!("ERROR: Could not get OBS video output");
            return;
        }

        log::debug!("[OK] Got OBS video output: {:p}", video_output);

        let mut video_info: obs_video_info = std::mem::zeroed();
        if obs_get_video_info(&mut video_info) {
            log::debug!(
                "OBS Video Info: {}x{} @ {}/{} fps",
                video_info.output_width,
                video_info.output_height,
                video_info.fps_num,
                video_info.fps_den
            );
        } else {
            log::debug!("WARNING: Could not get video info");
        }

        static FRAME_COUNTER: AtomicU64 = AtomicU64::new(0);
        static RENDER_SUCCESS: AtomicU64 = AtomicU64::new(0);

        extern "C" fn video_frame_callback(_param: *mut std::ffi::c_void, frame: *mut video_data) {
            if frame.is_null() {
                return;
            }

            let frame_num = FRAME_COUNTER.fetch_add(1, Ordering::Relaxed);

            // Log first frame
            if frame_num == 0 {
                log::info!("[OK][OK][OK] CPU CALLBACK: FIRST FRAME RECEIVED FROM OBS!");
            }

            unsafe {
                let frame_ref = &*frame;

                let mut video_info: obs_video_info = std::mem::zeroed();
                if !obs_get_video_info(&mut video_info) {
                    return;
                }

                let width = video_info.output_width;
                let height = video_info.output_height;
                let format = video_info.output_format;
                let linesize = frame_ref.linesize[0];

                let data_ptr = frame_ref.data[0];
                if data_ptr.is_null() || linesize == 0 {
                    return;
                }

                // BGRA (3) or RGBA (7) are the supported formats
                if format == 3 || format == 7 {
                    let data_len = (height * linesize) as usize;
                    let pixels = std::slice::from_raw_parts(data_ptr, data_len);

                    // Render directly to DisplayCore
                    #[cfg(target_os = "windows")]
                    {
                        if direct_render::render_cpu_frame(pixels, width, height, linesize) {
                            let success_count = RENDER_SUCCESS.fetch_add(1, Ordering::Relaxed);
                            if success_count == 0 {
                                log::info!("[OK] First frame rendered to display successfully!");
                            }
                            if frame_num > 0 && frame_num.is_multiple_of(300) {
                                log::debug!(
                                    "CPU direct render: {} frames presented",
                                    success_count + 1
                                );
                            }
                        } else {
                            // Render context not yet available or render failed
                            if frame_num < 10 {
                                log::debug!(
                                    "CPU frame {}: waiting for DisplayCore context...",
                                    frame_num
                                );
                            } else if frame_num.is_multiple_of(60) {
                                log::warn!(
                                    "CPU frame {}: DisplayCore not ready or render failed",
                                    frame_num
                                );
                            }
                        }
                    }
                } else if frame_num.is_multiple_of(300) {
                    log::debug!(
                        "ERROR: Unsupported format {}. Expected BGRA (3) or RGBA (7)",
                        format
                    );
                }
            }
        }

        let success = video_output_connect(
            video_output,
            std::ptr::null(),
            Some(video_frame_callback),
            std::ptr::null_mut(),
        );

        if success {
            log::debug!("[OK] CPU video callback registered");
        } else {
            log::debug!("ERROR: Failed to register video callback");
            return;
        }

        loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    }
}
