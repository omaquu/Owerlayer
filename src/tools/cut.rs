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
    let mouse = ctx.mouse;
    let current_stroke = &mut *ctx.current_stroke;
    let line_start = &mut *ctx.line_start;
    let ui = &mut *ctx.ui;
    let canvas_response = ctx.canvas_response;
    let painter = ui.painter_at(canvas_response.rect);
    let pos = mouse.pos;
    let left_down = mouse.left_down;
    let left_just_pressed = mouse.left_just_pressed;
    let left_just_released = mouse.left_just_released;
    let active_layer_idx = project.active_layer;
    let _render_offset = ctx.render_offset;
    if active_layer_idx >= project.layers.len() { return; }

    let mode = settings.cut_mode;
    
    // ---- Dragging selection boundary state ----
    if left_just_pressed {
        if let Some(sel) = &project.marquee_selection {
            let inside = match &sel.shape {
                SelectionShape::Rect(r) => r.contains(pos),
                SelectionShape::Circle { center, radius } => pos.distance(*center) <= *radius,
                SelectionShape::Poly(pts) => is_inside_poly(pts, pos),
            };
            if inside {
                *ctx.drag_state = 999; // Unique id for dragging selection outline
            }
        }
    }
    
    if *ctx.drag_state == 999 {
        let delta = mouse.delta;
        if let Some(sel) = &mut project.marquee_selection {
            match &mut sel.shape {
                SelectionShape::Rect(r) => { *r = r.translate(delta); }
                SelectionShape::Circle { center, .. } => { *center += delta; }
                SelectionShape::Poly(pts) => { for p in pts { *p += delta; } }
            }
        }
        if left_just_released {
            *ctx.drag_state = 0;
        }
        return; // Skip drawing new marquee selection while dragging existing one
    }

    // Clicking outside existing selection starts a new one
    if left_just_pressed {
        project.marquee_selection = None;
    }

    if mode == CutMode::Lasso {
        if left_down { current_stroke.push(pos); }
        if current_stroke.len() >= 2 {
            painter.add(egui::Shape::line(current_stroke.clone(), egui::Stroke::new(1.5, egui::Color32::WHITE)));
        }
        if left_just_released && current_stroke.len() >= 3 {
            project.marquee_selection = Some(MarqueeSelection {
                shape: SelectionShape::Poly(current_stroke.clone())
            });
            current_stroke.clear();
        }
    } else if mode == CutMode::Rect {
        if left_just_pressed { *line_start = Some(pos); }
        if let Some(start) = *line_start {
            let rect = egui::Rect::from_two_pos(start, pos);
            painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.5, egui::Color32::WHITE), egui::StrokeKind::Middle);
            painter.rect_filled(rect, 0.0, egui::Color32::from_white_alpha(15));
        }
        if left_just_released {
            if let Some(start) = line_start.take() {
                let rect = egui::Rect::from_two_pos(start, pos);
                if rect.width() > 2.0 && rect.height() > 2.0 {
                    project.marquee_selection = Some(MarqueeSelection {
                        shape: SelectionShape::Rect(rect)
                    });
                }
            }
        }
    } else if mode == CutMode::Circle {
        if left_just_pressed { *line_start = Some(pos); }
        if let Some(start) = *line_start {
            let radius = start.distance(pos);
            let pts: Vec<egui::Pos2> = (0..=40).map(|i| {
                let angle = i as f32 * std::f32::consts::PI * 2.0 / 40.0;
                start + egui::vec2(angle.cos() * radius, angle.sin() * radius)
            }).collect();
            painter.add(egui::Shape::line(pts, egui::Stroke::new(1.5, egui::Color32::WHITE)));
        }
        if left_just_released {
            if let Some(start) = line_start.take() {
                let radius = start.distance(pos);
                if radius > 5.0 {
                    project.marquee_selection = Some(MarqueeSelection {
                        shape: SelectionShape::Circle { center: start, radius }
                    });
                }
            }
        }
    } else if mode == CutMode::Polygon {
        if left_just_pressed {
            current_stroke.push(pos);
        }
        let right_clicked = ui.ctx().input(|i| i.pointer.secondary_pressed());
        let enter_pressed = ui.ctx().input(|i| i.key_pressed(egui::Key::Enter));
        let close_to_start = current_stroke.len() > 2 && pos.distance(current_stroke[0]) < 15.0 && left_just_pressed;

        if (right_clicked || enter_pressed || close_to_start) && !current_stroke.is_empty() {
            project.marquee_selection = Some(MarqueeSelection {
                shape: SelectionShape::Poly(current_stroke.clone())
            });
            current_stroke.clear();
        }
        if !current_stroke.is_empty() {
            let mut pts = current_stroke.clone();
            pts.push(pos);
            painter.add(egui::Shape::line(pts, egui::Stroke::new(1.5, egui::Color32::WHITE)));
        }
    } else if mode == CutMode::MagicWand {
        // Magic wand remains direct pixel clearing for instant use
        if left_just_pressed {
            let layer = &mut project.layers[active_layer_idx];
            for img in &mut layer.placed_images {
                let disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
                let disp_h = img.display_size.unwrap_or([img.size[1] as f32, img.size[1] as f32])[1];
                let img_rect = egui::Rect::from_min_size(img.position, egui::vec2(disp_w, disp_h));
                if img_rect.contains(pos) {
                    let px = ((pos.x - img.position.x) * (img.size[0] as f32 / disp_w)) as i32;
                    let py = ((pos.y - img.position.y) * (img.size[1] as f32 / disp_h)) as i32;
                    if px >= 0 && px < img.size[0] as i32 && py >= 0 && py < img.size[1] as i32 {
                        let start_idx = (py as usize * img.size[0] + px as usize) * 4;
                        let start_color = [img.pixels[start_idx], img.pixels[start_idx+1], img.pixels[start_idx+2], img.pixels[start_idx+3]];
                        if start_color[3] > 0 {
                            magic_wand_flood_fill(img, px, py, start_color, settings.magic_wand_threshold);
                            img.texture = None;
                            img.thumbnail_dirty = true;
                            *ctx.request_history_push = Some("Cut".into());
                        }
                    }
                }
            }
        }
    }
}

pub fn erase_marquee_selection(project: &mut crate::project::Project, settings: &crate::types::Settings) {
    let layer_idx = project.active_layer;
    if layer_idx >= project.layers.len() { return; }
    
    let sel = match &project.marquee_selection {
        Some(s) => s.clone(),
        None => return,
    };
    
    let layer = &mut project.layers[layer_idx];
    
    match &sel.shape {
        SelectionShape::Rect(rect) => {
            let rect = *rect;
            for img in &mut layer.placed_images {
                let disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
                let disp_h = img.display_size.unwrap_or([img.size[1] as f32, img.size[1] as f32])[1];
                let img_rect = egui::Rect::from_min_size(img.position, egui::vec2(disp_w, disp_h));
                if img_rect.intersects(rect) {
                    let mut modified = false;
                    if img.is_live && img.mask.is_none() {
                        img.mask = Some(vec![255; img.size[0] * img.size[1]]);
                    }

                    for py in 0..img.size[1] {
                        for px in 0..img.size[0] {
                            let local_pos = img.position + egui::vec2(px as f32 * (disp_w / img.size[0] as f32), py as f32 * (disp_h / img.size[1] as f32));
                            let inside = rect.contains(local_pos);
                            let should_erase = if settings.inverted_cut { !inside } else { inside };
                            if should_erase {
                                let idx = py * img.size[0] + px;
                                if img.is_live {
                                    let mask = img.mask.as_mut().unwrap();
                                    if mask[idx] != 0 { 
                                        mask[idx] = 0; 
                                        modified = true; 
                                        img.mask_dirty = true;
                                    }
                                } else {
                                    let b_idx = idx * 4;
                                    if img.pixels[b_idx + 3] != 0 { img.pixels[b_idx + 3] = 0; modified = true; }
                                }
                            }
                        }
                    }
                    if modified { img.texture = None; img.thumbnail_dirty = true; }
                }
            }
            let should_erase_point = |p: egui::Pos2| -> bool {
                let inside = rect.contains(p);
                if settings.inverted_cut { !inside } else { inside }
            };
            layer.strokes.retain(|s| !s.points.iter().any(|&p| should_erase_point(p)));
            layer.text_annotations.retain(|t| !should_erase_point(t.position));
        }
        SelectionShape::Circle { center, radius } => {
            let center = *center;
            let radius = *radius;
            let poly: Vec<egui::Pos2> = (0..40).map(|i| {
                let angle = i as f32 * std::f32::consts::PI * 2.0 / 40.0;
                center + egui::vec2(angle.cos() * radius, angle.sin() * radius)
            }).collect();
            
            for img in &mut layer.placed_images {
                let disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
                let disp_h = img.display_size.unwrap_or([img.size[1] as f32, img.size[1] as f32])[1];
                let mut modified = false;
                if img.is_live && img.mask.is_none() {
                    img.mask = Some(vec![255; img.size[0] * img.size[1]]);
                }

                for py in 0..img.size[1] {
                    for px in 0..img.size[0] {
                        let local_pos = img.position + egui::vec2(px as f32 * (disp_w / img.size[0] as f32), py as f32 * (disp_h / img.size[1] as f32));
                        let inside = is_inside_poly(&poly, local_pos);
                        let should_erase = if settings.inverted_cut { !inside } else { inside };
                        if should_erase {
                            let idx = py * img.size[0] + px;
                            if img.is_live {
                                let mask = img.mask.as_mut().unwrap();
                                if mask[idx] != 0 { 
                                    mask[idx] = 0; 
                                    modified = true; 
                                    img.mask_dirty = true;
                                }
                            } else {
                                let b_idx = idx * 4;
                                if img.pixels[b_idx+3] != 0 { img.pixels[b_idx+3] = 0; modified = true; }
                            }
                        }
                    }
                }
                if modified { img.texture = None; img.thumbnail_dirty = true; }
            }
            let should_erase_point = |p: egui::Pos2| -> bool {
                let inside = is_inside_poly(&poly, p);
                if settings.inverted_cut { !inside } else { inside }
            };
            layer.strokes.retain(|s| !s.points.iter().any(|&p| should_erase_point(p)));
            layer.text_annotations.retain(|t| !should_erase_point(t.position));
        }
        SelectionShape::Poly(poly) => {
            for img in &mut layer.placed_images {
                let disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
                let disp_h = img.display_size.unwrap_or([img.size[1] as f32, img.size[1] as f32])[1];
                let mut modified = false;
                if img.is_live && img.mask.is_none() {
                    img.mask = Some(vec![255; img.size[0] * img.size[1]]);
                }

                for py in 0..img.size[1] {
                    for px in 0..img.size[0] {
                        let local_pos = img.position + egui::vec2(px as f32 * (disp_w / img.size[0] as f32), py as f32 * (disp_h / img.size[1] as f32));
                        let inside = is_inside_poly(poly, local_pos);
                        let should_erase = if settings.inverted_cut { !inside } else { inside };
                        if should_erase {
                            let idx = py * img.size[0] + px;
                            if img.is_live {
                                let mask = img.mask.as_mut().unwrap();
                                if mask[idx] != 0 { 
                                    mask[idx] = 0; 
                                    modified = true; 
                                    img.mask_dirty = true;
                                }
                            } else {
                                let b_idx = idx * 4;
                                if img.pixels[b_idx+3] != 0 { img.pixels[b_idx+3] = 0; modified = true; }
                            }
                        }
                    }
                }
                if modified { img.texture = None; img.thumbnail_dirty = true; }
            }
            let should_erase_point = |p: egui::Pos2| -> bool {
                let inside = is_inside_poly(poly, p);
                if settings.inverted_cut { !inside } else { inside }
            };
            layer.strokes.retain(|s| !s.points.iter().any(|&p| should_erase_point(p)));
            layer.text_annotations.retain(|t| !should_erase_point(t.position));
        }
    }
}


