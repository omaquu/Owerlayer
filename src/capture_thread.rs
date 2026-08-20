// ──────────────────────────────────────────────────────────────
//  Background Capture Thread
//  Moves screen capture + blur processing off the UI thread
// ──────────────────────────────────────────────────────────────

use std::collections::HashMap;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::{Duration, Instant};

use crate::overlay::BlurEffect;
use eframe::egui;

#[cfg(windows)]
#[link(name = "winmm")]
extern "system" {
    fn timeBeginPeriod(uPeriod: u32) -> u32;
    fn timeEndPeriod(uPeriod: u32) -> u32;
}

// ── Public types ──

#[derive(Clone, Debug)]
pub struct CaptureRequest {
    pub id: usize,
    pub source_rect: [f32; 4],  // x, y, w, h in logical points
    pub ppp: f32,
    pub blur: f32,
    pub blur_effect: BlurEffect,
    pub window_offset: (i32, i32),
    pub use_absolute: bool,
    /// If non-zero, capture from a specific window HWND instead of screen rect
    pub hwnd: usize,
    pub mask: Option<Vec<u8>>,
    pub mask_size: [usize; 2],
    pub exclude_from_capture: bool,
    pub snip_points: Option<Vec<egui::Pos2>>,
    pub live_performance_mode: bool,
}

#[derive(Clone, Debug, Default)]
pub struct CapturePerfMetrics {
    pub wgc_gpu_copy_us: u128,
    pub wgc_map_wait_us: u128,
    pub wgc_pixel_swap_us: u128,
    pub gdi_capture_us: u128,
    pub thread_mask_effects_us: u128,
    pub thread_color32_conv_us: u128,
    pub thread_total_us: u128,
}

#[derive(Clone)]
pub struct CaptureResult {
    pub pixels: Vec<u8>,
    pub size: [usize; 2],
    pub color_image: Option<Arc<egui::ColorImage>>,
    pub perf: CapturePerfMetrics,
    pub is_bgra: bool,
}

#[derive(Clone)]
pub enum MaskCacheEntry {
    Mask {
        resized: Vec<u8>,
        original: Vec<u8>,
        size: [usize; 2],
    },
    Poly {
        mask: Vec<u8>,
        points: Vec<egui::Pos2>,
        size: [usize; 2],
    },
}

pub struct CaptureThread {
    requests: Arc<Mutex<HashMap<usize, CaptureRequest>>>,
    results: Arc<Mutex<HashMap<usize, CaptureResult>>>,
    running: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
    capture_interval_ms: Arc<std::sync::atomic::AtomicU64>,
}

impl CaptureThread {
    pub fn new(fps: f32, egui_ctx: egui::Context) -> Self {
        let interval = (1000.0 / fps.clamp(5.0, 360.0)) as u64;
        let requests: Arc<Mutex<HashMap<usize, CaptureRequest>>> = Arc::new(Mutex::new(HashMap::new()));
        let results: Arc<Mutex<HashMap<usize, CaptureResult>>> = Arc::new(Mutex::new(HashMap::new()));
        let running = Arc::new(AtomicBool::new(true));
        let capture_interval_ms = Arc::new(std::sync::atomic::AtomicU64::new(interval));

        let r_requests = Arc::clone(&requests);
        let r_results = Arc::clone(&results);
        let r_running = Arc::clone(&running);
        let r_interval = Arc::clone(&capture_interval_ms);
        let r_ctx = egui_ctx.clone();

        let handle = thread::Builder::new()
            .name("owerlayer-capture".to_string())
            .spawn(move || {
                #[cfg(windows)]
                unsafe {
                    timeBeginPeriod(1);
                }
                Self::capture_loop(r_requests, r_results, r_running, r_interval, r_ctx);
                #[cfg(windows)]
                unsafe {
                    timeEndPeriod(1);
                }
            })
            .expect("Failed to spawn capture thread");

        Self {
            requests,
            results,
            running,
            handle: Some(handle),
            capture_interval_ms,
        }
    }

    /// Update or register a capture request for a given image ID
    pub fn update_request(&self, id: usize, req: CaptureRequest) {
        if let Ok(mut map) = self.requests.lock() {
            map.insert(id, req);
        }
    }

    /// Remove a capture request (e.g., when object is deleted)
    pub fn remove_request(&self, id: usize) {
        if let Ok(mut map) = self.requests.lock() {
            map.remove(&id);
        }
        if let Ok(mut map) = self.results.lock() {
            map.remove(&id);
        }
    }

    /// Get the latest captured frame for an image ID (non-blocking)
    pub fn get_frame(&self, id: usize) -> Option<CaptureResult> {
        if let Ok(mut map) = self.results.lock() {
            map.remove(&id)
        } else {
            None
        }
    }

    /// Check if there are any active capture requests
    pub fn has_active_requests(&self) -> bool {
        if let Ok(map) = self.requests.lock() {
            !map.is_empty()
        } else {
            false
        }
    }

    /// Update the capture FPS
    pub fn set_fps(&mut self, fps: f32) {
        self.capture_interval_ms.store((1000.0 / fps.clamp(5.0, 360.0)) as u64, Ordering::Relaxed);
    }

    fn capture_loop(
        requests: Arc<Mutex<HashMap<usize, CaptureRequest>>>,
        results: Arc<Mutex<HashMap<usize, CaptureResult>>>,
        running: Arc<AtomicBool>,
        interval: Arc<std::sync::atomic::AtomicU64>,
        egui_ctx: egui::Context,
    ) {
        let mut mask_cache: HashMap<usize, MaskCacheEntry> = HashMap::new();
        #[cfg(windows)]
        let mut wgc_sessions: HashMap<isize, Result<crate::wgc_capture::wgc::WgcCaptureSession, Instant>> = HashMap::new();
        #[cfg(windows)]
        let mut wgc_window_sessions: HashMap<isize, Result<crate::wgc_capture::wgc::WgcCaptureSession, Instant>> = HashMap::new();

        while running.load(Ordering::Relaxed) {
            let start = Instant::now();

            // Snapshot current requests
            let reqs: Vec<CaptureRequest> = if let Ok(map) = requests.lock() {
                map.values().cloned().collect()
            } else {
                Vec::new()
            };

            // Prune caches for inactive requests
            let active_ids: std::collections::HashSet<usize> = reqs.iter().map(|r| r.id).collect();
            mask_cache.retain(|k, _| active_ids.contains(k));

            #[cfg(windows)]
            {
                let active_hwnds: std::collections::HashSet<isize> = reqs.iter().map(|r| r.hwnd as isize).collect();
                wgc_window_sessions.retain(|hwnd, _| active_hwnds.contains(hwnd));
            }

            // Poll all active WGC sessions once per loop iteration
            #[cfg(windows)]
            {
                for session_res in wgc_sessions.values() {
                    if let Ok(session) = session_res {
                        let _ = session.poll_new_frame(&active_ids);
                    }
                }
                for session_res in wgc_window_sessions.values() {
                    if let Ok(session) = session_res {
                        let _ = session.poll_new_frame(&active_ids);
                    }
                }
            }

            let mut got_new_frame = false;
            for req in &reqs {
                if !running.load(Ordering::Relaxed) { break; }

                let result = if req.hwnd != 0 {
                    // Window capture
                    #[cfg(windows)]
                    {
                        Self::capture_window(req, &mut mask_cache, &mut wgc_window_sessions)
                    }
                    #[cfg(not(windows))]
                    {
                        Self::capture_window(req, &mut mask_cache)
                    }
                } else {
                    // Screen rect capture
                    #[cfg(windows)]
                    {
                        Self::capture_screen(req, &mut mask_cache, &mut wgc_sessions)
                    }
                    #[cfg(not(windows))]
                    {
                        Self::capture_screen(req, &mut mask_cache)
                    }
                };

                if let Some(result) = result {
                    if let Ok(mut map) = results.lock() {
                        map.insert(req.id, result);
                        got_new_frame = true;
                    }
                }
            }

            if got_new_frame {
                egui_ctx.request_repaint();
            }

            // Sleep until next interval
            let elapsed = start.elapsed();
            let interval_ms = interval.load(Ordering::Relaxed);
            let target = Duration::from_millis(interval_ms);
            if elapsed < target {
                let remaining = target - elapsed;
                // If remaining time is more than 1.5ms, sleep for (remaining - 1ms)
                if remaining > Duration::from_micros(1500) {
                    thread::sleep(remaining - Duration::from_millis(1));
                }
                // Spin/yield until the exact time
                while start.elapsed() < target {
                    std::hint::spin_loop();
                }
            }
        }
    }

    fn resize_mask(mask: &[u8], old_w: usize, old_h: usize, new_w: usize, new_h: usize) -> Vec<u8> {
        let expected_mask_len = new_w * new_h;
        if mask.len() == expected_mask_len && old_w == new_w && old_h == new_h {
            return mask.to_vec();
        }
        let mut resized_mask = vec![255u8; expected_mask_len];
        if old_w > 0 && old_h > 0 && new_w > 0 && new_h > 0 {
            for py in 0..new_h {
                for px in 0..new_w {
                    let src_x = (px * old_w) / new_w;
                    let src_y = (py * old_h) / new_h;
                    let src_idx = src_y * old_w + src_x;
                    if src_idx < mask.len() {
                        resized_mask[py * new_w + px] = mask[src_idx];
                    }
                }
            }
        }
        resized_mask
    }

    fn apply_mask_to_captured_pixels(
        req: &CaptureRequest,
        pixels: &mut [u8],
        sw: usize,
        sh: usize,
        mask_cache: &mut HashMap<usize, MaskCacheEntry>,
    ) {
        let mask_valid = req.mask.is_some() && req.mask_size[0] > 0 && req.mask_size[1] > 0;
        if mask_valid {
            let mask = req.mask.as_ref().unwrap();
            let matches = match mask_cache.get(&req.id) {
                Some(MaskCacheEntry::Mask { original, size, .. }) => {
                    *size == [sw, sh] && original == mask
                }
                _ => false,
            };

            let final_mask = if matches {
                match mask_cache.get(&req.id).unwrap() {
                    MaskCacheEntry::Mask { resized, .. } => resized,
                    _ => unreachable!(),
                }
            } else {
                let new_mask = Self::resize_mask(mask, req.mask_size[0], req.mask_size[1], sw, sh);
                mask_cache.insert(
                    req.id,
                    MaskCacheEntry::Mask {
                        resized: new_mask.clone(),
                        original: mask.clone(),
                        size: [sw, sh],
                    },
                );
                match mask_cache.get(&req.id).unwrap() {
                    MaskCacheEntry::Mask { resized, .. } => resized,
                    _ => unreachable!(),
                }
            };
            
            pixels.par_chunks_mut(4).enumerate().for_each(|(i, chunk)| {
                if i < final_mask.len() && final_mask[i] == 0 {
                    if chunk.len() >= 4 {
                        chunk[3] = 0;
                    }
                }
            });
        } else if let Some(ref pts) = req.snip_points {
            let matches = match mask_cache.get(&req.id) {
                Some(MaskCacheEntry::Poly { points, size, .. }) => {
                    *size == [sw, sh] && points == pts
                }
                _ => false,
            };

            let final_mask = if matches {
                match mask_cache.get(&req.id).unwrap() {
                    MaskCacheEntry::Poly { mask, .. } => mask,
                    _ => unreachable!(),
                }
            } else {
                let mut new_mask = vec![255u8; sw * sh];
                let ppp = req.ppp;
                use rayon::prelude::*;
                if sw > 0 {
                    new_mask.par_chunks_mut(sw).enumerate().for_each(|(y, row)| {
                        for x in 0..sw {
                            let lp = egui::pos2(x as f32 / ppp, y as f32 / ppp);
                            if !crate::utils::is_inside_poly(pts, lp) {
                                row[x] = 0;
                            }
                        }
                    });
                }
                mask_cache.insert(
                    req.id,
                    MaskCacheEntry::Poly {
                        mask: new_mask.clone(),
                        points: pts.clone(),
                        size: [sw, sh],
                    },
                );
                match mask_cache.get(&req.id).unwrap() {
                    MaskCacheEntry::Poly { mask, .. } => mask,
                    _ => unreachable!(),
                }
            };
            
            pixels.par_chunks_mut(4).enumerate().for_each(|(i, chunk)| {
                if i < final_mask.len() && final_mask[i] == 0 {
                    if chunk.len() >= 4 {
                        chunk[3] = 0;
                    }
                }
            });
        } else {
            mask_cache.remove(&req.id);
        }
    }

    #[cfg(windows)]
    fn capture_screen(
        req: &CaptureRequest,
        mask_cache: &mut HashMap<usize, MaskCacheEntry>,
        wgc_sessions: &mut HashMap<isize, Result<crate::wgc_capture::wgc::WgcCaptureSession, Instant>>,
    ) -> Option<CaptureResult> {
        let total_start = std::time::Instant::now();
        let mut perf = CapturePerfMetrics::default();

        let (ox, oy) = if req.use_absolute { (0, 0) } else { req.window_offset };

        let sx = (req.source_rect[0] * req.ppp).round() as i32 + ox;
        let sy = (req.source_rect[1] * req.ppp).round() as i32 + oy;
        let sw = (req.source_rect[2] * req.ppp).round() as i32;
        let sh = (req.source_rect[3] * req.ppp).round() as i32;

        if sw <= 0 || sh <= 0 { return None; }

        let mut pixels = None;
        let mut wgc_attempted = false;

        // Try WGC if not in performance mode
        if !req.live_performance_mode {
            let h_monitor = crate::winapi_utils::get_monitor_from_rect(sx, sy, sw, sh);
            if h_monitor != 0 {
                let mut needs_retry = false;
                if let Some(entry) = wgc_sessions.get(&h_monitor) {
                    if let Err(last_failed) = entry {
                        if last_failed.elapsed() >= Duration::from_secs(2) {
                            needs_retry = true;
                        }
                    }
                }

                if needs_retry {
                    match crate::wgc_capture::wgc::WgcCaptureSession::start_monitor_capture(h_monitor) {
                        Ok(session) => {
                            wgc_sessions.insert(h_monitor, Ok(session));
                        }
                        Err(e) => {
                            eprintln!("Failed to start WGC capture on retry: {:?}", e);
                            wgc_sessions.insert(h_monitor, Err(Instant::now()));
                        }
                    }
                } else if !wgc_sessions.contains_key(&h_monitor) {
                    match crate::wgc_capture::wgc::WgcCaptureSession::start_monitor_capture(h_monitor) {
                        Ok(session) => {
                            wgc_sessions.insert(h_monitor, Ok(session));
                        }
                        Err(e) => {
                            eprintln!("Failed to start WGC capture: {:?}", e);
                            wgc_sessions.insert(h_monitor, Err(Instant::now()));
                        }
                    }
                }

                if let Some(Ok(session)) = wgc_sessions.get(&h_monitor) {
                    wgc_attempted = true;
                    if let Some((mon_x, mon_y, _mon_w, _mon_h)) = crate::winapi_utils::get_monitor_rect(h_monitor) {
                        let crop_x = sx - mon_x;
                        let crop_y = sy - mon_y;
                        
                        if let Ok(Some((crop_pixels, crop_w, crop_h, gpu_copy_us, map_wait_us, pixel_swap_us))) = session.get_latest_frame(req.id, Some((crop_x, crop_y, sw, sh))) {
                            perf.wgc_gpu_copy_us = gpu_copy_us;
                            perf.wgc_map_wait_us = map_wait_us;
                            perf.wgc_pixel_swap_us = pixel_swap_us;

                            let mut wgc_pixels = vec![0u8; (sw as usize) * (sh as usize) * 4];
                            
                            let dst_x_offset = (crop_x.max(0) - crop_x) as usize;
                            let dst_y_offset = (crop_y.max(0) - crop_y) as usize;
                            
                            for y in 0..crop_h {
                                let src_row_start = y * crop_w * 4;
                                let src_row_end = src_row_start + crop_w * 4;
                                
                                let dst_y = dst_y_offset + y;
                                let dst_row_start = (dst_y * (sw as usize) + dst_x_offset) * 4;
                                let dst_row_end = dst_row_start + crop_w * 4;
                                
                                if src_row_end <= crop_pixels.len() && dst_row_end <= wgc_pixels.len() {
                                    wgc_pixels[dst_row_start..dst_row_end].copy_from_slice(&crop_pixels[src_row_start..src_row_end]);
                                }
                            }
                            pixels = Some(wgc_pixels);
                        } else {
                            // WGC session is active, but no new frame was available.
                            // Return early to avoid GDI fallback stuttering.
                            return None;
                        }
                    }
                }
            }
        } else {
            let h_monitor = crate::winapi_utils::get_monitor_from_rect(sx, sy, sw, sh);
            if h_monitor != 0 {
                wgc_sessions.remove(&h_monitor);
            }
        }

        // Determine if pixels are BGRA (from WGC, no swap done) or RGBA (from GDI fallback)
        let is_bgra = wgc_attempted;

        // Fallback to GDI if WGC was not used or failed
        let mut pixels = match pixels {
            Some(p) => p,
            None => {
                if wgc_attempted {
                    // WGC is active/running on this monitor, but failed to return a frame.
                    // Return early to avoid GDI fallback stuttering.
                    return None;
                }
                let gdi_start = std::time::Instant::now();
                let p = crate::winapi_utils::capture_screen_rect(sx, sy, sw, sh)?; // Legacy capture
                perf.gdi_capture_us = gdi_start.elapsed().as_micros();
                p
            }
        };

        // Pre-retrieve final mask if needed
        let has_mask = req.mask.is_some() || req.snip_points.is_some();
        if !has_mask {
            mask_cache.remove(&req.id);
        }

        let mask_valid = req.mask.is_some() && req.mask_size[0] > 0 && req.mask_size[1] > 0;
        let final_mask = if mask_valid {
            let mask = req.mask.as_ref().unwrap();
            let matches = match mask_cache.get(&req.id) {
                Some(MaskCacheEntry::Mask { original, size, .. }) => {
                    *size == [sw as usize, sh as usize] && original == mask
                }
                _ => false,
            };

            let final_mask = if matches {
                match mask_cache.get(&req.id).unwrap() {
                    MaskCacheEntry::Mask { resized, .. } => resized,
                    _ => unreachable!(),
                }
            } else {
                let new_mask = Self::resize_mask(mask, req.mask_size[0], req.mask_size[1], sw as usize, sh as usize);
                mask_cache.insert(
                    req.id,
                    MaskCacheEntry::Mask {
                        resized: new_mask.clone(),
                        original: mask.clone(),
                        size: [sw as usize, sh as usize],
                    },
                );
                match mask_cache.get(&req.id).unwrap() {
                    MaskCacheEntry::Mask { resized, .. } => resized,
                    _ => unreachable!(),
                }
            };
            Some(final_mask)
        } else if let Some(ref pts) = req.snip_points {
            let matches = match mask_cache.get(&req.id) {
                Some(MaskCacheEntry::Poly { points, size, .. }) => {
                    *size == [sw as usize, sh as usize] && points == pts
                }
                _ => false,
            };

            let final_mask = if matches {
                match mask_cache.get(&req.id).unwrap() {
                    MaskCacheEntry::Poly { mask, .. } => mask,
                    _ => unreachable!(),
                }
            } else {
                let mut new_mask = vec![255u8; sw as usize * sh as usize];
                let ppp = req.ppp;
                use rayon::prelude::*;
                if sw > 0 {
                    new_mask.par_chunks_mut(sw as usize).enumerate().for_each(|(y, row)| {
                        for x in 0..sw as usize {
                            let lp = egui::pos2(x as f32 / ppp, y as f32 / ppp);
                            if !crate::utils::is_inside_poly(pts, lp) {
                                row[x] = 0;
                            }
                        }
                    });
                }
                mask_cache.insert(
                    req.id,
                    MaskCacheEntry::Poly {
                        mask: new_mask.clone(),
                        points: pts.clone(),
                        size: [sw as usize, sh as usize],
                    },
                );
                match mask_cache.get(&req.id).unwrap() {
                    MaskCacheEntry::Poly { mask, .. } => mask,
                    _ => unreachable!(),
                }
            };
            Some(final_mask)
        } else {
            None
        };

        let mask_effects_start = std::time::Instant::now();
        // Apply blur effect on background thread
        if req.blur > 0.1 {
            let w = sw as usize;
            let h = sh as usize;
            match req.blur_effect {
                BlurEffect::Gaussian => apply_box_blur(&mut pixels, w, h, req.blur as usize),
                BlurEffect::Pixelate => apply_pixelate(&mut pixels, w, h, (req.blur * req.ppp) as usize),
                BlurEffect::Glitch => apply_vhs_glitch(&mut pixels, w, h, req.blur / 100.0),
            }
        }

        perf.thread_mask_effects_us = mask_effects_start.elapsed().as_micros();

        let conv_start = std::time::Instant::now();
        let color_pixels: Vec<egui::Color32> = if is_bgra {
            use rayon::prelude::*;
            if let Some(mask) = final_mask {
                pixels.par_chunks_exact_mut(4).enumerate().map(|(i, chunk)| {
                    let b = chunk[0];
                    let g = chunk[1];
                    let r = chunk[2];
                    let a = if i < mask.len() && mask[i] == 0 { 0 } else { chunk[3] };
                    chunk[0] = r;
                    chunk[1] = g;
                    chunk[2] = b;
                    chunk[3] = a;
                    egui::Color32::from_rgba_unmultiplied(r, g, b, a)
                }).collect()
            } else {
                pixels.par_chunks_exact_mut(4).map(|chunk| {
                    let b = chunk[0];
                    let g = chunk[1];
                    let r = chunk[2];
                    let a = chunk[3];
                    chunk[0] = r;
                    chunk[1] = g;
                    chunk[2] = b;
                    egui::Color32::from_rgba_unmultiplied(r, g, b, a)
                }).collect()
            }
        } else {
            use rayon::prelude::*;
            if let Some(mask) = final_mask {
                pixels.par_chunks_exact_mut(4).enumerate().map(|(i, chunk)| {
                    let r = chunk[0];
                    let g = chunk[1];
                    let b = chunk[2];
                    let a = if i < mask.len() && mask[i] == 0 { 0 } else { chunk[3] };
                    chunk[3] = a;
                    egui::Color32::from_rgba_unmultiplied(r, g, b, a)
                }).collect()
            } else {
                pixels.par_chunks_exact_mut(4).map(|chunk| {
                    egui::Color32::from_rgba_unmultiplied(chunk[0], chunk[1], chunk[2], chunk[3])
                }).collect()
            }
        };

        let color_image = Some(Arc::new(egui::ColorImage {
            size: [sw as usize, sh as usize],
            pixels: color_pixels,
        }));
        perf.thread_color32_conv_us = conv_start.elapsed().as_micros();
        perf.thread_total_us = total_start.elapsed().as_micros();

        Some(CaptureResult {
            pixels,
            size: [sw as usize, sh as usize],
            color_image,
            perf,
            is_bgra: false, // BGRA was already swapped to RGBA on the background thread!
        })
    }

    #[cfg(not(windows))]
    fn capture_screen(
        req: &CaptureRequest,
        mask_cache: &mut HashMap<usize, MaskCacheEntry>,
    ) -> Option<CaptureResult> {
        let (ox, oy) = if req.use_absolute { (0, 0) } else { req.window_offset };

        let sx = (req.source_rect[0] * req.ppp).round() as i32 + ox;
        let sy = (req.source_rect[1] * req.ppp).round() as i32 + oy;
        let sw = (req.source_rect[2] * req.ppp).round() as i32;
        let sh = (req.source_rect[3] * req.ppp).round() as i32;

        if sw <= 0 || sh <= 0 { return None; }

        let mut pixels = crate::winapi_utils::capture_screen_rect(sx, sy, sw, sh)?;

        // Apply blur effect on background thread
        if req.blur > 0.1 {
            let w = sw as usize;
            let h = sh as usize;
            match req.blur_effect {
                BlurEffect::Gaussian => apply_box_blur(&mut pixels, w, h, req.blur as usize),
                BlurEffect::Pixelate => apply_pixelate(&mut pixels, w, h, (req.blur * req.ppp) as usize),
                BlurEffect::Glitch => apply_vhs_glitch(&mut pixels, w, h, req.blur / 100.0),
            }
        }

        Self::apply_mask_to_captured_pixels(req, &mut pixels, sw as usize, sh as usize, mask_cache);

        use rayon::prelude::*;
        let color_pixels: Vec<egui::Color32> = if pixels.len() > 100_000 {
            pixels
                .par_chunks_exact(4)
                .map(|chunk| {
                    egui::Color32::from_rgba_unmultiplied(chunk[0], chunk[1], chunk[2], chunk[3])
                })
                .collect()
        } else {
            pixels
                .chunks_exact(4)
                .map(|chunk| {
                    egui::Color32::from_rgba_unmultiplied(chunk[0], chunk[1], chunk[2], chunk[3])
                })
                .collect()
        };

        let color_image = Arc::new(egui::ColorImage {
            size: [sw as usize, sh as usize],
            pixels: color_pixels,
        });

        Some(CaptureResult {
            pixels,
            size: [sw as usize, sh as usize],
            color_image: Some(color_image),
            perf: CapturePerfMetrics::default(),
            is_bgra: false,
        })
    }

    #[cfg(windows)]
    fn capture_window(
        req: &CaptureRequest,
        mask_cache: &mut HashMap<usize, MaskCacheEntry>,
        wgc_window_sessions: &mut HashMap<isize, Result<crate::wgc_capture::wgc::WgcCaptureSession, Instant>>,
    ) -> Option<CaptureResult> {
        let total_start = std::time::Instant::now();
        let mut perf = CapturePerfMetrics::default();

        let hwnd = req.hwnd as isize;
        if hwnd == 0 { return None; }

        let mut crop = None;
        let mut r = windows_sys::Win32::Foundation::RECT { left: 0, top: 0, right: 0, bottom: 0 };
        let mut has_rect = false;
        unsafe {
            if windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect(hwnd as *mut _, &mut r) != 0 {
                has_rect = true;
            }
        }
        if has_rect {
            let (wx, wy) = req.window_offset;
            let sx = (req.source_rect[0] * req.ppp).round() as i32 + if req.use_absolute { 0 } else { wx };
            let sy = (req.source_rect[1] * req.ppp).round() as i32 + if req.use_absolute { 0 } else { wy };
            let sw = (req.source_rect[2] * req.ppp).round() as i32;
            let sh = (req.source_rect[3] * req.ppp).round() as i32;
            
            let cx = sx - r.left;
            let cy = sy - r.top;
            crop = Some((cx, cy, sw, sh));
        }

        let mut pixels = None;
        let mut pw = 0;
        let mut ph = 0;
        let mut wgc_attempted = false;

        // Try WGC for window capture
        if !req.live_performance_mode {
            let mut needs_retry = false;
            if let Some(entry) = wgc_window_sessions.get(&hwnd) {
                if let Err(last_failed) = entry {
                    if last_failed.elapsed() >= Duration::from_secs(2) {
                        needs_retry = true;
                    }
                }
            }

            if needs_retry {
                match crate::wgc_capture::wgc::WgcCaptureSession::start_window_capture(hwnd) {
                    Ok(session) => {
                        wgc_window_sessions.insert(hwnd, Ok(session));
                    }
                    Err(e) => {
                        eprintln!("Failed to start WGC window capture on retry: {:?}", e);
                        wgc_window_sessions.insert(hwnd, Err(Instant::now()));
                    }
                }
            } else if !wgc_window_sessions.contains_key(&hwnd) {
                match crate::wgc_capture::wgc::WgcCaptureSession::start_window_capture(hwnd) {
                    Ok(session) => {
                        wgc_window_sessions.insert(hwnd, Ok(session));
                    }
                    Err(e) => {
                        eprintln!("Failed to start WGC window capture: {:?}", e);
                        wgc_window_sessions.insert(hwnd, Err(Instant::now()));
                    }
                }
            }

            if let Some(Ok(session)) = wgc_window_sessions.get(&hwnd) {
                wgc_attempted = true;
                if let Ok(Some((crop_pixels, crop_w, crop_h, gpu_copy_us, map_wait_us, pixel_swap_us))) = session.get_latest_frame(req.id, crop) {
                    perf.wgc_gpu_copy_us = gpu_copy_us;
                    perf.wgc_map_wait_us = map_wait_us;
                    perf.wgc_pixel_swap_us = pixel_swap_us;

                    pixels = Some(crop_pixels);
                    pw = crop_w;
                    ph = crop_h;
                } else {
                    // WGC session is active, but no new frame was available.
                    return None;
                }
            }
        } else {
            wgc_window_sessions.remove(&hwnd);
        }

        let is_bgra = wgc_attempted;

        // Fallback to GDI legacy window capture
        let mut pixels = match pixels {
            Some(p) => p,
            None => {
                if wgc_attempted {
                    return None;
                }
                let gdi_start = std::time::Instant::now();
                let (p, w, h) = crate::winapi_utils::capture_window(req.hwnd)?;
                perf.gdi_capture_us = gdi_start.elapsed().as_micros();
                
                if let Some((cx, cy, cw, ch)) = crop {
                    let cx_clamped = cx.clamp(0, w as i32) as usize;
                    let cy_clamped = cy.clamp(0, h as i32) as usize;
                    let end_x_clamped = (cx + cw).clamp(0, w as i32) as usize;
                    let end_y_clamped = (cy + ch).clamp(0, h as i32) as usize;
                    
                    if end_x_clamped > cx_clamped && end_y_clamped > cy_clamped {
                        let cropped_w = end_x_clamped - cx_clamped;
                        let cropped_h = end_y_clamped - cy_clamped;
                        let mut cropped = vec![0u8; cropped_w * cropped_h * 4];
                        for y in 0..cropped_h {
                            let src_start = ((cy_clamped + y) * w + cx_clamped) * 4;
                            let dest_start = y * cropped_w * 4;
                            if src_start + cropped_w * 4 <= p.len() && dest_start + cropped_w * 4 <= cropped.len() {
                                cropped[dest_start..dest_start + cropped_w * 4].copy_from_slice(&p[src_start..src_start + cropped_w * 4]);
                            }
                        }
                        pw = cropped_w;
                        ph = cropped_h;
                        cropped
                    } else {
                        pw = w;
                        ph = h;
                        p
                    }
                } else {
                    pw = w;
                    ph = h;
                    p
                }
            }
        };

        if pw == 0 || ph == 0 { return None; }

        Self::apply_mask_to_captured_pixels(req, &mut pixels, pw, ph, mask_cache);

        let mask_effects_start = std::time::Instant::now();
        if req.blur > 0.1 {
            match req.blur_effect {
                BlurEffect::Gaussian => apply_box_blur(&mut pixels, pw, ph, req.blur as usize),
                BlurEffect::Pixelate => apply_pixelate(&mut pixels, pw, ph, (req.blur * req.ppp) as usize),
                BlurEffect::Glitch => apply_vhs_glitch(&mut pixels, pw, ph, req.blur / 100.0),
            }
        }
        perf.thread_mask_effects_us = mask_effects_start.elapsed().as_micros();

        let conv_start = std::time::Instant::now();
        let color_pixels: Vec<egui::Color32> = if is_bgra {
            use rayon::prelude::*;
            pixels.par_chunks_exact_mut(4).map(|chunk| {
                let b = chunk[0];
                let g = chunk[1];
                let r = chunk[2];
                let a = chunk[3];
                chunk[0] = r;
                chunk[1] = g;
                chunk[2] = b;
                egui::Color32::from_rgba_unmultiplied(r, g, b, a)
            }).collect()
        } else {
            use rayon::prelude::*;
            pixels.par_chunks_exact(4).map(|chunk| {
                egui::Color32::from_rgba_unmultiplied(chunk[0], chunk[1], chunk[2], chunk[3])
            }).collect()
        };

        let color_image = std::sync::Arc::new(egui::ColorImage {
            size: [pw, ph],
            pixels: color_pixels,
        });
        perf.thread_color32_conv_us = conv_start.elapsed().as_micros();
        perf.thread_total_us = total_start.elapsed().as_micros();

        Some(CaptureResult {
            pixels,
            size: [pw, ph],
            color_image: Some(color_image),
            perf,
            is_bgra: false,
        })
    }

    #[cfg(not(windows))]
    fn capture_window(req: &CaptureRequest, _mask_cache: &mut HashMap<usize, MaskCacheEntry>) -> Option<CaptureResult> {
        None
    }
}

impl Drop for CaptureThread {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

// ──────────────────────────────────────────────────────────────
//  Blur/Effect functions (moved from overlay.rs to run on bg thread)
// ──────────────────────────────────────────────────────────────

use rayon::prelude::*;

pub fn apply_box_blur(pixels: &mut [u8], width: usize, height: usize, radius: usize) {
    let radius = radius.min(100).min(width / 2).min(height / 2);
    if radius == 0 || width == 0 || height == 0 { return; }
 
    let pixels_per_row = width * 4;
    let mut intermediate = pixels.to_vec();
    
    // Horizontal pass (sliding window)
    intermediate.par_chunks_exact_mut(pixels_per_row).for_each(|row| {
        let row_copy = row.to_vec();
        let mut r = 0u32;
        let mut g = 0u32;
        let mut b = 0u32;
        let mut a = 0u32;
        
        let w_size = (radius * 2 + 1) as u32;
        for i in 0..=radius {
            let idx = i.min(width - 1) * 4;
            r += row_copy[idx] as u32;
            g += row_copy[idx + 1] as u32;
            b += row_copy[idx + 2] as u32;
            a += row_copy[idx + 3] as u32;
        }
        r += (row_copy[0] as u32) * radius as u32;
        g += (row_copy[1] as u32) * radius as u32;
        b += (row_copy[2] as u32) * radius as u32;
        a += (row_copy[3] as u32) * radius as u32;

        for x in 0..width {
            let idx = x * 4;
            row[idx] = (r / w_size) as u8;
            row[idx + 1] = (g / w_size) as u8;
            row[idx + 2] = (b / w_size) as u8;
            row[idx + 3] = (a / w_size) as u8;

            let left_idx = if x >= radius { x - radius } else { 0 } * 4;
            let right_idx = (x + radius + 1).min(width - 1) * 4;

            r = r + row_copy[right_idx] as u32 - row_copy[left_idx] as u32;
            g = g + row_copy[right_idx + 1] as u32 - row_copy[left_idx + 1] as u32;
            b = b + row_copy[right_idx + 2] as u32 - row_copy[left_idx + 2] as u32;
            a = a + row_copy[right_idx + 3] as u32 - row_copy[left_idx + 3] as u32;
        }
    });

    // Vertical pass (sliding window)
    let intermediate_shared = &intermediate;
    let w_size = (radius * 2 + 1) as u32;

    (0..width).into_par_iter().for_each(|x| {
        let mut r = 0u32;
        let mut g = 0u32;
        let mut b = 0u32;
        let mut a = 0u32;

        for y in 0..=radius {
            let idx = (y.min(height - 1) * width + x) * 4;
            r += intermediate_shared[idx] as u32;
            g += intermediate_shared[idx + 1] as u32;
            b += intermediate_shared[idx + 2] as u32;
            a += intermediate_shared[idx + 3] as u32;
        }
        let first_idx = x * 4;
        r += (intermediate_shared[first_idx] as u32) * radius as u32;
        g += (intermediate_shared[first_idx + 1] as u32) * radius as u32;
        b += (intermediate_shared[first_idx + 2] as u32) * radius as u32;
        a += (intermediate_shared[first_idx + 3] as u32) * radius as u32;

        for y in 0..height {
            let idx = (y * width + x) * 4;
            unsafe {
                let ptr = pixels.as_ptr() as *mut u8;
                *ptr.add(idx) = (r / w_size) as u8;
                *ptr.add(idx + 1) = (g / w_size) as u8;
                *ptr.add(idx + 2) = (b / w_size) as u8;
                *ptr.add(idx + 3) = (a / w_size) as u8;
            }

            let leave_y = if y >= radius { y - radius } else { 0 };
            let enter_y = (y + radius + 1).min(height - 1);

            let leave_idx = (leave_y * width + x) * 4;
            let enter_idx = (enter_y * width + x) * 4;

            r = r + intermediate_shared[enter_idx] as u32 - intermediate_shared[leave_idx] as u32;
            g = g + intermediate_shared[enter_idx + 1] as u32 - intermediate_shared[leave_idx + 1] as u32;
            b = b + intermediate_shared[enter_idx + 2] as u32 - intermediate_shared[leave_idx + 2] as u32;
            a = a + intermediate_shared[enter_idx + 3] as u32 - intermediate_shared[leave_idx + 3] as u32;
        }
    });
}

pub fn apply_pixelate(pixels: &mut [u8], width: usize, height: usize, scale: usize) {
    let scale = scale.max(1).min(64);
    if scale <= 1 { return; }
    
    for y in (0..height).step_by(scale) {
        for x in (0..width).step_by(scale) {
            let mut r = 0u32; let mut g = 0u32; let mut b = 0u32; let mut a = 0u32;
            let mut count = 0;
            
            for py in 0..scale {
                for px in 0..scale {
                    let nx = x + px;
                    let ny = y + py;
                    if nx < width && ny < height {
                        let idx = (ny * width + nx) * 4;
                        r += pixels[idx] as u32;
                        g += pixels[idx + 1] as u32;
                        b += pixels[idx + 2] as u32;
                        a += pixels[idx + 3] as u32;
                        count += 1;
                    }
                }
            }
            
            if count > 0 {
                let r = (r / count) as u8;
                let g = (g / count) as u8;
                let b = (b / count) as u8;
                let a = (a / count) as u8;
                
                for py in 0..scale {
                    for px in 0..scale {
                        let nx = x + px;
                        let ny = y + py;
                        if nx < width && ny < height {
                            let idx = (ny * width + nx) * 4;
                            pixels[idx] = r;
                            pixels[idx + 1] = g;
                            pixels[idx + 2] = b;
                            pixels[idx + 3] = a;
                        }
                    }
                }
            }
        }
    }
}

pub fn apply_vhs_glitch(pixels: &mut [u8], width: usize, height: usize, intensity: f32) {
    let intensity = intensity.min(1.0).max(0.0);
    if intensity < 0.01 { return; }
    
    let copy = pixels.to_vec();
    let mut rng = 12345u64;
    
    for y in 0..height {
        rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        // Increased shift range for dramatic blur at 100%
        let shift_range = 250.0 * intensity; // Much stronger shift at 100%
        let shift = (((rng >> 32) as f32 / 4294967295.0) * shift_range) as i32 - (shift_range * 0.5) as i32;
        
        let row_idx = y * width * 4;
        for x in 0..width {
            let nx = (x as i32 + shift).clamp(0, width as i32 - 1) as usize;
            let target_idx = row_idx + x * 4;
            let source_idx = row_idx + nx * 4;
            
            let color_offset = (10.0 * intensity) as i32;
            let rx = (nx as i32 + color_offset).clamp(0, width as i32 - 1) as usize;
            let bx = (nx as i32 - color_offset).clamp(0, width as i32 - 1) as usize;
            
            pixels[target_idx] = copy[row_idx + rx * 4];
            pixels[target_idx + 1] = copy[source_idx + 1];
            pixels[target_idx + 2] = copy[row_idx + bx * 4 + 2];
            pixels[target_idx + 3] = copy[source_idx + 3];
            
            if (rng % 150) < (15.0 * intensity) as u64 {
                pixels[target_idx] = pixels[target_idx].saturating_add(40);
                pixels[target_idx + 1] = pixels[target_idx + 1].saturating_add(40);
                pixels[target_idx + 2] = pixels[target_idx + 2].saturating_add(40);
            }
        }
    }
}

/// Downsample an RGBA image to a small thumbnail for the layers panel
pub fn downsample_thumbnail(pixels: &[u8], size: [usize; 2], target_size: usize) -> (Vec<u8>, [usize; 2]) {
    if pixels.is_empty() || size[0] == 0 || size[1] == 0 {
        return (vec![0u8; target_size * target_size * 4], [target_size, target_size]);
    }
    
    let aspect = size[0] as f32 / size[1] as f32;
    let (tw, th) = if aspect > 1.0 {
        (target_size, (target_size as f32 / aspect).max(1.0) as usize)
    } else {
        ((target_size as f32 * aspect).max(1.0) as usize, target_size)
    };
    
    let mut thumb = vec![0u8; tw * th * 4];
    for ty in 0..th {
        for tx in 0..tw {
            let sx = (tx * size[0]) / tw;
            let sy = (ty * size[1]) / th;
            let si = (sy * size[0] + sx) * 4;
            let ti = (ty * tw + tx) * 4;
            if si + 3 < pixels.len() && ti + 3 < thumb.len() {
                thumb[ti] = pixels[si];
                thumb[ti + 1] = pixels[si + 1];
                thumb[ti + 2] = pixels[si + 2];
                thumb[ti + 3] = pixels[si + 3];
            }
        }
    }
    (thumb, [tw, th])
}
