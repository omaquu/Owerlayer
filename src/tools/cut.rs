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

    let layer = &mut project.layers[active_layer_idx];
    let mode = settings.cut_mode;
    let mut cut_rect: Option<egui::Rect> = None;
    let mut cut_poly: Option<Vec<egui::Pos2>> = None;

    if mode == CutMode::Lasso {
        if left_down { current_stroke.push(pos); }
        if current_stroke.len() >= 2 {
            painter.add(egui::Shape::line(current_stroke.clone(), egui::Stroke::new(1.5, egui::Color32::WHITE)));
        }
        if left_just_released && current_stroke.len() >= 3 {
            cut_poly = Some(current_stroke.clone());
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
                if rect.width() > 2.0 && rect.height() > 2.0 { cut_rect = Some(rect); }
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
                    let pts: Vec<egui::Pos2> = (0..40).map(|i| {
                        let angle = i as f32 * std::f32::consts::PI * 2.0 / 40.0;
                        start + egui::vec2(angle.cos() * radius, angle.sin() * radius)
                    }).collect();
                    cut_poly = Some(pts);
                }
            }
        }
    } else if mode == CutMode::RegularPolygon {
        if left_just_pressed { *line_start = Some(pos); }
        if let Some(start) = *line_start {
            let radius = start.distance(pos);
            let n = settings.polygon_sides.max(3) as usize;
            let pts: Vec<egui::Pos2> = (0..=n).map(|i| {
                let angle = i as f32 * std::f32::consts::PI * 2.0 / n as f32 - std::f32::consts::PI / 2.0;
                start + egui::vec2(angle.cos() * radius, angle.sin() * radius)
            }).collect();
            painter.add(egui::Shape::line(pts, egui::Stroke::new(1.5, egui::Color32::WHITE)));
        }
        if left_just_released {
            if let Some(start) = line_start.take() {
                let radius = start.distance(pos);
                if radius > 5.0 {
                    let n = settings.polygon_sides.max(3) as usize;
                    let pts: Vec<egui::Pos2> = (0..n).map(|i| {
                        let angle = i as f32 * std::f32::consts::PI * 2.0 / n as f32 - std::f32::consts::PI / 2.0;
                        start + egui::vec2(angle.cos() * radius, angle.sin() * radius)
                    }).collect();
                    cut_poly = Some(pts);
                }
            }
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
                    if modified { img.texture = None; }
                }
                let should_erase_point = |p: egui::Pos2| -> bool {
                    let inside = is_inside_poly(&poly, p);
                    if settings.inverted_cut { !inside } else { inside }
                };
                layer.strokes.retain(|s| !s.points.iter().any(|&p| should_erase_point(p)));
                layer.text_annotations.retain(|t| !should_erase_point(t.position));
                *ctx.request_history_push = Some("Cut".into());
            }

}


