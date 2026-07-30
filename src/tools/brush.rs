use eframe::egui;
use crate::utils::*;
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

    let (lock_prompt_dismissed, lock_prompt_choice, is_layer_locked) = {
        let layer = &project.layers[active_layer_idx];
        (layer.lock_prompt_dismissed, layer.lock_prompt_choice, layer.locked)
    };
    let choice = if settings.auto_new_layer == Some(true) {
        Some(3)
    } else if lock_prompt_dismissed {
        lock_prompt_choice
    } else {
        None
    };

    let mut is_image_locked = false;
    if false { // all brush strokes are vector now
        let has_target_image = project.selected_object.map_or(false, |s| {
            s.object_type == ObjectType::Image
                && s.layer_idx == active_layer_idx
                && s.object_idx < project.layers[active_layer_idx].placed_images.len()
        });
        if has_target_image {
            let sel = project.selected_object.unwrap();
            is_image_locked = project.layers[active_layer_idx].placed_images[sel.object_idx].locked;
        } else {
            let reuse_idx = project.layers[active_layer_idx]
                .placed_images
                .iter()
                .rposition(|img| !img.locked);
            if reuse_idx.is_none() && !project.layers[active_layer_idx].placed_images.is_empty() {
                is_image_locked = true;
            }
        }
    }
    let is_locked = is_layer_locked || is_image_locked;

    let mouse = ctx.mouse;
    let current_stroke = &mut *ctx.current_stroke;
    let _line_start = &mut *ctx.line_start;
    let _active_tool = &mut *ctx.active_tool;
    let _last_tool_used = &mut *ctx.last_tool_used;
    let _remove_active_layer = &mut *ctx.remove_active_layer;
    let _snip_created = &mut *ctx.snip_created;
    let _new_selection = &mut *ctx.new_selection;
    let _switch_to_move = &mut *ctx.switch_to_move;
    let _embed_trigger = &mut *ctx.embed_trigger;
    let _pending_text = &mut *ctx.pending_text;
    let _initial_bounds = &mut *ctx.initial_bounds;
    let _initial_center = &mut *ctx.initial_center;
    let _initial_layer = &mut *ctx.initial_layer;
    let _drag_state = &mut *ctx.drag_state;
    let _dragging_source_rect = &mut *ctx.dragging_source_rect;
    let ui = &mut *ctx.ui;
    let canvas_response = ctx.canvas_response;
    let painter = ui.painter_at(canvas_response.rect);
    let pos = mouse.pos;
    let left_down = mouse.left_down;
    let _left_just_pressed = mouse.left_just_pressed;
    let left_just_released = mouse.left_just_released;
    let _right_just_pressed = ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Secondary));

            if left_down {
                let render_offset = ctx.render_offset;
                let world_pos = pos + render_offset;
                // Reject glitchy points (0,0) or huge jumps
                if pos.x < 1.0 && pos.y < 1.0 { return; }

                // ── Find or create target PlacedImage ──
                // Priority: keep the currently selected image, then fall back to the
                // last unlocked PlacedImage on this layer, then create a new one.
                let is_ask_mode = choice.is_none();
                if false { // all brush strokes are vector now
                    // ── Find or create target PlacedImage ──
                    let mut has_target_image = project.selected_object.map_or(false, |s| {
                        s.object_type == ObjectType::Image
                            && s.layer_idx == active_layer_idx
                            && s.object_idx < project.layers[active_layer_idx].placed_images.len()
                    });
                    if choice == Some(2) {
                        has_target_image = false; // Force creation of a new canvas
                    }

                    if !has_target_image && current_stroke.is_empty() {
                        let should_fallback = project.selected_object.is_none();
                        
                        if should_fallback {
                            let reuse_idx = project.layers[active_layer_idx]
                                .placed_images
                                .iter()
                                .rposition(|img| !img.locked);

                            if let Some(idx) = reuse_idx {
                                if project.layers[active_layer_idx].placed_images[idx].locked {
                                    *ctx.layer_prompt_open = true;
                                    return;
                                }
                                project.selected_object = Some(SelectedObject {
                                    layer_idx: active_layer_idx,
                                    object_type: ObjectType::Image,
                                    object_idx: idx,
                                });
                            } else {
                                let ppp = ui.ctx().pixels_per_point();
                                let logical_w = 800.0f32;
                                let logical_h = 600.0f32;
                                let img_pos = egui::pos2(world_pos.x - logical_w / 2.0, world_pos.y - logical_h / 2.0);
                                let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
                                let count = project.layers[active_layer_idx].placed_images.len();
                                let mut new_img = create_new_canvas(id, img_pos, logical_w, logical_h, ppp);
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
                            let ppp = ui.ctx().pixels_per_point();
                            let logical_w = 800.0f32;
                            let logical_h = 600.0f32;
                            let img_pos = egui::pos2(world_pos.x - logical_w / 2.0, world_pos.y - logical_h / 2.0);
                            let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
                            let count = project.layers[active_layer_idx].placed_images.len();
                            let mut new_img = create_new_canvas(id, img_pos, logical_w, logical_h, ppp);
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

                    if let Some(sel) = project.selected_object {
                        if sel.object_type == ObjectType::Image
                            && sel.layer_idx == active_layer_idx
                            && project.layers[active_layer_idx].placed_images.get(sel.object_idx).map_or(false, |img| img.locked)
                            && current_stroke.is_empty()
                        {
                            *ctx.layer_prompt_open = true;
                            return;
                        }
                    }
                }

                let prev_len = current_stroke.len();
                if let Some(last) = current_stroke.last().cloned() {
                    let dist = last.distance(pos);
                    if dist > 2000.0 { 
                        current_stroke.clear();
                        current_stroke.push(pos);
                    } else {
                        let step_dist = (settings.pen_width * 0.15).max(1.0);
                        if dist > step_dist {
                            let steps = (dist / step_dist) as usize;
                            for s in 1..=steps {
                                let t = s as f32 / steps as f32;
                                current_stroke.push(last.lerp(pos, t));
                            }
                        }
                    }
                } else { current_stroke.push(pos); }
                let new_points_count = current_stroke.len() - prev_len;

                let is_ask_mode = choice.is_none();
                if false { // all brush strokes are vector now
                    if let Some(sel) = project.selected_object {
                        if sel.object_type == ObjectType::Image && sel.layer_idx == active_layer_idx {
                            if let Some(img) = project.layers[active_layer_idx].placed_images.get_mut(sel.object_idx) {
                                let mut dw = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
                                let mut dh = img.display_size.unwrap_or([img.size[1] as f32, img.size[1] as f32])[1];
                                let mut iw = img.size[0];
                                let mut ih = img.size[1];
                                if iw > 0 && ih > 0 && dw > 0.0 && dh > 0.0 {
                                    let scale_x = iw as f32 / dw;
                                    let scale_y = ih as f32 / dh;
                                    let radius = (settings.pen_width / 2.0 * scale_x).max(1.0);
                                    let mut color = settings.pen_color;
                                    if settings.brush_mode == BrushMode::Highlighter {
                                        color[3] = (color[3] as f32 * settings.highlight_opacity) as u8;
                                    }

                                    if !img.locked && img.rotation.abs() < 0.01 && img.skew.length() < 0.01 && img.perspective == [egui::Vec2::ZERO; 4] {
                                        let mut min_tx = 0.0f32;
                                        let mut min_ty = 0.0f32;
                                        let mut max_tx = iw as f32;
                                        let mut max_ty = ih as f32;

                                        for &pt in current_stroke.iter().skip(current_stroke.len().saturating_sub(new_points_count)) {
                                            let world_pt = pt + render_offset;
                                            let center = img.position + egui::vec2(dw * 0.5, dh * 0.5);
                                            let rel_world = world_pt - center;
                                            let base_p = center + rel_world;
                                            let lx = (base_p.x - img.position.x) * scale_x;
                                            let ly = (base_p.y - img.position.y) * scale_y;
                                            if lx - radius < min_tx { min_tx = lx - radius; }
                                            if ly - radius < min_ty { min_ty = ly - radius; }
                                            if lx + radius > max_tx { max_tx = lx + radius; }
                                            if ly + radius > max_ty { max_ty = ly + radius; }
                                        }

                                        let exp_l = (-min_tx).ceil() as usize;
                                        let exp_t = (-min_ty).ceil() as usize;
                                        let exp_r = (max_tx - iw as f32).ceil() as usize;
                                        let exp_b = (max_ty - ih as f32).ceil() as usize;

                                        if exp_l > 0 || exp_t > 0 || exp_r > 0 || exp_b > 0 {
                                            let new_iw = iw + exp_l + exp_r;
                                            let new_ih = ih + exp_t + exp_b;
                                            let mut new_pixels = vec![0u8; new_iw * new_ih * 4];

                                            for y in 0..ih {
                                                let src_s = y * iw * 4;
                                                let dst_s = ((y + exp_t) * new_iw + exp_l) * 4;
                                                if src_s + iw * 4 <= img.pixels.len() && dst_s + iw * 4 <= new_pixels.len() {
                                                    new_pixels[dst_s..dst_s + iw * 4].copy_from_slice(&img.pixels[src_s..src_s + iw * 4]);
                                                }
                                            }

                                            img.pixels = new_pixels;
                                            img.size = [new_iw, new_ih];
                                            iw = new_iw; ih = new_ih;

                                            let disp_l = exp_l as f32 / scale_x;
                                            let disp_t = exp_t as f32 / scale_y;
                                            let disp_r = exp_r as f32 / scale_x;
                                            let disp_b = exp_b as f32 / scale_y;

                                            img.position.x -= disp_l;
                                            img.position.y -= disp_t;
                                            dw += disp_l + disp_r;
                                            dh += disp_t + disp_b;
                                            img.display_size = Some([dw, dh]);
                                        }
                                    }

                                    let points_to_process = if prev_len > 0 {
                                        &current_stroke[prev_len - 1..]
                                    } else {
                                        &current_stroke[..]
                                    };

                                    let mut canvas_pts = Vec::with_capacity(points_to_process.len());
                                    for &pt in points_to_process {
                                        let world_pt = pt + render_offset;
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
                                        canvas_pts.push(egui::pos2(lx, ly));
                                    }

                                    let mut dirs = Vec::with_capacity(canvas_pts.len());
                                    for i in 0..canvas_pts.len() {
                                        let dir = if i < canvas_pts.len() - 1 {
                                            let d = canvas_pts[i+1] - canvas_pts[i];
                                            if d.length() > 0.001 { d.normalized() } else { egui::vec2(1.0, 0.0) }
                                        } else if i > 0 {
                                            let d = canvas_pts[i] - canvas_pts[i-1];
                                            if d.length() > 0.001 { d.normalized() } else { egui::vec2(1.0, 0.0) }
                                        } else {
                                            egui::vec2(1.0, 0.0)
                                        };
                                        dirs.push(dir);
                                    }

                                    match settings.brush_mode {
                                        BrushMode::Spray => {
                                            let mut rng = 42u32;
                                            let half_w = radius;
                                            let dot_radius = scale_x.max(0.8);
                                            let start_idx = if prev_len > 0 { 1 } else { 0 };
                                            for &p in &canvas_pts[start_idx..] {
                                                for _ in 0..settings.spray_density {
                                                    rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
                                                    let rx = ((rng % 1000) as f32 / 500.0 - 1.0) * half_w;
                                                    rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
                                                    let ry = ((rng % 1000) as f32 / 500.0 - 1.0) * half_w;

                                                    let inside = match settings.brush_shape {
                                                        BrushShape::Round => rx*rx + ry*ry <= half_w*half_w,
                                                        BrushShape::Square => true,
                                                    };

                                                    if inside {
                                                        draw_pixel_shape(img, p.x + rx, p.y + ry, dot_radius, color, iw, ih, false, settings.brush_hardness);
                                                    }
                                                }
                                            }
                                        }
                                        BrushMode::Real => {
                                            let mut rng = 42u32;
                                            let num_bristles = (settings.pen_width * 0.4).clamp(4.0, 10.0) as usize;
                                            let mut bristle_offsets = Vec::new();
                                            
                                            for _ in 0..num_bristles {
                                                rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
                                                let rx = (rng % 1000) as f32 / 500.0 - 1.0 ;
                                                rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
                                                let ry = (rng % 1000) as f32 / 500.0 - 1.0 ;
                                                
                                                let (off_x, off_y) = if settings.brush_shape == BrushShape::Round {
                                                    let len = (rx*rx + ry*ry).sqrt().max(0.001);
                                                    let scale = if len > 1.0 { 1.0 / len } else { 1.0 };
                                                    (rx * scale * radius * 0.9, ry * scale * radius * 0.9)
                                                } else {
                                                    (rx * radius * 0.9, ry * radius * 0.9)
                                                };
                                                
                                                rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
                                                let alpha_mod = (rng % 100) as f32 / 100.0 * 0.5 + 0.1;
                                                let mut b_col = color;
                                                b_col[3] = (b_col[3] as f32 * alpha_mod) as u8;
                                                
                                                let b_radius = (radius * 0.5).max(1.0);
                                                bristle_offsets.push((off_x, off_y, b_col, b_radius));
                                            }

                                            if prev_len == 0 && !canvas_pts.is_empty() {
                                                let p = canvas_pts[0];
                                                for &(ox, oy, b_col, b_radius) in &bristle_offsets {
                                                    draw_pixel_shape(img, p.x + ox, p.y + oy, b_radius, b_col, iw, ih, settings.brush_shape == BrushShape::Square, settings.brush_hardness);
                                                }
                                            }

                                            for i in 1..canvas_pts.len() {
                                                let p1 = canvas_pts[i-1];
                                                let p2 = canvas_pts[i];
                                                let dist = p1.distance(p2);
                                                if dist > 0.001 {
                                                    let min_b_radius = bristle_offsets.iter().map(|&(_, _, _, r)| r).fold(f32::MAX, f32::min);
                                                    let step_size = (min_b_radius * 0.2).clamp(0.2, 1.0);
                                                    let steps = (dist / step_size).ceil() as usize;
                                                    for step in 1..=steps {
                                                        let t = step as f32 / steps as f32;
                                                        let pi = p1.lerp(p2, t);
                                                        for &(ox, oy, b_col, b_radius) in &bristle_offsets {
                                                            draw_pixel_shape(img, pi.x + ox, pi.y + oy, b_radius, b_col, iw, ih, settings.brush_shape == BrushShape::Square, settings.brush_hardness);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        BrushMode::Calligraphy => {
                                            let nib_angle = std::f32::consts::PI / 4.0;
                                            let nib_dir = egui::vec2(nib_angle.cos(), nib_angle.sin());

                                            if prev_len == 0 && !canvas_pts.is_empty() {
                                                let p = canvas_pts[0];
                                                let dir = dirs[0];
                                                if settings.brush_shape == BrushShape::Round {
                                                    let cross = (dir.x * nib_dir.y - dir.y * nib_dir.x).abs();
                                                    let thickness = radius * (cross * 0.85 + 0.15);
                                                    draw_pixel_shape(img, p.x, p.y, thickness, color, iw, ih, false, settings.brush_hardness);
                                                } else {
                                                    let steps = (radius * 2.0) as usize + 2;
                                                    for step in 0..=steps {
                                                        let t = step as f32 / steps as f32 * 2.0 - 1.0;
                                                        let px = p.x + nib_dir.x * radius * t;
                                                        let py = p.y + nib_dir.y * radius * t;
                                                        draw_pixel_shape(img, px, py, scale_x.max(1.0), color, iw, ih, false, settings.brush_hardness);
                                                    }
                                                }
                                            }

                                            for i in 1..canvas_pts.len() {
                                                let p1 = canvas_pts[i-1];
                                                let p2 = canvas_pts[i];
                                                let dir1 = dirs[i-1];
                                                let dir2 = dirs[i];
                                                let dist = p1.distance(p2);
                                                if dist > 0.001 {
                                                    let step_size = (radius * 0.1).clamp(0.2, 1.0);
                                                    let steps = (dist / step_size).ceil() as usize;
                                                    for step in 1..=steps {
                                                        let t = step as f32 / steps as f32;
                                                        let pi = p1.lerp(p2, t);
                                                        let di = dir1 * (1.0 - t) + dir2 * t;
                                                        let di = if di.length() > 0.001 { di.normalized() } else { dir1 };

                                                        if settings.brush_shape == BrushShape::Round {
                                                            let cross = (di.x * nib_dir.y - di.y * nib_dir.x).abs();
                                                            let thickness = radius * (cross * 0.85 + 0.15);
                                                            draw_pixel_shape(img, pi.x, pi.y, thickness, color, iw, ih, false, settings.brush_hardness);
                                                        } else {
                                                            let steps_nib = (radius * 2.0) as usize + 2;
                                                            for step_nib in 0..=steps_nib {
                                                                let tn = step_nib as f32 / steps_nib as f32 * 2.0 - 1.0;
                                                                let px = pi.x + nib_dir.x * radius * tn;
                                                                let py = pi.y + nib_dir.y * radius * tn;
                                                                draw_pixel_shape(img, px, py, scale_x.max(1.0), color, iw, ih, false, settings.brush_hardness);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        _ => {
                                            if prev_len == 0 && !canvas_pts.is_empty() {
                                                draw_pixel_shape(img, canvas_pts[0].x, canvas_pts[0].y, radius, color, iw, ih, settings.brush_shape == BrushShape::Square, settings.brush_hardness);
                                            }

                                            for i in 1..canvas_pts.len() {
                                                let p1 = canvas_pts[i-1];
                                                let p2 = canvas_pts[i];
                                                let dist = p1.distance(p2);
                                                if dist > 0.001 {
                                                    let spacing_factor = (settings.brush_spacing / 100.0).clamp(0.02, 5.0);
                                                    let step_size = (radius * 2.0 * spacing_factor).max(0.5);
                                                    let steps = (dist / step_size).ceil() as usize;
                                                    for step in 1..=steps {
                                                        let t = step as f32 / steps as f32;
                                                        let pi = p1.lerp(p2, t);
                                                         draw_pixel_shape(img, pi.x, pi.y, radius, color, iw, ih, settings.brush_shape == BrushShape::Square, settings.brush_hardness);
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    img.clear_texture();
                                    img.thumbnail_dirty = true;
                                }
                            }
                        }
                    }
                }
            }

            if left_just_released && !current_stroke.is_empty() {
                let ask_mode = choice.is_none();
                if is_locked || ask_mode {
                    let s = Stroke::new(
                        current_stroke.clone(),
                        settings.pen_color,
                        settings.pen_width,
                        StrokeKind::Freehand,
                        settings.brush_mode,
                        Some(settings.background_color),
                        settings.brush_shadow,
                        settings.brush_shape,
                        settings.brush_outline,
                        settings.brush_arrow,
                        settings.spray_density,
                        settings.highlight_opacity,
                        settings.arrow_size,
                    );
                    *ctx.pending_stroke = Some(s);
                    *ctx.layer_prompt_open = true;
                } else {
                    if true { // all brush strokes saved as vector
                        if let Some(layer) = project.get_active_layer_mut() {
                            let s = Stroke::new(
                                current_stroke.clone(),
                                settings.pen_color,
                                settings.pen_width,
                                StrokeKind::Freehand,
                                settings.brush_mode,
                                Some(settings.background_color),
                                settings.brush_shadow,
                                settings.brush_shape,
                                settings.brush_outline,
                                settings.brush_arrow,
                                settings.spray_density,
                                settings.highlight_opacity,
                                settings.arrow_size,
                            );
                            layer.strokes.push(s);
                        }
                    } else if let Some(sel) = project.selected_object {
                        if sel.object_type == ObjectType::Image && sel.layer_idx == active_layer_idx {
                            if let Some(img) = project.layers[active_layer_idx].placed_images.get_mut(sel.object_idx) {
                                crop_to_content(img);
                            }
                        }
                    }
                    *ctx.request_history_push = Some("Brush".into());
                }
                current_stroke.clear();
            }

            // Visual cursor
            let cursor_r = settings.pen_width / 2.0; // Width is diameter, radius is half
            let color = color32(&settings.pen_color);
            let outline_color = if color.r() as u32 + color.g() as u32 + color.b() as u32 > 382 { egui::Color32::from_rgba_unmultiplied(0, 0, 0, 120) } else { egui::Color32::from_rgba_unmultiplied(255, 255, 255, 120) };

            if settings.brush_shape == BrushShape::Square {
                let rect = egui::Rect::from_center_size(pos, egui::vec2(cursor_r * 2.0, cursor_r * 2.0));
                painter.rect_filled(rect, 0.0, color.gamma_multiply(0.2));
                painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.5, color), egui::StrokeKind::Middle);
                painter.rect_stroke(rect.expand(1.5), 0.0, egui::Stroke::new(0.8, outline_color), egui::StrokeKind::Middle);
            } else {
                painter.circle_filled(pos, cursor_r, color.gamma_multiply(0.2));
                painter.circle_stroke(pos, cursor_r, egui::Stroke::new(1.5, color));
                painter.circle_stroke(pos, cursor_r + 1.5, egui::Stroke::new(0.8, outline_color));
            }

}

    pub fn draw_stroke(p: &egui::Painter, s: &Stroke, stroke_color: egui::Color32, offset: egui::Vec2, width: f32, l_op: f32) {
        if s.points.is_empty() { return; }
        
        // Calculate initial bounds for perspective calculation
        let mut min = egui::pos2(f32::MAX, f32::MAX);
        let mut max = egui::pos2(f32::MIN, f32::MIN);
        for &pt in &s.points {
            min.x = min.x.min(pt.x); min.y = min.y.min(pt.y);
            max.x = max.x.max(pt.x); max.y = max.y.max(pt.y);
        }
        let initial_rect = egui::Rect::from_min_max(min, max);
        let center = initial_rect.center();

        let pts: Vec<egui::Pos2> = s.points.iter().map(|&pt| {
            let mut transformed = transform_point_complex(pt, center, s.rotation, s.skew, s.perspective, initial_rect, s.scale);
            if s.flipped_h { transformed.x = center.x - (transformed.x - center.x); }
            if s.flipped_v { transformed.y = center.y - (transformed.y - center.y); }
            transformed + offset
        }).collect();
        
        let mut stroke_color = stroke_color;
        if s.brush_mode == BrushMode::Highlighter {
            stroke_color = egui::Color32::from_rgba_unmultiplied(stroke_color.r(), stroke_color.g(), stroke_color.b(), (stroke_color.a() as f32 * s.highlight_opacity) as u8);
        }
        // Shadow and Outline are handled by draw_layer_strokes
        match s.kind {
            StrokeKind::Freehand => {
                match s.brush_mode {
                    BrushMode::Spray => {
                        let mut rng = 42u32;
                        let half_w = width * 0.5;
                        for pt in &pts {
                            for _ in 0..s.spray_density {
                                rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
                                let rx = ((rng % 1000) as f32 / 500.0 - 1.0) * half_w;
                                rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
                                let ry = ((rng % 1000) as f32 / 500.0 - 1.0) * half_w;
                                
                                let inside = match s.brush_shape {
                                    BrushShape::Round => rx*rx + ry*ry <= half_w*half_w,
                                    BrushShape::Square => true,
                                };

                                if inside {
                                    p.circle_filled(*pt + egui::vec2(rx, ry), 1.0, stroke_color);
                                }
                            }
                        }
                    }
                    BrushMode::Calligraphy => {
                        let nib_angle = std::f32::consts::PI / 4.0;
                        let nib_dir = egui::vec2(nib_angle.cos(), nib_angle.sin());
                        
                        if s.brush_shape == BrushShape::Round {
                            // Round calligraphy: Dynamic width continuous mesh to avoid overlap artifacts
                            let mut mesh = egui::Mesh::default();
                            for i in 0..pts.len() {
                                let dir = if i < pts.len() - 1 {
                                    let d = pts[i+1] - pts[i];
                                    if d.length() > 0.001 { d.normalized() } else { egui::vec2(1.0, 0.0) }
                                } else if i > 0 {
                                    let d = pts[i] - pts[i-1];
                                    if d.length() > 0.001 { d.normalized() } else { egui::vec2(1.0, 0.0) }
                                } else {
                                    egui::vec2(1.0, 0.0)
                                };
                                
                                let cross = (dir.x * nib_dir.y - dir.y * nib_dir.x).abs();
                                let thickness = width * (cross * 0.85 + 0.15); // Dynamic thickness based on angle
                                let perp = egui::vec2(-dir.y, dir.x) * thickness * 0.5;
                                
                                mesh.vertices.push(egui::epaint::Vertex { pos: pts[i] + perp, uv: egui::Pos2::ZERO, color: stroke_color });
                                mesh.vertices.push(egui::epaint::Vertex { pos: pts[i] - perp, uv: egui::Pos2::ZERO, color: stroke_color });
                                
                                if i > 0 {
                                    let idx = (i as u32) * 2;
                                    mesh.indices.extend_from_slice(&[idx-2, idx-1, idx]);
                                    mesh.indices.extend_from_slice(&[idx-1, idx+1, idx]);
                                }
                            }
                            p.add(egui::Shape::mesh(mesh));
                        } else {
                            // Square calligraphy: Flat angled nib
                            let perp = nib_dir * width * 0.5;
                            for window in pts.windows(2) {
                                draw_quad(p, window[0] - perp, window[0] + perp, window[1] + perp, window[1] - perp, stroke_color);
                            }
                        }
                    }
                    BrushMode::Real => {
                        let mut rng = 42u32;
                        let num_bristles = (width * 0.4).clamp(4.0, 10.0) as usize;
                        let mut bristle_offsets = Vec::new();
                        
                        for _ in 0..num_bristles {
                            rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
                            let rx = (rng % 1000) as f32 / 500.0 - 1.0 ;
                            rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
                            let ry = (rng % 1000) as f32 / 500.0 - 1.0 ;
                            
                            let (off_x, off_y) = if s.brush_shape == BrushShape::Round {
                                let len = (rx*rx + ry*ry).sqrt().max(0.001);
                                let scale = if len > 1.0 { 1.0 / len } else { 1.0 };
                                (rx * scale * width * 0.45, ry * scale * width * 0.45)
                            } else {
                                (rx * width * 0.45, ry * width * 0.45)
                            };
                            
                            rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
                            let alpha_mod = (rng % 100) as f32 / 100.0 * 0.5 + 0.1;
                            let mut b_col = stroke_color;
                            b_col = egui::Color32::from_rgba_unmultiplied(b_col.r(), b_col.g(), b_col.b(), (b_col.a() as f32 * alpha_mod) as u8);
                            
                            let b_width = (width * 0.25).max(1.0);
                            bristle_offsets.push((egui::vec2(off_x, off_y), b_col, b_width));
                        }
                        
                        let mut mesh = egui::Mesh::default();
                        for (b_off, col, b_width) in bristle_offsets {
                            let mut smoothed: Vec<egui::Pos2> = Vec::new();
                            let min_dist = b_width * 0.3;
                            for &pt in &pts {
                                let p = pt + b_off;
                                if smoothed.is_empty() || smoothed.last().unwrap().distance(p) > min_dist {
                                    smoothed.push(p);
                                }
                            }
                            if smoothed.len() < 2 { continue; }
                            
                            let start_idx_base = mesh.vertices.len() as u32;
                            for i in 0..smoothed.len() {
                                let dir = if i < smoothed.len() - 1 {
                                    let d = smoothed[i+1] - smoothed[i];
                                    if d.length() > 0.001 { d.normalized() } else { egui::vec2(1.0, 0.0) }
                                } else if i > 0 {
                                    let d = smoothed[i] - smoothed[i-1];
                                    if d.length() > 0.001 { d.normalized() } else { egui::vec2(1.0, 0.0) }
                                } else {
                                    egui::vec2(1.0, 0.0)
                                };
                                let perp = egui::vec2(-dir.y, dir.x) * b_width * 0.5;
                                let pt = smoothed[i];
                                
                                mesh.vertices.push(egui::epaint::Vertex { pos: pt + perp, uv: egui::Pos2::ZERO, color: col });
                                mesh.vertices.push(egui::epaint::Vertex { pos: pt - perp, uv: egui::Pos2::ZERO, color: col });
                                
                                if i > 0 {
                                    let idx = start_idx_base + (i as u32) * 2;
                                    mesh.indices.extend_from_slice(&[idx-2, idx-1, idx]);
                                    mesh.indices.extend_from_slice(&[idx-1, idx+1, idx]);
                                }
                            }
                        }
                        p.add(egui::Shape::mesh(mesh));
                    }
                    _ => {
                        // Solid or Highlighter: Custom continuous non-overlapping mesh
                        let mut mesh = egui::Mesh::default();
                        
                        let mut smoothed: Vec<egui::Pos2> = Vec::new();
                        let min_dist = (width * 0.15).clamp(2.0, 10.0);
                        for &pt in &pts {
                            if smoothed.is_empty() || smoothed.last().unwrap().distance(pt) > min_dist {
                                smoothed.push(pt);
                            }
                        }
                        if smoothed.len() < 2 { smoothed = pts.clone(); }
                        let render_pts = smoothed;

                        for i in 0..render_pts.len() {
                            let dir = if i > 0 && i < render_pts.len() - 1 {
                                let d1 = (render_pts[i] - render_pts[i-1]).normalized();
                                let d2 = (render_pts[i+1] - render_pts[i]).normalized();
                                let mut miter = d1 + d2;
                                if miter.length() < 0.001 { miter = d1; }
                                miter.normalized()
                            } else if i < render_pts.len() - 1 {
                                let d = render_pts[i+1] - render_pts[i];
                                if d.length() > 0.001 { d.normalized() } else { egui::vec2(1.0, 0.0) }
                            } else if i > 0 {
                                let d = render_pts[i] - render_pts[i-1];
                                if d.length() > 0.001 { d.normalized() } else { egui::vec2(1.0, 0.0) }
                            } else {
                                egui::vec2(1.0, 0.0)
                            };

                            let perp = egui::vec2(-dir.y, dir.x) * width * 0.5;
                            
                            mesh.vertices.push(egui::epaint::Vertex { pos: render_pts[i] + perp, uv: egui::Pos2::ZERO, color: stroke_color });
                            mesh.vertices.push(egui::epaint::Vertex { pos: render_pts[i] - perp, uv: egui::Pos2::ZERO, color: stroke_color });
                            
                            if i > 0 {
                                let idx = (i as u32) * 2;
                                mesh.indices.extend_from_slice(&[idx-2, idx-1, idx]);
                                mesh.indices.extend_from_slice(&[idx-1, idx+1, idx]);
                            }
                        }

                        if render_pts.len() >= 2 {
                            let mut add_cap = |pt: egui::Pos2, dir: egui::Vec2, is_start: bool| {
                                if s.brush_shape == BrushShape::Round {
                                    let steps = 12;
                                    let center_idx = mesh.vertices.len() as u32;
                                    mesh.vertices.push(egui::epaint::Vertex { pos: pt, uv: egui::Pos2::ZERO, color: stroke_color });
                                    
                                    let base_angle = dir.angle() + if is_start { std::f32::consts::PI / 2.0 } else { -std::f32::consts::PI / 2.0 };
                                    for i in 0..=steps {
                                        let a = base_angle + (i as f32 / steps as f32) * std::f32::consts::PI;
                                        let pos = pt + egui::vec2(a.cos(), a.sin()) * width * 0.5;
                                        mesh.vertices.push(egui::epaint::Vertex { pos, uv: egui::Pos2::ZERO, color: stroke_color });
                                        if i > 0 {
                                            mesh.indices.extend_from_slice(&[center_idx, center_idx + i as u32, center_idx + i as u32 + 1]);
                                        }
                                    }
                                } else {
                                    let perp = egui::vec2(-dir.y, dir.x) * width * 0.5;
                                    let ext = dir * if is_start { -width * 0.5 } else { width * 0.5 };
                                    
                                    let p1 = pt + perp;
                                    let p2 = pt - perp;
                                    let p3 = pt - perp + ext;
                                    let p4 = pt + perp + ext;
                                    
                                    let start_idx = mesh.vertices.len() as u32;
                                    mesh.vertices.push(egui::epaint::Vertex { pos: p1, uv: egui::Pos2::ZERO, color: stroke_color });
                                    mesh.vertices.push(egui::epaint::Vertex { pos: p2, uv: egui::Pos2::ZERO, color: stroke_color });
                                    mesh.vertices.push(egui::epaint::Vertex { pos: p3, uv: egui::Pos2::ZERO, color: stroke_color });
                                    mesh.vertices.push(egui::epaint::Vertex { pos: p4, uv: egui::Pos2::ZERO, color: stroke_color });
                                    
                                    mesh.indices.extend_from_slice(&[start_idx, start_idx+1, start_idx+2]);
                                    mesh.indices.extend_from_slice(&[start_idx, start_idx+2, start_idx+3]);
                                }
                            };
                            
                            add_cap(render_pts[0], (render_pts[1] - render_pts[0]).normalized(), true);
                            add_cap(render_pts[render_pts.len()-1], (render_pts[render_pts.len()-1] - render_pts[render_pts.len()-2]).normalized(), false);
                        }
                        
                        p.add(egui::Shape::mesh(mesh));
                    }
                }

                if s.arrow && pts.len() >= 2 {
                    let end = pts[pts.len()-1];
                    let prev = pts[pts.len()-2];
                    let dir = (end - prev).normalized();
                    let perp = egui::vec2(-dir.y, dir.x);
                    let head_len = match s.arrow_size {
                        crate::types::ArrowSize::Small => (width * 1.5).max(8.0),
                        crate::types::ArrowSize::Medium => (width * 2.5).max(14.0),
                        crate::types::ArrowSize::Large => (width * 4.0).max(24.0),
                    };
                    let tip = end + dir * head_len; 
                    let p1 = end + perp * head_len * 0.45;
                    let p2 = end - perp * head_len * 0.45;

                    match s.brush_mode {
                        BrushMode::Highlighter => {
                            let mut hi_col = stroke_color;
                            hi_col = egui::Color32::from_rgba_unmultiplied(hi_col.r(), hi_col.g(), hi_col.b(), (hi_col.a() as f32 * s.highlight_opacity) as u8);
                            p.add(egui::Shape::convex_polygon(vec![tip, p1, p2], hi_col, egui::Stroke::NONE));
                        }
                        BrushMode::Spray => {
                            // Spray dots in arrow shape
                            let mut rng = 12345u32;
                            for _ in 0..(s.spray_density * 3) {
                                rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
                                let t = (rng % 1000) as f32 / 1000.0;
                                rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
                                let u = (rng % 1000) as f32 / 1000.0;
                                let su = if t + u > 1.0 { 1.0 - t } else { t };
                                let sv = if t + u > 1.0 { 1.0 - u } else { u };
                                let pt = egui::pos2(
                                    tip.x * (1.0 - su - sv) + p1.x * su + p2.x * sv,
                                    tip.y * (1.0 - su - sv) + p1.y * su + p2.y * sv,
                                );
                                p.circle_filled(pt, 1.0, stroke_color);
                            }
                        }
                        BrushMode::Calligraphy => {
                            // Angled calligraphy arrow
                            let nib_angle = std::f32::consts::PI / 4.0;
                            let nib_perp = egui::vec2(nib_angle.cos(), nib_angle.sin()) * head_len * 0.3;
                            let tip_l = tip + nib_perp;
                            let tip_r = tip - nib_perp;
                            p.add(egui::Shape::convex_polygon(vec![tip_l, tip_r, p2, p1], stroke_color, egui::Stroke::NONE));
                        }
                        _ => {
                            // Solid / Real: standard filled triangle
                            p.add(egui::Shape::convex_polygon(vec![tip, p1, p2], stroke_color, egui::Stroke::NONE));
                        }
                    }
                }
            }
            StrokeKind::Poly => {
                let mut polys = Vec::new();
                let mut current_poly = Vec::new();
                for pt in pts {
                    if pt.x.is_nan() || pt.y.is_nan() {
                        if !current_poly.is_empty() {
                            polys.push(current_poly);
                            current_poly = Vec::new();
                        }
                    } else {
                        current_poly.push(pt);
                    }
                }
                if !current_poly.is_empty() {
                    polys.push(current_poly);
                }

                for poly in polys {
                    if poly.len() > 1 {
                        if let Some(bg) = s.background_color {
                            let mut bg_col = color32(&bg);
                            bg_col = egui::Color32::from_rgba_unmultiplied(bg_col.r(), bg_col.g(), bg_col.b(), (bg_col.a() as f32 * l_op * s.opacity) as u8);
                            if poly.len() >= 3 {
                                p.add(egui::Shape::convex_polygon(poly.clone(), bg_col, egui::Stroke::NONE));
                            }
                        }
                        p.add(egui::Shape::line(poly, egui::Stroke::new(width, stroke_color)));
                    }
                }
            }
            StrokeKind::Line => {
                if s.points.len() == 2 {
                    p.line_segment([s.points[0] + offset, s.points[1] + offset], egui::Stroke::new(width, stroke_color));
                }
            }
            StrokeKind::Arrow => {
                if s.points.len() == 2 {
                    draw_arrow(p, s.points[0] + offset, s.points[1] + offset, width, stroke_color);
                }
            }
            StrokeKind::Rect => {
                if s.points.len() == 2 {
                    let rect = egui::Rect::from_two_pos(s.points[0] + offset, s.points[1] + offset);
                    if let Some(bg) = s.background_color {
                        let mut bg_col = color32(&bg);
                        bg_col = egui::Color32::from_rgba_unmultiplied(bg_col.r(), bg_col.g(), bg_col.b(), (bg_col.a() as f32 * l_op * s.opacity) as u8);
                        p.rect_filled(rect, 0.0, bg_col);
                    }
                    p.rect_stroke(rect, 0.0, egui::Stroke::new(width, stroke_color), egui::StrokeKind::Middle);
                }
            }
            StrokeKind::Circle => {
                if s.points.len() == 2 {
                    let center = s.points[0] + offset;
                    let radius = s.points[0].distance(s.points[1]);
                    if let Some(bg) = s.background_color {
                        let mut bg_col = color32(&bg);
                        bg_col = egui::Color32::from_rgba_unmultiplied(bg_col.r(), bg_col.g(), bg_col.b(), (bg_col.a() as f32 * l_op * s.opacity) as u8);
                        p.circle_filled(center, radius, bg_col);
                    }
                    p.circle_stroke(center, radius, egui::Stroke::new(width, stroke_color));
                }
            }
            StrokeKind::Star => {
                if s.points.len() == 2 {
                    let center = s.points[0] + offset;
                    let radius = s.points[0].distance(s.points[1]);
                    draw_star_shape(p, center, radius, stroke_color, s.background_color.as_ref().map(|bg| {
                        let mut bg_c = color32(bg);
                        bg_c = egui::Color32::from_rgba_unmultiplied(bg_c.r(), bg_c.g(), bg_c.b(), (bg_c.a() as f32 * l_op * s.opacity) as u8);
                        bg_c
                    }), width);
                }
            }
            StrokeKind::Heart => {
                if s.points.len() == 2 {
                    let center = s.points[0] + offset;
                    let radius = s.points[0].distance(s.points[1]);
                    draw_heart_shape(p, center, radius, stroke_color, s.background_color.as_ref().map(|bg| {
                        let mut bg_c = color32(bg);
                        bg_c = egui::Color32::from_rgba_unmultiplied(bg_c.r(), bg_c.g(), bg_c.b(), (bg_c.a() as f32 * l_op * s.opacity) as u8);
                        bg_c
                    }), width);
                }
            }
            _ => {}
        }
    }

pub fn draw_layer_strokes(p: &egui::Painter, layer: &crate::project::Layer, render_offset: egui::Vec2, l_op: f32) {
    for s in layer.strokes.iter() {
        if !s.visible { continue; }
        let mut stroke_c = color32(&s.color);
        stroke_c = egui::Color32::from_rgba_unmultiplied(stroke_c.r(), stroke_c.g(), stroke_c.b(), (stroke_c.a() as f32 * l_op * s.opacity) as u8);
        stroke_c = crate::utils::apply_color_effects(stroke_c, s.grayscale, s.invert, s.sepia, false, 0.0);

        if layer.shadow || s.shadow {
            let (s_col_arr, s_off, s_spread) = if s.shadow { (s.shadow_color, s.shadow_offset, s.shadow_spread) } else { (layer.shadow_color, layer.shadow_offset, layer.shadow_spread) };
            let mut s_col = egui::Color32::from_rgba_unmultiplied(s_col_arr[0], s_col_arr[1], s_col_arr[2], (s_col_arr[3] as f32 * l_op * s.opacity) as u8);
            s_col = crate::utils::apply_color_effects(s_col, s.grayscale, s.invert, s.sepia, false, 0.0);
            let offset = egui::vec2(s_off[0], s_off[1]);
            draw_stroke(p, s, s_col, render_offset + offset, s.width + s_spread * 2.0, l_op);
        }
        if layer.outline || s.outline {
            let (o_col_arr, o_width) = if s.outline { (s.outline_color, s.outline_width) } else { (layer.outline_color, layer.outline_width) };
            let mut o_col = egui::Color32::from_rgba_unmultiplied(o_col_arr[0], o_col_arr[1], o_col_arr[2], (o_col_arr[3] as f32 * l_op * s.opacity) as u8);
            o_col = crate::utils::apply_color_effects(o_col, s.grayscale, s.invert, s.sepia, false, 0.0);
            draw_stroke(p, s, o_col, render_offset, s.width + o_width * 2.0, l_op);
        }
        
        draw_stroke(p, s, stroke_c, render_offset, s.width, l_op);

        if layer.glow || s.glow {
            let (g_col_arr, g_str, g_spread) = if s.glow { (s.glow_color, s.glow_strength, s.glow_spread) } else { (layer.glow_color, layer.glow_strength, layer.glow_spread) };
            let glow_alpha = (g_col_arr[3] as f32 * l_op * s.opacity * (g_str / 100.0).clamp(0.0, 1.0)) as u8;
            let mut g_col = egui::Color32::from_rgba_unmultiplied(g_col_arr[0], g_col_arr[1], g_col_arr[2], glow_alpha);
            g_col = crate::utils::apply_color_effects(g_col, s.grayscale, s.invert, s.sepia, false, 0.0);
            draw_stroke(p, s, g_col, render_offset, s.width + g_spread * 2.0, l_op);
        }
    }
}

pub fn render_preview(ctx: &mut ToolContext) {
    if ctx.current_stroke.len() < 2 { return; }
    
    let render_offset = ctx.render_offset;
    let settings = &ctx.settings;
    let painter = ctx.ui.painter_at(ctx.canvas_response.rect);
    
    let pen_c = color32(&settings.pen_color);
    
    let pts: Vec<_> = ctx.current_stroke.clone();
    let s = Stroke::new(pts, settings.pen_color, settings.pen_width, StrokeKind::Freehand, settings.brush_mode, Some(settings.background_color), settings.brush_shadow, settings.brush_shape, settings.brush_outline, settings.brush_arrow, settings.spray_density, settings.highlight_opacity, settings.arrow_size);
    draw_stroke(&painter, &s, pen_c, egui::Vec2::ZERO, s.width, 1.0);
}

pub fn create_new_canvas(id: usize, pos: egui::Pos2, logical_w: f32, logical_h: f32, ppp: f32) -> crate::types::PlacedImage {
    let canvas_w = (logical_w * ppp).round() as usize;
    let canvas_h = (logical_h * ppp).round() as usize;
    let pixels = vec![0u8; canvas_w * canvas_h * 4];
    let mut new_img = crate::types::PlacedImage::new(id, pos, [canvas_w, canvas_h], pixels);
    new_img.display_size = Some([logical_w, logical_h]);
    new_img
}

fn draw_pixel_shape(img: &mut crate::types::PlacedImage, lx: f32, ly: f32, r: f32, color: [u8; 4], iw: usize, ih: usize, is_square: bool, hardness_pct: f32) {
    let r_ceil = r.ceil() as i32;
    let lx_floor = lx.floor() as i32;
    let ly_floor = ly.floor() as i32;
    let hardness = (hardness_pct / 100.0).clamp(0.0, 1.0);
    for dy in -r_ceil..=r_ceil {
        for dx in -r_ceil..=r_ceil {
            let px = (lx_floor + dx) as usize;
            let py = (ly_floor + dy) as usize;
            if px < iw && py < ih {
                let px_center_x = px as f32 + 0.5;
                let px_center_y = py as f32 + 0.5;
                let rx_dist = (px_center_x - lx).abs();
                let ry_dist = (px_center_y - ly).abs();
                let dist = if is_square {
                    rx_dist.max(ry_dist)
                } else {
                    (rx_dist.powi(2) + ry_dist.powi(2)).sqrt()
                };

                let coverage = if r <= 0.5 {
                    (1.0 - dist).clamp(0.0, 1.0) * (r * 2.0)
                } else {
                    let outer_r = r;
                    let inner_r = (r * hardness).min(r - 0.5);
                    if dist <= inner_r {
                        1.0
                    } else if dist >= outer_r {
                        0.0
                    } else {
                        let fade = (outer_r - dist) / (outer_r - inner_r).max(0.001);
                        fade.clamp(0.0, 1.0)
                    }
                };

                if coverage > 0.0 {
                    let mut pixel_color = color;
                    pixel_color[3] = (pixel_color[3] as f32 * coverage) as u8;
                    let idx = (py * iw + px) * 4;
                    if idx + 3 < img.pixels.len() {
                        let src_a = pixel_color[3] as f32 / 255.0;
                        let dst_a = img.pixels[idx + 3] as f32 / 255.0;
                        if img.locked {
                            if dst_a > 0.001 {
                                let blend_a = src_a;
                                let inv_blend = 1.0 - blend_a;
                                img.pixels[idx]     = (pixel_color[0] as f32 * blend_a + img.pixels[idx] as f32 * inv_blend) as u8;
                                img.pixels[idx + 1] = (pixel_color[1] as f32 * blend_a + img.pixels[idx + 1] as f32 * inv_blend) as u8;
                                img.pixels[idx + 2] = (pixel_color[2] as f32 * blend_a + img.pixels[idx + 2] as f32 * inv_blend) as u8;
                            }
                        } else {
                            let out_a = src_a + dst_a * (1.0 - src_a);
                            if out_a > 0.001 {
                                img.pixels[idx]     = ((pixel_color[0] as f32 * src_a + img.pixels[idx] as f32 * dst_a * (1.0 - src_a)) / out_a) as u8;
                                img.pixels[idx + 1] = ((pixel_color[1] as f32 * src_a + img.pixels[idx + 1] as f32 * dst_a * (1.0 - src_a)) / out_a) as u8;
                                img.pixels[idx + 2] = ((pixel_color[2] as f32 * src_a + img.pixels[idx + 2] as f32 * dst_a * (1.0 - src_a)) / out_a) as u8;
                                img.pixels[idx + 3] = (out_a * 255.0) as u8;
                            }
                        }
                    }
                }
            }
        }
    }
}

fn accumulate_pixel_coverage(coverages: &mut [u8], lx: f32, ly: f32, r: f32, iw: usize, ih: usize, is_square: bool) {
    let r_ceil = r.ceil() as i32;
    let lx_floor = lx.floor() as i32;
    let ly_floor = ly.floor() as i32;
    for dy in -r_ceil..=r_ceil {
        for dx in -r_ceil..=r_ceil {
            let px = (lx_floor + dx) as usize;
            let py = (ly_floor + dy) as usize;
            if px < iw && py < ih {
                let px_center_x = px as f32 + 0.5;
                let px_center_y = py as f32 + 0.5;
                let rx_dist = (px_center_x - lx).abs();
                let ry_dist = (px_center_y - ly).abs();
                let dist = if is_square {
                    rx_dist.max(ry_dist)
                } else {
                    (rx_dist.powi(2) + ry_dist.powi(2)).sqrt()
                };

                let coverage = if r <= 0.5 {
                    (1.0 - dist).clamp(0.0, 1.0) * (r * 2.0)
                } else {
                    let edge_width = 1.0f32;
                    let inner_r = r - edge_width * 0.5;
                    let outer_r = r + edge_width * 0.5;
                    if dist <= inner_r {
                        1.0
                    } else if dist >= outer_r {
                        0.0
                    } else {
                        ((outer_r - dist) / edge_width).clamp(0.0, 1.0)
                    }
                };

                if coverage > 0.0 {
                    let pixel_idx = py * iw + px;
                    let cov_u8 = (coverage * 255.0) as u8;
                    if cov_u8 > coverages[pixel_idx] {
                        coverages[pixel_idx] = cov_u8;
                    }
                }
            }
        }
    }
}


pub fn rasterize_stroke_to_image(img: &mut crate::types::PlacedImage, s: &Stroke, _settings: &Settings) {
    let mut iw = img.size[0];
    let mut ih = img.size[1];
    let mut dw = img.display_size.unwrap_or([iw as f32, ih as f32])[0];
    let mut dh = img.display_size.unwrap_or([ih as f32, ih as f32])[1];
    if iw == 0 || ih == 0 || dw <= 0.0 || dh <= 0.0 { return; }
    let mut scale_x = iw as f32 / dw;
    let mut scale_y = ih as f32 / dh;
    let mut radius = (s.width / 2.0 * scale_x).max(1.0);

    // --- Pre-enlargement Phase (for unrotated canvas) ---
    if img.rotation.abs() < 0.01 && img.skew.length() < 0.01 && img.perspective == [egui::Vec2::ZERO; 4] {
        let mut min_tx = 0.0f32;
        let mut min_ty = 0.0f32;
        let mut max_tx = iw as f32;
        let mut max_ty = ih as f32;

        for &pt in &s.points {
            let center = img.position + egui::vec2(dw * 0.5, dh * 0.5);
            let rel_world = pt - center;
            let base_p = center + rel_world;
            let lx = (base_p.x - img.position.x) * scale_x;
            let ly = (base_p.y - img.position.y) * scale_y;
            if lx - radius < min_tx { min_tx = lx - radius; }
            if ly - radius < min_ty { min_ty = ly - radius; }
            if lx + radius > max_tx { max_tx = lx + radius; }
            if ly + radius > max_ty { max_ty = ly + radius; }
        }

        let exp_l = (-min_tx).ceil() as usize;
        let exp_t = (-min_ty).ceil() as usize;
        let exp_r = (max_tx - iw as f32).ceil() as usize;
        let exp_b = (max_ty - ih as f32).ceil() as usize;

        if exp_l > 0 || exp_t > 0 || exp_r > 0 || exp_b > 0 {
            let new_iw = iw + exp_l + exp_r;
            let new_ih = ih + exp_t + exp_b;
            let mut new_pixels = vec![0u8; new_iw * new_ih * 4];

            for y in 0..ih {
                let src_s = y * iw * 4;
                let dst_s = ((y + exp_t) * new_iw + exp_l) * 4;
                if src_s + iw * 4 <= img.pixels.len() && dst_s + iw * 4 <= new_pixels.len() {
                    new_pixels[dst_s..dst_s + iw * 4].copy_from_slice(&img.pixels[src_s..src_s + iw * 4]);
                }
            }

            img.pixels = new_pixels;
            img.size = [new_iw, new_ih];
            iw = new_iw; ih = new_ih;

            let disp_l = exp_l as f32 / scale_x;
            let disp_t = exp_t as f32 / scale_y;
            let disp_r = exp_r as f32 / scale_x;
            let disp_b = exp_b as f32 / scale_y;

            img.position.x -= disp_l;
            img.position.y -= disp_t;
            dw += disp_l + disp_r;
            dh += disp_t + disp_b;
            img.display_size = Some([dw, dh]);
            
            // Re-read scale and radius for updated canvas size
            scale_x = iw as f32 / dw;
            scale_y = ih as f32 / dh;
            radius = (s.width / 2.0 * scale_x).max(1.0);
        }
    }

    let mut color = s.color;
    if s.brush_mode == BrushMode::Highlighter {
        color[3] = (color[3] as f32 * s.highlight_opacity) as u8;
    }

    let mut canvas_pts = Vec::with_capacity(s.points.len());
    for &pt in &s.points {
        let center = img.position + egui::vec2(dw * 0.5, dh * 0.5);
        let rel_world = pt - center;
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
        canvas_pts.push(egui::pos2(lx, ly));
    }

    let mut dirs = Vec::with_capacity(canvas_pts.len());
    for i in 0..canvas_pts.len() {
        let dir = if i < canvas_pts.len() - 1 {
            let d = canvas_pts[i+1] - canvas_pts[i];
            if d.length() > 0.001 { d.normalized() } else { egui::vec2(1.0, 0.0) }
        } else if i > 0 {
            let d = canvas_pts[i] - canvas_pts[i-1];
            if d.length() > 0.001 { d.normalized() } else { egui::vec2(1.0, 0.0) }
        } else {
            egui::vec2(1.0, 0.0)
        };
        dirs.push(dir);
    }

    match s.brush_mode {
        BrushMode::Spray => {
            let mut rng = 42u32;
            let half_w = radius;
            let dot_radius = scale_x.max(0.8);
            for &p in &canvas_pts {
                for _ in 0..s.spray_density {
                    rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
                    let rx = ((rng % 1000) as f32 / 500.0 - 1.0) * half_w;
                    rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
                    let ry = ((rng % 1000) as f32 / 500.0 - 1.0) * half_w;

                    let inside = match s.brush_shape {
                        BrushShape::Round => rx*rx + ry*ry <= half_w*half_w,
                        BrushShape::Square => true,
                    };

                    if inside {
                        draw_pixel_shape(img, p.x + rx, p.y + ry, dot_radius, color, iw, ih, false, 100.0);
                    }
                }
            }
        }
        BrushMode::Real => {
            let mut rng = 42u32;
            let num_bristles = (s.width * 0.4).clamp(4.0, 10.0) as usize;
            let mut bristle_offsets = Vec::new();
            
            for _ in 0..num_bristles {
                rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
                let rx = (rng % 1000) as f32 / 500.0 - 1.0 ;
                rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
                let ry = (rng % 1000) as f32 / 500.0 - 1.0 ;
                
                let (off_x, off_y) = if s.brush_shape == BrushShape::Round {
                    let len = (rx*rx + ry*ry).sqrt().max(0.001);
                    let scale = if len > 1.0 { 1.0 / len } else { 1.0 };
                    (rx * scale * radius * 0.9, ry * scale * radius * 0.9)
                } else {
                    (rx * radius * 0.9, ry * radius * 0.9)
                };
                
                rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
                let alpha_mod = (rng % 100) as f32 / 100.0 * 0.5 + 0.1;
                let mut b_col = color;
                b_col[3] = (b_col[3] as f32 * alpha_mod) as u8;
                
                let b_radius = (radius * 0.5).max(1.0);
                bristle_offsets.push((off_x, off_y, b_col, b_radius));
            }

            if !canvas_pts.is_empty() {
                let p = canvas_pts[0];
                for &(ox, oy, b_col, b_radius) in &bristle_offsets {
                    draw_pixel_shape(img, p.x + ox, p.y + oy, b_radius, b_col, iw, ih, s.brush_shape == BrushShape::Square, 100.0);
                }
            }

            for i in 1..canvas_pts.len() {
                let p1 = canvas_pts[i-1];
                let p2 = canvas_pts[i];
                let dist = p1.distance(p2);
                if dist > 0.001 {
                    let min_b_radius = bristle_offsets.iter().map(|&(_, _, _, r)| r).fold(f32::MAX, f32::min);
                    let step_size = (min_b_radius * 0.2).clamp(0.2, 1.0);
                    let steps = (dist / step_size).ceil() as usize;
                    for step in 1..=steps {
                        let t = step as f32 / steps as f32;
                        let pi = p1.lerp(p2, t);
                        for &(ox, oy, b_col, b_radius) in &bristle_offsets {
                            draw_pixel_shape(img, pi.x + ox, pi.y + oy, b_radius, b_col, iw, ih, s.brush_shape == BrushShape::Square, 100.0);
                        }
                    }
                }
            }
        }
        BrushMode::Calligraphy | BrushMode::Solid | BrushMode::Highlighter => {
            let mut coverages = vec![0u8; iw * ih];
            
            if s.brush_mode == BrushMode::Calligraphy {
                let nib_angle = std::f32::consts::PI / 4.0;
                let nib_dir = egui::vec2(nib_angle.cos(), nib_angle.sin());

                if !canvas_pts.is_empty() {
                    let p = canvas_pts[0];
                    let dir = dirs[0];
                    if s.brush_shape == BrushShape::Round {
                        let cross = (dir.x * nib_dir.y - dir.y * nib_dir.x).abs();
                        let thickness = radius * (cross * 0.85 + 0.15);
                        accumulate_pixel_coverage(&mut coverages, p.x, p.y, thickness, iw, ih, false);
                    } else {
                        let steps = (radius * 2.0) as usize + 2;
                        for step in 0..=steps {
                            let t = step as f32 / steps as f32 * 2.0 - 1.0;
                            let px = p.x + nib_dir.x * radius * t;
                            let py = p.y + nib_dir.y * radius * t;
                            accumulate_pixel_coverage(&mut coverages, px, py, scale_x.max(1.0), iw, ih, false);
                        }
                    }
                }

                for i in 1..canvas_pts.len() {
                    let p1 = canvas_pts[i-1];
                    let p2 = canvas_pts[i];
                    let dir1 = dirs[i-1];
                    let dir2 = dirs[i];
                    let dist = p1.distance(p2);
                    if dist > 0.001 {
                        let step_size = (radius * 0.1).clamp(0.2, 1.0);
                        let steps = (dist / step_size).ceil() as usize;
                        for step in 1..=steps {
                            let t = step as f32 / steps as f32;
                            let pi = p1.lerp(p2, t);
                            let di = dir1 * (1.0 - t) + dir2 * t;
                            let di = if di.length() > 0.001 { di.normalized() } else { dir1 };

                            if s.brush_shape == BrushShape::Round {
                                let cross = (di.x * nib_dir.y - di.y * nib_dir.x).abs();
                                let thickness = radius * (cross * 0.85 + 0.15);
                                accumulate_pixel_coverage(&mut coverages, pi.x, pi.y, thickness, iw, ih, false);
                            } else {
                                let steps_nib = (radius * 2.0) as usize + 2;
                                for step_nib in 0..=steps_nib {
                                    let tn = step_nib as f32 / steps_nib as f32 * 2.0 - 1.0;
                                    let px = pi.x + nib_dir.x * radius * tn;
                                    let py = pi.y + nib_dir.y * radius * tn;
                                    accumulate_pixel_coverage(&mut coverages, px, py, scale_x.max(1.0), iw, ih, false);
                                }
                            }
                        }
                    }
                }
            } else {
                if !canvas_pts.is_empty() {
                    accumulate_pixel_coverage(&mut coverages, canvas_pts[0].x, canvas_pts[0].y, radius, iw, ih, s.brush_shape == BrushShape::Square);
                }
                for i in 1..canvas_pts.len() {
                    let p1 = canvas_pts[i-1];
                    let p2 = canvas_pts[i];
                    let dist = p1.distance(p2);
                    if dist > 0.001 {
                        let step_size = (radius * 0.1).clamp(0.2, 1.0);
                        let steps = (dist / step_size).ceil() as usize;
                        for step in 1..=steps {
                            let t = step as f32 / steps as f32;
                            let pi = p1.lerp(p2, t);
                            accumulate_pixel_coverage(&mut coverages, pi.x, pi.y, radius, iw, ih, s.brush_shape == BrushShape::Square);
                        }
                    }
                }
            }

            let original_pixels = img.pixels.clone();
            for y in 0..ih {
                for x in 0..iw {
                    let pixel_idx = y * iw + x;
                    let cov_u8 = coverages[pixel_idx];
                    if cov_u8 > 0 {
                        let coverage = cov_u8 as f32 / 255.0;
                        let mut pixel_color = color;
                        pixel_color[3] = (pixel_color[3] as f32 * coverage) as u8;

                        let idx = pixel_idx * 4;
                        if idx + 3 < img.pixels.len() {
                            let src_a = pixel_color[3] as f32 / 255.0;
                            let dst_a = original_pixels[idx + 3] as f32 / 255.0;
                            if img.locked {
                                if dst_a > 0.001 {
                                    let blend_a = src_a;
                                    let inv_blend = 1.0 - blend_a;
                                    img.pixels[idx]     = (pixel_color[0] as f32 * blend_a + original_pixels[idx] as f32 * inv_blend) as u8;
                                    img.pixels[idx + 1] = (pixel_color[1] as f32 * blend_a + original_pixels[idx + 1] as f32 * inv_blend) as u8;
                                    img.pixels[idx + 2] = (pixel_color[2] as f32 * blend_a + original_pixels[idx + 2] as f32 * inv_blend) as u8;
                                }
                            } else {
                                let out_a = src_a + dst_a * (1.0 - src_a);
                                if out_a > 0.001 {
                                    img.pixels[idx]     = ((pixel_color[0] as f32 * src_a + original_pixels[idx] as f32 * dst_a * (1.0 - src_a)) / out_a) as u8;
                                    img.pixels[idx + 1] = ((pixel_color[1] as f32 * src_a + original_pixels[idx + 1] as f32 * dst_a * (1.0 - src_a)) / out_a) as u8;
                                    img.pixels[idx + 2] = ((pixel_color[2] as f32 * src_a + original_pixels[idx + 2] as f32 * dst_a * (1.0 - src_a)) / out_a) as u8;
                                    img.pixels[idx + 3] = (out_a * 255.0) as u8;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    img.clear_texture();
    img.thumbnail_dirty = true;
    crop_to_content(img);
}

pub fn get_content_pixel_bbox(img: &crate::types::PlacedImage) -> Option<egui::Rect> {
    let iw = img.size[0];
    let ih = img.size[1];
    if iw == 0 || ih == 0 { return None; }
    let mut min_x = iw;
    let mut min_y = ih;
    let mut max_x = 0;
    let mut max_y = 0;
    let mut found = false;
    for y in 0..ih {
        for x in 0..iw {
            let idx = (y * iw + x) * 4;
            let is_opaque = idx + 3 < img.pixels.len() && img.pixels[idx + 3] > 0;
            let mask_opaque = if let Some(ref mask) = img.mask {
                let m_size = img.mask_size.unwrap_or(img.size);
                if m_size[0] > 0 && m_size[1] > 0 {
                    let mx = ((x * m_size[0]) / iw).min(m_size[0] - 1);
                    let my = ((y * m_size[1]) / ih).min(m_size[1] - 1);
                    let m_idx = my * m_size[0] + mx;
                    m_idx < mask.len() && mask[m_idx] > 0
                } else {
                    true
                }
            } else {
                true
            };
            if is_opaque && mask_opaque {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
                found = true;
            }
        }
    }
    if found {
        Some(egui::Rect::from_min_max(
            egui::pos2(min_x as f32, min_y as f32),
            egui::pos2((max_x + 1) as f32, (max_y + 1) as f32),
        ))
    } else {
        None
    }
}

pub fn crop_to_content(img: &mut crate::types::PlacedImage) {
    let iw = img.size[0];
    let ih = img.size[1];
    if iw == 0 || ih == 0 { return; }
    
    if let Some(bbox) = get_content_pixel_bbox(img) {
        let min_x = bbox.min.x.floor() as i32;
        let min_y = bbox.min.y.floor() as i32;
        let max_x = bbox.max.x.ceil() as i32;
        let max_y = bbox.max.y.ceil() as i32;
        
        let new_w = (max_x - min_x).max(1) as usize;
        let new_h = (max_y - min_y).max(1) as usize;
        
        if new_w < iw || new_h < ih {
            let mut new_pixels = vec![0u8; new_w * new_h * 4];
            for y in 0..new_h {
                let old_y = y as i32 + min_y;
                if old_y < 0 || old_y >= ih as i32 { continue; }
                for x in 0..new_w {
                    let old_x = x as i32 + min_x;
                    if old_x < 0 || old_x >= iw as i32 { continue; }
                    let old_idx = (old_y as usize * iw + old_x as usize) * 4;
                    let new_idx = (y * new_w + x) * 4;
                    if old_idx + 3 < img.pixels.len() && new_idx + 3 < new_pixels.len() {
                        new_pixels[new_idx..new_idx+4].copy_from_slice(&img.pixels[old_idx..old_idx+4]);
                    }
                }
            }
            
            if let Some(ref mask) = img.mask {
                let m_size = img.mask_size.unwrap_or([iw, ih]);
                let mask_min_x = (min_x * m_size[0] as i32) / iw as i32;
                let mask_min_y = (min_y * m_size[1] as i32) / ih as i32;
                let mask_max_x = (max_x * m_size[0] as i32) / iw as i32;
                let mask_max_y = (max_y * m_size[1] as i32) / ih as i32;
                let new_m_w = (mask_max_x - mask_min_x).max(1) as usize;
                let new_m_h = (mask_max_y - mask_min_y).max(1) as usize;

                let mut new_mask = vec![255u8; new_m_w * new_m_h];
                for y in 0..new_m_h {
                    let old_y = y as i32 + mask_min_y;
                    if old_y < 0 || old_y >= m_size[1] as i32 { continue; }
                    for x in 0..new_m_w {
                        let old_x = x as i32 + mask_min_x;
                        if old_x < 0 || old_x >= m_size[0] as i32 { continue; }
                        let old_idx = old_y as usize * m_size[0] + old_x as usize;
                        let new_idx = y * new_m_w + x;
                        if old_idx < mask.len() && new_idx < new_mask.len() {
                            new_mask[new_idx] = mask[old_idx];
                        }
                    }
                }
                img.mask = Some(new_mask);
                img.mask_size = Some([new_m_w, new_m_h]);
                img.mask_dirty = true;
                img.cached_mask_outline = None;
            }
            
            let dw = img.display_size.unwrap_or([iw as f32, ih as f32])[0];
            let dh = img.display_size.unwrap_or([ih as f32, ih as f32])[1];
            let scale_x = if dw > 0.1 { iw as f32 / dw } else { 1.0 };
            let scale_y = if dh > 0.1 { ih as f32 / dh } else { 1.0 };
            
            let p_local_pts = egui::pos2(
                img.position.x + min_x as f32 / scale_x,
                img.position.y + min_y as f32 / scale_y,
            );
            let old_center = img.position + egui::vec2(dw * 0.5, dh * 0.5);
            let old_rect = egui::Rect::from_min_size(img.position, egui::vec2(dw, dh));
            
            let new_screen_pos = crate::utils::transform_point_complex(
                p_local_pts,
                old_center,
                img.rotation,
                img.skew,
                img.perspective,
                old_rect,
                img.scale,
            );
            
            img.position = new_screen_pos;
            img.size = [new_w, new_h];
            img.display_size = Some([new_w as f32 / scale_x, new_h as f32 / scale_y]);
            img.pixels = new_pixels;

            if let Some(ref mut pts) = img.snip_points {
                let shift_x = min_x as f32 / scale_x;
                let shift_y = min_y as f32 / scale_y;
                for p in pts {
                    if !p.x.is_nan() && !p.y.is_nan() {
                        p.x -= shift_x;
                        p.y -= shift_y;
                    }
                }
            }
            if let Some(mut src) = img.source_rect {
                src[0] += min_x as f32 / scale_x;
                src[1] += min_y as f32 / scale_y;
                src[2] = new_w as f32 / scale_x;
                src[3] = new_h as f32 / scale_y;
                img.source_rect = Some(src);
            }
            img.clear_texture();
            img.thumbnail_dirty = true;
        }
    }
}

pub fn merge_images(dest: &mut crate::types::PlacedImage, src: &crate::types::PlacedImage) {
    let dest_iw = dest.size[0];
    let dest_ih = dest.size[1];
    let src_iw = src.size[0];
    let src_ih = src.size[1];
    if dest_iw == 0 || dest_ih == 0 || src_iw == 0 || src_ih == 0 { return; }

    let dest_dw = dest.display_size.unwrap_or([dest_iw as f32, dest_ih as f32])[0];
    let dest_dh = dest.display_size.unwrap_or([dest_ih as f32, dest_ih as f32])[1];
    let dest_scale_x = if dest_dw > 0.1 { dest_iw as f32 / dest_dw } else { 1.0 };
    let dest_scale_y = if dest_dh > 0.1 { dest_ih as f32 / dest_dh } else { 1.0 };

    // Relative offset of src from dest in screen points
    let offset_pts = src.position - dest.position;
    let offset_px_x = (offset_pts.x * dest_scale_x).round() as i32;
    let offset_px_y = (offset_pts.y * dest_scale_y).round() as i32;

    // src bounds in dest's pixel coordinate space
    let src_local_min_x = offset_px_x;
    let src_local_min_y = offset_px_y;
    let src_local_max_x = offset_px_x + src_iw as i32;
    let src_local_max_y = offset_px_y + src_ih as i32;

    // Get current dest content bbox
    let dest_content_bbox = get_content_pixel_bbox(dest);
    let src_local_bbox = egui::Rect::from_min_max(
        egui::pos2(src_local_min_x as f32, src_local_min_y as f32),
        egui::pos2(src_local_max_x as f32, src_local_max_y as f32),
    );

    let final_local_bbox = match dest_content_bbox {
        Some(db) => db.union(src_local_bbox),
        None => src_local_bbox,
    };

    let new_min_x = final_local_bbox.min.x.floor() as i32;
    let new_min_y = final_local_bbox.min.y.floor() as i32;
    let new_max_x = final_local_bbox.max.x.ceil() as i32;
    let new_max_y = final_local_bbox.max.y.ceil() as i32;

    let new_w = (new_max_x - new_min_x).max(1) as usize;
    let new_h = (new_max_y - new_min_y).max(1) as usize;

    let mut new_pixels = vec![0u8; new_w * new_h * 4];

    // 1. Copy dest pixels
    for y in 0..dest_ih {
        let new_y = y as i32 - new_min_y;
        if new_y < 0 || new_y >= new_h as i32 { continue; }
        for x in 0..dest_iw {
            let new_x = x as i32 - new_min_x;
            if new_x < 0 || new_x >= new_w as i32 { continue; }
            let old_idx = (y * dest_iw + x) * 4;
            let new_idx = (new_y as usize * new_w + new_x as usize) * 4;
            if old_idx + 3 < dest.pixels.len() && new_idx + 3 < new_pixels.len() {
                new_pixels[new_idx..new_idx+4].copy_from_slice(&dest.pixels[old_idx..old_idx+4]);
            }
        }
    }

    // 2. Blend/Copy src pixels
    for y in 0..src_ih {
        let old_y = y as i32 + offset_px_y;
        let new_y = old_y - new_min_y;
        if new_y < 0 || new_y >= new_h as i32 { continue; }
        for x in 0..src_iw {
            let old_x = x as i32 + offset_px_x;
            let new_x = old_x - new_min_x;
            if new_x < 0 || new_x >= new_w as i32 { continue; }

            let src_idx = (y * src_iw + x) * 4;
            let new_idx = (new_y as usize * new_w + new_x as usize) * 4;
            if src_idx + 3 < src.pixels.len() && new_idx + 3 < new_pixels.len() {
                let src_a = src.pixels[src_idx + 3] as f32 / 255.0;
                if src_a > 0.001 {
                    let dst_a = new_pixels[new_idx + 3] as f32 / 255.0;
                    let out_a = src_a + dst_a * (1.0 - src_a);
                    if out_a > 0.001 {
                        new_pixels[new_idx]     = ((src.pixels[src_idx] as f32 * src_a + new_pixels[new_idx] as f32 * dst_a * (1.0 - src_a)) / out_a) as u8;
                        new_pixels[new_idx + 1] = ((src.pixels[src_idx + 1] as f32 * src_a + new_pixels[new_idx + 1] as f32 * dst_a * (1.0 - src_a)) / out_a) as u8;
                        new_pixels[new_idx + 2] = ((src.pixels[src_idx + 2] as f32 * src_a + new_pixels[new_idx + 2] as f32 * dst_a * (1.0 - src_a)) / out_a) as u8;
                        new_pixels[new_idx + 3] = (out_a * 255.0) as u8;
                    }
                }
            }
        }
    }

    if let Some(ref mask) = dest.mask {
        let m_size = dest.mask_size.unwrap_or([dest_iw, dest_ih]);
        let scale_mask_x = m_size[0] as f32 / dest_iw as f32;
        let scale_mask_y = m_size[1] as f32 / dest_ih as f32;
        
        let new_m_w = (new_w as f32 * scale_mask_x).round().max(1.0) as usize;
        let new_m_h = (new_h as f32 * scale_mask_y).round().max(1.0) as usize;
        
        let mut new_mask = vec![255u8; new_m_w * new_m_h];
        
        let mask_min_x_m = (new_min_x as f32 * scale_mask_x).round() as i32;
        let mask_min_y_m = (new_min_y as f32 * scale_mask_y).round() as i32;
        
        for y in 0..m_size[1] {
            let new_y = y as i32 - mask_min_y_m;
            if new_y < 0 || new_y >= new_m_h as i32 { continue; }
            for x in 0..m_size[0] {
                let new_x = x as i32 - mask_min_x_m;
                if new_x < 0 || new_x >= new_m_w as i32 { continue; }
                let old_idx = y * m_size[0] + x;
                let new_idx = new_y as usize * new_m_w + new_x as usize;
                if old_idx < mask.len() && new_idx < new_mask.len() {
                    new_mask[new_idx] = mask[old_idx];
                }
            }
        }
        dest.mask = Some(new_mask);
        dest.mask_size = Some([new_m_w, new_m_h]);
        dest.mask_dirty = true;
        dest.cached_mask_outline = None;
    }

    // 3. Update dest image positioning & dimensions
    let p_local_pts = egui::pos2(
        dest.position.x + new_min_x as f32 / dest_scale_x,
        dest.position.y + new_min_y as f32 / dest_scale_y,
    );
    let old_center = dest.position + egui::vec2(dest_dw * 0.5, dest_dh * 0.5);
    let old_rect = egui::Rect::from_min_size(dest.position, egui::vec2(dest_dw, dest_dh));
    
    let new_screen_pos = crate::utils::transform_point_complex(
        p_local_pts,
        old_center,
        dest.rotation,
        dest.skew,
        dest.perspective,
        old_rect,
        dest.scale,
    );

    dest.position = new_screen_pos;
    dest.size = [new_w, new_h];
    dest.display_size = Some([new_w as f32 / dest_scale_x, new_h as f32 / dest_scale_y]);
    dest.pixels = new_pixels;
    dest.clear_texture();
    dest.thumbnail_dirty = true;

    if let Some(ref mut pts) = dest.snip_points {
        let shift_x = new_min_x as f32 / dest_scale_x;
        let shift_y = new_min_y as f32 / dest_scale_y;
        for p in pts {
            if !p.x.is_nan() && !p.y.is_nan() {
                p.x -= shift_x;
                p.y -= shift_y;
            }
        }
    }
    if let Some(mut src) = dest.source_rect {
        src[0] += new_min_x as f32 / dest_scale_x;
        src[1] += new_min_y as f32 / dest_scale_y;
        src[2] = new_w as f32 / dest_scale_x;
        src[3] = new_h as f32 / dest_scale_y;
        dest.source_rect = Some(src);
    }

    crop_to_content(dest);
}

