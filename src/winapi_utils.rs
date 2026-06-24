#[cfg(windows)]
use windows_sys::Win32::Foundation::{POINT, RECT};
#[cfg(windows)]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN,
    SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
    WindowFromPoint, GetAncestor, GA_ROOT, GetWindowTextW,
    GetWindowRect, IsWindowVisible, SetWindowDisplayAffinity, SetForegroundWindow,
    EnumWindows, ShowWindow, SW_HIDE, SW_SHOW,
};
use windows_sys::Win32::Graphics::Gdi::{
    GetDC, ReleaseDC, CreateCompatibleDC, CreateCompatibleBitmap, SelectObject,
    DeleteObject, DeleteDC, BitBlt, GetDIBits, GetWindowDC,
    SRCCOPY, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
};
use windows_sys::Win32::System::DataExchange::{
    OpenClipboard, CloseClipboard, GetClipboardData, IsClipboardFormatAvailable,
    EmptyClipboard, SetClipboardData,
};
use windows_sys::Win32::System::Memory::{GlobalLock, GlobalUnlock, GlobalAlloc, GMEM_MOVEABLE};

#[cfg(windows)]
pub fn force_dpi_aware() {
    use windows_sys::Win32::UI::HiDpi::SetProcessDpiAwarenessContext;
    unsafe {
        // DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE (V1) - less aggressive FSO trigger than V2
        let _ = SetProcessDpiAwarenessContext(-3isize as _);
    }
}

#[cfg(windows)]
extern "system" {
    pub fn PrintWindow(hwnd: isize, hdc: isize, nflags: u32) -> i32;
}

#[cfg(windows)]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;

use std::sync::atomic::{AtomicPtr, Ordering};
use std::ptr;

static OVERLAY_HWND: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(ptr::null_mut());

/// Get screen resolution. If multi_monitor is true, returns virtual screen size.
#[cfg(windows)]
pub fn get_screen_size(multi_monitor: bool) -> (f32, f32) {
    if multi_monitor {
        unsafe { (GetSystemMetrics(SM_CXVIRTUALSCREEN) as f32, GetSystemMetrics(SM_CYVIRTUALSCREEN) as f32) }
    } else {
        unsafe { (GetSystemMetrics(SM_CXSCREEN) as f32, GetSystemMetrics(SM_CYSCREEN) as f32) }
    }
}

#[cfg(not(windows))]
pub fn get_screen_size(_multi_monitor: bool) -> (f32, f32) { (1920.0, 1080.0) }

/// Get virtual screen origin (X, Y).
#[cfg(windows)]
pub fn get_virtual_origin() -> (f32, f32) {
    unsafe { (GetSystemMetrics(SM_XVIRTUALSCREEN) as f32, GetSystemMetrics(SM_YVIRTUALSCREEN) as f32) }
}

#[cfg(not(windows))]
pub fn get_virtual_origin() -> (f32, f32) { (0.0, 0.0) }

#[cfg(windows)]
pub fn get_monitor_rects() -> Vec<RECT> {
    use windows_sys::Win32::Graphics::Gdi::EnumDisplayMonitors;
    use windows_sys::Win32::Foundation::{BOOL, LPARAM};
// use windows_sys::Win32::Graphics::Gdi::HDC;

    unsafe extern "system" fn monitor_enum(
        _hmonitor: windows_sys::Win32::Graphics::Gdi::HMONITOR,
        _hdc: windows_sys::Win32::Graphics::Gdi::HDC,
        lprect: *mut RECT,
        lparam: LPARAM,
    ) -> BOOL {
        let rects = &mut *(lparam as *mut Vec<RECT>);
        if !lprect.is_null() {
            rects.push(*lprect);
        }
        1
    }

    let mut rects = Vec::new();
    unsafe {
        EnumDisplayMonitors(
            std::ptr::null_mut(),
            std::ptr::null(),
            Some(monitor_enum),
            &mut rects as *mut _ as LPARAM,
        );
    }
    rects
}

#[cfg(not(windows))]
pub fn get_monitor_rects() -> Vec<()> { Vec::new() }

/// Poll mouse position and left-button state via WinAPI.
/// Works regardless of window focus or hit-testing.
#[cfg(windows)]
pub fn poll_mouse() -> (f32, f32, bool) {
    unsafe {
        let mut pt = POINT { x: 0, y: 0 };
        GetCursorPos(&mut pt);
        let lb = GetAsyncKeyState(0x01) & (0x8000u16 as i16) != 0;
        (pt.x as f32, pt.y as f32, lb)
    }
}

#[cfg(not(windows))]
pub fn poll_mouse() -> (f32, f32, bool) { (0.0, 0.0, false) }

#[cfg(windows)]
#[allow(dead_code)]
pub fn window_from_point(x: i32, y: i32) -> Option<isize> {
    unsafe {
        let pt = POINT { x, y };
        let hwnd = WindowFromPoint(pt);
        if hwnd.is_null() { None } else { Some(hwnd as isize) }
    }
}

#[cfg(windows)]
pub fn get_primary_monitor_scale() -> f32 {
    use windows_sys::Win32::Graphics::Gdi::{MonitorFromPoint, MONITOR_DEFAULTTOPRIMARY};
    use windows_sys::Win32::UI::HiDpi::GetDpiForMonitor;
    unsafe {
        let pt = POINT { x: 0, y: 0 };
        let h_monitor = MonitorFromPoint(pt, MONITOR_DEFAULTTOPRIMARY);
        let mut dpi_x = 0;
        let mut dpi_y = 0;
        let _ = GetDpiForMonitor(h_monitor, 0, &mut dpi_x, &mut dpi_y);
        if dpi_x == 0 { 1.0 } else { dpi_x as f32 / 96.0 }
    }
}

#[cfg(windows)]
pub fn get_monitor_height_at_point(x: i32, y: i32) -> f32 {
    use windows_sys::Win32::Graphics::Gdi::{MonitorFromPoint, GetMonitorInfoW, MONITORINFO, MONITOR_DEFAULTTONEAREST};
    unsafe {
        let pt = POINT { x, y };
        let h_monitor = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(h_monitor, &mut info as *mut _ as *mut _) != 0 {
            (info.rcMonitor.bottom - info.rcMonitor.top) as f32
        } else {
            1080.0
        }
    }
}

#[cfg(windows)]
pub fn get_max_monitor_height() -> f32 {
    use windows_sys::Win32::Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, MONITORINFO, HMONITOR, HDC};
    use windows_sys::Win32::Foundation::{RECT, LPARAM, BOOL};

    unsafe extern "system" fn enum_proc(h_mon: HMONITOR, _: HDC, _: *mut RECT, l_param: LPARAM) -> BOOL {
        let max_h = &mut *(l_param as *mut f32);
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(h_mon, &mut info as *mut _ as *mut _) != 0 {
            let h = (info.rcMonitor.bottom - info.rcMonitor.top) as f32;
            if h > *max_h { *max_h = h; }
        }
        1
    }
    let mut max_h = 1080.0f32;
    unsafe { EnumDisplayMonitors(std::ptr::null_mut(), std::ptr::null(), Some(enum_proc), &mut max_h as *mut _ as LPARAM); }
    max_h
}

#[cfg(not(windows))]
pub fn get_primary_monitor_scale() -> f32 { 1.0 }

#[cfg(windows)]
pub fn get_monitor_size_pos(index: usize) -> (f32, f32, f32, f32) {
    use windows_sys::Win32::Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, MONITORINFO, HMONITOR, HDC};
    use windows_sys::Win32::Foundation::{RECT, LPARAM, BOOL};

    struct MonData { target: usize, current: usize, found: Option<(f32, f32, f32, f32)> }
    unsafe extern "system" fn enum_proc(h_mon: HMONITOR, _: HDC, _: *mut RECT, l_param: LPARAM) -> BOOL {
        let data = &mut *(l_param as *mut MonData);
        if data.current == data.target {
            let mut info: MONITORINFO = std::mem::zeroed();
            info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
            GetMonitorInfoW(h_mon, &mut info as *mut _ as *mut _);
            data.found = Some((
                (info.rcMonitor.right - info.rcMonitor.left) as f32,
                (info.rcMonitor.bottom - info.rcMonitor.top) as f32,
                info.rcMonitor.left as f32,
                info.rcMonitor.top as f32
            ));
            0 // stop
        } else {
            data.current += 1;
            1 // continue
        }
    }

    let mut data = MonData { target: index, current: 0, found: None };
    unsafe { EnumDisplayMonitors(std::ptr::null_mut(), std::ptr::null(), Some(enum_proc), &mut data as *mut _ as LPARAM); }
    data.found.unwrap_or((1920.0, 1080.0, 0.0, 0.0))
}

#[allow(dead_code)]
pub fn get_window_rect(hwnd: isize) -> RECT {
    unsafe {
        let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        GetWindowRect(hwnd as *mut std::ffi::c_void, &mut rect);
        rect
    }
}

// ──────────────────────────────────────────────────────────────
//  Overlay HWND management
// ──────────────────────────────────────────────────────────────

/// Find the Owerlayer window by title, cache its HWND, and
/// re-assert HWND_TOPMOST. Call once on the first frame.
#[cfg(windows)]
pub fn get_clipboard_text() -> Option<String> {
    unsafe {
        if OpenClipboard(std::ptr::null_mut()) == 0 { return None; }
        let format_available = IsClipboardFormatAvailable(1); // CF_TEXT = 1
        let mut result = None;
        if format_available != 0 {
            let h_data = GetClipboardData(13); // CF_UNICODETEXT = 13
            if !h_data.is_null() {
                let ptr = GlobalLock(h_data);
                if !ptr.is_null() {
                    let mut len = 0;
                    let ptr_u16 = ptr as *const u16;
                    while *ptr_u16.add(len) != 0 { len += 1; }
                    let slice = std::slice::from_raw_parts(ptr_u16, len);
                    result = Some(String::from_utf16_lossy(slice));
                    GlobalUnlock(h_data);
                }
            }
        }
        CloseClipboard();
        result
    }
}

#[cfg(not(windows))]
pub fn get_clipboard_text() -> Option<String> { None }

#[cfg(windows)]
pub fn copy_image_to_clipboard(pixels: &[u8], width: usize, height: usize) {
    unsafe {
        if OpenClipboard(std::ptr::null_mut()) == 0 { return; }
        EmptyClipboard();
        
        let row_pitch = width * 4;
        let image_size = row_pitch * height;
        let total_size = std::mem::size_of::<BITMAPINFOHEADER>() + image_size;
        
        let h_mem = GlobalAlloc(GMEM_MOVEABLE, total_size);
        if !h_mem.is_null() {
            let ptr = GlobalLock(h_mem);
            if !ptr.is_null() {
                let header = BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width as i32,
                    biHeight: height as i32,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: 0,
                    biSizeImage: image_size as u32,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                };
                
                std::ptr::copy_nonoverlapping(
                    &header as *const BITMAPINFOHEADER as *const u8,
                    ptr as *mut u8,
                    std::mem::size_of::<BITMAPINFOHEADER>(),
                );
                
                let dest_pixels = (ptr as *mut u8).add(std::mem::size_of::<BITMAPINFOHEADER>());
                for y in 0..height {
                    let src_y = height - 1 - y;
                    let src_row_start = src_y * row_pitch;
                    let dest_row_start = y * row_pitch;
                    for x in 0..width {
                        let src_idx = src_row_start + x * 4;
                        let dest_idx = dest_row_start + x * 4;
                        
                        let r = pixels[src_idx];
                        let g = pixels[src_idx + 1];
                        let b = pixels[src_idx + 2];
                        let a = pixels[src_idx + 3];
                        
                        *dest_pixels.add(dest_idx) = b;
                        *dest_pixels.add(dest_idx + 1) = g;
                        *dest_pixels.add(dest_idx + 2) = r;
                        *dest_pixels.add(dest_idx + 3) = a;
                    }
                }
                
                GlobalUnlock(h_mem);
                SetClipboardData(8, h_mem); // CF_DIB = 8
            }
        }
        CloseClipboard();
    }
}

#[cfg(not(windows))]
pub fn copy_image_to_clipboard(_pixels: &[u8], _width: usize, _height: usize) {}

#[cfg(windows)]
pub fn get_clipboard_image() -> Option<(Vec<u8>, usize, usize)> {
    unsafe {
        if OpenClipboard(std::ptr::null_mut()) == 0 { return None; }
        
        let mut result = None;
        if IsClipboardFormatAvailable(8) != 0 { // CF_DIB = 8
            let h_data = GetClipboardData(8);
            if !h_data.is_null() {
                let ptr = GlobalLock(h_data);
                if !ptr.is_null() {
                    let header_ptr = ptr as *const BITMAPINFOHEADER;
                    let header = &*header_ptr;
                    
                    let width = header.biWidth as usize;
                    let height = header.biHeight.abs() as usize;
                    let is_top_down = header.biHeight < 0;
                    
                    if width > 0 && height > 0 {
                        let bit_count = header.biBitCount;
                        if bit_count == 24 || bit_count == 32 {
                            let mut rgba_pixels = vec![255u8; width * height * 4];
                            let pixel_data_ptr = (ptr as *const u8).add(header.biSize as usize);
                            
                            let bytes_per_pixel = (bit_count / 8) as usize;
                            let row_pitch = (width * bytes_per_pixel + 3) & !3;
                            
                            for y in 0..height {
                                let src_y = if is_top_down { y } else { height - 1 - y };
                                let src_row_start = src_y * row_pitch;
                                let dest_row_start = y * width * 4;
                                
                                for x in 0..width {
                                    let src_idx = src_row_start + x * bytes_per_pixel;
                                    let dest_idx = dest_row_start + x * 4;
                                    
                                    if bit_count == 32 {
                                        let b = *pixel_data_ptr.add(src_idx);
                                        let g = *pixel_data_ptr.add(src_idx + 1);
                                        let r = *pixel_data_ptr.add(src_idx + 2);
                                        let a = *pixel_data_ptr.add(src_idx + 3);
                                        
                                        rgba_pixels[dest_idx] = r;
                                        rgba_pixels[dest_idx + 1] = g;
                                        rgba_pixels[dest_idx + 2] = b;
                                        rgba_pixels[dest_idx + 3] = a;
                                    } else {
                                        let b = *pixel_data_ptr.add(src_idx);
                                        let g = *pixel_data_ptr.add(src_idx + 1);
                                        let r = *pixel_data_ptr.add(src_idx + 2);
                                        
                                        rgba_pixels[dest_idx] = r;
                                        rgba_pixels[dest_idx + 1] = g;
                                        rgba_pixels[dest_idx + 2] = b;
                                        rgba_pixels[dest_idx + 3] = 255;
                                    }
                                }
                            }
                            result = Some((rgba_pixels, width, height));
                        }
                    }
                    GlobalUnlock(h_data);
                }
            }
        }
        CloseClipboard();
        result
    }
}

#[cfg(not(windows))]
pub fn get_clipboard_image() -> Option<(Vec<u8>, usize, usize)> { None }

#[cfg(windows)]
pub fn setup_overlay_window(fso_fix: bool) {
    use windows_sys::Win32::Foundation::{HWND, LPARAM, BOOL};
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    unsafe {
        let mut hwnd: HWND = ptr::null_mut();
        let pid = windows_sys::Win32::System::Threading::GetCurrentProcessId();
        
        unsafe extern "system" fn enum_proc(h: HWND, lp: LPARAM) -> BOOL {
            let data = &mut *(lp as *mut (u32, HWND));
            let mut window_pid = 0;
            GetWindowThreadProcessId(h, &mut window_pid);
            if window_pid == data.0 {
                let mut title = [0u16; 128];
                let len = GetWindowTextW(h, title.as_mut_ptr(), 128);
                let title_str = String::from_utf16_lossy(&title[..len as usize]);
                if title_str == "Owerlayer" {
                    data.1 = h;
                    return 0; // stop
                }
            }
            1 // continue
        }

        let mut data: (u32, HWND) = (pid, ptr::null_mut());
        EnumWindows(Some(enum_proc), &mut data as *mut _ as LPARAM);
        hwnd = data.1;

        if hwnd.is_null() { return; }
        OVERLAY_HWND.store(hwnd, Ordering::Relaxed);

        // Set styles to encourage Windows to treat this as a transparent overlay, not a fullscreen app
        let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        
        // Remove caption, frame, etc.
        let new_style = style & !(WS_CAPTION | WS_THICKFRAME | WS_MINIMIZEBOX | WS_MAXIMIZEBOX | WS_SYSMENU | WS_CLIPCHILDREN | WS_CLIPSIBLINGS);
        // Do NOT add WS_EX_TOOLWINDOW — it hides the window from OBS's window capture list.
        // Keep WS_EX_APPWINDOW to ensure OBS can enumerate and capture this window.
        let mut new_ex_style = (ex_style & !WS_EX_TOOLWINDOW) | WS_EX_APPWINDOW;
        
        SetWindowLongW(hwnd, GWL_STYLE, new_style as i32);

        let is_dcomp = (ex_style & 0x00400000) != 0;
        if is_dcomp && fso_fix {
            // DirectComposition is active and FSO fix is enabled (window is slightly offset to prevent FSO).
            // Do NOT add WS_EX_LAYERED to avoid OBS capture freeze and DWM composition conflicts.
            let final_ex_style = new_ex_style & !WS_EX_LAYERED;
            SetWindowLongW(hwnd, GWL_EXSTYLE, final_ex_style as i32);
        } else {
            // Either not DirectComposition, or FSO fix is disabled.
            // Add WS_EX_LAYERED and set layered window attributes.
            new_ex_style |= WS_EX_LAYERED;
            SetWindowLongW(hwnd, GWL_EXSTYLE, new_ex_style as i32);
            SetLayeredWindowAttributes(hwnd, 0, 255, LWA_ALPHA);
        }

        // Ensure always-on-top
        SetWindowPos(
            hwnd, HWND_TOPMOST,
            0, 0, 0, 0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
    }
}

#[cfg(not(windows))]
pub fn setup_overlay_window(_fso_fix: bool) {}

#[cfg(windows)]
pub fn reposition_overlay_window(x: i32, y: i32, w: i32, h: i32) {
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    let hwnd = OVERLAY_HWND.load(Ordering::Relaxed);
    if !hwnd.is_null() {
        unsafe {
            SetWindowPos(
                hwnd, ptr::null_mut(),
                x, y, w, h,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
    }
}

#[cfg(not(windows))]
pub fn reposition_overlay_window(_x: i32, _y: i32, _w: i32, _h: i32) {}

/// Re-assert always-on-top positioning so other windows
/// cannot push the overlay behind them.
#[cfg(windows)]
#[allow(dead_code)]
pub fn force_topmost() {
    set_topmost(true);
}

#[cfg(windows)]
pub fn set_topmost(enabled: bool) {
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    let hwnd = OVERLAY_HWND.load(Ordering::Relaxed);
    if !hwnd.is_null() {
        unsafe {
            let order = if enabled { HWND_TOPMOST } else { HWND_NOTOPMOST };
            SetWindowPos(
                hwnd, order,
                0, 0, 0, 0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
    }
}

#[cfg(not(windows))]
pub fn force_topmost() {}

#[cfg(not(windows))]
pub fn set_topmost(_enabled: bool) {}

/// Set window capture exclusion state.
/// enabled = true: Hides from OBS/Mirror (0x11)
/// enabled = false: Visible to everyone (0x00)
#[cfg(windows)]
pub fn set_capture_exclusion(enabled: bool) {
    let hwnd = OVERLAY_HWND.load(Ordering::Relaxed);
    if !hwnd.is_null() {
        unsafe {
            let affinity = if enabled { 0x00000011 } else { 0x00000000 };
            SetWindowDisplayAffinity(hwnd, affinity);
        }
    }
}

#[cfg(not(windows))]
pub fn set_capture_exclusion(_enabled: bool) {}

/// Sets window visibility.
#[cfg(windows)]
pub fn set_window_visibility(visible: bool) {
    let hwnd = OVERLAY_HWND.load(Ordering::Relaxed);
    if !hwnd.is_null() {
        unsafe {
            ShowWindow(hwnd, if visible { SW_SHOW } else { SW_HIDE });
        }
    }
}

#[cfg(not(windows))]
pub fn set_window_visibility(_visible: bool) {}

/// Force the overlay to become the foreground window so it
/// can receive keyboard input (used when entering edit mode).
#[cfg(windows)]
#[allow(dead_code)]
pub fn force_focus() {
    let hwnd = OVERLAY_HWND.load(Ordering::Relaxed);
    if !hwnd.is_null() {
        unsafe {
            SetForegroundWindow(hwnd);
        }
    }
}

#[cfg(not(windows))]
pub fn force_focus() {}

/// Captures a rectangle of the desktop screen and returns RGBA bytes.
pub fn capture_screen_rect(x: i32, y: i32, width: i32, height: i32) -> Option<Vec<u8>> {
    if width <= 0 || height <= 0 || width > 8192 || height > 8192 { return None; }
    
    unsafe {
        let screen_dc = GetDC(std::ptr::null_mut());
        if screen_dc.is_null() { return None; }
        
        let mem_dc = CreateCompatibleDC(screen_dc);
        if mem_dc.is_null() {
            ReleaseDC(std::ptr::null_mut(), screen_dc);
            return None;
        }
        
        let bitmap = CreateCompatibleBitmap(screen_dc, width, height);
        if bitmap.is_null() {
            DeleteDC(mem_dc);
            ReleaseDC(std::ptr::null_mut(), screen_dc);
            return None;
        }
        
        let old_obj = SelectObject(mem_dc, bitmap);
        
        // Copy pixels from screen to memory DC
        let success = BitBlt(mem_dc, 0, 0, width, height, screen_dc, x, y, SRCCOPY);
        
        if success == 0 {
            SelectObject(mem_dc, old_obj);
            DeleteObject(bitmap);
            DeleteDC(mem_dc);
            ReleaseDC(std::ptr::null_mut(), screen_dc);
            return None;
        }
        
        // Prepare to read bits
        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = width;
        bmi.bmiHeader.biHeight = -height; // Negative means top-down DIB
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB as u32;
        
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        
        let lines = GetDIBits(
            mem_dc,
            bitmap,
            0,
            height as u32,
            pixels.as_mut_ptr() as *mut _,
            &mut bmi,
            DIB_RGB_COLORS,
        );
        
        SelectObject(mem_dc, old_obj);
        DeleteObject(bitmap);
        DeleteDC(mem_dc);
        ReleaseDC(std::ptr::null_mut(), screen_dc);
        
        if lines == 0 { return None; }
        
        if pixels.len() > 200_000 {
            use rayon::prelude::*;
            pixels.par_chunks_exact_mut(4).for_each(|chunk| {
                let b = chunk[0];
                let g = chunk[1];
                let r = chunk[2];
                let a = 255;
                chunk[0] = r;
                chunk[1] = g;
                chunk[2] = b;
                chunk[3] = a;
            });
        } else {
            for chunk in pixels.chunks_exact_mut(4) {
                let b = chunk[0];
                let g = chunk[1];
                let r = chunk[2];
                let a = 255;
                chunk[0] = r;
                chunk[1] = g;
                chunk[2] = b;
                chunk[3] = a;
            }
        }
        
        Some(pixels)
    }
}

// ──────────────────────────────────────────────────────────────
//  Window capture
// ──────────────────────────────────────────────────────────────

/// Returns the top-level window under the given screen pixel coordinates.
/// Returns (hwnd, title, [x, y, w, h]) in screen pixels.
#[cfg(windows)]
pub fn get_window_at_point(sx: i32, sy: i32) -> Option<(usize, String, [i32; 4])> {
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetWindow, GetTopWindow, GW_HWNDNEXT,
        };
        let overlay = OVERLAY_HWND.load(Ordering::Relaxed);
        let mut hwnd = GetTopWindow(std::ptr::null_mut());
        while !hwnd.is_null() {
            if hwnd != overlay && IsWindowVisible(hwnd) != 0 {
                let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
                GetWindowRect(hwnd, &mut r);
                if sx >= r.left && sx < r.right && sy >= r.top && sy < r.bottom {
                    let mut buf = [0u16; 256];
                    let len = GetWindowTextW(hwnd, buf.as_mut_ptr(), 256);
                    if len > 0 {
                        let title = String::from_utf16_lossy(&buf[..len as usize]);
                        let w = r.right - r.left;
                        let h = r.bottom - r.top;
                        return Some((hwnd as usize, title, [r.left, r.top, w, h]));
                    }
                }
            }
            hwnd = GetWindow(hwnd, GW_HWNDNEXT);
        }
        None
    }
}

#[cfg(not(windows))]
pub fn get_window_at_point(_sx: i32, _sy: i32) -> Option<(usize, String, [i32; 4])> { None }

/// Captures a specific window's content as RGBA bytes.
/// Uses PrintWindow(PW_RENDERFULLCONTENT) which works for hardware-accelerated
/// apps (browsers, games) even if the window is occluded or off-screen.
/// Returns (pixels_rgba, width, height).
#[cfg(windows)]
pub fn capture_window(hwnd: usize) -> Option<(Vec<u8>, usize, usize)> {
    unsafe {
        let hwnd_ptr = hwnd as *mut std::ffi::c_void;

        // Re-read window rect (window may have moved/resized)
        let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        GetWindowRect(hwnd_ptr, &mut r);
        let w = (r.right - r.left) as i32;
        let h = (r.bottom - r.top) as i32;
        if w <= 0 || h <= 0 { return None; }

        // Get the window's own DC (includes non-client area like title bar)
        let win_dc = GetWindowDC(hwnd_ptr);
        if win_dc.is_null() { return None; }

        // Create an off-screen compatible bitmap
        let mem_dc = CreateCompatibleDC(win_dc);
        if mem_dc.is_null() { ReleaseDC(hwnd_ptr, win_dc); return None; }
        let bitmap = CreateCompatibleBitmap(win_dc, w, h);
        if bitmap.is_null() { DeleteDC(mem_dc); ReleaseDC(hwnd_ptr, win_dc); return None; }

        let old = SelectObject(mem_dc, bitmap);

        // Copy from window DC → memory DC using PrintWindow (Windows 10 PW_RENDERFULLCONTENT = 2)
        PrintWindow(hwnd_ptr as isize, mem_dc as isize, 2);

        // Read pixels from bitmap
        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = w;
        bmi.bmiHeader.biHeight = -h; // top-down DIB
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB as u32;

        let mut pixels = vec![0u8; (w * h * 4) as usize];
        let lines = GetDIBits(mem_dc, bitmap, 0, h as u32,
            pixels.as_mut_ptr() as *mut _, &mut bmi, DIB_RGB_COLORS);

        SelectObject(mem_dc, old);
        DeleteObject(bitmap);
        DeleteDC(mem_dc);
        ReleaseDC(hwnd_ptr, win_dc);

        if lines == 0 { return None; }

        if pixels.len() > 200_000 {
            use rayon::prelude::*;
            pixels.par_chunks_exact_mut(4).for_each(|chunk| {
                let b = chunk[0];
                chunk[0] = chunk[2]; // R
                chunk[2] = b;        // B
                chunk[3] = 255;      // Force opaque
            });
        } else {
            for chunk in pixels.chunks_exact_mut(4) {
                let b = chunk[0];
                chunk[0] = chunk[2]; // R
                chunk[2] = b;        // B
                chunk[3] = 255;      // Force opaque
            }
        }

        Some((pixels, w as usize, h as usize))
    }
}

#[cfg(windows)]
pub fn capture_window_rect(hwnd: usize, x: i32, y: i32, width: i32, height: i32) -> Option<Vec<u8>> {
    let (p, w, h) = capture_window(hwnd)?;
    unsafe {
        let hwnd_ptr = hwnd as *mut std::ffi::c_void;
        let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        GetWindowRect(hwnd_ptr, &mut r);
        
        let rx = x - r.left;
        let ry = y - r.top;
        
        let mut cropped = vec![0u8; (width * height * 4) as usize];
        for dy in 0..height {
            let src_y = ry + dy;
            if src_y >= 0 && src_y < h as i32 {
                for dx in 0..width {
                    let src_x = rx + dx;
                    if src_x >= 0 && src_x < w as i32 {
                        let src_idx = (src_y as usize * w + src_x as usize) * 4;
                        let dst_idx = (dy as usize * width as usize + dx as usize) * 4;
                        if src_idx + 3 < p.len() && dst_idx + 3 < cropped.len() {
                            cropped[dst_idx..dst_idx+4].copy_from_slice(&p[src_idx..src_idx+4]);
                        }
                    }
                }
            }
        }
        Some(cropped)
    }
}

#[cfg(not(windows))]
pub fn capture_window(_hwnd: usize) -> Option<(Vec<u8>, usize, usize)> { None }

#[cfg(not(windows))]
pub fn capture_window_rect(_hwnd: usize, _x: i32, _y: i32, _width: i32, _height: i32) -> Option<Vec<u8>> { None }

#[cfg(windows)]
pub fn get_window_screen_pos() -> (i32, i32) {
    let hwnd = OVERLAY_HWND.load(Ordering::Relaxed);
    if !hwnd.is_null() {
        let mut r = windows_sys::Win32::Foundation::RECT { left: 0, top: 0, right: 0, bottom: 0 };
        unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect(hwnd, &mut r); }
        (r.left, r.top)
    } else {
        (0, 0)
    }
}

#[cfg(windows)]
pub fn get_window_screen_pos_opt() -> Option<(i32, i32)> {
    let hwnd = OVERLAY_HWND.load(Ordering::Relaxed);
    if !hwnd.is_null() {
        let mut r = windows_sys::Win32::Foundation::RECT { left: 0, top: 0, right: 0, bottom: 0 };
        unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect(hwnd, &mut r); }
        Some((r.left, r.top))
    } else {
        None
    }
}

#[cfg(not(windows))]
pub fn get_window_screen_pos() -> (i32, i32) { (0, 0) }

#[cfg(not(windows))]
pub fn get_window_screen_pos_opt() -> Option<(i32, i32)> { None }

#[cfg(windows)]
pub fn list_visible_windows() -> Vec<(usize, String)> {
    unsafe {
        let mut windows: Vec<(usize, String)> = Vec::new();
        
        unsafe extern "system" fn enum_window_proc(hwnd: windows_sys::Win32::Foundation::HWND, lparam: windows_sys::Win32::Foundation::LPARAM) -> windows_sys::Win32::Foundation::BOOL {
            let windows = &mut *(lparam as *mut Vec<(usize, String)>);
            
            if IsWindowVisible(hwnd) != 0 {
                let mut buf = [0u16; 512];
                let len = GetWindowTextW(hwnd, buf.as_mut_ptr(), 512);
                if len > 0 {
                    let title = String::from_utf16_lossy(&buf[..len as usize]);
                    // Filter out our own overlay and small system windows
                    if title != "Owerlayer" && title != "Program Manager" {
                        windows.push((hwnd as usize, title));
                    }
                }
            }
            1 // Continue enumeration
        }

        EnumWindows(Some(enum_window_proc), &mut windows as *mut _ as isize);
        windows.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));
        windows
    }
}

pub fn enumerate_visible_windows() -> Vec<(isize, String)> {
    #[cfg(windows)]
    {
        list_visible_windows().into_iter().map(|(hwnd, title)| (hwnd as isize, title)).collect()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

#[cfg(windows)]
pub struct GdiCaptureCache {
    pub mem_dc: windows_sys::Win32::Graphics::Gdi::HDC,
    pub bitmap: windows_sys::Win32::Graphics::Gdi::HBITMAP,
    pub width: i32,
    pub height: i32,
}

#[cfg(windows)]
unsafe impl Send for GdiCaptureCache {}
#[cfg(windows)]
unsafe impl Sync for GdiCaptureCache {}

#[cfg(windows)]
impl Drop for GdiCaptureCache {
    fn drop(&mut self) {
        unsafe {
            use windows_sys::Win32::Graphics::Gdi::*;
            if !self.mem_dc.is_null() {
                DeleteDC(self.mem_dc);
            }
            if !self.bitmap.is_null() {
                DeleteObject(self.bitmap);
            }
        }
    }
}

#[cfg(windows)]
pub fn capture_screen_rect_cached(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    cache_opt: &mut Option<GdiCaptureCache>,
) -> Option<Vec<u8>> {
    if width <= 0 || height <= 0 || width > 8192 || height > 8192 { return None; }
    
    unsafe {
        use windows_sys::Win32::Graphics::Gdi::*;
        
        let screen_dc = GetDC(std::ptr::null_mut());
        if screen_dc.is_null() { return None; }
        
        let mut needs_init = true;
        if let Some(ref cache) = cache_opt {
            if cache.width == width && cache.height == height && !cache.mem_dc.is_null() && !cache.bitmap.is_null() {
                needs_init = false;
            }
        }
        
        if needs_init {
            // Clean up old cache
            *cache_opt = None;
            
            let mem_dc = CreateCompatibleDC(screen_dc);
            if mem_dc.is_null() {
                ReleaseDC(std::ptr::null_mut(), screen_dc);
                return None;
            }
            let bitmap = CreateCompatibleBitmap(screen_dc, width, height);
            if bitmap.is_null() {
                DeleteDC(mem_dc);
                ReleaseDC(std::ptr::null_mut(), screen_dc);
                return None;
            }
            *cache_opt = Some(GdiCaptureCache {
                mem_dc,
                bitmap,
                width,
                height,
            });
        }
        
        let cache = cache_opt.as_ref().unwrap();
        let old_obj = SelectObject(cache.mem_dc, cache.bitmap);
        
        let success = BitBlt(cache.mem_dc, 0, 0, width, height, screen_dc, x, y, SRCCOPY);
        
        if success == 0 {
            SelectObject(cache.mem_dc, old_obj);
            ReleaseDC(std::ptr::null_mut(), screen_dc);
            return None;
        }
        
        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = width;
        bmi.bmiHeader.biHeight = -height;
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB as u32;
        
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        
        let lines = GetDIBits(
            cache.mem_dc,
            cache.bitmap,
            0,
            height as u32,
            pixels.as_mut_ptr() as *mut _,
            &mut bmi,
            DIB_RGB_COLORS,
        );
        
        SelectObject(cache.mem_dc, old_obj);
        ReleaseDC(std::ptr::null_mut(), screen_dc);
        
        if lines == 0 { return None; }
        
        Some(pixels)
    }
}

#[cfg(windows)]
pub fn get_monitor_from_rect(x: i32, y: i32, w: i32, h: i32) -> isize {
    use windows_sys::Win32::Graphics::Gdi::*;
    use windows_sys::Win32::Foundation::RECT;
    unsafe {
        let rect = RECT {
            left: x,
            top: y,
            right: x + w,
            bottom: y + h,
        };
        let h_monitor = MonitorFromRect(&rect, MONITOR_DEFAULTTOPRIMARY);
        h_monitor as isize
    }
}

#[cfg(windows)]
pub fn get_monitor_rect(h_monitor: isize) -> Option<(i32, i32, i32, i32)> {
    use windows_sys::Win32::Graphics::Gdi::*;
    unsafe {
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(h_monitor as _, &mut info as *mut _ as *mut _) != 0 {
            Some((
                info.rcMonitor.left,
                info.rcMonitor.top,
                info.rcMonitor.right - info.rcMonitor.left,
                info.rcMonitor.bottom - info.rcMonitor.top,
            ))
        } else {
            None
        }
    }
}

#[cfg(not(windows))]
pub struct GdiCaptureCache;

#[cfg(not(windows))]
pub fn capture_screen_rect_cached(
    _x: i32,
    _y: i32,
    _width: i32,
    _height: i32,
    _cache_opt: &mut Option<GdiCaptureCache>,
) -> Option<Vec<u8>> {
    None
}

#[cfg(not(windows))]
pub fn list_visible_windows() -> Vec<(usize, String)> { Vec::new() }

#[cfg(not(windows))]
pub fn get_monitor_from_rect(_x: i32, _y: i32, _w: i32, _h: i32) -> isize { 0 }

#[cfg(not(windows))]
pub fn get_monitor_rect(_h_monitor: isize) -> Option<(i32, i32, i32, i32)> { None }

// ──────────────────────────────────────────────────────────────
//  Primary monitor helpers (viewport coordinates)
// ──────────────────────────────────────────────────────────────

/// Primary monitor size and screen position: (width, height, screen_x, screen_y).
#[cfg(windows)]
pub fn get_primary_monitor_rect() -> (f32, f32, f32, f32) {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromPoint, MONITOR_DEFAULTTOPRIMARY, MONITORINFO,
    };
    unsafe {
        let pt = POINT { x: 0, y: 0 };
        let h_monitor = MonitorFromPoint(pt, MONITOR_DEFAULTTOPRIMARY);
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(h_monitor, &mut info as *mut _ as *mut _) != 0 {
            return (
                (info.rcMonitor.right - info.rcMonitor.left) as f32,
                (info.rcMonitor.bottom - info.rcMonitor.top) as f32,
                info.rcMonitor.left as f32,
                info.rcMonitor.top as f32,
            );
        }
        (1920.0, 1080.0, 0.0, 0.0)
    }
}

#[cfg(not(windows))]
pub fn get_primary_monitor_rect() -> (f32, f32, f32, f32) {
    (1920.0, 1080.0, 0.0, 0.0)
}

/// Top-left of the primary monitor in egui viewport coordinates.
#[cfg(windows)]
pub fn primary_monitor_viewport_origin() -> (f32, f32) {
    let (_w, _h, mx, my) = get_primary_monitor_rect();
    let (vx, vy) = get_virtual_origin();
    (mx - vx + 1.0, my - vy + 1.0)
}

#[cfg(not(windows))]
pub fn primary_monitor_viewport_origin() -> (f32, f32) {
    (0.0, 0.0)
}

/// Reposition the overlay HWND to cover only the primary monitor.
#[cfg(windows)]
pub fn reposition_overlay_to_primary_monitor(fso_fix: bool) {
    let (w, h, mx, my) = get_primary_monitor_rect();
    if fso_fix {
        reposition_overlay_window(mx as i32 - 2, my as i32 - 2, w as i32 + 4, h as i32 + 4);
    } else {
        reposition_overlay_window(mx as i32, my as i32, w as i32, h as i32);
    }
}

#[cfg(not(windows))]
pub fn reposition_overlay_to_primary_monitor(_fso_fix: bool) {}

#[cfg(windows)]
#[link(name = "dwmapi")]
extern "system" {
    fn DwmFlush() -> i32;
}

/// Force DWM to recomposite after overlay style/passthrough changes (fixes frozen desktop until alt-tab).
#[cfg(windows)]
pub fn refresh_overlay_composition() {
    use windows_sys::Win32::Graphics::Gdi::RedrawWindow;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, HWND_TOPMOST, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
        SWP_SHOWWINDOW,
    };
    let hwnd = OVERLAY_HWND.load(Ordering::Relaxed);
    if hwnd.is_null() {
        return;
    }
    unsafe {
        SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED | SWP_SHOWWINDOW,
        );
        const RDW_INVALIDATE: u32 = 0x0001;
        const RDW_UPDATENOW: u32 = 0x0100;
        const RDW_ALLCHILDREN: u32 = 0x0080;
        RedrawWindow(
            hwnd,
            std::ptr::null(),
            std::ptr::null_mut(),
            RDW_INVALIDATE | RDW_UPDATENOW | RDW_ALLCHILDREN,
        );
    }
}

#[cfg(not(windows))]
pub fn refresh_overlay_composition() {}
