// ──────────────────────────────────────────────────────────────
//  Background Capture Thread
//  Moves screen capture + blur processing off the UI thread
// ──────────────────────────────────────────────────────────────

use std::collections::HashMap;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::{Duration, Instant};

use crate::overlay::BlurEffect;

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
}

#[derive(Clone)]
pub struct CaptureResult {
    pub pixels: Vec<u8>,
    pub size: [usize; 2],
}

pub struct CaptureThread {
    requests: Arc<Mutex<HashMap<usize, CaptureRequest>>>,
    results: Arc<Mutex<HashMap<usize, CaptureResult>>>,
    running: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
    capture_interval_ms: u64,
}

impl CaptureThread {
    pub fn new(fps: f32) -> Self {
        let interval = (1000.0 / fps.clamp(5.0, 240.0)) as u64;
        let requests: Arc<Mutex<HashMap<usize, CaptureRequest>>> = Arc::new(Mutex::new(HashMap::new()));
        let results: Arc<Mutex<HashMap<usize, CaptureResult>>> = Arc::new(Mutex::new(HashMap::new()));
        let running = Arc::new(AtomicBool::new(true));

        let r_requests = Arc::clone(&requests);
        let r_results = Arc::clone(&results);
        let r_running = Arc::clone(&running);

        let handle = thread::Builder::new()
            .name("owerlayer-capture".to_string())
            .spawn(move || {
                Self::capture_loop(r_requests, r_results, r_running, interval);
            })
            .expect("Failed to spawn capture thread");

        Self {
            requests,
            results,
            running,
            handle: Some(handle),
            capture_interval_ms: interval,
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
        self.capture_interval_ms = (1000.0 / fps.clamp(5.0, 240.0)) as u64;
    }

    fn capture_loop(
        requests: Arc<Mutex<HashMap<usize, CaptureRequest>>>,
        results: Arc<Mutex<HashMap<usize, CaptureResult>>>,
        running: Arc<AtomicBool>,
        interval_ms: u64,
    ) {
        while running.load(Ordering::Relaxed) {
            let start = Instant::now();

            // Snapshot current requests
            let reqs: Vec<CaptureRequest> = if let Ok(map) = requests.lock() {
                map.values().cloned().collect()
            } else {
                Vec::new()
            };

            for req in &reqs {
                if !running.load(Ordering::Relaxed) { break; }

                let result = if req.hwnd != 0 {
                    // Window capture
                    Self::capture_window(req)
                } else {
                    // Screen rect capture
                    Self::capture_screen(req)
                };

                if let Some(result) = result {
                    if let Ok(mut map) = results.lock() {
                        map.insert(req.id, result);
                    }
                }
            }

            // Sleep until next interval
            let elapsed = start.elapsed();
            let target = Duration::from_millis(interval_ms);
            if elapsed < target {
                thread::sleep(target - elapsed);
            }
        }
    }

    fn capture_screen(req: &CaptureRequest) -> Option<CaptureResult> {
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

        Some(CaptureResult {
            pixels,
            size: [sw as usize, sh as usize],
        })
    }

    fn capture_window(req: &CaptureRequest) -> Option<CaptureResult> {
        let (pixels, pw, ph) = crate::winapi_utils::capture_window(req.hwnd)?;
        Some(CaptureResult {
            pixels,
            size: [pw, ph],
        })
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
    
    // Horizontal
    intermediate.par_chunks_exact_mut(pixels_per_row).for_each(|row| {
        let row_copy = row.to_vec();
        for x in 0..width {
            let mut r = 0u32; let mut g = 0u32; let mut b = 0u32; let mut a = 0u32;
            let mut count = 0;
            for i in -(radius as i32)..=(radius as i32) {
                let nx = x as i32 + i;
                if nx >= 0 && nx < width as i32 {
                    let idx = nx as usize * 4;
                    r += row_copy[idx] as u32;
                    g += row_copy[idx + 1] as u32;
                    b += row_copy[idx + 2] as u32;
                    a += row_copy[idx + 3] as u32;
                    count += 1;
                }
            }
            let idx = x * 4;
            row[idx] = (r / count) as u8;
            row[idx + 1] = (g / count) as u8;
            row[idx + 2] = (b / count) as u8;
            row[idx + 3] = (a / count) as u8;
        }
    });

    // Vertical (intermediate -> pixels)
    let intermediate_shared = &intermediate;
    pixels.par_chunks_exact_mut(pixels_per_row).enumerate().for_each(|(y, row)| {
        for x in 0..width {
            let mut r = 0u32; let mut g = 0u32; let mut b = 0u32; let mut a = 0u32;
            let mut count = 0;
            for i in -(radius as i32)..=(radius as i32) {
                let ny = y as i32 + i;
                if ny >= 0 && ny < height as i32 {
                    let idx = (ny as usize * width + x) * 4;
                    r += intermediate_shared[idx] as u32;
                    g += intermediate_shared[idx + 1] as u32;
                    b += intermediate_shared[idx + 2] as u32;
                    a += intermediate_shared[idx + 3] as u32;
                    count += 1;
                }
            }
            let idx = x * 4;
            row[idx] = (r / count) as u8;
            row[idx + 1] = (g / count) as u8;
            row[idx + 2] = (b / count) as u8;
            row[idx + 3] = (a / count) as u8;
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
