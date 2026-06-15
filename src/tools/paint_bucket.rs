use eframe::egui;
use crate::overlay::*;
use crate::utils::*;
use crate::tools::ToolContext;

fn expand_image_to_rect(img: &mut crate::types::PlacedImage, target_rect: egui::Rect, ppp: f32) {
    let old_iw = img.size[0];
    let old_ih = img.size[1];
    if old_iw == 0 || old_ih == 0 { return; }

    let new_dw = target_rect.width();
    let new_dh = target_rect.height();
    let new_iw = (new_dw * ppp).round() as usize;
    let new_ih = (new_dh * ppp).round() as usize;
    
    if new_iw <= old_iw && new_ih <= old_ih {
        return; // Already large enough
    }
    
    let mut new_pixels = vec![0u8; new_iw * new_ih * 4];
    
    // Relative offset of old image top-left from new target_rect top-left in display space
    let offset_dw = img.position.x - target_rect.min.x;
    let offset_dh = img.position.y - target_rect.min.y;
    
    // Convert offsets to pixel space of the new image
    let offset_px = (offset_dw * ppp).round() as i32;
    let offset_py = (offset_dh * ppp).round() as i32;
    
    for y in 0..old_ih {
        let dest_y = y as i32 + offset_py;
        if dest_y < 0 || dest_y >= new_ih as i32 { continue; }
        for x in 0..old_iw {
            let dest_x = x as i32 + offset_px;
            if dest_x < 0 || dest_x >= new_iw as i32 { continue; }
            
            let old_idx = (y * old_iw + x) * 4;
            let new_idx = (dest_y as usize * new_iw + dest_x as usize) * 4;
            
            if old_idx + 3 < img.pixels.len() && new_idx + 3 < new_pixels.len() {
                new_pixels[new_idx..new_idx+4].copy_from_slice(&img.pixels[old_idx..old_idx+4]);
            }
        }
    }
    
    img.position = target_rect.min;
    img.size = [new_iw, new_ih];
    img.display_size = Some([new_dw, new_dh]);
    img.pixels = new_pixels;
    img.texture = None;
    img.thumbnail_dirty = true;
}

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
    
    if left_just_pressed && is_locked {
        *ctx.layer_prompt_open = true;
        return;
    }
    
    if left_just_pressed && !is_locked {
        // 1. Vector shape interior fill check
        let mut filled_vector_shape = false;
        let mut check_layer_idx = Some(active_layer_idx);
        if let Some(idx) = check_layer_idx {
            let layer = &mut project.layers[idx];
            // Iterate in reverse order to target top-most stroke first
            for s in layer.strokes.iter_mut().rev() {
                if s.points.is_empty() { continue; }
                let inside = match s.kind {
                    StrokeKind::Rect => {
                        if s.points.len() >= 2 {
                            let r = egui::Rect::from_two_pos(s.points[0], s.points[1]);
                            r.contains(pos)
                        } else { false }
                    }
                    StrokeKind::Circle => {
                        if s.points.len() >= 2 {
                            let r = egui::Rect::from_two_pos(s.points[0], s.points[1]);
                            let center = r.center();
                            let radius = r.width().min(r.height()) * 0.5;
                            pos.distance(center) <= radius
                        } else { false }
                    }
                    StrokeKind::Star => {
                        if s.points.len() >= 2 {
                            let radius = s.points[0].distance(s.points[1]);
                            let pts = get_star_points(s.points[0], radius);
                            is_inside_poly(&pts, pos)
                        } else { false }
                    }
                    StrokeKind::Heart => {
                        if s.points.len() >= 2 {
                            let radius = s.points[0].distance(s.points[1]);
                            let pts = get_heart_points(s.points[0], radius);
                            is_inside_poly(&pts, pos)
                        } else { false }
                    }
                    StrokeKind::Poly => {
                        is_inside_poly(&s.points, pos)
                    }
                    _ => false,
                };
                if inside {
                    s.background_color = Some(settings.pen_color);
                    filled_vector_shape = true;
                    *ctx.request_history_push = Some("Paint Bucket".into());
                    break;
                }
            }
        }
        
        if filled_vector_shape {
            return;
        }

        // 2. Fallback to bitmap canvas flood fill
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
                
                // Expand the reused image to cover the marquee selection!
                if let Some(sel) = &project.marquee_selection {
                    let ppp = ctx.ui.ctx().pixels_per_point();
                    let img = &mut project.layers[active_layer_idx].placed_images[idx];
                    let img_rect = egui::Rect::from_min_size(img.position, egui::vec2(
                        img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0],
                        img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[1],
                    ));
                    let sel_rect = sel.bounds().expand(50.0);
                    if !img_rect.contains(sel_rect.min) || !img_rect.contains(sel_rect.max) {
                        let target_rect = img_rect.union(sel_rect);
                        expand_image_to_rect(img, target_rect, ppp);
                    }
                }
            } else {
                let ppp = ctx.ui.ctx().pixels_per_point();
                let (img_pos, logical_w, logical_h) = if let Some(sel) = &project.marquee_selection {
                    let sel_rect = sel.bounds().expand(50.0);
                    (sel_rect.min, sel_rect.width(), sel_rect.height())
                } else {
                    let render_offset = ctx.render_offset;
                    let world_pos = pos + render_offset;
                    let logical_w = 800.0f32;
                    let logical_h = 600.0f32;
                    (egui::pos2(world_pos.x - logical_w / 2.0, world_pos.y - logical_h / 2.0), logical_w, logical_h)
                };
                
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
        } else {
            // Reused active image already exists, expand it to cover the marquee selection!
            if let Some(sel) = &project.marquee_selection {
                if let Some(sel_obj) = project.selected_object {
                    if sel_obj.object_type == ObjectType::Image && sel_obj.layer_idx == active_layer_idx {
                        let ppp = ctx.ui.ctx().pixels_per_point();
                        let img = &mut project.layers[active_layer_idx].placed_images[sel_obj.object_idx];
                        let img_rect = egui::Rect::from_min_size(img.position, egui::vec2(
                            img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0],
                            img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[1],
                        ));
                        let sel_rect = sel.bounds().expand(50.0);
                        if !img_rect.contains(sel_rect.min) || !img_rect.contains(sel_rect.max) {
                            let target_rect = img_rect.union(sel_rect);
                            expand_image_to_rect(img, target_rect, ppp);
                        }
                    }
                }
            }
        }
        
        // Target the selected image
        if let Some(sel) = project.selected_object {
            if sel.object_type == ObjectType::Image && sel.layer_idx == active_layer_idx {
                let strokes = project.layers[active_layer_idx].strokes.clone();
                project.layers[active_layer_idx].strokes.clear();
                
                let img = &mut project.layers[active_layer_idx].placed_images[sel.object_idx];
                if !img.locked {
                    for stroke in &strokes {
                        crate::tools::brush::rasterize_stroke_to_image(img, stroke, settings);
                    }

                    if project.marquee_selection.is_some() {
                        paint_bucket_flood_fill(img, 0, 0, [0,0,0,0], settings.pen_color, settings.magic_wand_threshold, &project.marquee_selection);
                        img.texture = None;
                        img.thumbnail_dirty = true;
                        *ctx.request_history_push = Some("Paint Bucket".into());
                    } else {
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
                                
                                paint_bucket_flood_fill(img, px, py, start_color, fill_color, settings.magic_wand_threshold, &project.marquee_selection);
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
}

pub fn paint_bucket_flood_fill(
    img: &mut crate::types::PlacedImage,
    start_x: i32,
    start_y: i32,
    target_color: [u8; 4],
    fill_color: [u8; 4],
    threshold: f32,
    selection: &Option<crate::types::MarqueeSelection>,
) {
    let w = img.size[0] as i32;
    let h = img.size[1] as i32;
    if start_x < 0 || start_x >= w || start_y < 0 || start_y >= h { return; }
    
    let dw = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
    let dh = img.display_size.unwrap_or([img.size[1] as f32, img.size[1] as f32])[1];
    let scale_x = dw / img.size[0] as f32;
    let scale_y = dh / img.size[1] as f32;
    
    let center = img.position + egui::vec2(dw * 0.5, dh * 0.5);

    let map_to_world = |x: i32, y: i32| -> egui::Pos2 {
        let mut local_x = x as f32 * scale_x;
        let mut local_y = y as f32 * scale_y;
        if img.flipped_h {
            local_x = dw - local_x;
        }
        if img.flipped_v {
            local_y = dh - local_y;
        }
        let p_untrans = img.position + egui::vec2(local_x, local_y);
        transform_point_complex(
            p_untrans,
            center,
            img.rotation,
            img.skew,
            img.perspective,
            egui::Rect::from_min_size(img.position, egui::vec2(dw, dh)),
            img.scale,
        )
    };

    if let Some(sel) = selection {
        for y in 0..h {
            for x in 0..w {
                let idx = (y * w + x) as usize;
                let pixel_idx = idx * 4;
                let world_pos = map_to_world(x, y);
                
                if sel.contains(world_pos) {
                    img.pixels[pixel_idx] = fill_color[0];
                    img.pixels[pixel_idx + 1] = fill_color[1];
                    img.pixels[pixel_idx + 2] = fill_color[2];
                    img.pixels[pixel_idx + 3] = fill_color[3];
                }
            }
        }
        return;
    }

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
            // Check selection boundary constraint if selection is active
            let mut in_selection = true;
            if let Some(sel) = selection {
                let world_pos = map_to_world(x, y);
                in_selection = sel.contains(world_pos);
            }

            if in_selection {
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
}
