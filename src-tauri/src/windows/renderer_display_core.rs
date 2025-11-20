// Display Core compositor implementation for rendering to specialized displays removed from desktop
// Based on: https://github.com/microsoft/Windows-classic-samples/tree/main/Samples/DisplayCoreCustomCompositor

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;

use crate::monitor::MonitorInfo;

#[cfg(target_os = "windows")]
use windows::{
    core::*, Devices::Display::Core::*, Graphics::DirectX::Direct3D11::*, Win32::Foundation::*,
    Win32::Graphics::Direct3D::*, Win32::Graphics::Direct3D11::*, Win32::Graphics::Dxgi::*,
};

pub struct DisplayCoreRenderer {
    abort: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

#[cfg(target_os = "windows")]
struct CompositorContext {
    display_device: DisplayDevice,
    display_source: DisplaySource,
    task_pool: DisplayTaskPool,
    d3d_device: ID3D11Device,
    d3d_context: ID3D11DeviceContext,
    primaries: Vec<DisplaySurface>,
    scanouts: Vec<DisplayScanout>,
    width: u32,
    height: u32,
}

impl DisplayCoreRenderer {
    pub fn start_for_device(monitor: MonitorInfo, _max_fps: u32, vsync: bool) -> Self {
        let abort = Arc::new(AtomicBool::new(false));
        let abort_clone = abort.clone();

        let handle = thread::spawn(move || {
            log::info!(
                "displaycore: starting for monitor: {} ({}) vsync={}",
                monitor.name,
                monitor.device_name,
                vsync
            );

            #[cfg(target_os = "windows")]
            {
                match Self::initialize_compositor(&monitor) {
                    Ok(ctx) => {
                        log::info!("displaycore: compositor initialized successfully");

                        // Install the DirectRenderContext for OBS callbacks to use
                        use super::direct_render::{
                            set_render_context, DirectRenderContext, DirectRenderContextConfig,
                        };

                        let direct_ctx = DirectRenderContext::new(DirectRenderContextConfig {
                            display_device: ctx.display_device,
                            display_source: ctx.display_source,
                            task_pool: ctx.task_pool,
                            d3d_device: ctx.d3d_device,
                            d3d_context: ctx.d3d_context,
                            primaries: ctx.primaries,
                            scanouts: ctx.scanouts,
                            width: ctx.width,
                            height: ctx.height,
                            vsync_enabled: vsync,
                        });

                        set_render_context(direct_ctx);
                        log::info!("displaycore: DirectRenderContext installed - OBS callbacks will drive rendering");

                        // Keep thread alive until abort signal
                        while !abort_clone.load(Ordering::SeqCst) {
                            thread::sleep(Duration::from_millis(100));
                        }

                        // Clear context when stopping
                        super::direct_render::clear_render_context();
                        log::info!("displaycore: DirectRenderContext cleared");
                    }
                    Err(e) => {
                        log::error!("displaycore: failed to initialize compositor: {}", e);
                    }
                }
            }

            log::info!("displaycore: thread exiting for monitor {}", monitor.name);
        });

        DisplayCoreRenderer {
            abort,
            handle: Some(handle),
        }
    }

    #[cfg(target_os = "windows")]
    fn initialize_compositor(monitor: &MonitorInfo) -> Result<CompositorContext> {
        use windows::Graphics::DirectX::*;

        // Create DisplayManager
        let display_manager = DisplayManager::Create(DisplayManagerOptions::None)?;
        log::debug!("displaycore: DisplayManager created");

        // Find matching DisplayTarget
        let targets = display_manager.GetCurrentTargets()?;
        let count = targets.Size()?;
        log::debug!("displaycore: found {} display targets", count);

        let mut display_target: Option<DisplayTarget> = None;
        for i in 0..count {
            let target = targets.GetAt(i)?;
            let stable_id = target.StableMonitorId()?;
            let adapter = target.Adapter()?;
            let adapter_id = adapter.Id()?;

            let device_name = format!("DISPLAY_CORE_{:#x}_{}", adapter_id.LowPart, stable_id);
            log::debug!("displaycore: target {} device_name={}", i, device_name);

            // Match by stable_id from the JSON
            if let Some(ref target_stable_id) = monitor.stable_id {
                if stable_id == target_stable_id {
                    log::info!(
                        "displaycore: matched target by stable_id: {}",
                        target_stable_id
                    );
                    display_target = Some(target);
                    break;
                }
            }
        }

        let display_target =
            display_target.ok_or_else(|| Error::new(E_FAIL, "No matching display target found"))?;

        // Check the usage kind to ensure it's a specialized display
        let usage_kind = display_target.UsageKind()?;
        log::debug!("displaycore: Target UsageKind={:?}", usage_kind);

        // Note: DisplayMonitorUsageKind values:
        // 0 = Standard, 1 = HeadMounted, 2 = Specialized (removed from desktop)
        // We need the display to be in "Specialized" mode (removed from desktop)
        // This is typically value 2
        if usage_kind.0 != 2 {
            log::warn!("displaycore: Target may not be in 'Specialized/Removed from Desktop' mode");
            log::warn!(
                "displaycore: Expected UsageKind=2 (Specialized), got {}",
                usage_kind.0
            );
        }

        // Check if target is virtual
        let is_virtual = display_target.IsVirtualTopologyEnabled()?;
        log::debug!("displaycore: IsVirtualTopologyEnabled={}", is_virtual);

        // Get adapter for D3D device creation
        let adapter = display_target.Adapter()?;
        let adapter_id = adapter.Id()?;
        log::debug!(
            "displaycore: Using adapter LowPart={:#x}, HighPart={:#x}",
            adapter_id.LowPart,
            adapter_id.HighPart
        );

        // Before trying to acquire, filter targets to only specialized ones
        // This matches the C++ reference implementation which filters before acquiring
        log::info!("displaycore: Filtering targets to only include specialized/removed monitors");
        let mut specialized_targets = Vec::new();
        for i in 0..count {
            let target = targets.GetAt(i)?;
            let kind = target.UsageKind()?;
            if kind.0 == 2 {
                // SpecialPurpose (value 2)
                let sid = target.StableMonitorId()?;
                log::debug!("displaycore: Found specialized target: {}", sid);
                specialized_targets.push(target);
            }
        }
        log::info!(
            "displaycore: Found {} specialized targets out of {} total",
            specialized_targets.len(),
            count
        );

        if specialized_targets.is_empty() {
            return Err(Error::new(E_FAIL, "No specialized targets found"));
        }

        // SOLUTION: Use TryAcquireTarget (singular) which doesn't require IIterable
        // This matches the C++ reference implementation flow
        log::info!("displaycore: Attempting to acquire target using TryAcquireTarget");
        let _acquire_result = display_manager.TryAcquireTarget(&display_target)?;
        // If the call succeeds (doesn't throw), the target is acquired
        log::info!("displaycore: Successfully acquired target");

        // After acquiring target successfully, we need to get the display resolution
        // The C++ code gets this from path.SourceResolution() which requires state
        // Since we can't easily create a modifiable state (IIterable limitation),
        // we'll get the resolution from the target's properties or use TryGetMonitor

        let width: u32;
        let height: u32;

        // Try to get resolution from the monitor's current mode
        if let Ok(monitor) = display_target.TryGetMonitor() {
            if let Ok(native_resolution) = monitor.NativeResolutionInRawPixels() {
                width = native_resolution.Width as u32;
                height = native_resolution.Height as u32;
                log::info!(
                    "displaycore: Got resolution from monitor: {}x{}",
                    width,
                    height
                );
            } else {
                // Fallback to common resolution
                width = 1920;
                height = 1080;
                log::warn!(
                    "displaycore: Could not get native resolution, using fallback: {}x{}",
                    width,
                    height
                );
            }
        } else {
            // Fallback to common resolution
            width = 1920;
            height = 1080;
            log::warn!(
                "displaycore: Could not access monitor, using fallback: {}x{}",
                width,
                height
            );
        }

        // Create D3D11 device on the matching adapter
        let (d3d_device, d3d_context) = Self::create_d3d11_device(&adapter)?;
        log::debug!("displaycore: D3D11 device created");

        // Create DisplayDevice for presenting
        let display_device = display_manager.CreateDisplayDevice(&adapter)?;
        log::debug!("displaycore: DisplayDevice created");

        // Create DisplaySource which identifies where to render
        let display_source = display_device.CreateScanoutSource(&display_target)?;
        log::debug!("displaycore: DisplaySource created");

        // Create task pool for queueing presents
        let task_pool = display_device.CreateTaskPool()?;
        log::debug!("displaycore: DisplayTaskPool created");

        // Create primaries and scanouts (following C++ reference)
        const SURFACE_COUNT: usize = 2;
        let mut primaries = Vec::new();
        let mut scanouts = Vec::new();

        // Create primary description using factory method
        let multisample_desc = Direct3DMultisampleDescription {
            Count: 1,
            Quality: 0,
        };

        // Use standard BGRA format
        let primary_desc = DisplayPrimaryDescription::CreateWithProperties(
            None,
            width,
            height,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            DirectXColorSpace::RgbFullG22NoneP709,
            false,
            multisample_desc,
        )?;

        for i in 0..SURFACE_COUNT {
            let primary = display_device.CreatePrimary(&display_target, &primary_desc)?;
            let scanout = display_device.CreateSimpleScanout(&display_source, &primary, 0, 1)?;
            log::debug!("displaycore: Created primary and scanout {}", i);

            primaries.push(primary);
            scanouts.push(scanout);
        }

        Ok(CompositorContext {
            display_device,
            display_source,
            task_pool,
            d3d_device,
            d3d_context,
            primaries,
            scanouts,
            width,
            height,
        })
    }

    #[cfg(target_os = "windows")]
    fn create_d3d11_device(
        adapter: &DisplayAdapter,
    ) -> Result<(ID3D11Device, ID3D11DeviceContext)> {
        unsafe {
            let adapter_id = adapter.Id()?;

            // Enumerate DXGI adapters to find matching one
            let dxgi_factory: IDXGIFactory1 = CreateDXGIFactory1()?;
            let mut dxgi_adapter: Option<IDXGIAdapter1> = None;

            for i in 0..10 {
                if let Ok(adapter_enum) = dxgi_factory.EnumAdapters1(i) {
                    let desc = adapter_enum.GetDesc1()?;

                    // Match by LUID
                    if desc.AdapterLuid.LowPart == adapter_id.LowPart
                        && desc.AdapterLuid.HighPart == adapter_id.HighPart
                    {
                        log::info!(
                            "displaycore: Found matching DXGI adapter: {}",
                            String::from_utf16_lossy(&desc.Description)
                        );
                        dxgi_adapter = Some(adapter_enum);
                        break;
                    }
                }
            }

            let dxgi_adapter = dxgi_adapter
                .ok_or_else(|| Error::new(E_FAIL, "Could not find matching DXGI adapter"))?;

            let feature_levels = [D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_11_0];

            let mut device: Option<ID3D11Device> = None;
            let mut context: Option<ID3D11DeviceContext> = None;
            let mut feature_level = D3D_FEATURE_LEVEL_11_0;

            // Create device with BGRA support for DisplayCore
            // Note: We create our own separate device to avoid conflicts with OBS's D3D11 usage
            let mut create_flags = D3D11_CREATE_DEVICE_BGRA_SUPPORT;

            // Add debug flag in debug builds for better diagnostics
            if cfg!(debug_assertions) {
                create_flags |= D3D11_CREATE_DEVICE_DEBUG;
            }

            D3D11CreateDevice(
                &dxgi_adapter,
                D3D_DRIVER_TYPE_UNKNOWN,
                HMODULE::default(),
                create_flags,
                Some(&feature_levels),
                D3D11_SDK_VERSION,
                Some(&mut device),
                Some(&mut feature_level),
                Some(&mut context),
            )?;

            log::info!(
                "displaycore: D3D11 device created with feature level {:?}",
                feature_level
            );

            // Log adapter info for cross-device debugging
            if let Ok(dxgi_device) = device.as_ref().unwrap().cast::<IDXGIDevice>() {
                if let Ok(adapter) = dxgi_device.GetAdapter() {
                    if let Ok(adapter1) = adapter.cast::<IDXGIAdapter1>() {
                        if let Ok(desc) = adapter1.GetDesc1() {
                            let adapter_name = String::from_utf16_lossy(&desc.Description);
                            log::info!(
                                "[GPU DIAG] DisplayCore D3D11 adapter: {} (LUID: {:#x}:{:#x})",
                                adapter_name.trim_end_matches('\0'),
                                desc.AdapterLuid.HighPart,
                                desc.AdapterLuid.LowPart
                            );
                        }
                    }
                }
            }

            Ok((device.unwrap(), context.unwrap()))
        }
    }

    pub fn stop(&mut self) {
        self.abort.store(true, Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for DisplayCoreRenderer {
    fn drop(&mut self) {
        self.stop();
    }
}
