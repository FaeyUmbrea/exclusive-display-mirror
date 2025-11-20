// Shared render context for direct OBS callback -> DisplayCore rendering
// This eliminates the intermediate frame buffer and allows OBS callbacks
// to render directly to the display surfaces.

#[cfg(target_os = "windows")]
use std::sync::atomic::{AtomicPtr, AtomicU64, Ordering};

#[cfg(target_os = "windows")]
use windows::{
    core::*, Devices::Display::Core::*, Win32::Foundation::*, Win32::Graphics::Direct3D11::*,
    Win32::Graphics::Dxgi::Common::*, Win32::System::WinRT::Display::*,
};

#[cfg(target_os = "windows")]
pub struct DirectRenderContextConfig {
    pub display_device: DisplayDevice,
    pub display_source: DisplaySource,
    pub task_pool: DisplayTaskPool,
    pub d3d_device: ID3D11Device,
    pub d3d_context: ID3D11DeviceContext,
    pub primaries: Vec<DisplaySurface>,
    pub scanouts: Vec<DisplayScanout>,
    pub width: u32,
    pub height: u32,
    pub vsync_enabled: bool,
}

/// Context needed for rendering frames directly to DisplayCore surfaces
#[cfg(target_os = "windows")]
pub struct DirectRenderContext {
    pub display_device: DisplayDevice,
    pub display_source: DisplaySource, // Added for WaitForVBlank
    pub task_pool: DisplayTaskPool,
    pub d3d_device: ID3D11Device,
    pub d3d_context: ID3D11DeviceContext,
    pub primaries: Vec<DisplaySurface>,
    pub scanouts: Vec<DisplayScanout>,
    pub width: u32,
    pub height: u32,
    pub current_surface: usize,
    pub vsync_enabled: bool, // Added to control VBlank waiting

    // Cached textures for performance - avoid per-frame allocations
    staging_texture: Option<ID3D11Texture2D>,
    staging_width: u32,
    staging_height: u32,
    d3d_textures: Vec<Option<ID3D11Texture2D>>, // Cached opened primary surfaces
}

#[cfg(target_os = "windows")]
impl DirectRenderContext {
    /// Create a new context with caches initialized
    pub fn new(config: DirectRenderContextConfig) -> Self {
        let num_surfaces = config.primaries.len();
        Self {
            display_device: config.display_device,
            display_source: config.display_source,
            task_pool: config.task_pool,
            d3d_device: config.d3d_device,
            d3d_context: config.d3d_context,
            primaries: config.primaries,
            scanouts: config.scanouts,
            width: config.width,
            height: config.height,
            current_surface: 0,
            vsync_enabled: config.vsync_enabled,
            staging_texture: None,
            staging_width: 0,
            staging_height: 0,
            d3d_textures: vec![None; num_surfaces],
        }
    }

    /// Advance to next surface in the swap chain
    pub fn next_surface(&mut self) {
        self.current_surface = (self.current_surface + 1) % self.primaries.len();
    }
}

// Global static for the render context, accessible from OBS callbacks
#[cfg(target_os = "windows")]
static RENDER_CONTEXT: AtomicPtr<DirectRenderContext> = AtomicPtr::new(std::ptr::null_mut());

// Frame counters for statistics
#[cfg(target_os = "windows")]
static FRAMES_RENDERED: AtomicU64 = AtomicU64::new(0);
#[cfg(target_os = "windows")]
static FRAMES_DROPPED: AtomicU64 = AtomicU64::new(0);

// Minimum frame interval in microseconds (default: ~16.6ms = 60fps)
// Can be adjusted based on display refresh rate
#[cfg(target_os = "windows")]
static MIN_FRAME_INTERVAL_US: AtomicU64 = AtomicU64::new(16000); // ~60fps

/// Set the target frame rate for rendering
#[cfg(target_os = "windows")]
pub fn set_target_fps(fps: u32) {
    if fps > 0 {
        let interval_us = 1_000_000 / fps as u64;
        MIN_FRAME_INTERVAL_US.store(interval_us, Ordering::Relaxed);
        log::info!(
            "direct_render: Target framerate set to {}fps ({}us interval)",
            fps,
            interval_us
        );
    }
}

/// Store the render context for callback access
#[cfg(target_os = "windows")]
pub fn set_render_context(ctx: DirectRenderContext) {
    let ptr = Box::into_raw(Box::new(ctx));
    let old = RENDER_CONTEXT.swap(ptr, Ordering::SeqCst);

    // Clean up old context if any
    if !old.is_null() {
        unsafe {
            let _ = Box::from_raw(old);
        }
    }

    log::info!("direct_render: Render context installed at {:?}", ptr);
}

/// Clear the render context (called when stopping renderer)
#[cfg(target_os = "windows")]
pub fn clear_render_context() {
    let old = RENDER_CONTEXT.swap(std::ptr::null_mut(), Ordering::SeqCst);
    if !old.is_null() {
        unsafe {
            let _ = Box::from_raw(old);
        }
        log::info!("direct_render: Render context cleared");
    }
}

/// Check if render context is available
#[cfg(target_os = "windows")]
pub fn has_render_context() -> bool {
    !RENDER_CONTEXT.load(Ordering::SeqCst).is_null()
}

/// Render a CPU frame directly to the display
/// Returns true on success, false on failure
#[cfg(target_os = "windows")]
pub fn render_cpu_frame(pixels: &[u8], width: u32, height: u32, stride: u32) -> bool {
    let ctx_ptr = RENDER_CONTEXT.load(Ordering::SeqCst);
    if ctx_ptr.is_null() {
        return false;
    }

    unsafe {
        let ctx = &mut *ctx_ptr;

        match render_cpu_frame_inner(ctx, pixels, width, height, stride) {
            Ok(()) => {
                FRAMES_RENDERED.fetch_add(1, Ordering::Relaxed);
                ctx.next_surface();
                true
            }
            Err(e) => {
                FRAMES_DROPPED.fetch_add(1, Ordering::Relaxed);
                let frame_count = FRAMES_RENDERED.load(Ordering::Relaxed);
                if frame_count < 5 || frame_count.is_multiple_of(300) {
                    log::error!("direct_render: CPU frame failed: {:?}", e);
                }
                false
            }
        }
    }
}

/// Render a GPU texture handle directly to the display
/// Returns true on success, false on failure  
#[cfg(target_os = "windows")]
pub fn render_gpu_frame(source_texture: &ID3D11Texture2D, _width: u32, _height: u32) -> bool {
    let ctx_ptr = RENDER_CONTEXT.load(Ordering::SeqCst);
    if ctx_ptr.is_null() {
        return false;
    }

    unsafe {
        let ctx = &mut *ctx_ptr;

        match render_gpu_frame_inner(ctx, source_texture) {
            Ok(()) => {
                FRAMES_RENDERED.fetch_add(1, Ordering::Relaxed);
                ctx.next_surface();
                true
            }
            Err(e) => {
                FRAMES_DROPPED.fetch_add(1, Ordering::Relaxed);
                let frame_count = FRAMES_RENDERED.load(Ordering::Relaxed);
                if frame_count < 5 || frame_count.is_multiple_of(300) {
                    log::error!("direct_render: GPU frame failed: {:?}", e);
                }
                false
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn render_cpu_frame_inner(
    ctx: &mut DirectRenderContext,
    pixels: &[u8],
    width: u32,
    height: u32,
    stride: u32,
) -> Result<()> {
    let surface_index = ctx.current_surface;

    // Get or create cached D3D texture for this surface
    let d3d_texture = if let Some(ref cached) = ctx.d3d_textures[surface_index] {
        cached.clone()
    } else {
        // First time - open the shared handle and cache it
        let display_surface = &ctx.primaries[surface_index];
        let device_interop: IDisplayDeviceInterop = ctx.display_device.cast()?;
        let surface_inspectable: IInspectable = display_surface.cast()?;
        let empty_name = windows::core::HSTRING::new();

        let surface_handle = unsafe {
            device_interop.CreateSharedHandle(
                &surface_inspectable,
                std::ptr::null(),
                GENERIC_ALL.0,
                &empty_name,
            )?
        };

        let d3d_device5: ID3D11Device5 = ctx.d3d_device.cast()?;
        let texture: ID3D11Texture2D = unsafe { d3d_device5.OpenSharedResource1(surface_handle)? };
        ctx.d3d_textures[surface_index] = Some(texture.clone());
        texture
    };

    // Get or create cached staging texture (reuse if same size)
    let needs_new_staging =
        ctx.staging_texture.is_none() || ctx.staging_width != width || ctx.staging_height != height;

    if needs_new_staging {
        let staging_desc = D3D11_TEXTURE2D_DESC {
            Width: width,
            Height: height,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_DYNAMIC, // Dynamic for fast CPU updates
            BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
            CPUAccessFlags: D3D11_CPU_ACCESS_WRITE.0 as u32,
            MiscFlags: 0,
        };

        let mut staging_texture: Option<ID3D11Texture2D> = None;
        unsafe {
            ctx.d3d_device
                .CreateTexture2D(&staging_desc, None, Some(&mut staging_texture))?;
        }
        ctx.staging_texture = staging_texture;
        ctx.staging_width = width;
        ctx.staging_height = height;

        let rendered = FRAMES_RENDERED.load(Ordering::Relaxed);
        if rendered < 5 {
            log::debug!(
                "direct_render: Created new staging texture {}x{}",
                width,
                height
            );
        }
    }

    let staging_texture = ctx
        .staging_texture
        .as_ref()
        .ok_or_else(|| Error::new(E_FAIL, "Failed to get staging texture"))?;

    // Map, copy pixels, unmap (fast update path)
    unsafe {
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        ctx.d3d_context.Map(
            staging_texture,
            0,
            D3D11_MAP_WRITE_DISCARD,
            0,
            Some(&mut mapped),
        )?;

        // Copy row by row in case strides differ
        let dst_ptr = mapped.pData as *mut u8;
        let src_stride = stride as usize;
        let dst_stride = mapped.RowPitch as usize;

        if src_stride == dst_stride {
            // Fast path: strides match, single memcpy
            std::ptr::copy_nonoverlapping(pixels.as_ptr(), dst_ptr, pixels.len());
        } else {
            // Row-by-row copy
            let row_bytes = (width * 4) as usize;
            for y in 0..height as usize {
                let src = pixels.as_ptr().add(y * src_stride);
                let dst = dst_ptr.add(y * dst_stride);
                std::ptr::copy_nonoverlapping(src, dst, row_bytes);
            }
        }

        ctx.d3d_context.Unmap(staging_texture, 0);

        // Copy to display surface
        if width == ctx.width && height == ctx.height {
            ctx.d3d_context.CopyResource(&d3d_texture, staging_texture);
        } else {
            let copy_width = width.min(ctx.width);
            let copy_height = height.min(ctx.height);

            let src_box = D3D11_BOX {
                left: 0,
                top: 0,
                front: 0,
                right: copy_width,
                bottom: copy_height,
                back: 1,
            };

            ctx.d3d_context.CopySubresourceRegion(
                &d3d_texture,
                0,
                0,
                0,
                0,
                staging_texture,
                0,
                Some(&src_box),
            );
        }

        ctx.d3d_context.Flush();
    }

    // Submit scanout
    // Submit scanout
    if ctx.vsync_enabled {
        let start = std::time::Instant::now();
        let _ = ctx.display_device.WaitForVBlank(&ctx.display_source);
        let elapsed = start.elapsed();
        if elapsed.as_millis() > 2 {
            log::trace!("VSync wait: {:?}", elapsed);
        }
    } else if FRAMES_RENDERED.load(Ordering::Relaxed).is_multiple_of(300) {
        log::debug!("VSync disabled, skipping wait");
    }

    let task = ctx.task_pool.CreateTask()?;
    task.SetScanout(&ctx.scanouts[surface_index])?;
    ctx.task_pool.ExecuteTask(&task)?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn render_gpu_frame_inner(
    ctx: &mut DirectRenderContext,
    source_texture: &ID3D11Texture2D,
) -> Result<()> {
    let surface_index = ctx.current_surface;

    // Get or create cached D3D texture for this surface
    let d3d_texture = if let Some(ref cached) = ctx.d3d_textures[surface_index] {
        cached.clone()
    } else {
        // First time - open the shared handle and cache it
        let display_surface = &ctx.primaries[surface_index];
        let device_interop: IDisplayDeviceInterop = ctx.display_device.cast()?;
        let surface_inspectable: IInspectable = display_surface.cast()?;
        let empty_name = windows::core::HSTRING::new();

        let surface_handle = unsafe {
            device_interop.CreateSharedHandle(
                &surface_inspectable,
                std::ptr::null(),
                GENERIC_ALL.0,
                &empty_name,
            )?
        };

        let d3d_device5: ID3D11Device5 = ctx.d3d_device.cast()?;
        let texture: ID3D11Texture2D = unsafe { d3d_device5.OpenSharedResource1(surface_handle)? };
        ctx.d3d_textures[surface_index] = Some(texture.clone());
        texture
    };

    // Direct GPU copy
    unsafe {
        ctx.d3d_context.CopyResource(&d3d_texture, source_texture);
        ctx.d3d_context.Flush();
    }

    // Submit scanout
    if ctx.vsync_enabled {
        let start = std::time::Instant::now();
        let _ = ctx.display_device.WaitForVBlank(&ctx.display_source);
        let elapsed = start.elapsed();
        if elapsed.as_millis() > 2 {
            log::trace!("VSync wait (GPU): {:?}", elapsed);
        }
    }

    let task = ctx.task_pool.CreateTask()?;
    task.SetScanout(&ctx.scanouts[surface_index])?;
    ctx.task_pool.ExecuteTask(&task)?;

    Ok(())
}

/// Get frame statistics
#[cfg(target_os = "windows")]
pub fn get_render_stats() -> (u64, u64) {
    (
        FRAMES_RENDERED.load(Ordering::Relaxed),
        FRAMES_DROPPED.load(Ordering::Relaxed),
    )
}

/// Reset statistics
#[cfg(target_os = "windows")]
pub fn reset_render_stats() {
    FRAMES_RENDERED.store(0, Ordering::Relaxed);
    FRAMES_DROPPED.store(0, Ordering::Relaxed);
}
