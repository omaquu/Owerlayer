use eframe::egui;
use crate::overlay::*;
use crate::tools::ToolContext;

fn get_transformed_points(s: &crate::overlay::Stroke) -> Vec<egui::Pos2> {
    if s.points.is_empty() { return Vec::new(); }
    let mut min = egui::pos2(f32::MAX, f32::MAX);
    let mut max = egui::pos2(f32::MIN, f32::MIN);
    for &pt in &s.points {
        min.x = min.x.min(pt.x); min.y = min.y.min(pt.y);
        max.x = max.x.max(pt.x); max.y = max.y.max(pt.y);
    }
    let initial_rect = egui::Rect::from_min_max(min, max);
    let center = initial_rect.center();

    s.points.iter().map(|&pt| {
        let mut transformed = crate::utils::transform_point_complex(pt, center, s.rotation, s.skew, s.perspective, initial_rect, s.scale);
        if s.flipped_h { transformed.x = center.x - (transformed.x - center.x); }
        if s.flipped_v { transformed.y = center.y - (transformed.y - center.y); }
        transformed
    }).collect()
}

fn hit_test_image(img: &crate::types::PlacedImage, pos: egui::Pos2, r: f32, brush_shape: BrushShape) -> bool {
    let disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
    let disp_h = img.display_size.unwrap_or([img.size[1] as f32, img.size[1] as f32])[1];
    let center = img.position + egui::vec2(disp_w * 0.5, disp_h * 0.5);
    
    let max_scale = img.scale.x.abs().max(img.scale.y.abs()).max(0.001);
    let half_diagonal = (disp_w * disp_w + disp_h * disp_h).sqrt() * 0.5 * max_scale;
    if pos.distance(center) > half_diagonal + r {
        return false;
    }
    
    let scale_x = img.size[0] as f32 / disp_w;
    let scale_y = img.size[1] as f32 / disp_h;
    let rel_world = pos - center;
    let cos = img.rotation.cos();
    let sin = img.rotation.sin();
    let px_rot = rel_world.x * cos + rel_world.y * sin;
    let py_rot = rel_world.y * cos - rel_world.x * sin;
    
    let mut sx = img.scale.x;
    let mut sy = img.scale.y;
    if img.flipped_h { sx *= -1.0; }
    if img.flipped_v { sy *= -1.0; }
    
    let kx = img.skew.x;
    let ky = img.skew.y;
    let det = 1.0 - kx * ky;
    let (rel_x, rel_y) = if det.abs() > 0.001 && sx.abs() > 0.001 && sy.abs() > 0.001 {
        ((px_rot - py_rot * kx) / (sx * det), (py_rot - px_rot * ky) / (sy * det))
    } else {
        (px_rot / sx.max(0.001), py_rot / sy.max(0.001))
    };
    
    let base_p = center + egui::vec2(rel_x, rel_y);
    let lx = (base_p.x - img.position.x) * scale_x;
    let ly = (base_p.y - img.position.y) * scale_y;
    
    let rx_local = r * scale_x / max_scale;
    let ry_local = r * scale_y / max_scale;
    
    if lx < -rx_local || lx > img.size[0] as f32 + rx_local ||
       ly < -ry_local || ly > img.size[1] as f32 + ry_local {
        return false;
    }
    
    let img_rect = egui::Rect::from_min_size(img.position, egui::vec2(disp_w, disp_h));
    let mut draw_scale = img.scale;
    if img.flipped_h { draw_scale.x *= -1.0; }
    if img.flipped_v { draw_scale.y *= -1.0; }
    
    // Check if the closest pixel on the image boundaries or inside is within brush radius
    let clamp_lx = lx.clamp(0.0, img.size[0] as f32);
    let clamp_ly = ly.clamp(0.0, img.size[1] as f32);
    let px_norm = clamp_lx / img.size[0] as f32;
    let py_norm = clamp_ly / img.size[1] as f32;
    let p_local = img.position + egui::vec2(px_norm * disp_w, py_norm * disp_h);
    
    let pixel_screen_pos = crate::utils::transform_point_complex(
        p_local,
        center,
        img.rotation,
        img.skew,
        img.perspective,
        img_rect,
        draw_scale
    );
    
    if brush_shape == BrushShape::Square {
        (pixel_screen_pos.x - pos.x).abs() <= r && (pixel_screen_pos.y - pos.y).abs() <= r
    } else {
        pixel_screen_pos.distance(pos) <= r
    }
}


pub fn update(ctx: &mut ToolContext) {
    if *ctx.layer_prompt_open { return; }
    let project = &mut *ctx.project;
    let settings = &mut *ctx.settings;
    let mouse = ctx.mouse;
    let ui = &mut *ctx.ui;
    let canvas_response = ctx.canvas_response;
    let painter = ui.painter_at(canvas_response.rect);
    let pos = mouse.pos;
    let left_down = mouse.left_down;
    let active_layer_idx = project.active_layer;
    if active_layer_idx >= project.layers.len() { return; }

    // Block erasing on a locked layer
    if project.layers[active_layer_idx].locked {
        *ctx.layer_prompt_open = true;
        return;
    }

    let layer = &mut project.layers[active_layer_idx];
    if left_down {
        let r = settings.pen_width.max(1.0);
        let hit_test = |s: &crate::overlay::Stroke| -> bool {
            let pts = get_transformed_points(s);
            if pts.is_empty() { return false; }
            let w = s.width;
            let r_total = r + w * 0.5;

            match s.kind {
                crate::overlay::StrokeKind::Line | crate::overlay::StrokeKind::Arrow => {
                    if pts.len() < 2 { return pts.iter().any(|&p| p.distance(pos) < r); }
                    let p1 = pts[0];
                    let p2 = pts[1];
                    let d = (p2.x - p1.x).powi(2) + (p2.y - p1.y).powi(2);
                    if d == 0.0 { return p1.distance(pos) < r_total; }
                    let t = ((pos.x - p1.x) * (p2.x - p1.x) + (pos.y - p1.y) * (p2.y - p1.y)) / d;
                    let t = t.clamp(0.0, 1.0);
                    let p = egui::pos2(p1.x + t * (p2.x - p1.x), p1.y + t * (p2.y - p1.y));
                    p.distance(pos) < r_total
                },
                crate::overlay::StrokeKind::Rect | crate::overlay::StrokeKind::Blur => {
                    if pts.len() < 2 { return pts.iter().any(|&p| p.distance(pos) < r); }
                    let rect = egui::Rect::from_two_pos(pts[0], pts[1]);
                    if s.background_color.is_some() || s.kind == crate::overlay::StrokeKind::Blur {
                        rect.distance_to_pos(pos) <= r
                    } else {
                        let edges = [
                            (rect.left_top(), rect.right_top()),
                            (rect.right_top(), rect.right_bottom()),
                            (rect.right_bottom(), rect.left_bottom()),
                            (rect.left_bottom(), rect.left_top()),
                        ];
                        edges.iter().any(|&(p1, p2)| {
                            let d = (p2.x - p1.x).powi(2) + (p2.y - p1.y).powi(2);
                            if d == 0.0 { return p1.distance(pos) < r_total; }
                            let t = ((pos.x - p1.x) * (p2.x - p1.x) + (pos.y - p1.y) * (p2.y - p1.y)) / d;
                            let t = t.clamp(0.0, 1.0);
                            let p = egui::pos2(p1.x + t * (p2.x - p1.x), p1.y + t * (p2.y - p1.y));
                            p.distance(pos) < r_total
                        })
                    }
                },
                crate::overlay::StrokeKind::Circle => {
                    if pts.len() < 2 { return pts.iter().any(|&p| p.distance(pos) < r); }
                    let center = pts[0];
                    let radius = pts[0].distance(pts[1]);
                    let d_mouse = pos.distance(center);
                    if s.background_color.is_some() {
                        d_mouse <= radius + r
                    } else {
                        (d_mouse - radius).abs() <= r_total
                    }
                },
                crate::overlay::StrokeKind::Star | crate::overlay::StrokeKind::Heart => {
                    if pts.len() < 2 { return pts.iter().any(|&p| p.distance(pos) < r); }
                    let center = pts[0];
                    let radius = pts[0].distance(pts[1]);
                    let d_mouse = pos.distance(center);
                    if s.background_color.is_some() {
                        d_mouse <= radius + r
                    } else {
                        (d_mouse - radius).abs() <= r_total
                    }
                },
                _ => {
                    if pts.len() < 2 { return pts.iter().any(|&p| p.distance(pos) < r); }
                    for window in pts.windows(2) {
                        let p1 = window[0];
                        let p2 = window[1];
                        let d = (p2.x - p1.x).powi(2) + (p2.y - p1.y).powi(2);
                        if d == 0.0 {
                            if p1.distance(pos) < r_total { return true; }
                        } else {
                            let t = ((pos.x - p1.x) * (p2.x - p1.x) + (pos.y - p1.y) * (p2.y - p1.y)) / d;
                            let t = t.clamp(0.0, 1.0);
                            let p = egui::pos2(p1.x + t * (p2.x - p1.x), p1.y + t * (p2.y - p1.y));
                            if p.distance(pos) < r_total { return true; }
                        }
                    }
                    false
                }
            }
        };

        if settings.eraser_mode == EraserMode::Stroke {
            let mut s_idx = 0;
            layer.strokes.retain(|s| {
                let idx = s_idx;
                s_idx += 1;
                if let Some(sel) = project.selected_object {
                    if sel == (SelectedObject { layer_idx: active_layer_idx, object_type: ObjectType::Stroke, object_idx: idx }) {
                        !hit_test(s)
                    } else {
                        true // Keep other objects
                    }
                } else {
                    !hit_test(s) // No selection: erase anything
                }
            });

            let mut img_idx = 0;
            layer.placed_images.retain(|img| {
                let idx = img_idx;
                img_idx += 1;
                if let Some(sel) = project.selected_object {
                    if sel == (SelectedObject { layer_idx: active_layer_idx, object_type: ObjectType::Image, object_idx: idx }) {
                        let hit = hit_test_image(img, pos, r, settings.brush_shape);
                        !hit
                    } else {
                        true // Keep other images
                    }
                } else {
                    // No selection: erase anything
                    let hit = hit_test_image(img, pos, r, settings.brush_shape);
                    !hit
                }
            });

            // In Stroke mode: touching the anchor point of a text annotation deletes it
            let mut to_remove = Vec::new();
            for i in 0..layer.text_annotations.len() {
                if let Some(sel) = project.selected_object {
                    if sel != (SelectedObject { layer_idx: active_layer_idx, object_type: ObjectType::Text, object_idx: i }) {
                        continue; // Keep other text
                    }
                }
                if let Some(hit_rect) = crate::utils::object_bounds(layer, crate::types::ObjectType::Text, i) {
                    let hit = if settings.brush_shape == BrushShape::Square {
                        hit_rect.intersects(egui::Rect::from_center_size(pos, egui::vec2(r*2.0, r*2.0)))
                    } else {
                        hit_rect.distance_to_pos(pos) <= r
                    };
                    if hit { to_remove.push(i); }
                }
            }
            for i in to_remove.into_iter().rev() {
                layer.text_annotations.remove(i);
            }
        } else {
            // Split mode (Pixel mode on vector lines)
            let mut new_strokes = Vec::new();
            let mut keep_strokes = Vec::new();
            
            let old_strokes = std::mem::take(&mut layer.strokes);
            for (s_idx, s) in old_strokes.into_iter().enumerate() {
                let is_selected = Some(SelectedObject { layer_idx: active_layer_idx, object_type: ObjectType::Stroke, object_idx: s_idx }) == project.selected_object;
                
                // If we have a selection and this stroke is not it, keep it unchanged
                if project.selected_object.is_some() && !is_selected {
                    keep_strokes.push(s);
                    continue;
                }

                // Only split freehand strokes (solid or arrow); keep fixed shapes
                let is_splittable = s.kind == crate::overlay::StrokeKind::Freehand;
                if !is_splittable {
                    keep_strokes.push(s);
                    continue;
                }
                
                let pts = get_transformed_points(&s);
                let mut segments: Vec<Vec<egui::Pos2>> = Vec::new();
                let mut current_segment = Vec::new();
                
                for (idx, &pt_orig) in s.points.iter().enumerate() {
                    let pt_trans = if idx < pts.len() { pts[idx] } else { pt_orig };
                    if pt_trans.distance(pos) < r {
                        if current_segment.len() >= 2 {
                            segments.push(current_segment);
                        }
                        current_segment = Vec::new();
                    } else {
                        current_segment.push(pt_orig);
                    }
                }
                if current_segment.len() >= 2 {
                    segments.push(current_segment);
                }
                
                let num_segments = segments.len();
                for (idx, seg) in segments.into_iter().enumerate() {
                    let is_last = idx == num_segments - 1;
                    let has_original_end = is_last && s.points.last() == seg.last();
                    let mut s2 = Stroke::new(
                        seg,
                        s.color,
                        s.width,
                        s.kind,
                        s.brush_mode,
                        s.background_color,
                        s.shadow,
                        s.brush_shape,
                        s.outline,
                        if has_original_end { s.arrow } else { false }, // Only the last segment containing the original end keeps the arrow!
                        s.spray_density,
                        s.highlight_opacity,
                        s.arrow_size,
                    );
                    s2.opacity = s.opacity;
                    s2.rotation = s.rotation;
                    s2.scale = s.scale;
                    s2.skew = s.skew;
                    s2.perspective = s.perspective;
                    s2.flipped_h = s.flipped_h;
                    s2.flipped_v = s.flipped_v;
                    new_strokes.push(s2);
                }
            }
            layer.strokes.extend(keep_strokes);
            layer.strokes.extend(new_strokes);

            for (img_idx, img) in layer.placed_images.iter_mut().enumerate() {
                if let Some(sel) = project.selected_object {
                    if sel != (SelectedObject { layer_idx: active_layer_idx, object_type: ObjectType::Image, object_idx: img_idx }) {
                        continue; // Keep other images intact
                    }
                }

                let disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
                let disp_h = img.display_size.unwrap_or([img.size[1] as f32, img.size[1] as f32])[1];
                let center = img.position + egui::vec2(disp_w * 0.5, disp_h * 0.5);
                
                // Bounding circle pre-filter
                let max_scale_factor = img.scale.x.abs().max(img.scale.y.abs()).max(0.001);
                let half_diagonal = (disp_w * disp_w + disp_h * disp_h).sqrt() * 0.5 * max_scale_factor;
                let overlaps = pos.distance(center) <= half_diagonal + r;

                if overlaps {
                    let mut modified = false;
                    if img.is_live && img.mask.is_none() {
                        img.mask = Some(vec![255; img.size[0] * img.size[1]]);
                        img.mask_size = Some(img.size);
                    }
                    
                    let scale_x = img.size[0] as f32 / disp_w;
                    let scale_y = img.size[1] as f32 / disp_h;
                    
                    // Compute inverse transform to get local mouse position (lx, ly)
                    let rel_world = pos - center;
                    let cos = img.rotation.cos();
                    let sin = img.rotation.sin();
                    let px_rot = rel_world.x * cos + rel_world.y * sin;
                    let py_rot = rel_world.y * cos - rel_world.x * sin;
                    
                    let mut sx = img.scale.x;
                    let mut sy = img.scale.y;
                    if img.flipped_h { sx *= -1.0; }
                    if img.flipped_v { sy *= -1.0; }
                    
                    let kx = img.skew.x;
                    let ky = img.skew.y;
                    let det = 1.0 - kx * ky;
                    let (rel_x, rel_y) = if det.abs() > 0.001 && sx.abs() > 0.001 && sy.abs() > 0.001 {
                        ((px_rot - py_rot * kx) / (sx * det), (py_rot - px_rot * ky) / (sy * det))
                    } else {
                        (px_rot / sx.max(0.001), py_rot / sy.max(0.001))
                    };
                    
                    let base_p = center + egui::vec2(rel_x, rel_y);
                    let lx = (base_p.x - img.position.x) * scale_x;
                    let ly = (base_p.y - img.position.y) * scale_y;
                    
                    // Detailed candidate pixel range based on brush radius mapped to local space
                    let r_local_x = (r * scale_x / max_scale_factor) * 2.0;
                    let r_local_y = (r * scale_y / max_scale_factor) * 2.0;
                    
                    let min_px = ((lx - r_local_x).floor() as i32).max(0) as usize;
                    let max_px = ((lx + r_local_x).ceil() as i32).min(img.size[0] as i32) as usize;
                    let min_py = ((ly - r_local_y).floor() as i32).max(0) as usize;
                    let max_py = ((ly + r_local_y).ceil() as i32).min(img.size[1] as i32) as usize;

                    let img_rect = egui::Rect::from_min_size(img.position, egui::vec2(disp_w, disp_h));
                    let mut draw_scale = img.scale;
                    if img.flipped_h { draw_scale.x *= -1.0; }
                    if img.flipped_v { draw_scale.y *= -1.0; }

                    for py in min_py..max_py {
                        for px in min_px..max_px {
                            let px_norm = px as f32 / img.size[0] as f32;
                            let py_norm = py as f32 / img.size[1] as f32;
                            let p_local = img.position + egui::vec2(px_norm * disp_w, py_norm * disp_h);
                            
                            // Map pixel local coordinate back to screen space using full forward transform
                            let pixel_screen_pos = crate::utils::transform_point_complex(
                                p_local,
                                center,
                                img.rotation,
                                img.skew,
                                img.perspective,
                                img_rect,
                                draw_scale
                            );
                            
                            let erase_hit = if settings.brush_shape == BrushShape::Square {
                                (pixel_screen_pos.x - pos.x).abs() <= r && (pixel_screen_pos.y - pos.y).abs() <= r
                            } else {
                                pixel_screen_pos.distance(pos) < r
                            };
                            
                            if erase_hit {
                                let idx = py * img.size[0] + px;
                                if img.is_live {
                                    let mask = img.mask.as_mut().unwrap();
                                    if idx < mask.len() && mask[idx] != 0 { 
                                        mask[idx] = 0; 
                                        modified = true; 
                                        img.mask_dirty = true;
                                    }
                                } else {
                                    let b_idx = idx * 4;
                                    if b_idx + 3 < img.pixels.len() && img.pixels[b_idx + 3] != 0 {
                                        img.pixels[b_idx + 3] = 0;
                                        modified = true;
                                    }
                                }
                            }
                        }
                    }
                    if modified {
                        img.texture = None;
                    }
                }
            }
        }
    }

    if ctx.mouse.left_just_released {
        for img in &mut project.layers[active_layer_idx].placed_images {
            if !img.is_live {
                crate::tools::brush::crop_to_content(img);
            }
        }
        *ctx.request_history_push = Some("Erase".into());
    }

    // Visual cursor
    let cursor_r = settings.pen_width.max(1.0);
    let color = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 180);
    let outline_color = egui::Color32::from_rgba_unmultiplied(0, 0, 0, 120);

    if settings.brush_shape == BrushShape::Square {
        let rect = egui::Rect::from_center_size(pos, egui::vec2(cursor_r * 2.0, cursor_r * 2.0));
        painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.5, color), egui::StrokeKind::Middle);
        painter.rect_stroke(rect.expand(1.5), 0.0, egui::Stroke::new(0.8, outline_color), egui::StrokeKind::Middle);
    } else {
        painter.circle_stroke(pos, cursor_r, egui::Stroke::new(1.5, color));
        painter.circle_stroke(pos, cursor_r + 1.5, egui::Stroke::new(0.8, outline_color));
    }
}
