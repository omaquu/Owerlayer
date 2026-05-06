#[cfg(windows)]
pub mod wgc {
    use std::sync::{Arc, Mutex};
    use windows::{
        core::*,
        Graphics::Capture::*,
        Graphics::DirectX::Direct3D11::*,
        Graphics::DirectX::*,
        Win32::Graphics::Direct3D11::*,
        Win32::Graphics::Dxgi::*,
        Win32::System::WinRT::Direct3D11::*,
        Win32::System::WinRT::Graphics::Capture::*,
        Win32::System::Com::*,
    };

    // WGC Implementation is quite involved, requiring D3D11 device setup,
    // Frame pools, and event handlers.
    // This is a minimal skeleton to get started.

    pub struct WgcCaptureSession {
        item: GraphicsCaptureItem,
        frame_pool: Direct3D11CaptureFramePool,
        session: GraphicsCaptureSession,
        d3d_device: ID3D11Device,
        d3d_context: ID3D11DeviceContext,
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
                    None,
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
                let winrt_device = CreateDirect3D11DeviceFromDXGIDevice(&dxgi_device)?;

                // 4. Create Capture Item using Interop
                let interop: IGraphicsCaptureItemInterop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
                let item: GraphicsCaptureItem = interop.CreateForMonitor(windows::Win32::Graphics::Gdi::HMONITOR(monitor_handle as _))?;

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
                session.StartCapture()?;

                Ok(Self {
                    item,
                    frame_pool,
                    session,
                    d3d_device,
                    d3d_context,
                })
            }
        }

        pub fn get_latest_frame(&self) -> Result<Option<(Vec<u8>, usize, usize)>> {
            unsafe {
                let frame = match self.frame_pool.TryGetNextFrame()? {
                    Some(f) => f,
                    None => return Ok(None),
                };

                let surface = frame.Surface()?;
                let access: IDirect3DDxgiInterfaceAccess = surface.cast()?;
                let dxgi_surface: IDXGISurface = access.GetInterface()?;
                let source_texture: ID3D11Texture2D = dxgi_surface.cast()?;

                // Create staging texture to read CPU pixels
                let mut desc = D3D11_TEXTURE2D_DESC::default();
                source_texture.GetDesc(&mut desc);
                desc.Usage = D3D11_USAGE_STAGING;
                desc.BindFlags = 0;
                desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
                desc.MiscFlags = 0;

                let mut staging_texture = None;
                self.d3d_device.CreateTexture2D(&desc, None, Some(&mut staging_texture))?;
                let staging_texture = staging_texture.unwrap();

                self.d3d_context.CopyResource(&staging_texture, &source_texture);

                let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
                self.d3d_context.Map(
                    &staging_texture,
                    0,
                    D3D11_MAP_READ,
                    0,
                    Some(&mut mapped),
                )?;

                let width = desc.Width as usize;
                let height = desc.Height as usize;
                let pitch = mapped.RowPitch as usize;

                let mut pixels = vec![0u8; width * height * 4];
                let src_data = std::slice::from_raw_parts(mapped.pData as *const u8, pitch * height);

                for y in 0..height {
                    let src_row = &src_data[y * pitch .. y * pitch + width * 4];
                    let dst_row = &mut pixels[y * width * 4 .. (y + 1) * width * 4];
                    dst_row.copy_from_slice(src_row);
                    
                    // BGRA to RGBA conversion
                    for x in 0..width {
                        let i = x * 4;
                        let b = dst_row[i];
                        dst_row[i] = dst_row[i + 2];
                        dst_row[i + 2] = b;
                    }
                }

                self.d3d_context.Unmap(&staging_texture, 0);

                Ok(Some((pixels, width, height)))
            }
        }
    }
}
