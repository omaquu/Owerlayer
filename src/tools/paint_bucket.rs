use eframe::egui;
use crate::overlay::*;
use crate::tools::ToolContext;

pub fn update(ctx: &mut ToolContext) {
    if *ctx.layer_prompt_open { return; }
    
    if ctx.mouse.left_just_pressed {
        ctx.auto_create_layer();
    }
    
    let project = &mut *ctx.project;
    let settings = &mut *ctx.settings;
    
    let active_layer_idx = project.active_layer;
    if active_layer_idx >= project.layers.len() { return; }

    let layer = &project.layers[active_layer_idx];
    let is_locked = layer.locked;
    
    let mouse = ctx.mouse;
    let pos = mouse.pos;
    let left_just_pressed = mouse.left_just_pressed;
    
    if left_just_pressed && !is_locked {
        // Find or create target PlacedImage
        let has_target_image = project.selected_object.map_or(false, |s| {
            s.object_type == ObjectType::Image
                && s.layer_idx == active_layer_idx
                && s.object_idx < project.layers[active_layer_idx].placed_images.len()
        });
        
        if !has_target_image {
            let reuse_idx = project.layers[active_layer_idx]
                .placed_images
                .iter()
                .rposition(|img| !img.locked);

            if let Some(idx) = reuse_idx {
                project.selected_object = Some(SelectedObject {
                    layer_idx: active_layer_idx,
                    object_type: ObjectType::Image,
                    object_idx: idx,
                });
            } else {
                let ppp = ctx.ui.ctx().pixels_per_point();
                let logical_w = 800.0f32;
                let logical_h = 600.0f32;
                let render_offset = ctx.render_offset;
                let world_pos = pos + render_offset;
                let img_pos = egui::pos2(world_pos.x - logical_w / 2.0, world_pos.y - logical_h / 2.0);
                let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
                let count = project.layers[active_layer_idx].placed_images.len();
                let mut new_img = crate::tools::brush::create_new_canvas(id, img_pos, logical_w, logical_h, ppp);
                new_img.name = format!("Canvas {}", count + 1);
                project.layers[active_layer_idx].placed_images.push(new_img);
                project.layers[active_layer_idx].expanded = true;
                let new_idx = project.layers[active_layer_idx].placed_images.len() - 1;
                project.selected_object = Some(SelectedObject {
                    layer_idx: active_layer_idx,
                    object_type: ObjectType::Image,
                    object_idx: new_idx,
                });
            }
        }
        
        // Target the selected image
        if let Some(sel) = project.selected_object {
            if sel.object_type == ObjectType::Image && sel.layer_idx == active_layer_idx {
                let img = &mut project.layers[active_layer_idx].placed_images[sel.object_idx];
                if !img.locked {
                    let dw = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
                    let dh = img.display_size.unwrap_or([img.size[1] as f32, img.size[1] as f32])[1];
                    let iw = img.size[0];
                    let ih = img.size[1];
                    if iw > 0 && ih > 0 && dw > 0.0 && dh > 0.0 {
                        let scale_x = iw as f32 / dw;
                        let scale_y = ih as f32 / dh;
                        
                        let render_offset = ctx.render_offset;
                        let world_pt = pos + render_offset;
                        let center = img.position + egui::vec2(dw * 0.5, dh * 0.5);
                        let rel_world = world_pt - center;
                        
                        let cos = img.rotation.cos();
                        let sin = img.rotation.sin();
                        let px_rot = rel_world.x * cos + rel_world.y * sin;
                        let py_rot = rel_world.y * cos - rel_world.x * sin;
                        
                        let sx = img.scale.x; let sy = img.scale.y;
                        let kx = img.skew.x; let ky = img.skew.y;
                        let det = 1.0 - kx * ky;
                        let (rel_x, rel_y) = if det.abs() > 0.001 && sx.abs() > 0.001 && sy.abs() > 0.001 {
                            ((px_rot - py_rot * kx) / (sx * det), (py_rot - px_rot * ky) / (sy * det))
                        } else {
                            (px_rot / sx.max(0.001), py_rot / sy.max(0.001))
                        };
                        
                        let base_p = center + egui::vec2(rel_x, rel_y);
                        let lx = (base_p.x - img.position.x) * scale_x;
                        let ly = (base_p.y - img.position.y) * scale_y;
                        
                        let px = lx.round() as i32;
                        let py = ly.round() as i32;
                        
                        if px >= 0 && px < iw as i32 && py >= 0 && py < ih as i32 {
                            let idx = (py as usize * iw + px as usize) * 4;
                            let start_color = [img.pixels[idx], img.pixels[idx+1], img.pixels[idx+2], img.pixels[idx+3]];
                            let fill_color = settings.pen_color;
                            
                            paint_bucket_flood_fill(img, px, py, start_color, fill_color, settings.magic_wand_threshold);
                            img.texture = None;
                            img.thumbnail_dirty = true;
                            *ctx.request_history_push = Some("Paint Bucket".into());
                        }
                    }
                }
            }
        }
    }
}

pub fn paint_bucket_flood_fill(img: &mut crate::types::PlacedImage, start_x: i32, start_y: i32, target_color: [u8; 4], fill_color: [u8; 4], threshold: f32) {
    let w = img.size[0] as i32;
    let h = img.size[1] as i32;
    if start_x < 0 || start_x >= w || start_y < 0 || start_y >= h { return; }
    
    if target_color == fill_color {
        return;
    }
    
    let mut stack = vec![(start_x, start_y)];
    let mut visited = vec![false; (w * h) as usize];
    
    let color_diff = |c1: [u8; 4], c2: [u8; 4]| -> f32 {
        let dr = (c1[0] as f32 - c2[0] as f32).abs();
        let dg = (c1[1] as f32 - c2[1] as f32).abs();
        let db = (c1[2] as f32 - c2[2] as f32).abs();
        (dr + dg + db) / 3.0
    };

    while let Some((x, y)) = stack.pop() {
        if x < 0 || x >= w || y < 0 || y >= h { continue; }
        let idx = (y * w + x) as usize;
        if visited[idx] { continue; }
        visited[idx] = true;

        let pixel_idx = idx * 4;
        let current_color = [img.pixels[pixel_idx], img.pixels[pixel_idx+1], img.pixels[pixel_idx+2], img.pixels[pixel_idx+3]];
        
        let is_match = if target_color[3] == 0 {
            current_color[3] == 0
        } else {
            current_color[3] > 0 && color_diff(current_color, target_color) <= threshold
        };

        if is_match {
            img.pixels[pixel_idx] = fill_color[0];
            img.pixels[pixel_idx + 1] = fill_color[1];
            img.pixels[pixel_idx + 2] = fill_color[2];
            img.pixels[pixel_idx + 3] = fill_color[3];
            
            stack.push((x + 1, y));
            stack.push((x - 1, y));
            stack.push((x, y + 1));
            stack.push((x, y - 1));
        }
    }
}
