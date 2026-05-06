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
    let new_selection = &mut *ctx.new_selection;
    let switch_to_move = &mut *ctx.switch_to_move;
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
                match settings.mirror_mode {
                MirrorMode::Rect => {
                    if left_just_pressed { *line_start = Some(pos); }
                    if let Some(start) = *line_start {
                        let rect = egui::Rect::from_two_pos(start, pos);
                        painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, egui::Color32::WHITE), egui::StrokeKind::Middle);
                    }
                    if left_just_released {
                        if let Some(start) = line_start.take() {
                            let rect = egui::Rect::from_two_pos(start, pos);
                            if rect.width() > 4.0 && rect.height() > 4.0 {
                                let mut mask = None;
                                if settings.mirror_mode == MirrorMode::Circle {
                                    let w = rect.width();
                                    let h = rect.height();
                                    let ppp = ui.ctx().pixels_per_point();
                                    let mut m = vec![255u8; (w * ppp) as usize * (h * ppp) as usize];
                                    let center = egui::pos2(w * 0.5, h * 0.5);
                                    let radius = w.min(h) * 0.5;
                                    for py in 0..(h * ppp) as usize {
                                        for px in 0..(w * ppp) as usize {
                                            let lp = egui::pos2(px as f32 / ppp, py as f32 / ppp);
                                            if lp.distance(center) > radius {
                                                m[py * (w * ppp) as usize + px] = 0;
                                            }
                                        }
                                    }
                                    mask = Some(m);
                                }
                                let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
                                let mut img = PlacedImage::new(id, rect.min, [rect.width() as usize, rect.height() as usize], Vec::new());
                                img.is_live = true;
                                img.source_rect = Some([rect.min.x, rect.min.y, rect.width(), rect.height()]);
                                img.mask = mask;
                                layer.placed_images.push(img);
                            }
                        }
                    }
                }
                MirrorMode::Circle => {
                    if left_just_pressed { *line_start = Some(pos); }
                    if let Some(start) = *line_start {
                        let rect = egui::Rect::from_two_pos(start, pos);
                        painter.circle_stroke(rect.center(), rect.width().min(rect.height()) * 0.5, egui::Stroke::new(1.0, egui::Color32::WHITE));
                    }
                    if left_just_released {
                        if let Some(start) = line_start.take() {
                            let rect = egui::Rect::from_two_pos(start, pos);
                            if rect.width() > 4.0 && rect.height() > 4.0 {
                                let ppp = ui.ctx().pixels_per_point();
                                let w = rect.width();
                                let h = rect.height();
                                let mut mask = vec![255u8; (w * ppp) as usize * (h * ppp) as usize];
                                let center = egui::pos2(w * 0.5, h * 0.5);
                                let radius = w.min(h) * 0.5;
                                for py in 0..(h * ppp) as usize {
                                    for px in 0..(w * ppp) as usize {
                                        let lp = egui::pos2(px as f32 / ppp, py as f32 / ppp);
                                        if lp.distance(center) > radius {
                                            mask[py * (w * ppp) as usize + px] = 0;
                                        }
                                    }
                                }
                                let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
                                let mut img = PlacedImage::new(id, rect.min, [w as usize, h as usize], Vec::new());
                                img.is_live = true;
                                img.source_rect = Some([rect.min.x, rect.min.y, w, h]);
                                img.mask = Some(mask);
                                layer.placed_images.push(img);
                            }
                        }
                    }
                }
                MirrorMode::Lasso | MirrorMode::Poly => {
                    if left_just_pressed {
                        if settings.mirror_mode == MirrorMode::Poly {
                            if current_stroke.is_empty() { current_stroke.push(pos); }
                            else {
                                if pos.distance(current_stroke[0]) < 10.0 {
                                    if current_stroke.len() >= 3 {
                                        let mut min = egui::pos2(f32::MAX, f32::MAX);
                                        let mut max = egui::pos2(f32::MIN, f32::MIN);
                                        for &p in current_stroke.iter() {
                                            min.x = min.x.min(p.x); min.y = min.y.min(p.y);
                                            max.x = max.x.max(p.x); max.y = max.y.max(p.y);
                                        }
                                        let rect = egui::Rect::from_min_max(min, max);
                                        let ppp = ui.ctx().pixels_per_point();
                                        let mut mask = vec![255u8; (rect.width() * ppp) as usize * (rect.height() * ppp) as usize];
                                        for py in 0..(rect.height() * ppp) as usize {
                                            for px in 0..(rect.width() * ppp) as usize {
                                                let p = rect.min + egui::vec2(px as f32 / ppp, py as f32 / ppp);
                                                if !is_inside_poly(&current_stroke, p) {
                                                    mask[py * (rect.width() * ppp) as usize + px] = 0;
                                                }
                                            }
                                        }
                                        let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
                                        let mut img = PlacedImage::new(id, rect.min, [rect.width() as usize, rect.height() as usize], Vec::new());
                                        img.is_live = true;
                                        img.source_rect = Some([rect.min.x, rect.min.y, rect.width(), rect.height()]);
                                        img.mask = Some(mask);
                                        layer.placed_images.push(img);
                                    }
                                    current_stroke.clear();
                                } else {
                                    current_stroke.push(pos);
                                }
                            }
                        } else {
                            current_stroke.clear();
                            current_stroke.push(pos);
                        }
                    }
                    if left_down && settings.mirror_mode == MirrorMode::Lasso {
                        if let Some(last) = current_stroke.last() {
                            if pos.distance(*last) > 4.0 { current_stroke.push(pos); }
                        }
                    }
                    if left_just_released && settings.mirror_mode == MirrorMode::Lasso {
                        if current_stroke.len() >= 3 {
                            let mut min = egui::pos2(f32::MAX, f32::MAX);
                            let mut max = egui::pos2(f32::MIN, f32::MIN);
                            for &p in current_stroke.iter() {
                                min.x = min.x.min(p.x); min.y = min.y.min(p.y);
                                max.x = max.x.max(p.x); max.y = max.y.max(p.y);
                            }
                            let rect = egui::Rect::from_min_max(min, max);
                            let ppp = ui.ctx().pixels_per_point();
                            let mut mask = vec![255u8; (rect.width() * ppp) as usize * (rect.height() * ppp) as usize];
                            for py in 0..(rect.height() * ppp) as usize {
                                for px in 0..(rect.width() * ppp) as usize {
                                    let p = rect.min + egui::vec2(px as f32 / ppp, py as f32 / ppp);
                                    if !is_inside_poly(&current_stroke, p) {
                                        mask[py * (rect.width() * ppp) as usize + px] = 0;
                                    }
                                }
                            }
                            let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
                            let mut img = PlacedImage::new(id, rect.min, [rect.width() as usize, rect.height() as usize], Vec::new());
                            img.is_live = true;
                            img.source_rect = Some([rect.min.x, rect.min.y, rect.width(), rect.height()]);
                            img.mask = Some(mask);
                            layer.placed_images.push(img);
                        }
                        current_stroke.clear();
                    }
                    if !current_stroke.is_empty() {
                        painter.add(egui::Shape::line(current_stroke.clone(), egui::Stroke::new(1.0, egui::Color32::WHITE)));
                        for &p in current_stroke.iter() { painter.circle_filled(p, 2.0, egui::Color32::WHITE); }
                    }
                }
                MirrorMode::Window => {
                    // Window mode now behaves like Rect mode initially
                    if left_just_pressed { *line_start = Some(pos); }
                    if let Some(start) = *line_start {
                        let rect = egui::Rect::from_two_pos(start, pos);
                        painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, egui::Color32::WHITE), egui::StrokeKind::Middle);
                    }
                    if left_just_released {
                        if let Some(start) = line_start.take() {
                            let rect = egui::Rect::from_two_pos(start, pos);
                            let w = rect.width();
                            let h = rect.height();
                            if w > 4.0 && h > 4.0 {
                                let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
                                let mut img = PlacedImage::new(id, rect.min, [w as usize, h as usize], Vec::new());
                                img.is_live = true;
                                img.source_rect = Some([rect.min.x, rect.min.y, w, h]);
                                layer.placed_images.push(img);
                                // Request auto-selection and tool switch after borrow ends
                                *new_selection = Some(crate::project::SelectedObject {
                                    layer_idx: active_layer_idx,
                                    object_type: crate::project::ObjectType::Image,
                                    object_idx: layer.placed_images.len() - 1,
                                });
                                *switch_to_move = true;
                            }
                        }
                    }
                }
            }

}
