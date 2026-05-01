#[cfg(windows)]
use windows_sys::Win32::Foundation::POINT;
#[cfg(windows)]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN,
};
use windows_sys::Win32::Graphics::Gdi::{
    GetDC, ReleaseDC, CreateCompatibleDC, CreateCompatibleBitmap, SelectObject, DeleteObject, DeleteDC, BitBlt, GetDIBits,
    SRCCOPY, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
};
#[cfg(windows)]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;

use std::sync::atomic::{AtomicPtr, Ordering};
use std::ptr;

static OVERLAY_HWND: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(ptr::null_mut());

/// Get primary monitor resolution before window creation.
#[cfg(windows)]
pub fn get_screen_size() -> (f32, f32) {
    unsafe {
        let w = GetSystemMetrics(SM_CXSCREEN);
        let h = GetSystemMetrics(SM_CYSCREEN);
        (w as f32, h as f32)
    }
}

#[cfg(not(windows))]
pub fn get_screen_size() -> (f32, f32) { (1920.0, 1080.0) }

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

// ──────────────────────────────────────────────────────────────
//  Overlay HWND management
// ──────────────────────────────────────────────────────────────

/// Find the Owerlayer window by title, cache its HWND, and
/// re-assert HWND_TOPMOST. Call once on the first frame.
#[cfg(windows)]
pub fn setup_overlay_window() {
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    unsafe {
        let title: Vec<u16> = "Owerlayer\0".encode_utf16().collect();
        let hwnd = FindWindowW(ptr::null(), title.as_ptr());
        if hwnd.is_null() { return; }

        OVERLAY_HWND.store(hwnd, Ordering::Relaxed);

        // Ensure always-on-top
        SetWindowPos(
            hwnd, HWND_TOPMOST,
            0, 0, 0, 0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
}

#[cfg(not(windows))]
pub fn setup_overlay_window() {}

/// Re-assert always-on-top positioning so other windows
/// cannot push the overlay behind them.
#[cfg(windows)]
#[allow(dead_code)]
pub fn force_topmost() {
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    let hwnd = OVERLAY_HWND.load(Ordering::Relaxed);
    if !hwnd.is_null() {
        unsafe {
            SetWindowPos(
                hwnd, HWND_TOPMOST,
                0, 0, 0, 0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
    }
}

#[cfg(not(windows))]
pub fn force_topmost() {}

/// Force the overlay to become the foreground window so it
/// can receive keyboard input (used when entering edit mode).
#[cfg(windows)]
#[allow(dead_code)]
pub fn force_focus() {
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    let hwnd = OVERLAY_HWND.load(Ordering::Relaxed);
    if !hwnd.is_null() {
        unsafe {
            SetForegroundWindow(hwnd);
        }
    }
}

/// Captures a rectangle of the desktop screen and returns RGBA bytes.
pub fn capture_screen_rect(x: i32, y: i32, width: i32, height: i32) -> Option<Vec<u8>> {
    if width <= 0 || height <= 0 { return None; }
    
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
        
        // GDI outputs BGRA, we need to swizzle it to RGBA for egui
        for chunk in pixels.chunks_exact_mut(4) {
            let b = chunk[0];
            let g = chunk[1];
            let r = chunk[2];
            let a = 255; // Force alpha to 255 (desktop might have 0 alpha)
            chunk[0] = r;
            chunk[1] = g;
            chunk[2] = b;
            chunk[3] = a;
        }
        
        Some(pixels)
    }
}

#[cfg(not(windows))]
pub fn force_focus() {}
