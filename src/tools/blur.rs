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
    let pos = mouse.pos;
    let left_just_pressed = mouse.left_just_pressed;
    let left_just_released = mouse.left_just_released;

    if settings.shape_type == ShapeType::Poly {
        if left_just_pressed {
            ctx.current_stroke.push(pos);
        }
        let right_clicked = ui.input(|i| i.pointer.secondary_pressed());
        let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
        let close_to_start = ctx.current_stroke.len() > 2 && pos.distance(ctx.current_stroke[0]) < 15.0 && left_just_pressed;

        if (right_clicked || enter_pressed || close_to_start) && !ctx.current_stroke.is_empty() {
            let bounds = egui::Rect::from_points(&ctx.current_stroke);
            let w = bounds.width();
            let h = bounds.height();
            let ppp = ui.ctx().pixels_per_point();
            let sw = (w * ppp) as i32;
            let sh = (h * ppp) as i32;
            if sw > 4 && sh > 4 {
                let poly: Vec<egui::Pos2> = ctx.current_stroke.iter().map(|p| egui::pos2(p.x - bounds.min.x, p.y - bounds.min.y)).collect();
                let mut mask = vec![255u8; sw as usize * sh as usize];
                for py in 0..sh as usize {
                    for px in 0..sw as usize {
                        let lp = egui::pos2(px as f32 / ppp, py as f32 / ppp);
                        if !is_inside_poly(&poly, lp) {
                            mask[py * sw as usize + px] = 0;
                        }
                    }
                }
                
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
                
                let pw = (w * ppp).round() as usize;
                let ph = (h * ppp).round() as usize;
                let mut img = PlacedImage::new(id, bounds.min, [pw, ph], Vec::new());
                img.name = "Blur".to_string();
                img.display_size = Some([w, h]);
                img.is_live = true;
                img.blur = settings.blur_strength;
                img.blur_effect = settings.blur_effect;
                img.mask = Some(mask);
                img.mask_size = Some([pw, ph]);
                img.mask_dirty = img.mask.is_some();
                img.snip_points = Some(poly);
                
                // Set source_rect so capture thread knows what region to capture
                let (wx, wy) = crate::winapi_utils::get_window_screen_pos();
                let window_origin = egui::vec2(wx as f32 / ppp, wy as f32 / ppp);
                let screen_rect = bounds.translate(window_origin);
                img.source_rect = Some([screen_rect.min.x, screen_rect.min.y, screen_rect.width(), screen_rect.height()]);
                img.snip_source_overlay = settings.snip_source_overlay;
                
                target_layer.placed_images.push(img);
                target_layer.expanded = true;
                *ctx.request_history_push = Some("Blur".into());
            }
            ctx.current_stroke.clear();
        }
    } else {
        if left_just_pressed { *line_start = Some(pos); }
        if left_just_released {
            if let Some(start) = line_start.take() {
                let shape = settings.shape_type;
                
                // Determine dimensions, coordinates, and mask depending on the shape
                let (rect, mask, snip_points) = match shape {
                    ShapeType::Circle => {
                        let radius = start.distance(pos);
                        if radius > 2.0 {
                            let rect = egui::Rect::from_center_size(start, egui::vec2(radius * 2.0, radius * 2.0));
                            let w = rect.width();
                            let h = rect.height();
                            let ppp = ui.ctx().pixels_per_point();
                            let mut mask = vec![255u8; (w * ppp) as usize * (h * ppp) as usize];
                            let center = egui::pos2(w * 0.5, h * 0.5);
                            let mask_radius = radius;
                            for py in 0..(h * ppp) as usize {
                                for px in 0..(w * ppp) as usize {
                                    let lp = egui::pos2(px as f32 / ppp, py as f32 / ppp);
                                    if lp.distance(center) > mask_radius {
                                        mask[py * (w * ppp) as usize + px] = 0;
                                    }
                                }
                            }
                            
                            let mut local_pts = Vec::new();
                            let segments = 64;
                            let c_center = egui::pos2(w * 0.5, h * 0.5);
                            let c_radius = radius;
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
                
                let ppp = ui.ctx().pixels_per_point();
                let pw = (rect.width() * ppp).round() as usize;
                let ph = (rect.height() * ppp).round() as usize;
                let mut img = PlacedImage::new(id, rect.min, [pw, ph], Vec::new());
                img.name = "Blur".to_string();
                img.display_size = Some([rect.width(), rect.height()]);
                img.is_live = true;
                img.blur = settings.blur_strength;
                img.blur_effect = settings.blur_effect;
                img.mask = mask;
                img.mask_size = img.mask.as_ref().map(|_| [pw, ph]);
                img.mask_dirty = img.mask.is_some();
                img.snip_points = snip_points;
                
                // Set source_rect so capture thread knows what region to capture
                let (wx, wy) = crate::winapi_utils::get_window_screen_pos();
                let window_origin = egui::vec2(wx as f32 / ppp, wy as f32 / ppp);
                let screen_rect = rect.translate(window_origin);
                img.source_rect = Some([screen_rect.min.x, screen_rect.min.y, screen_rect.width(), screen_rect.height()]);
                img.snip_source_overlay = settings.snip_source_overlay;
                
                target_layer.placed_images.push(img);
                target_layer.expanded = true;
                *ctx.request_history_push = Some("Blur".into());
            }
        }
    }
}

pub fn render_preview(ctx: &mut ToolContext) {
    let settings = &ctx.settings;
    let painter = ctx.ui.painter_at(ctx.canvas_response.rect);
    let time = ctx.ui.input(|i| i.time);
    let render_offset = ctx.render_offset;

    if settings.shape_type == ShapeType::Poly {
        if !ctx.current_stroke.is_empty() {
            let mut pts: Vec<egui::Pos2> = ctx.current_stroke.iter().map(|p| *p - render_offset).collect();
            pts.push(ctx.mouse.pos - render_offset);
            if pts.len() >= 2 {
                pts.push(pts[0]);
            }
            crate::utils::draw_dashed_path(&painter, &pts, time);
        }
    } else {
        let start = match ctx.line_start { Some(s) => *s, None => return };
        let pos = ctx.mouse.pos;
        
        match settings.shape_type {
            ShapeType::Circle => {
                let radius = start.distance(pos);
                let center = start - render_offset;
                let mut pts = Vec::with_capacity(61);
                for i in 0..=60 {
                    let angle = i as f32 * std::f32::consts::TAU / 60.0;
                    pts.push(center + egui::vec2(angle.cos() * radius, angle.sin() * radius));
                }
                crate::utils::draw_dashed_path(&painter, &pts, time);
            }
            ShapeType::Star => {
                let radius = start.distance(pos);
                let pts = get_star_points(start - render_offset, radius);
                if pts.len() >= 2 {
                    let mut closed_pts = pts.clone();
                    closed_pts.push(pts[0]);
                    crate::utils::draw_dashed_path(&painter, &closed_pts, time);
                }
            }
            ShapeType::Heart => {
                let radius = start.distance(pos);
                let pts = get_heart_points(start - render_offset, radius);
                if pts.len() >= 2 {
                    let mut closed_pts = pts.clone();
                    closed_pts.push(pts[0]);
                    crate::utils::draw_dashed_path(&painter, &closed_pts, time);
                }
            }
            _ => {
                let rect = egui::Rect::from_two_pos(start, pos).translate(-render_offset);
                let pts = vec![
                    rect.left_top(),
                    rect.right_top(),
                    rect.right_bottom(),
                    rect.left_bottom(),
                    rect.left_top(),
                ];
                crate::utils::draw_dashed_path(&painter, &pts, time);
            }
        }
    }
    ctx.ui.ctx().request_repaint();
}
