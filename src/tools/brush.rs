use eframe::egui;
use crate::utils::*;
use crate::overlay::*;

use crate::tools::ToolContext;

pub fn update(ctx: &mut ToolContext) {
    if ctx.mouse.left_just_pressed {
        ctx.auto_create_layer();
    }

    let project = &mut *ctx.project;
    let settings = &mut *ctx.settings;
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
    let active_layer_idx = project.active_layer;
    let _ppp = ui.ctx().pixels_per_point();
    let _render_offset = ctx.render_offset;
    let _frame_count = ctx.frame_count;

    let layer = &mut project.layers[active_layer_idx];
            if left_down {
                // Reject glitchy points (0,0) or huge jumps
                if pos.x < 1.0 && pos.y < 1.0 { return; }

                if let Some(last) = current_stroke.last().cloned() {
                    let dist = last.distance(pos);
                    if dist > 2000.0 { 
                        current_stroke.clear();
                        current_stroke.push(pos);
                    } else if dist > 0.5 {
                        let steps = (dist / 0.5) as usize;
                        for s in 1..=steps {
                            let t = s as f32 / steps as f32;
                            current_stroke.push(last.lerp(pos, t));
                        }
                    }
                } else { current_stroke.push(pos); }
            }
            if left_just_released && !current_stroke.is_empty() {
                    let s = Stroke::new(current_stroke.clone(), settings.pen_color, settings.pen_width, StrokeKind::Freehand, settings.brush_mode, Some(settings.background_color), settings.brush_shadow, settings.brush_shape, settings.brush_outline, settings.brush_arrow);
                    layer.strokes.push(s);
                    current_stroke.clear();
                    *ctx.request_history_push = Some("Brush".into());
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

    pub fn draw_stroke(p: &egui::Painter, s: &Stroke, _col: egui::Color32, offset: egui::Vec2, _w: f32, l_op: f32) {
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
        
        let mut base_c = color32(&s.color);
        let total_alpha = (base_c.a() as f32 * l_op * s.opacity) as u8;
        base_c = egui::Color32::from_rgba_unmultiplied(base_c.r(), base_c.g(), base_c.b(), total_alpha);

        let mut stroke_color = base_c;
        if s.brush_mode == BrushMode::Highlighter {
            stroke_color = egui::Color32::from_rgba_unmultiplied(base_c.r(), base_c.g(), base_c.b(), (102.0 * l_op * s.opacity) as u8);
        }

        // --- Pass 1: Shadow (Only if stroke has shadow enabled) ---
        if s.shadow {
            let s_off = egui::vec2(2.0, 2.0);
            let s_col = egui::Color32::from_black_alpha((100.0 * l_op) as u8);
            match s.kind {
                StrokeKind::Freehand => {
                    if s.brush_shape == BrushShape::Round {
                        p.add(egui::Shape::line(pts.iter().map(|&pt| pt + s_off).collect(), egui::Stroke::new(s.width, s_col)));
                    } else {
                        for &pt in &pts { p.rect_filled(egui::Rect::from_center_size(pt + s_off, egui::vec2(s.width, s.width)), 0.0, s_col); }
                    }
                }
                _ => {} 
            }
        }

        // --- Pass 2: Outline (Only if stroke has outline enabled) ---
        if s.outline {
            let o_col = if stroke_color.r() as u32 + stroke_color.g() as u32 + stroke_color.b() as u32 > 382 { egui::Color32::BLACK } else { egui::Color32::WHITE };
            let o_col = egui::Color32::from_rgba_unmultiplied(o_col.r(), o_col.g(), o_col.b(), total_alpha);
            let o_w = s.width + 2.0;
            match s.kind {
                StrokeKind::Freehand => {
                    if s.brush_shape == BrushShape::Round {
                        p.add(egui::Shape::line(pts.clone(), egui::Stroke::new(o_w, o_col)));
                    } else {
                        for &pt in &pts { p.rect_filled(egui::Rect::from_center_size(pt, egui::vec2(o_w, o_w)), 0.0, o_col); }
                    }
                }
                _ => {}
            }
        }

        // --- Pass 3: Main Stroke ---
        match s.kind {
            StrokeKind::Freehand => {
                match s.brush_mode {
                    BrushMode::Spray => {
                        let mut rng = 42u32;
                        let half_w = s.width * 0.5;
                        for pt in &pts {
                            for _ in 0..15 {
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
                                let thickness = s.width * (cross * 0.85 + 0.15); // Dynamic thickness based on angle
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
                            let perp = nib_dir * s.width * 0.5;
                            for window in pts.windows(2) {
                                draw_quad(p, window[0] - perp, window[0] + perp, window[1] + perp, window[1] - perp, stroke_color);
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
                                (rx * scale * s.width * 0.45, ry * scale * s.width * 0.45)
                            } else {
                                (rx * s.width * 0.45, ry * s.width * 0.45)
                            };
                            
                            rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
                            let alpha_mod = (rng % 100) as f32 / 100.0 * 0.5 + 0.1;
                            let mut b_col = stroke_color;
                            b_col = egui::Color32::from_rgba_unmultiplied(b_col.r(), b_col.g(), b_col.b(), (b_col.a() as f32 * alpha_mod) as u8);
                            
                            let b_width = (s.width * 0.25).max(1.0);
                            bristle_offsets.push((egui::vec2(off_x, off_y), b_col, b_width));
                        }
                        
                        let mut mesh = egui::Mesh::default();
                        let mut smoothed: Vec<egui::Pos2> = Vec::new();
                        let min_dist = (s.width * 0.15).clamp(2.0, 10.0);
                        for &pt in &pts {
                            if smoothed.is_empty() || smoothed.last().unwrap().distance(pt) > min_dist {
                                smoothed.push(pt);
                            }
                        }
                        if smoothed.len() < 2 { smoothed = pts.clone(); }

                        for (offset, col, b_width) in bristle_offsets {
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
                                let pt = smoothed[i] + offset;
                                
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
                        let min_dist = (s.width * 0.15).clamp(2.0, 10.0);
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

                            let perp = egui::vec2(-dir.y, dir.x) * s.width * 0.5;
                            
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
                                        let pos = pt + egui::vec2(a.cos(), a.sin()) * s.width * 0.5;
                                        mesh.vertices.push(egui::epaint::Vertex { pos, uv: egui::Pos2::ZERO, color: stroke_color });
                                        if i > 0 {
                                            mesh.indices.extend_from_slice(&[center_idx, center_idx + i as u32, center_idx + i as u32 + 1]);
                                        }
                                    }
                                } else {
                                    let perp = egui::vec2(-dir.y, dir.x) * s.width * 0.5;
                                    let ext = dir * if is_start { -s.width * 0.5 } else { s.width * 0.5 };
                                    
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
                    let head_len = (s.width * 4.5).max(14.0);
                    // The arrow tip should be ahead of the brush. Base is at 'end'.
                    let tip = end + dir * head_len; 
                    let p1 = end + perp * head_len * 0.45;
                    let p2 = end - perp * head_len * 0.45;
                    p.add(egui::Shape::convex_polygon(vec![tip, p1, p2], stroke_color, egui::Stroke::NONE));
                }
            }
            StrokeKind::Poly => {
                if s.points.len() > 1 {
                    if let Some(bg) = s.background_color {
                        let mut bg_col = color32(&bg);
                        bg_col = egui::Color32::from_rgba_unmultiplied(bg_col.r(), bg_col.g(), bg_col.b(), (bg_col.a() as f32 * l_op * s.opacity) as u8);
                        p.add(egui::Shape::convex_polygon(pts.clone(), bg_col, egui::Stroke::NONE));
                    }
                    p.add(egui::Shape::line(pts, egui::Stroke::new(s.width, stroke_color)));
                }
            }
            StrokeKind::Line => {
                if s.points.len() == 2 {
                    p.line_segment([s.points[0] + offset, s.points[1] + offset], egui::Stroke::new(s.width, stroke_color));
                }
            }
            StrokeKind::Arrow => {
                if s.points.len() == 2 {
                    draw_arrow(p, s.points[0] + offset, s.points[1] + offset, s.width, stroke_color);
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
                    p.rect_stroke(rect, 0.0, egui::Stroke::new(s.width, stroke_color), egui::StrokeKind::Middle);
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
                    p.circle_stroke(center, radius, egui::Stroke::new(s.width, stroke_color));
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
                    }), s.width);
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
                    }), s.width);
                }
            }
            _ => {}
        }
    }

pub fn draw_layer_strokes(p: &egui::Painter, layer: &crate::project::Layer, render_offset: egui::Vec2, l_op: f32) {
    for s in layer.strokes.iter() {
        let mut stroke_c = color32(&s.color);
        stroke_c = egui::Color32::from_rgba_unmultiplied(stroke_c.r(), stroke_c.g(), stroke_c.b(), (stroke_c.a() as f32 * l_op * s.opacity) as u8);

        // Layer-level shadow/outline (Only if layer explicitly has them)
        if layer.shadow {
            let s_col = egui::Color32::from_rgba_unmultiplied(layer.shadow_color[0], layer.shadow_color[1], layer.shadow_color[2], (layer.shadow_color[3] as f32 * l_op) as u8);
            let offset = egui::vec2(layer.shadow_offset[0], layer.shadow_offset[1]);
            draw_stroke(p, s, s_col, render_offset + offset, s.width, l_op);
        }
        if layer.outline {
            let o_col = egui::Color32::from_rgba_unmultiplied(layer.outline_color[0], layer.outline_color[1], layer.outline_color[2], (layer.outline_color[3] as f32 * l_op) as u8);
            draw_stroke(p, s, o_col, render_offset, s.width + layer.outline_width, l_op);
        }
        
        draw_stroke(p, s, stroke_c, render_offset, s.width, l_op);
    }
}

pub fn render_preview(ctx: &mut ToolContext) {
    if ctx.current_stroke.len() < 2 { return; }
    
    let render_offset = ctx.render_offset;
    let settings = &ctx.settings;
    let painter = ctx.ui.painter_at(ctx.canvas_response.rect);
    
    let pen_c = color32(&settings.pen_color);
    
    let pts: Vec<_> = ctx.current_stroke.iter().map(|&p| p - render_offset).collect();
    let s = Stroke::new(pts, settings.pen_color, settings.pen_width, StrokeKind::Freehand, settings.brush_mode, Some(settings.background_color), settings.brush_shadow, settings.brush_shape, settings.brush_outline, settings.brush_arrow);
    draw_stroke(&painter, &s, pen_c, egui::Vec2::ZERO, s.width, 1.0);
}
