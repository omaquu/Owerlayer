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
    let _canvas_response = ctx.canvas_response;
    let pos = mouse.pos;
    let left_just_pressed = mouse.left_just_pressed;
    let left_just_released = mouse.left_just_released;
    let active_layer_idx = project.active_layer;
    let _render_offset = ctx.render_offset;
    if active_layer_idx >= project.layers.len() { return; }
    let layer = &mut project.layers[active_layer_idx];

                if settings.shape_type == ShapeType::Poly {
                    if left_just_pressed { 
                        current_stroke.push(pos); 
                    }
                    let finish = ui.ctx().input(|i| i.key_pressed(egui::Key::Enter));
                    if finish && current_stroke.len() >= 2 {
                        let mut final_points = current_stroke.clone();
                        final_points.push(final_points[0]);
                        let s = Stroke::new(final_points, settings.pen_color, settings.stroke_width, StrokeKind::Poly, settings.brush_mode, Some(settings.background_color), settings.brush_shadow, settings.brush_shape, settings.brush_outline, false, settings.spray_density, settings.highlight_opacity);
                        let is_locked = layer.locked;
                        let ask_mode = settings.auto_new_layer.is_none();
                        if is_locked || ask_mode {
                            *ctx.pending_stroke = Some(s);
                            *ctx.layer_prompt_open = true;
                        } else {
                            layer.strokes.push(s);
                            layer.expanded = true;
                            *ctx.request_history_push = Some("Shape".into());
                        }
                        current_stroke.clear();
                    }
                } else {
                    if left_just_pressed { 
                        *line_start = Some(pos); 
                    }
                    if left_just_released {
                        if let Some(start) = line_start.take() {
                            let kind = match settings.shape_type {
                                ShapeType::Rect => StrokeKind::Rect,
                                ShapeType::Circle => StrokeKind::Circle,
                                ShapeType::Star => StrokeKind::Star,
                                ShapeType::Heart => StrokeKind::Heart,
                                ShapeType::Arrow => StrokeKind::Arrow,
                                _ => StrokeKind::Rect,
                            };
                            let s = Stroke::new(vec![start, pos], settings.pen_color, settings.stroke_width, kind, settings.brush_mode, Some(settings.background_color), settings.brush_shadow, settings.brush_shape, settings.brush_outline, false, settings.spray_density, settings.highlight_opacity);
                            let is_locked = layer.locked;
                            let ask_mode = settings.auto_new_layer.is_none();
                            if is_locked || ask_mode {
                                *ctx.pending_stroke = Some(s);
                                *ctx.layer_prompt_open = true;
                            } else {
                                layer.strokes.push(s);
                                layer.expanded = true;
                                *ctx.request_history_push = Some("Shape".into());
                            }
                        }
                    }
                }

}

pub fn render_preview(ctx: &mut ToolContext) {
    let settings = &ctx.settings;
    if settings.shape_type == ShapeType::Poly {
        let pts = ctx.current_stroke.clone();
        if !pts.is_empty() {
            let painter = ctx.ui.painter_at(ctx.canvas_response.rect);
            let pen_c = color32(&settings.pen_color);
            let stroke = egui::Stroke::new(settings.stroke_width, pen_c);
            let render_offset = ctx.render_offset;
            
            let render_pts: Vec<egui::Pos2> = pts.iter().map(|&p| p - render_offset).collect();
            for window in render_pts.windows(2) {
                painter.line_segment([window[0], window[1]], stroke);
            }
            
            let last_pt = render_pts[render_pts.len() - 1];
            let mouse_pos = ctx.mouse.pos - render_offset;
            painter.line_segment([last_pt, mouse_pos], egui::Stroke::new(settings.stroke_width, pen_c.gamma_multiply(0.6)));
            
            for &pt in &render_pts {
                painter.circle_filled(pt, 3.0, pen_c);
                painter.circle_stroke(pt, 4.0, egui::Stroke::new(1.0, egui::Color32::WHITE));
            }
        }
        return;
    }

    let start = match ctx.line_start { Some(s) => *s, None => return };
    let pos = ctx.mouse.pos;
    let render_offset = ctx.render_offset;
    let painter = ctx.ui.painter_at(ctx.canvas_response.rect);
    
    let pen_c = color32(&settings.pen_color);
    let rect = egui::Rect::from_two_pos(start, pos).translate(-render_offset);
    
    match settings.shape_type {
        ShapeType::Rect => {
            let bg_c = color32(&settings.background_color);
            painter.rect_filled(rect, 0.0, egui::Color32::from_rgba_unmultiplied(bg_c.r(), bg_c.g(), bg_c.b(), bg_c.a()/2));
            painter.rect_stroke(rect, 0.0, egui::Stroke::new(settings.stroke_width, pen_c), egui::StrokeKind::Middle);
        }
        ShapeType::Circle => {
            let radius = (start - pos).length();
            let bg_c = color32(&settings.background_color);
            painter.circle_filled(start - render_offset, radius, egui::Color32::from_rgba_unmultiplied(bg_c.r(), bg_c.g(), bg_c.b(), bg_c.a()/2));
            painter.circle_stroke(start - render_offset, radius, egui::Stroke::new(settings.stroke_width, pen_c));
        }
        ShapeType::Star => {
            let radius = (start - pos).length();
            let bg_c = color32(&settings.background_color);
            draw_star_shape(&painter, start - render_offset, radius, pen_c, Some(egui::Color32::from_rgba_unmultiplied(bg_c.r(), bg_c.g(), bg_c.b(), bg_c.a()/2)), settings.stroke_width);
        }
        ShapeType::Heart => {
            let radius = (start - pos).length();
            let bg_c = color32(&settings.background_color);
            draw_heart_shape(&painter, start - render_offset, radius, pen_c, Some(egui::Color32::from_rgba_unmultiplied(bg_c.r(), bg_c.g(), bg_c.b(), bg_c.a()/2)), settings.stroke_width);
        }
        ShapeType::Arrow => {
            draw_arrow(&painter, start - render_offset, pos - render_offset, settings.stroke_width, pen_c);
        }
        ShapeType::Poly => {}
    }
}
