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
    // frame.bbox is in egui logical points; position must include render_offset for world coords
    let position = egui::pos2(frame.bbox[0], frame.bbox[1]) + render_offset;
    // display_size is the logical size of the bbox
    let display_w = frame.bbox[2] - frame.bbox[0];
    let display_h = frame.bbox[3] - frame.bbox[1];

    let mut img = PlacedImage::new(id, position, [w, h], frame.pixels);
    img.name = "Rasterized".to_string();
    // Physical pixels may differ from logical size due to ppp; always set display_size
    img.display_size = Some([display_w, display_h]);

    let layer = &mut project.layers[req.layer_idx];

    match req.object_idx {
        Some((ObjectType::Stroke, idx)) => {
            if idx < layer.strokes.len() {
                let s = &layer.strokes[idx];
                img.name = s.name.clone();
                img.shadow = s.shadow;
                img.rotation = s.rotation;
                img.flipped_h = s.flipped_h;
                img.flipped_v = s.flipped_v;
                img.perspective = s.perspective;
                img.scale = s.scale;
                img.skew = s.skew;
                img.visible = s.visible;
                img.opacity = s.opacity;
                img.outline = s.outline;
                img.outline_color = s.outline_color;
                img.outline_width = s.outline_width;
                img.shadow_color = s.shadow_color;
                img.shadow_offset = s.shadow_offset;
                img.shadow_blur = s.shadow_blur;
                img.blur = s.blur;
                img.blur_effect = s.blur_effect.clone();
                img.grayscale = s.grayscale;
                img.invert = s.invert;
                img.sepia = s.sepia;
                img.glow = s.glow;
                img.glow_strength = s.glow_strength;
                img.locked = s.locked;
                
                layer.strokes.remove(idx);
            }
            layer.placed_images.push(img);
        }
        Some((ObjectType::Text, idx)) => {
            if idx < layer.text_annotations.len() {
                let t = &layer.text_annotations[idx];
                img.name = t.name.clone();
                img.shadow = t.shadow;
                img.rotation = t.rotation;
                img.flipped_h = t.flipped_h;
                img.flipped_v = t.flipped_v;
                img.perspective = t.perspective;
                img.scale = t.scale;
                img.skew = t.skew;
                img.visible = t.visible;
                img.opacity = t.opacity;
                img.outline = t.outline;
                img.outline_color = t.outline_color;
                img.outline_width = t.outline_width;
                img.shadow_color = t.shadow_color;
                img.shadow_offset = t.shadow_offset;
                img.shadow_blur = t.shadow_blur;
                img.blur = t.blur;
                img.blur_effect = unsafe { std::mem::transmute(t.blur_effect.clone()) }; // Transmute as they are identical layout
                img.grayscale = t.grayscale;
                img.invert = t.invert;
                img.sepia = t.sepia;
                img.glow = t.glow;
                img.glow_strength = t.glow_strength;
                img.locked = t.locked;

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
