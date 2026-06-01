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

    let layer = &project.layers[active_layer_idx];
    let is_layer_locked = layer.locked;
    let mut is_image_locked = false;
    if !settings.brush_arrow {
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
                let is_ask_mode = settings.auto_new_layer.is_none();
                if !is_locked && !is_ask_mode && !settings.brush_arrow {
                    // ── Find or create target PlacedImage ──
                    let has_target_image = project.selected_object.map_or(false, |s| {
                        s.object_type == ObjectType::Image
                            && s.layer_idx == active_layer_idx
                            && s.object_idx < project.layers[active_layer_idx].placed_images.len()
                    });

                    if !has_target_image && current_stroke.is_empty() {
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

                let is_ask_mode = settings.auto_new_layer.is_none();
                if !is_locked && !is_ask_mode && !settings.brush_arrow {
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

                                    match settings.brush_mode {
                                        BrushMode::Spray => {
                                            let mut rng = 42u32;
                                            let half_w = radius;
                                            let dot_radius = scale_x.max(0.8);
                                            for &pt in current_stroke.iter().skip(current_stroke.len().saturating_sub(new_points_count)) {
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
                                                        draw_pixel_shape(img, lx + rx, ly + ry, dot_radius, color, iw, ih, false);
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

                                            for &pt in current_stroke.iter().skip(current_stroke.len().saturating_sub(new_points_count)) {
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

                                                for &(off_x, off_y, b_col, b_radius) in &bristle_offsets {
                                                    draw_pixel_shape(img, lx + off_x, ly + off_y, b_radius, b_col, iw, ih, false);
                                                }
                                            }
                                        }
                                        BrushMode::Calligraphy => {
                                            let nib_angle = std::f32::consts::PI / 4.0;
                                            let nib_dir = egui::vec2(nib_angle.cos(), nib_angle.sin());

                                            for i in current_stroke.len().saturating_sub(new_points_count)..current_stroke.len() {
                                                let pt = current_stroke[i];
                                                let dir = if i > 0 {
                                                    let d = pt - current_stroke[i-1];
                                                    if d.length() > 0.001 { d.normalized() } else { egui::vec2(1.0, 0.0) }
                                                } else if current_stroke.len() > 1 {
                                                    let d = current_stroke[1] - current_stroke[0];
                                                    if d.length() > 0.001 { d.normalized() } else { egui::vec2(1.0, 0.0) }
                                                } else {
                                                    egui::vec2(1.0, 0.0)
                                                };

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

                                                if settings.brush_shape == BrushShape::Round {
                                                    let cross = (dir.x * nib_dir.y - dir.y * nib_dir.x).abs();
                                                    let thickness = radius * (cross * 0.85 + 0.15);
                                                    draw_pixel_shape(img, lx, ly, thickness, color, iw, ih, false);
                                                } else {
                                                    let steps = (radius * 2.0) as usize + 2;
                                                    for step in 0..=steps {
                                                        let t = step as f32 / steps as f32 * 2.0 - 1.0;
                                                        let px = lx + nib_dir.x * radius * t;
                                                        let py = ly + nib_dir.y * radius * t;
                                                        draw_pixel_shape(img, px, py, scale_x.max(1.0), color, iw, ih, false);
                                                    }
                                                }
                                            }
                                        }
                                        _ => {
                                            for &pt in current_stroke.iter().skip(current_stroke.len().saturating_sub(new_points_count)) {
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

                                                draw_pixel_shape(img, lx, ly, radius, color, iw, ih, settings.brush_shape == BrushShape::Square);
                                            }
                                        }
                                    }

                                    img.texture = None; // Force texture rebuild
                                    img.thumbnail_dirty = true;
                                }
                            }
                        }
                    }
                }
            }

            if left_just_released && !current_stroke.is_empty() {
                let has_existing_canvas = project.selected_object.map_or(false, |s| {
                    s.object_type == ObjectType::Image && s.layer_idx == active_layer_idx
                });
                let ask_mode = settings.auto_new_layer.is_none() || has_existing_canvas;
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
                    );
                    *ctx.pending_stroke = Some(s);
                    *ctx.layer_prompt_open = true;
                } else {
                    if settings.brush_arrow {
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
                    let head_len = (width * 4.5).max(14.0);
                    // The arrow tip should be ahead of the brush. Base is at 'end'.
                    let tip = end + dir * head_len; 
                    let p1 = end + perp * head_len * 0.45;
                    let p2 = end - perp * head_len * 0.45;
                    p.add(egui::Shape::convex_polygon(vec![tip, p1, p2], stroke_color, egui::Stroke::NONE));
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
    let s = Stroke::new(pts, settings.pen_color, settings.pen_width, StrokeKind::Freehand, settings.brush_mode, Some(settings.background_color), settings.brush_shadow, settings.brush_shape, settings.brush_outline, settings.brush_arrow, settings.spray_density, settings.highlight_opacity);
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

fn draw_pixel_shape(img: &mut crate::types::PlacedImage, lx: f32, ly: f32, r: f32, color: [u8; 4], iw: usize, ih: usize, is_square: bool) {
    let r_ceil = r.ceil() as i32;
    for dy in -r_ceil..=r_ceil {
        for dx in -r_ceil..=r_ceil {
            let px = (lx as i32 + dx) as usize;
            let py = (ly as i32 + dy) as usize;
            if px < iw && py < ih {
                let coverage = if is_square {
                    1.0f32
                } else {
                    let dist = ((dx as f32).powi(2) + (dy as f32).powi(2)).sqrt();
                    if r <= 0.5 {
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
    for dy in -r_ceil..=r_ceil {
        for dx in -r_ceil..=r_ceil {
            let px = (lx as i32 + dx) as usize;
            let py = (ly as i32 + dy) as usize;
            if px < iw && py < ih {
                let coverage = if is_square {
                    1.0f32
                } else {
                    let dist = ((dx as f32).powi(2) + (dy as f32).powi(2)).sqrt();
                    if r <= 0.5 {
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

    match s.brush_mode {
        BrushMode::Spray => {
            let mut rng = 42u32;
            let half_w = radius;
            let dot_radius = scale_x.max(0.8);
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
                        draw_pixel_shape(img, lx + rx, ly + ry, dot_radius, color, iw, ih, false);
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

                for &(ox, oy, b_col, b_radius) in &bristle_offsets {
                    draw_pixel_shape(img, lx + ox, ly + oy, b_radius, b_col, iw, ih, s.brush_shape == BrushShape::Square);
                }
            }
        }
        BrushMode::Calligraphy | BrushMode::Solid | BrushMode::Highlighter => {
            let mut coverages = vec![0u8; iw * ih];
            
            if s.brush_mode == BrushMode::Calligraphy {
                let nib_angle = std::f32::consts::PI / 4.0;
                let nib_dir = egui::vec2(nib_angle.cos(), nib_angle.sin());

                for i in 0..s.points.len() {
                    let pt = s.points[i];
                    let dir = if i > 0 {
                        let d = pt - s.points[i-1];
                        if d.length() > 0.001 { d.normalized() } else { egui::vec2(1.0, 0.0) }
                    } else if s.points.len() > 1 {
                        let d = s.points[1] - s.points[0];
                        if d.length() > 0.001 { d.normalized() } else { egui::vec2(1.0, 0.0) }
                    } else {
                        egui::vec2(1.0, 0.0)
                    };

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

                    if s.brush_shape == BrushShape::Round {
                        let cross = (dir.x * nib_dir.y - dir.y * nib_dir.x).abs();
                        let thickness = radius * (cross * 0.85 + 0.15);
                        accumulate_pixel_coverage(&mut coverages, lx, ly, thickness, iw, ih, false);
                    } else {
                        let steps = (radius * 2.0) as usize + 2;
                        for step in 0..=steps {
                            let t = step as f32 / steps as f32 * 2.0 - 1.0;
                            let px = lx + nib_dir.x * radius * t;
                            let py = ly + nib_dir.y * radius * t;
                            accumulate_pixel_coverage(&mut coverages, px, py, scale_x.max(1.0), iw, ih, false);
                        }
                    }
                }
            } else {
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

                    accumulate_pixel_coverage(&mut coverages, lx, ly, radius, iw, ih, s.brush_shape == BrushShape::Square);
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
    img.texture = None;
    img.thumbnail_dirty = true;
    crop_to_content(img);
}

pub fn get_content_pixel_bbox(img: &crate::types::PlacedImage) -> Option<egui::Rect> {
    let iw = img.size[0];
    let ih = img.size[1];
    let mut min_x = iw;
    let mut min_y = ih;
    let mut max_x = 0;
    let mut max_y = 0;
    let mut found = false;
    for y in 0..ih {
        for x in 0..iw {
            let idx = (y * iw + x) * 4;
            if idx + 3 < img.pixels.len() && img.pixels[idx + 3] > 0 {
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
            
            let dw = img.display_size.unwrap_or([iw as f32, ih as f32])[0];
            let dh = img.display_size.unwrap_or([ih as f32, ih as f32])[1];
            let scale_x = iw as f32 / dw;
            let scale_y = ih as f32 / dh;
            
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
    let dest_scale_x = dest_iw as f32 / dest_dw;
    let dest_scale_y = dest_ih as f32 / dest_dh;

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
    dest.texture = None;
    dest.thumbnail_dirty = true;

    crop_to_content(dest);
}

