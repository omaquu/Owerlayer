#[cfg(windows)]
pub mod wgc {
    use windows::{
        core::*,
        Graphics::Capture::*,
        Graphics::DirectX::Direct3D11::*,
        Graphics::DirectX::*,
        Win32::Graphics::Direct3D::*,
        Win32::Graphics::Direct3D11::*,
        Win32::Graphics::Dxgi::*,
        Win32::System::WinRT::Direct3D11::*,
        Win32::System::WinRT::Graphics::Capture::*,
        Win32::System::Com::*,
    };

    pub struct WgcCaptureSession {
        _item: GraphicsCaptureItem,
        frame_pool: Direct3D11CaptureFramePool,
        _session: GraphicsCaptureSession,
        d3d_device: ID3D11Device,
        d3d_context: ID3D11DeviceContext,
        staging_textures: std::sync::Mutex<std::collections::HashMap<usize, ID3D11Texture2D>>,
        shared_texture: std::sync::Mutex<Option<ID3D11Texture2D>>,
        frame_counter: std::sync::atomic::AtomicU64,
        client_last_state: std::sync::Mutex<std::collections::HashMap<usize, (u64, Option<(i32, i32, i32, i32)>)>>,
    }

    impl WgcCaptureSession {
        pub fn start_monitor_capture(monitor_handle: isize) -> Result<Self> {
            unsafe {
                // 1. Init COM
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

                // 2. Create D3D11 Device
                let mut d3d_device: Option<ID3D11Device> = None;
                let mut d3d_context: Option<ID3D11DeviceContext> = None;
                D3D11CreateDevice(
                    None,
                    D3D_DRIVER_TYPE_HARDWARE,
                    windows::Win32::Foundation::HMODULE::default(),
                    D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                    Some(&[D3D_FEATURE_LEVEL_11_0]),
                    D3D11_SDK_VERSION,
                    Some(&mut d3d_device),
                    None,
                    Some(&mut d3d_context),
                )?;
                let d3d_device = d3d_device.unwrap();
                let d3d_context = d3d_context.unwrap();

                // 3. Create WinRT Device wrapper - cast IInspectable to IDirect3DDevice
                let dxgi_device: IDXGIDevice = d3d_device.cast()?;
                let inspectable = CreateDirect3D11DeviceFromDXGIDevice(&dxgi_device)?;
                let winrt_device: IDirect3DDevice = inspectable.cast()?;

                // 4. Create Capture Item using Interop
                let interop: IGraphicsCaptureItemInterop =
                    windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
                let item: GraphicsCaptureItem =
                    interop.CreateForMonitor(windows::Win32::Graphics::Gdi::HMONITOR(monitor_handle as _))?;

                // 5. Create Frame Pool
                let size = item.Size()?;
                let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
                    &winrt_device,
                    DirectXPixelFormat::B8G8R8A8UIntNormalized,
                    2,
                    size,
                )?;

                // 6. Create Session
                let session = frame_pool.CreateCaptureSession(&item)?;
                let _ = session.SetIsBorderRequired(false);
                session.StartCapture()?;

                Ok(Self {
                    _item: item,
                    frame_pool,
                    _session: session,
                    d3d_device,
                    d3d_context,
                    staging_textures: std::sync::Mutex::new(std::collections::HashMap::new()),
                    shared_texture: std::sync::Mutex::new(None),
                    frame_counter: std::sync::atomic::AtomicU64::new(0),
                    client_last_state: std::sync::Mutex::new(std::collections::HashMap::new()),
                })
            }
        }

        pub fn start_window_capture(window_handle: isize) -> Result<Self> {
            unsafe {
                // 1. Init COM
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

                // 2. Create D3D11 Device
                let mut d3d_device: Option<ID3D11Device> = None;
                let mut d3d_context: Option<ID3D11DeviceContext> = None;
                D3D11CreateDevice(
                    None,
                    D3D_DRIVER_TYPE_HARDWARE,
                    windows::Win32::Foundation::HMODULE::default(),
                    D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                    Some(&[D3D_FEATURE_LEVEL_11_0]),
                    D3D11_SDK_VERSION,
                    Some(&mut d3d_device),
                    None,
                    Some(&mut d3d_context),
                )?;
                let d3d_device = d3d_device.unwrap();
                let d3d_context = d3d_context.unwrap();

                // 3. Create WinRT Device wrapper
                let dxgi_device: IDXGIDevice = d3d_device.cast()?;
                let inspectable = CreateDirect3D11DeviceFromDXGIDevice(&dxgi_device)?;
                let winrt_device: IDirect3DDevice = inspectable.cast()?;

                // 4. Create Capture Item using Interop
                let interop: IGraphicsCaptureItemInterop =
                    windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
                let item: GraphicsCaptureItem =
                    interop.CreateForWindow(windows::Win32::Foundation::HWND(window_handle as _))?;

                // 5. Create Frame Pool
                let size = item.Size()?;
                let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
                    &winrt_device,
                    DirectXPixelFormat::B8G8R8A8UIntNormalized,
                    2,
                    size,
                )?;

                // 6. Create Session
                let session = frame_pool.CreateCaptureSession(&item)?;
                let _ = session.SetIsBorderRequired(false);
                session.StartCapture()?;

                Ok(Self {
                    _item: item,
                    frame_pool,
                    _session: session,
                    d3d_device,
                    d3d_context,
                    staging_textures: std::sync::Mutex::new(std::collections::HashMap::new()),
                    shared_texture: std::sync::Mutex::new(None),
                    frame_counter: std::sync::atomic::AtomicU64::new(0),
                    client_last_state: std::sync::Mutex::new(std::collections::HashMap::new()),
                })
            }
        }

        pub fn poll_new_frame(&self, active_clients: &std::collections::HashSet<usize>) -> Result<bool> {
            unsafe {
                // Prune inactive clients to prevent memory growth
                {
                    let mut states = self.client_last_state.lock().unwrap();
                    states.retain(|k, _| active_clients.contains(k));
                }
                {
                    let mut stagings = self.staging_textures.lock().unwrap();
                    stagings.retain(|k, _| active_clients.contains(k));
                }

                let frame = match self.frame_pool.TryGetNextFrame() {
                    Ok(f) => f,
                    Err(_) => return Ok(false),
                };

                let mut latest_frame = frame;
                while let Ok(next_frame) = self.frame_pool.TryGetNextFrame() {
                    if next_frame.Surface().is_ok() {
                        latest_frame = next_frame;
                    } else {
                        break;
                    }
                }
                let frame = latest_frame;

                let surface = frame.Surface()?;
                let access: IDirect3DDxgiInterfaceAccess = surface.cast()?;
                let dxgi_surface: IDXGISurface = access.GetInterface()?;
                let source_texture: ID3D11Texture2D = dxgi_surface.cast()?;

                // Get source texture desc and build shared texture desc
                let mut desc = D3D11_TEXTURE2D_DESC::default();
                source_texture.GetDesc(&mut desc as *mut _);
                desc.Usage = D3D11_USAGE_DEFAULT;
                desc.BindFlags = 0;
                desc.CPUAccessFlags = 0;
                desc.MiscFlags = 0;

                // Reuse or create shared texture
                let mut shared_lock = self.shared_texture.lock().unwrap();
                let need_new = if let Some(ref tex) = *shared_lock {
                    let mut shared_desc = D3D11_TEXTURE2D_DESC::default();
                    tex.GetDesc(&mut shared_desc as *mut _);
                    shared_desc.Width != desc.Width || shared_desc.Height != desc.Height
                } else {
                    true
                };

                if need_new {
                    let mut new_tex: Option<ID3D11Texture2D> = None;
                    self.d3d_device.CreateTexture2D(&desc as *const _, None, Some(&mut new_tex))?;
                    *shared_lock = new_tex;
                }
                let shared_texture = shared_lock.as_ref().unwrap().clone();

                // Copy source to shared default texture
                self.d3d_context.CopyResource(&shared_texture, &source_texture);

                // Increment frame counter to signal a new frame is available
                self.frame_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                Ok(true)
            }
        }

        pub fn get_latest_frame(&self, client_id: usize, crop: Option<(i32, i32, i32, i32)>) -> Result<Option<(Vec<u8>, usize, usize, u128, u128, u128)>> {
            unsafe {
                let current_counter = self.frame_counter.load(std::sync::atomic::Ordering::SeqCst);
                
                // Check if this client has already processed the latest frame at the same crop coordinates
                {
                    let mut states = self.client_last_state.lock().unwrap();
                    if let Some(&(last_seen, last_crop)) = states.get(&client_id) {
                        if last_seen >= current_counter && last_crop == crop {
                            return Ok(None);
                        }
                    }
                    states.insert(client_id, (current_counter, crop));
                }

                // Get the shared texture
                let shared_lock = self.shared_texture.lock().unwrap();
                let source_texture = match &*shared_lock {
                    Some(tex) => tex.clone(),
                    None => return Ok(None),
                };

                // Get source texture desc and build staging desc
                let mut desc = D3D11_TEXTURE2D_DESC::default();
                source_texture.GetDesc(&mut desc as *mut _);
                desc.Usage = D3D11_USAGE_STAGING;
                desc.BindFlags = 0;
                desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
                desc.MiscFlags = 0;

                let mut crop_width = desc.Width as usize;
                let mut crop_height = desc.Height as usize;
                let mut start_x = 0;
                let mut start_y = 0;
                let mut end_x = desc.Width as usize;
                let mut end_y = desc.Height as usize;

                let use_crop = if let Some((cx, cy, cw, ch)) = crop {
                    let cx_clamped = cx.clamp(0, desc.Width as i32) as usize;
                    let cy_clamped = cy.clamp(0, desc.Height as i32) as usize;
                    let end_x_clamped = (cx + cw).clamp(0, desc.Width as i32) as usize;
                    let end_y_clamped = (cy + ch).clamp(0, desc.Height as i32) as usize;
                    
                    if end_x_clamped > cx_clamped && end_y_clamped > cy_clamped {
                        start_x = cx_clamped;
                        start_y = cy_clamped;
                        end_x = end_x_clamped;
                        end_y = end_y_clamped;
                        crop_width = end_x - start_x;
                        crop_height = end_y - start_y;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                };

                let target_w = if let Some((_, _, cw, _ch)) = crop { cw as usize } else { crop_width };
                let target_h = if let Some((_, _, _cw, ch)) = crop { ch as usize } else { crop_height };

                desc.Width = crop_width as u32;
                desc.Height = crop_height as u32;

                // Reuse or create staging texture for this client
                let staging_texture = {
                    let mut stagings = self.staging_textures.lock().unwrap();
                    let need_new = if let Some(tex) = stagings.get(&client_id) {
                        let mut staging_desc = D3D11_TEXTURE2D_DESC::default();
                        tex.GetDesc(&mut staging_desc as *mut _);
                        staging_desc.Width < crop_width as u32 || staging_desc.Height < crop_height as u32
                    } else {
                        true
                    };

                    if need_new {
                        let mut padded_desc = desc;
                        padded_desc.Width = ((crop_width as f32) * 1.25).ceil() as u32;
                        padded_desc.Height = ((crop_height as f32) * 1.25).ceil() as u32;
                        let mut new_tex: Option<ID3D11Texture2D> = None;
                        self.d3d_device.CreateTexture2D(&padded_desc as *const _, None, Some(&mut new_tex))?;
                        stagings.insert(client_id, new_tex.unwrap());
                    }
                    stagings.get(&client_id).unwrap().clone()
                };

                let gpu_copy_start = std::time::Instant::now();
                if use_crop {
                    let box_region = D3D11_BOX {
                        left: start_x as u32,
                        top: start_y as u32,
                        front: 0,
                        right: end_x as u32,
                        bottom: end_y as u32,
                        back: 1,
                    };
                    let dst: ID3D11Resource = staging_texture.cast()?;
                    let src: ID3D11Resource = source_texture.cast()?;
                    self.d3d_context.CopySubresourceRegion(
                        &dst,
                        0,
                        0,
                        0,
                        0,
                        &src,
                        0,
                        Some(&box_region),
                    );
                } else {
                    self.d3d_context.CopyResource(&staging_texture, &source_texture);
                }
                let gpu_copy_time = gpu_copy_start.elapsed().as_micros();

                let map_start = std::time::Instant::now();
                let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
                self.d3d_context.Map(
                    &staging_texture,
                    0,
                    D3D11_MAP_READ,
                    0,
                    Some(&mut mapped),
                )?;
                let map_wait_time = map_start.elapsed().as_micros();

                let width = crop_width;
                let height = crop_height;
                let pitch = mapped.RowPitch as usize;

                let swap_start = std::time::Instant::now();

                let needs_padding = target_w != width || target_h != height;
                let (out_pixels, out_w, out_h) = if needs_padding {
                    let mut pixels = vec![0u8; target_w * target_h * 4];
                    let src_data = std::slice::from_raw_parts(mapped.pData as *const u8, pitch * height);
                    let dst_offset_x = if let Some((cx, _, _, _)) = crop {
                        if cx < 0 { (-cx) as usize } else { 0 }
                    } else { 0 };
                    let dst_offset_y = if let Some((_, cy, _, _)) = crop {
                        if cy < 0 { (-cy) as usize } else { 0 }
                    } else { 0 };

                    use rayon::prelude::*;
                    let row_copy_bytes = width.min(target_w.saturating_sub(dst_offset_x)) * 4;
                    pixels
                        .par_chunks_mut(target_w * 4)
                        .skip(dst_offset_y)
                        .take(height)
                        .enumerate()
                        .for_each(|(y, dst_row)| {
                            let src_row = &src_data[y * pitch..y * pitch + width * 4];
                            let dst_start = dst_offset_x * 4;
                            let dst_end = dst_start + row_copy_bytes;
                            dst_row[dst_start..dst_end].copy_from_slice(&src_row[..row_copy_bytes]);
                        });
                    (pixels, target_w, target_h)
                } else {
                    let mut pixels = vec![0u8; width * height * 4];
                    let src_data = std::slice::from_raw_parts(mapped.pData as *const u8, pitch * height);

                    use rayon::prelude::*;
                    let row_width_bytes = width * 4;
                    pixels
                        .par_chunks_mut(row_width_bytes)
                        .enumerate()
                        .for_each(|(y, dst_row)| {
                            let src_row = &src_data[y * pitch..y * pitch + row_width_bytes];
                            dst_row.copy_from_slice(src_row);
                        });
                    (pixels, width, height)
                };
                let pixel_swap_time = swap_start.elapsed().as_micros();

                self.d3d_context.Unmap(&staging_texture, 0);

                Ok(Some((out_pixels, out_w, out_h, gpu_copy_time, map_wait_time, pixel_swap_time)))
            }
        }
    }
}
