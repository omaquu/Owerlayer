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
    let snip_created = &mut *ctx.snip_created;
    let ui = &mut *ctx.ui;
    let canvas_response = ctx.canvas_response;
    let painter = ui.painter_at(canvas_response.rect);
    let pos = mouse.pos;
    let left_down = mouse.left_down;
    let left_just_pressed = mouse.left_just_pressed;
    let left_just_released = mouse.left_just_released;
    let active_layer_idx = project.active_layer;
    let _ppp = ui.ctx().pixels_per_point();
    let render_offset = ctx.render_offset;
    let _frame_count = ctx.frame_count;

                let layer = &mut project.layers[active_layer_idx];
                let mode = settings.snip_mode;
            if mode == SnipMode::Rect {
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
                        let w = rect.width();
                        let h = rect.height();
                        if w > 5.0 && h > 5.0 {
                            let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
                            if settings.snip_live {
                                *snip_created = true;
                                let mut img = PlacedImage::new(id, rect.min, [w.round() as usize, h.round() as usize], Vec::new());
                                img.is_live = true;
                                img.source_rect = Some([rect.min.x, rect.min.y, w, h]);
                                img.blur = settings.blur_strength;
                                img.blur_effect = settings.blur_effect;
                                img.show_source_rect = true;
                                layer.placed_images.push(img);
                            } else {
                                let ppp = ui.ctx().pixels_per_point();
                                let (wx, wy) = crate::winapi_utils::get_window_screen_pos();
                                let sx = (rect.min.x * ppp) as i32 + if settings.use_absolute_screen_coords { 0 } else { wx };
                                let sy = (rect.min.y * ppp) as i32 + if settings.use_absolute_screen_coords { 0 } else { wy };
                                if let Some(pixels) = crate::winapi_utils::capture_screen_rect(sx, sy, (w * ppp) as i32, (h * ppp) as i32) {
                                    *snip_created = true;
                                    let mut img = PlacedImage::new(id, rect.min, [w.round() as usize, h.round() as usize], pixels);
                                    img.shadow = settings.snip_shadow;
                                    layer.placed_images.push(img);
                                }
                            }
                        }
                    }
                }
            } else if mode == SnipMode::Circle {
                if left_just_pressed { 
                    *line_start = Some(pos); 
                }
                if left_just_released {
                    if let Some(start) = line_start.take() {
                        let rect = egui::Rect::from_two_pos(start, pos);
                        let w = rect.width();
                        let h = rect.height();
                        if w > 5.0 && h > 5.0 {
                            let ppp = ui.ctx().pixels_per_point();
                            let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
                            
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

                            if settings.snip_live {
                                *snip_created = true;
                                let mut img = PlacedImage::new(id, rect.min, [w.round() as usize, h.round() as usize], Vec::new());
                                img.is_live = true;
                                img.source_rect = Some([rect.min.x, rect.min.y, w, h]);
                                img.mask = Some(mask);
                                img.blur = settings.blur_strength;
                                img.blur_effect = settings.blur_effect;
                                img.show_source_rect = true;
                                layer.placed_images.push(img);
                            } else {
                                let (wx, wy) = crate::winapi_utils::get_window_screen_pos();
                                let sx = (rect.min.x * ppp) as i32 + if settings.use_absolute_screen_coords { 0 } else { wx };
                                let sy = (rect.min.y * ppp) as i32 + if settings.use_absolute_screen_coords { 0 } else { wy };
                                if let Some(mut pixels) = crate::winapi_utils::capture_screen_rect(sx, sy, (w * ppp) as i32, (h * ppp) as i32) {
                                    *snip_created = true;
                                    for (i, &m) in mask.iter().enumerate() { if m == 0 { pixels[i*4+3] = 0; } }
                                    let mut img = PlacedImage::new(id, rect.min, [w.round() as usize, h.round() as usize], pixels);
                                    img.shadow = settings.snip_shadow;
                                    layer.placed_images.push(img);
                                }
                            }
                        }
                    }
                }
            } else if mode == SnipMode::Lasso {
                if left_just_pressed { }
                if left_down { current_stroke.push(pos); }
                if !current_stroke.is_empty() {
                    painter.add(egui::Shape::line(current_stroke.clone(), egui::Stroke::new(1.0, egui::Color32::WHITE)));
                }
                if left_just_released && current_stroke.len() > 3 {
                    let bounds = egui::Rect::from_points(&current_stroke);
                    let ppp = ui.ctx().pixels_per_point();
                    let sw = (bounds.width() * ppp) as i32;
                    let sh = (bounds.height() * ppp) as i32;
                    
                    if sw > 5 && sh > 5 {
                        let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
                        let poly: Vec<egui::Pos2> = current_stroke.iter().map(|p| egui::pos2(p.x - bounds.min.x, p.y - bounds.min.y)).collect();
                        let mut mask = vec![255u8; sw as usize * sh as usize];
                        for py in 0..sh as usize {
                            for px in 0..sw as usize {
                                let lp = egui::pos2(px as f32 / ppp, py as f32 / ppp);
                                if !is_inside_poly(&poly, lp) {
                                    mask[py * sw as usize + px] = 0;
                                }
                            }
                        }

                        if settings.snip_live {
                            *snip_created = true;
                            let mut img = PlacedImage::new(id, bounds.min, [bounds.width().round() as usize, bounds.height().round() as usize], Vec::new());
                            img.is_live = true;
                            img.source_rect = Some([bounds.min.x, bounds.min.y, bounds.width(), bounds.height()]);
                            img.mask = Some(mask);
                            img.blur = settings.blur_strength;
                            img.blur_effect = settings.blur_effect;
                            layer.placed_images.push(img);
                        } else {
                            let (wx, wy) = crate::winapi_utils::get_window_screen_pos();
                            let sx = (bounds.min.x * ppp) as i32 + if settings.use_absolute_screen_coords { 0 } else { wx };
                            let sy = (bounds.min.y * ppp) as i32 + if settings.use_absolute_screen_coords { 0 } else { wy };
                            if let Some(mut pixels) = crate::winapi_utils::capture_screen_rect(sx, sy, sw, sh) {
                                *snip_created = true;
                                for (i, &m) in mask.iter().enumerate() { if m == 0 { pixels[i*4+3] = 0; } }
                                let mut img = PlacedImage::new(id, bounds.min, [bounds.width().round() as usize, bounds.height().round() as usize], pixels);
                                img.shadow = settings.snip_shadow;
                                layer.placed_images.push(img);
                            }
                        }
                    }
                    current_stroke.clear();
                }
            } else if mode == SnipMode::Polygon {
                if left_just_pressed { 
                    current_stroke.push(pos); 
                }
                let right_clicked = ui.input(|i| i.pointer.secondary_pressed());
                let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                let close_to_start = current_stroke.len() > 2 && pos.distance(current_stroke[0]) < 15.0 && left_just_pressed;
                
                if (right_clicked || enter_pressed || close_to_start) && !current_stroke.is_empty() {
                    let bounds = egui::Rect::from_points(&current_stroke);
                    let ppp = ui.ctx().pixels_per_point();
                    let sw = (bounds.width() * ppp) as i32;
                    let sh = (bounds.height() * ppp) as i32;
                    
                    if sw > 5 && sh > 5 {
                        let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
                        let poly: Vec<egui::Pos2> = current_stroke.iter().map(|p| egui::pos2(p.x - bounds.min.x, p.y - bounds.min.y)).collect();
                        let mut mask = vec![255u8; sw as usize * sh as usize];
                        for py in 0..sh as usize {
                            for px in 0..sw as usize {
                                let lp = egui::pos2(px as f32 / ppp, py as f32 / ppp);
                                if !is_inside_poly(&poly, lp) {
                                    mask[py * sw as usize + px] = 0;
                                }
                            }
                        }

                        if settings.snip_live {
                            *snip_created = true;
                            let mut img = PlacedImage::new(id, bounds.min, [bounds.width().round() as usize, bounds.height().round() as usize], Vec::new());
                            img.is_live = true;
                            img.source_rect = Some([bounds.min.x, bounds.min.y, bounds.width(), bounds.height()]);
                            img.mask = Some(mask);
                            img.blur = settings.blur_strength;
                            img.blur_effect = settings.blur_effect;
                            img.show_source_rect = true;
                            layer.placed_images.push(img);
                        } else {
                            let (wx, wy) = crate::winapi_utils::get_window_screen_pos();
                            let sx = (bounds.min.x * ppp) as i32 + if settings.use_absolute_screen_coords { 0 } else { wx };
                            let sy = (bounds.min.y * ppp) as i32 + if settings.use_absolute_screen_coords { 0 } else { wy };
                            if let Some(mut pixels) = crate::winapi_utils::capture_screen_rect(sx, sy, sw, sh) {
                                *snip_created = true;
                                for (i, &m) in mask.iter().enumerate() { if m == 0 { pixels[i*4+3] = 0; } }
                                let mut img = PlacedImage::new(id, bounds.min, [bounds.width().round() as usize, bounds.height().round() as usize], pixels);
                                img.shadow = settings.snip_shadow;
                                layer.placed_images.push(img);
                            }
                        }
                    }
                    current_stroke.clear();
                }
                if !current_stroke.is_empty() {
                    let mut pts = current_stroke.clone();
                    pts.push(pos);
                    painter.add(egui::Shape::line(pts, egui::Stroke::new(1.0, egui::Color32::WHITE)));
                }
            } else if mode == SnipMode::RegularPolygon {
                if left_just_pressed { *line_start = Some(pos); }
                if let Some(start) = *line_start {
                    let radius = start.distance(pos);
                    let n = settings.polygon_sides.max(3) as usize;
                    let pts: Vec<egui::Pos2> = (0..=n).map(|i| {
                        let angle = i as f32 * std::f32::consts::PI * 2.0 / n as f32 - std::f32::consts::PI / 2.0;
                        start + egui::vec2(angle.cos() * radius, angle.sin() * radius)
                    }).collect();
                    painter.add(egui::Shape::line(pts, egui::Stroke::new(1.0, egui::Color32::WHITE)));
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
                            let bounds = egui::Rect::from_points(&pts);
                            let ppp = ui.ctx().pixels_per_point();
                            let sw = (bounds.width() * ppp) as i32;
                            let sh = (bounds.height() * ppp) as i32;
                            if sw > 5 && sh > 5 {
                                let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
                                let poly: Vec<egui::Pos2> = pts.iter().map(|p| egui::pos2(p.x - bounds.min.x, p.y - bounds.min.y)).collect();
                                let mut mask = vec![255u8; sw as usize * sh as usize];
                                for py in 0..sh as usize {
                                    for px in 0..sw as usize {
                                        let lp = egui::pos2(px as f32 / ppp, py as f32 / ppp);
                                        if !is_inside_poly(&poly, lp) { mask[py * sw as usize + px] = 0; }
                                    }
                                }
                                if settings.snip_live {
                                    *snip_created = true;
                                    let mut img = PlacedImage::new(id, bounds.min, [bounds.width().round() as usize, bounds.height().round() as usize], Vec::new());
                                    img.is_live = true;
                                    img.source_rect = Some([bounds.min.x, bounds.min.y, bounds.width(), bounds.height()]);
                                    img.mask = Some(mask);
                                    img.blur = settings.blur_strength;
                                    img.blur_effect = settings.blur_effect;
                                    img.show_source_rect = true;
                                    layer.placed_images.push(img);
                                } else {
                                    let (wx, wy) = crate::winapi_utils::get_window_screen_pos();
                                    let sx = (bounds.min.x * ppp) as i32 + if settings.use_absolute_screen_coords { 0 } else { wx };
                                    let sy = (bounds.min.y * ppp) as i32 + if settings.use_absolute_screen_coords { 0 } else { wy };
                                    if let Some(mut pixels) = crate::winapi_utils::capture_screen_rect(sx, sy, sw, sh) {
                                        *snip_created = true;
                                        for (i, &m) in mask.iter().enumerate() { if m == 0 { pixels[i*4+3] = 0; } }
                                        let mut img = PlacedImage::new(id, bounds.min, [bounds.width().round() as usize, bounds.height().round() as usize], pixels);
                                        img.shadow = settings.snip_shadow;
                                        layer.placed_images.push(img);
                                    }
                                }
                            }
                        }
                    }
                }
            } else if mode == SnipMode::Star || mode == SnipMode::Heart {

                if left_just_pressed { 
                    *line_start = Some(pos); 
                }
                if let Some(start) = *line_start {
                    let radius = start.distance(pos);
                    let pts = if mode == SnipMode::Star { get_star_points(start, radius) } else { get_heart_points(start, radius) };
                    painter.add(egui::Shape::line(pts.clone(), egui::Stroke::new(1.0, egui::Color32::WHITE)));
                }
                if left_just_released {
                    if let Some(start) = line_start.take() {
                        let radius = start.distance(pos);
                        if radius > 5.0 {
                            let pts = if mode == SnipMode::Star { get_star_points(start, radius) } else { get_heart_points(start, radius) };
                            let bounds = egui::Rect::from_points(&pts);
                            let ppp = ui.ctx().pixels_per_point();
                            let sw = (bounds.width() * ppp) as i32;
                            let sh = (bounds.height() * ppp) as i32;
                            
                            if sw > 5 && sh > 5 {
                                let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
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
                                
                                if settings.snip_live {
                                    *snip_created = true;
                                    let mut img = PlacedImage::new(id, bounds.min, [bounds.width().round() as usize, bounds.height().round() as usize], Vec::new());
                                    img.is_live = true;
                                    img.source_rect = Some([bounds.min.x, bounds.min.y, bounds.width(), bounds.height()]);
                                    img.mask = Some(mask);
                                    layer.placed_images.push(img);
                                } else {
                                    let (wx, wy) = crate::winapi_utils::get_window_screen_pos();
                                    let sx = (bounds.min.x * ppp) as i32 + if settings.use_absolute_screen_coords { 0 } else { wx };
                                    let sy = (bounds.min.y * ppp) as i32 + if settings.use_absolute_screen_coords { 0 } else { wy };
                                    if let Some(mut pixels) = crate::winapi_utils::capture_screen_rect(sx, sy, sw, sh) {
                                        *snip_created = true;
                                        for (i, &m) in mask.iter().enumerate() { if m == 0 { pixels[i*4+3] = 0; } }
                                        let mut img = PlacedImage::new(id, bounds.min, [bounds.width().round() as usize, bounds.height().round() as usize], pixels);
                                        img.shadow = settings.snip_shadow;
                                        layer.placed_images.push(img);
                                    }
                                }
                            }
                        }
                    }
                }
            } else if mode == SnipMode::Window {
                if left_just_pressed { 
                    *line_start = Some(pos); 
                }
                if let Some(start) = *line_start {
                    let rect = egui::Rect::from_two_pos(start, pos).translate(-render_offset);
                    painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, egui::Color32::WHITE), egui::StrokeKind::Middle);
                }
                if left_just_released {
                    if let Some(start) = line_start.take() {
                        let rect = egui::Rect::from_two_pos(start, pos);
                        let w = rect.width();
                        let h = rect.height();
                        if w > 5.0 && h > 5.0 {
                            let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
                            *snip_created = true;
                            let mut img = PlacedImage::new(id, rect.min, [w.round() as usize, h.round() as usize], Vec::new());
                            img.is_live = true;
                            img.source_rect = Some([rect.min.x, rect.min.y, w, h]);
                            img.show_source_rect = true;
                            layer.placed_images.push(img);
                        }
                    }
                }
            }

            if *snip_created {
                project.layers.push(crate::project::Layer::new(&format!("Snip {}", project.layers.len() + 1)));
                project.active_layer = project.layers.len() - 1;
                let prev_idx = project.active_layer - 1;
                if let Some(snip) = project.layers[prev_idx].placed_images.pop() {
                    let snip_clone = snip.clone();
                    std::thread::spawn(move || {
                        if let Some(mut pics) = directories::UserDirs::new().and_then(|d| d.picture_dir().map(|p| p.to_path_buf())) {
                            pics.push("Owerlayer");
                            pics.push("Snips");
                            let _ = std::fs::create_dir_all(&pics);
                            let time_str = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                            let path = pics.join(format!("snip_{}.png", time_str));
                            if !snip_clone.pixels.is_empty() {
                                let mut img_buf = image::RgbaImage::new(snip_clone.size[0] as u32, snip_clone.size[1] as u32);
                                img_buf.copy_from_slice(&snip_clone.pixels);
                                let _ = img_buf.save(path);
                            }
                        }
                    });
                    project.layers.last_mut().unwrap().placed_images.push(snip);
                }
                *snip_created = false;
                *ctx.request_history_push = Some("Snip".into());
            }
}

pub fn render_preview(ctx: &mut ToolContext) {
    let start = match ctx.line_start { Some(s) => *s, None => return };
    let pos = ctx.mouse.pos;
    let render_offset = ctx.render_offset;
    let painter = ctx.ui.painter_at(ctx.canvas_response.rect);
    
    let rect = egui::Rect::from_two_pos(start, pos).translate(-render_offset);
    painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, egui::Color32::WHITE), egui::StrokeKind::Middle);
    painter.rect_filled(rect, 0.0, egui::Color32::from_black_alpha(40));
}

