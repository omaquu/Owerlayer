use eframe::egui;
use crate::types::*;
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
    let line_start = &mut *ctx.line_start;
    let ui = &mut *ctx.ui;
    let canvas_response = ctx.canvas_response;
    let painter = ui.painter_at(canvas_response.rect);
    let pos = mouse.pos;
    let left_down = mouse.left_down;
    let left_just_pressed = mouse.left_just_pressed;
    let left_just_released = mouse.left_just_released;
    let active_layer_idx = project.active_layer;
    let render_offset = ctx.render_offset;

    let layer = &mut project.layers[active_layer_idx];
                let mode = settings.cut_mode;
            let mut cut_rect: Option<egui::Rect> = None;
            let mut cut_poly: Option<Vec<egui::Pos2>> = None;

            if mode == CutMode::Lasso {
                if left_just_pressed { }
                if left_down { 
                    current_stroke.push(pos); 
                }
                if !current_stroke.is_empty() {
                    painter.add(egui::Shape::line(current_stroke.clone(), egui::Stroke::new(1.0, egui::Color32::WHITE)));
                }
                if left_just_released && !current_stroke.is_empty() {
                    cut_poly = Some(current_stroke.clone());
                    current_stroke.clear();
                }
            } else if mode == CutMode::Rect {
                if left_just_pressed { 
                    *line_start = Some(pos); 
                }
                if let Some(start) = *line_start {
                    let rect = egui::Rect::from_two_pos(start, pos);
                    painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, egui::Color32::WHITE), egui::StrokeKind::Middle);
                }
                if left_just_released {
                    if let Some(start) = line_start.take() {
                        let rect = egui::Rect::from_two_pos(start, pos);
                        if rect.width() > 2.0 && rect.height() > 2.0 { cut_rect = Some(rect); }
                    }
                }
            } else if mode == CutMode::Polygon {
                if left_just_pressed { 
                    current_stroke.push(pos); 
                }
                let right_clicked = ui.input(|i| i.pointer.secondary_pressed());
                let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                let close_to_start = current_stroke.len() > 2 && pos.distance(current_stroke[0]) < 15.0 && left_just_pressed;
                if (right_clicked || enter_pressed || close_to_start) && !current_stroke.is_empty() {
                    cut_poly = Some(current_stroke.clone());
                    current_stroke.clear();
                }
                if !current_stroke.is_empty() {
                    let mut pts = current_stroke.clone();
                    pts.push(pos);
                    painter.add(egui::Shape::line(pts, egui::Stroke::new(1.0, egui::Color32::WHITE)));
                }
            } else if mode == CutMode::MagicWand {
                if left_just_pressed {
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
                                }
                            }
                        }
                    }
                }
            }

            if let Some(rect) = cut_rect {
                painter.rect_filled(rect, 0.0, egui::Color32::from_white_alpha(180));
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
                                        if mask[idx] != 0 { mask[idx] = 0; modified = true; }
                                    } else {
                                        let b_idx = idx * 4;
                                        if img.pixels[b_idx + 3] != 0 { img.pixels[b_idx + 3] = 0; modified = true; }
                                    }
                                }
                            }
                        }
                        if modified { img.texture = None; }
                    }
                }
                let should_erase_point = |p: egui::Pos2| -> bool {
                    let inside = rect.contains(p);
                    if settings.inverted_cut { !inside } else { inside }
                };
                layer.strokes.retain(|s| !s.points.iter().any(|&p| should_erase_point(p)));
                layer.text_annotations.retain(|t| !should_erase_point(t.position));
            } else if let Some(poly) = cut_poly {
                painter.add(egui::Shape::convex_polygon(poly.clone(), egui::Color32::from_white_alpha(180), egui::Stroke::NONE));
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
                                    if mask[idx] != 0 { mask[idx] = 0; modified = true; }
                                } else {
                                    let b_idx = idx * 4;
                                    if img.pixels[b_idx+3] != 0 { img.pixels[b_idx+3] = 0; modified = true; }
                                }
                            }
                        }
                    }
                    if modified { img.texture = None; }
                }
                let should_erase_point = |p: egui::Pos2| -> bool {
                    let inside = is_inside_poly(&poly, p);
                    if settings.inverted_cut { !inside } else { inside }
                };
                layer.strokes.retain(|s| !s.points.iter().any(|&p| should_erase_point(p)));
                layer.text_annotations.retain(|t| !should_erase_point(t.position));
            } else if mode == CutMode::Circle {
                if left_just_pressed { 
                    *line_start = Some(pos); 
                }
                if let Some(start) = *line_start {
                    painter.circle_stroke(start, start.distance(pos), egui::Stroke::new(1.0, egui::Color32::WHITE));
                }
                if left_just_released {
                    if let Some(start) = line_start.take() {
                        let radius = start.distance(pos);
                        if radius > 2.0 {
                            for img in &mut layer.placed_images {
                                let disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
                                let disp_h = img.display_size.unwrap_or([img.size[1] as f32, img.size[1] as f32])[1];
                                let mut modified = false;
                                if img.is_live && img.mask.is_none() { img.mask = Some(vec![255; img.size[0] * img.size[1]]); }
                                for py in 0..img.size[1] {
                                    for px in 0..img.size[0] {
                                        let local_pos = img.position + egui::vec2(px as f32 * (disp_w / img.size[0] as f32), py as f32 * (disp_h / img.size[1] as f32));
                                        let inside = start.distance(local_pos) <= radius;
                                        let should_erase = if settings.inverted_cut { !inside } else { inside };
                                        if should_erase {
                                            let idx = py * img.size[0] + px;
                                            if img.is_live { let mask = img.mask.as_mut().unwrap(); if mask[idx] != 0 { mask[idx] = 0; modified = true; } }
                                            else { let b_idx = idx * 4; if img.pixels[b_idx + 3] != 0 { img.pixels[b_idx + 3] = 0; modified = true; } }
                                        }
                                    }
                                }
                                if modified { img.texture = None; }
                            }
                            let should_erase_point = |p: egui::Pos2| -> bool {
                                let inside = start.distance(p) <= radius;
                                if settings.inverted_cut { !inside } else { inside }
                            };
                            layer.strokes.retain(|s| !s.points.iter().any(|&p| should_erase_point(p)));
                            layer.text_annotations.retain(|t| !should_erase_point(t.position));
                        }
                    }
                }
            } else if mode == CutMode::Star || mode == CutMode::Heart {
                if left_just_pressed { 
                    *line_start = Some(pos); 
                }
                if let Some(start) = *line_start {
                    let radius = start.distance(pos);
                    let pts = if mode == CutMode::Star { get_star_points(start, radius) } else { get_heart_points(start, radius) };
                    painter.add(egui::Shape::line(pts, egui::Stroke::new(1.0, egui::Color32::WHITE)));
                }
                if left_just_released {
                    if let Some(start) = line_start.take() {
                        let radius = start.distance(pos);
                        if radius > 5.0 {
                            let poly = if mode == CutMode::Star { get_star_points(start, radius) } else { get_heart_points(start, radius) };
                            for img in &mut layer.placed_images {
                                let disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
                                let disp_h = img.display_size.unwrap_or([img.size[1] as f32, img.size[1] as f32])[1];
                                let mut modified = false;
                                if img.is_live && img.mask.is_none() { img.mask = Some(vec![255; img.size[0] * img.size[1]]); }
                                for py in 0..img.size[1] {
                                    for px in 0..img.size[0] {
                                        let local_pos = img.position + egui::vec2(px as f32 * (disp_w / img.size[0] as f32), py as f32 * (disp_h / img.size[1] as f32));
                                        let inside = is_inside_poly(&poly, local_pos);
                                        let should_erase = if settings.inverted_cut { !inside } else { inside };
                                        if should_erase {
                                            let idx = py * img.size[0] + px;
                                            if img.is_live { let mask = img.mask.as_mut().unwrap(); if mask[idx] != 0 { mask[idx] = 0; modified = true; } }
                                            else { let b_idx = idx * 4; if img.pixels[b_idx + 3] != 0 { img.pixels[b_idx + 3] = 0; modified = true; } }
                                        }
                                    }
                                }
                                if modified { img.texture = None; }
                            }
                            let should_erase_point = |p: egui::Pos2| -> bool {
                                let inside = is_inside_poly(&poly, p);
                                if settings.inverted_cut { !inside } else { inside }
                            };
                            layer.strokes.retain(|s| !s.points.iter().any(|&p| should_erase_point(p)));
                            layer.text_annotations.retain(|t| !should_erase_point(t.position));
                        }
                    }
                }
            }

}

fn get_star_points(center: egui::Pos2, radius: f32) -> Vec<egui::Pos2> {
    let mut points = Vec::new();
    for i in 0..10 {
        let r = if i % 2 == 0 { radius } else { radius * 0.4 };
        let angle = i as f32 * std::f32::consts::PI / 5.0 - std::f32::consts::PI / 2.0;
        points.push(center + egui::vec2(angle.cos() * r, angle.sin() * r));
    }
    points
}

fn get_heart_points(center: egui::Pos2, radius: f32) -> Vec<egui::Pos2> {
    let mut points = Vec::new();
    let steps = 40;
    for i in 0..steps {
        let t = i as f32 * std::f32::consts::PI * 2.0 / steps as f32;
        let x = 16.0 * t.sin().powi(3);
        let y = -(13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos());
        points.push(center + egui::vec2(x * radius / 16.0, y * radius / 16.0));
    }
    points
}
