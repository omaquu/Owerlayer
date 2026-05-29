use eframe::egui;
use crate::utils::*;
use crate::overlay::*;

use crate::tools::ToolContext;

pub fn update(ctx: &mut ToolContext) {
    if *ctx.layer_prompt_open { return; }
    let project = &mut *ctx.project;
    let settings = &mut *ctx.settings;
    let mouse = ctx.mouse;
    let line_start = &mut *ctx.line_start;
    let ui = &mut *ctx.ui;
    let canvas_response = ctx.canvas_response;
    let _painter = ui.painter_at(canvas_response.rect);
    let pos = mouse.pos;
    let left_just_pressed = mouse.left_just_pressed;
    let left_just_released = mouse.left_just_released;
    let _render_offset = ctx.render_offset;

    if left_just_pressed { *line_start = Some(pos); }
    if left_just_released {
        if let Some(start) = line_start.take() {
            let shape = settings.shape_type;
            
            // Determine dimensions, coordinates, and mask depending on the shape
            let (rect, mask, snip_points) = match shape {
                ShapeType::Circle => {
                    let rect = egui::Rect::from_two_pos(start, pos);
                    let w = rect.width();
                    let h = rect.height();
                    if w > 4.0 && h > 4.0 {
                        let ppp = ui.ctx().pixels_per_point();
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
                        
                        let mut local_pts = Vec::new();
                        let segments = 64;
                        let c_center = egui::pos2(w * 0.5, h * 0.5);
                        let c_radius = w.min(h) * 0.5;
                        for idx in 0..=segments {
                            let angle = (idx as f32 / segments as f32) * std::f32::consts::TAU;
                            let px = c_center.x + c_radius * angle.cos();
                            let py = c_center.y + c_radius * angle.sin();
                            local_pts.push(egui::pos2(px, py));
                        }
                        (rect, Some(mask), Some(local_pts))
                    } else {
                        return;
                    }
                }
                ShapeType::Star | ShapeType::Heart => {
                    let radius = start.distance(pos);
                    if radius > 4.0 {
                        let pts = if shape == ShapeType::Star {
                            get_star_points(start, radius)
                        } else {
                            get_heart_points(start, radius)
                        };
                        let bounds = egui::Rect::from_points(&pts);
                        let w = bounds.width();
                        let h = bounds.height();
                        let ppp = ui.ctx().pixels_per_point();
                        let sw = (w * ppp) as i32;
                        let sh = (h * ppp) as i32;
                        
                        if sw > 4 && sh > 4 {
                            let poly: Vec<egui::Pos2> = pts.iter().map(|p| egui::pos2(p.x - bounds.min.x, p.y - bounds.min.y)).collect();
                            let mut mask = vec![255u8; sw as usize * sh as usize];
                            for py in 0..sh as usize {
                                for px in 0..sw as usize {
                                    let lp = egui::pos2(px as f32 / ppp, py as f32 / ppp);
                                    if !is_inside_poly(&poly, lp) {
                                        mask[py * sw as usize + px] = 0;
                                    }
                                }
                            }
                            (bounds, Some(mask), Some(poly))
                        } else {
                            return;
                        }
                    } else {
                        return;
                    }
                }
                _ => {
                    // Default/Rectangle
                    let rect = egui::Rect::from_two_pos(start, pos);
                    let w = rect.width();
                    let h = rect.height();
                    if w > 4.0 && h > 4.0 {
                        let snip_pts = vec![
                            egui::pos2(0.0, 0.0),
                            egui::pos2(w, 0.0),
                            egui::pos2(w, h),
                            egui::pos2(0.0, h),
                            egui::pos2(0.0, 0.0),
                        ];
                        (rect, None, Some(snip_pts))
                    } else {
                        return;
                    }
                }
            };

            let mut blur_layer_idx = None;
            for (idx, l) in project.layers.iter().enumerate() {
                if l.name == "Blurs" { blur_layer_idx = Some(idx); break; }
            }
            let l_idx = if let Some(idx) = blur_layer_idx {
                idx
            } else {
                project.layers.push(crate::project::Layer::new("Blurs"));
                project.layers.len() - 1
            };
            let target_layer = &mut project.layers[l_idx];

            let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
            let mut img = PlacedImage::new(id, rect.min, [rect.width().round() as usize, rect.height().round() as usize], Vec::new());
            img.is_live = true;
            img.blur = settings.blur_strength;
            img.blur_effect = settings.blur_effect;
            img.mask = mask;
            img.mask_dirty = img.mask.is_some();
            img.snip_points = snip_points;
            
            target_layer.placed_images.push(img);
            target_layer.expanded = true;
            *ctx.request_history_push = Some("Blur".into());
        }
    }
}

pub fn render_preview(ctx: &mut ToolContext) {
    let start = match ctx.line_start { Some(s) => *s, None => return };
    let pos = ctx.mouse.pos;
    let render_offset = ctx.render_offset;
    let painter = ctx.ui.painter_at(ctx.canvas_response.rect);
    let settings = &ctx.settings;
    
    let color = egui::Color32::WHITE;
    let stroke = egui::Stroke::new(1.0, color);
    
    match settings.shape_type {
        ShapeType::Circle => {
            let rect = egui::Rect::from_two_pos(start, pos);
            let center = rect.center() - render_offset;
            let radius = rect.width().min(rect.height()) * 0.5;
            painter.circle_stroke(center, radius, stroke);
        }
        ShapeType::Star => {
            let radius = start.distance(pos);
            let pts = get_star_points(start - render_offset, radius);
            if pts.len() >= 2 {
                let mut closed_pts = pts.clone();
                closed_pts.push(pts[0]);
                painter.add(egui::Shape::line(closed_pts, stroke));
            }
        }
        ShapeType::Heart => {
            let radius = start.distance(pos);
            let pts = get_heart_points(start - render_offset, radius);
            if pts.len() >= 2 {
                let mut closed_pts = pts.clone();
                closed_pts.push(pts[0]);
                painter.add(egui::Shape::line(closed_pts, stroke));
            }
        }
        _ => {
            let rect = egui::Rect::from_two_pos(start, pos).translate(-render_offset);
            painter.rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Middle);
        }
    }
}
