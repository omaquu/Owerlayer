use eframe::egui;
use std::sync::{Arc, Mutex};
use crate::types::{RasterizeRequest, ObjectType, PlacedImage};
use crate::project::Project;

/// Shared buffer for glReadPixels result
pub type CaptureBuffer = Arc<Mutex<Option<CapturedFrame>>>;

pub struct CapturedFrame {
    pub pixels: Vec<u8>,
    pub width: i32,
    pub height: i32,
    pub bbox: [f32; 4], // min_x, min_y, max_x, max_y in egui coords
}

pub fn new_capture_buffer() -> CaptureBuffer {
    Arc::new(Mutex::new(None))
}

/// Phase 1: Compute bounding box of the target objects.
/// Returns [min_x, min_y, max_x, max_y] in egui logical coords.
pub fn compute_target_bbox(project: &Project, req: &RasterizeRequest, render_offset: egui::Vec2) -> Option<[f32; 4]> {
    if req.layer_idx >= project.layers.len() { return None; }
    let layer = &project.layers[req.layer_idx];

    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;

    let mut found = false;

    match req.object_idx {
        Some((ObjectType::Stroke, idx)) => {
            if idx < layer.strokes.len() {
                let s = &layer.strokes[idx];
                for pt in &s.points {
                    let p = *pt - render_offset;
                    min_x = min_x.min(p.x); min_y = min_y.min(p.y);
                    max_x = max_x.max(p.x); max_y = max_y.max(p.y);
                }
                // Expand by stroke width
                let expand = s.width * 0.5 + 4.0;
                min_x -= expand; min_y -= expand;
                max_x += expand; max_y += expand;
                found = true;
            }
        }
        Some((ObjectType::Text, idx)) => {
            if idx < layer.text_annotations.len() {
                let ann = &layer.text_annotations[idx];
                let p = ann.position - render_offset;
                min_x = p.x; min_y = p.y;
                max_x = p.x + ann.exact_size[0];
                max_y = p.y + ann.exact_size[1];
                // Expand for shadow/outline
                let expand = 8.0;
                min_x -= expand; min_y -= expand;
                max_x += expand; max_y += expand;
                found = true;
            }
        }
        Some((ObjectType::Image, _)) => {
            // Images are already rasterized, no need
            return None;
        }
        None => {
            // Rasterize whole layer: union of all object bboxes
            for s in &layer.strokes {
                for pt in &s.points {
                    let p = *pt - render_offset;
                    min_x = min_x.min(p.x); min_y = min_y.min(p.y);
                    max_x = max_x.max(p.x); max_y = max_y.max(p.y);
                }
                let expand = s.width * 0.5 + 4.0;
                min_x -= expand; min_y -= expand;
                max_x += expand; max_y += expand;
                found = true;
            }
            for ann in &layer.text_annotations {
                let p = ann.position - render_offset;
                min_x = min_x.min(p.x); min_y = min_y.min(p.y);
                max_x = max_x.max(p.x + ann.exact_size[0]);
                max_y = max_y.max(p.y + ann.exact_size[1]);
                found = true;
            }
            for img in &layer.placed_images {
                let p = img.position - render_offset;
                let ds = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32]);
                min_x = min_x.min(p.x); min_y = min_y.min(p.y);
                max_x = max_x.max(p.x + ds[0]);
                max_y = max_y.max(p.y + ds[1]);
                found = true;
            }
            if found {
                let expand = 8.0;
                min_x -= expand; min_y -= expand;
                max_x += expand; max_y += expand;
            }
        }
    }

    if found && max_x > min_x && max_y > min_y {
        Some([min_x, min_y, max_x, max_y])
    } else {
        None
    }
}

/// Phase 2: After glReadPixels, process the captured frame into a PlacedImage
/// and replace the original objects in the project.
pub fn finalize_rasterize(project: &mut Project, req: &RasterizeRequest, frame: CapturedFrame, render_offset: egui::Vec2) {
    if req.layer_idx >= project.layers.len() { return; }

    let id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as usize;

    let w = frame.width as usize;
    let h = frame.height as usize;
    let position = egui::pos2(frame.bbox[0], frame.bbox[1]) + render_offset;

    let mut img = PlacedImage::new(id, position, [w, h], frame.pixels);
    img.name = "Rasterized".to_string();
    img.display_size = Some([frame.bbox[2] - frame.bbox[0], frame.bbox[3] - frame.bbox[1]]);

    let layer = &mut project.layers[req.layer_idx];

    match req.object_idx {
        Some((ObjectType::Stroke, idx)) => {
            // Copy FX properties from the stroke before removing it
            if idx < layer.strokes.len() {
                layer.strokes.remove(idx);
            }
            layer.placed_images.push(img);
        }
        Some((ObjectType::Text, idx)) => {
            if idx < layer.text_annotations.len() {
                layer.text_annotations.remove(idx);
            }
            layer.placed_images.push(img);
        }
        None => {
            // Rasterize entire layer: clear all objects and replace with single image
            layer.strokes.clear();
            layer.text_annotations.clear();
            layer.placed_images.clear();
            layer.placed_images.push(img);
        }
        _ => {}
    }

    // Clear selection since the object indices changed
    project.selected_object = None;
}

/// Create the PaintCallback that performs glReadPixels on the target region.
pub fn create_capture_callback(
    bbox: [f32; 4],
    ppp: f32,
    screen_height: f32,
    capture_buf: CaptureBuffer,
) -> egui::PaintCallback {
    egui::PaintCallback {
        rect: egui::Rect::from_min_max(
            egui::pos2(bbox[0], bbox[1]),
            egui::pos2(bbox[2], bbox[3]),
        ),
        callback: Arc::new(egui_glow::CallbackFn::new(move |_info, render_ctx: &egui_glow::Painter| {
            let gl = render_ctx.gl();

            // Convert egui logical coords to physical pixel coords
            let px_x = (bbox[0] * ppp).floor() as i32;
            let px_w = ((bbox[2] - bbox[0]) * ppp).ceil() as i32;
            let px_h = ((bbox[3] - bbox[1]) * ppp).ceil() as i32;
            // GL Y is inverted (0 = bottom)
            let px_y = ((screen_height - bbox[3]) * ppp).floor() as i32;

            if px_w <= 0 || px_h <= 0 { return; }

            let mut pixels = vec![0u8; (px_w * px_h * 4) as usize];
            unsafe {
                use glow::HasContext;
                gl.pixel_store_i32(glow::PACK_ALIGNMENT, 1);
                gl.pixel_store_i32(glow::PACK_ROW_LENGTH, 0);
                gl.pixel_store_i32(glow::PACK_SKIP_PIXELS, 0);
                gl.pixel_store_i32(glow::PACK_SKIP_ROWS, 0);
                gl.read_pixels(
                    px_x, px_y, px_w, px_h,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    glow::PixelPackData::Slice(Some(&mut pixels)),
                );
            }

            // Flip vertically (GL returns bottom-up)
            let row_bytes = (px_w * 4) as usize;
            let mut flipped = vec![0u8; pixels.len()];
            for y in 0..px_h as usize {
                let src_row = (px_h as usize - 1 - y) * row_bytes;
                let dst_row = y * row_bytes;
                flipped[dst_row..dst_row + row_bytes].copy_from_slice(&pixels[src_row..src_row + row_bytes]);
            }

            if let Ok(mut buf) = capture_buf.lock() {
                *buf = Some(CapturedFrame {
                    pixels: flipped,
                    width: px_w,
                    height: px_h,
                    bbox,
                });
            }
        })),
    }
}
