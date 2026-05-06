use eframe::egui;
use crate::types::*;
use crate::utils::*;
use crate::overlay::*;

use crate::tools::ToolContext;

pub fn update(ctx: &mut ToolContext) {
    let project = &mut *ctx.project;
    let mouse = ctx.mouse;
    let active_tool = &mut *ctx.active_tool;
    let remove_active_layer = &mut *ctx.remove_active_layer;
    let new_selection = &mut *ctx.new_selection;
    let switch_to_move = &mut *ctx.switch_to_move;
    let line_start = &mut *ctx.line_start;
    let dragging_source_rect = &mut *ctx.dragging_source_rect;
    let initial_layer = &mut *ctx.initial_layer;
    let initial_bounds = &mut *ctx.initial_bounds;
    let initial_center = &mut *ctx.initial_center;
    let drag_state = &mut *ctx.drag_state;
    let pending_text = &mut *ctx.pending_text;
    let ui = &mut *ctx.ui;
    let canvas_response = ctx.canvas_response;
    let painter = ui.painter_at(canvas_response.rect);
    let pos = mouse.pos;
    let left_down = mouse.left_down;
    let left_just_pressed = mouse.left_just_pressed;
    let left_just_released = mouse.left_just_released;
    let render_offset = ctx.render_offset;
    let active_layer_idx = project.active_layer;

                let mut found_handle = false;

                if let Some(sel) = project.selected_object {
                    let layer = &mut project.layers[sel.layer_idx];
                    if let Some(raw_bounds) = object_bounds(layer, sel.object_type, sel.object_idx) {
                        // bounds = translated for drawing; raw_bounds = for hit-testing with pos
                        let bounds = raw_bounds.translate(-render_offset);
                        let draw_corners = [
                            bounds.left_top(), bounds.right_top(),
                            bounds.left_bottom(), bounds.right_bottom(),
                        ];
                        let hit_corners = [
                            raw_bounds.left_top(), raw_bounds.right_top(),
                            raw_bounds.left_bottom(), raw_bounds.right_bottom(),
                        ];

                        #[cfg(feature = "webengine")]
                        if sel.object_type == crate::project::ObjectType::Image {
                            if sel.object_idx < layer.placed_images.len() {
                                let img = &mut layer.placed_images[sel.object_idx];
                                if let Some(widget) = &mut img.web_widget {
                                    let disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
                                    let disp_h = img.display_size.unwrap_or([img.size[1] as f32, img.size[1] as f32])[1];
                                    let img_rect = egui::Rect::from_min_size(img.position, egui::vec2(disp_w, disp_h));
                                    
                                    if img_rect.contains(pos) {
                                        let rel_x = (pos.x - img.position.x) / disp_w * widget.width as f32;
                                        let rel_y = (pos.y - img.position.y) / disp_h * widget.height as f32;
                                        let is_move = !left_just_pressed && !left_just_released;
                                        widget.inject_mouse_event(rel_x, rel_y, mouse.left_down, is_move);
                                    }

                                    // Key events
                                    ui.input(|i| {
                                        for event in &i.events {
                                            match event {
                                                egui::Event::Text(text) => { widget.inject_text_event(text); }
                                                egui::Event::Key { key, pressed, .. } => { widget.inject_raw_key_event(*key, *pressed); }
                                                _ => {}
                                            }
                                        }
                                    });
                                }
                            }
                        }
                
                // --- Highlight Mirror Source Rect ---
                if ui.input(|i| i.pointer.button_double_clicked(egui::PointerButton::Primary)) && raw_bounds.contains(pos) {
                    for img in &mut layer.placed_images {
                        if img.is_live && img.source_rect.is_some() {
                            let disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
                            let disp_h = img.display_size.unwrap_or([img.size[1] as f32, img.size[1] as f32])[1];
                            let img_rect = egui::Rect::from_min_size(img.position, egui::vec2(disp_w, disp_h));
                            if img_rect.contains(pos) {
                                img.show_source_rect = !img.show_source_rect;
                            }
                        }
                    }
                }

                for img in &layer.placed_images {
                    if img.is_live && img.source_rect.is_some() && img.show_source_rect {
                        let src = img.source_rect.unwrap();
                        let src_rect = egui::Rect::from_min_size(egui::pos2(src[0], src[1]), egui::vec2(src[2], src[3]));
                        painter.rect_stroke(src_rect, 0.0, egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 100, 0)), egui::StrokeKind::Middle);
                        painter.text(src_rect.left_top() - egui::vec2(0.0, 10.0), egui::Align2::LEFT_BOTTOM, "Source", egui::FontId::proportional(10.0), egui::Color32::from_rgb(255, 100, 0));
                        
                        // Handles for source rect
                        let s_corners = [src_rect.left_top(), src_rect.right_top(), src_rect.left_bottom(), src_rect.right_bottom()];
                        for (idx, &sc) in s_corners.iter().enumerate() {
                            let handle_rect = egui::Rect::from_center_size(sc, egui::vec2(8.0, 8.0));
                            painter.rect_filled(handle_rect, 0.0, egui::Color32::from_rgb(255, 150, 50));
                            if left_just_pressed && handle_rect.contains(pos) {
                                *line_start = Some(egui::pos2(-4.0, idx as f32)); // Move source rect handle
                                *initial_bounds = Some(src_rect);
                                *dragging_source_rect = true;
                            }
                        }
                        if left_just_pressed && src_rect.contains(pos) && !*dragging_source_rect {
                            *line_start = Some(pos);
                            *initial_bounds = Some(src_rect);
                            *dragging_source_rect = true;
                        }
                    }
                }

                // Draw selection rect
                painter.rect_stroke(bounds, 0.0, egui::Stroke::new(1.0, egui::Color32::WHITE), egui::StrokeKind::Middle);
                
                // --- Transformation Buttons ---
                // Determine what kind of object is selected
                let sel_is_blur = sel.object_type == crate::project::ObjectType::Image
                    && sel.object_idx < layer.placed_images.len()
                    && (layer.placed_images[sel.object_idx].name == "Blur" || layer.placed_images[sel.object_idx].blur > 0.0);
                let sel_is_embed = sel.object_type == crate::project::ObjectType::Image
                    && sel.object_idx < layer.placed_images.len()
                    && layer.placed_images[sel.object_idx].url.is_some();

                // Use two rows: row 1 = URL bar (for embeds), row 2 = buttons
                let bar_height = if sel_is_embed { 52.0 } else { 32.0 };
                let top_btns_rect = egui::Rect::from_min_size(bounds.left_top() - egui::vec2(0.0, bar_height), egui::vec2(bounds.width().max(320.0), bar_height));
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(top_btns_rect), |ui| {
                    ui.visuals_mut().widgets.inactive.bg_fill = egui::Color32::from_black_alpha(150);

                    // Row 1: URL input bar (embed widgets only)
                    if sel_is_embed {
                        if sel.object_idx < layer.placed_images.len() {
                            let img = &mut layer.placed_images[sel.object_idx];
                            if let Some(ref mut url) = img.url {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("🌐").size(12.0));
                                    let edit_resp = ui.add(egui::TextEdit::singleline(url).desired_width(ui.available_width() - 10.0).hint_text("Enter URL..."));
                                    if edit_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                        #[cfg(feature = "webengine")]
                                        if let Some(widget) = &mut img.web_widget {
                                            crate::web_engine::navigate_widget(widget, url);
                                        }
                                    }
                                });
                            }
                        }
                    }

                    // Row 2: Action buttons
                    ui.horizontal(|ui| {
                        // Reset Transform button
                        if ui.add(egui::Button::new(egui::RichText::new("↺").color(egui::Color32::from_rgb(100, 200, 255)))).on_hover_text("Reset Transforms").clicked() {
                            for img in &mut layer.placed_images {
                                img.rotation = 0.0;
                                img.skew = egui::vec2(0.0, 0.0);
                                img.perspective = [egui::Vec2::ZERO; 4];
                                img.flipped_h = false;
                                img.flipped_v = false;
                            }
                            for s in &mut layer.strokes {
                                s.rotation = 0.0;
                                s.skew = egui::vec2(0.0, 0.0);
                            }
                        }
                        ui.separator();

                        // Flip buttons
                        if ui.button("⬌").on_hover_text("Flip Horizontal").clicked() {
                            let center = bounds.center().x;
                            for img in &mut layer.placed_images { img.position.x = center - (img.position.x - center) - (img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0]); img.flipped_h = !img.flipped_h; }
                            for ann in &mut layer.text_annotations { ann.position.x = center - (ann.position.x - center) - (ann.text.len() as f32 * ann.font_size * 0.6); }
                            for s in &mut layer.strokes { for p in &mut s.points { p.x = center - (p.x - center); } }
                        }
                        if ui.button("⬍").on_hover_text("Flip Vertical").clicked() {
                            let center = bounds.center().y;
                            for img in &mut layer.placed_images { img.position.y = center - (img.position.y - center) - (img.display_size.unwrap_or([img.size[1] as f32, img.size[1] as f32])[1]); img.flipped_v = !img.flipped_v; }
                            for ann in &mut layer.text_annotations { ann.position.y = center - (ann.position.y - center) - ann.font_size; }
                            for s in &mut layer.strokes { for p in &mut s.points { p.y = center - (p.y - center); } }
                        }

                        ui.separator();

                        // Opacity
                        let mut op_val = layer.opacity * 100.0;
                        if ui.add(egui::DragValue::new(&mut op_val).range(0.0..=100.0).prefix("Op: ").suffix("%")).changed() {
                            layer.opacity = op_val / 100.0;
                        }

                        // Blur controls — ONLY for blur objects
                        if sel_is_blur {
                            ui.separator();
                            let img = &mut layer.placed_images[sel.object_idx];
                            let mut blur_val = img.blur;
                            if ui.add(egui::DragValue::new(&mut blur_val).range(0.0..=100.0).prefix("B: ")).changed() {
                                img.blur = blur_val;
                            }
                            ui.selectable_value(&mut img.blur_effect, BlurEffect::Gaussian, "Blur");
                            ui.selectable_value(&mut img.blur_effect, BlurEffect::Pixelate, "Pixel");
                            ui.selectable_value(&mut img.blur_effect, BlurEffect::Glitch, "VHS");
                        }

                        // Embed widget buttons — ONLY for embed objects with URLs
                        if sel_is_embed {
                            ui.separator();
                            let img = &mut layer.placed_images[sel.object_idx];
                            if ui.button("📋 Paste").on_hover_text("Paste URL from clipboard").clicked() {
                                if let Some(text) = crate::winapi_utils::get_clipboard_text() {
                                    img.url = Some(text.clone());
                                    #[cfg(feature = "webengine")]
                                    if let Some(widget) = &mut img.web_widget {
                                        crate::web_engine::navigate_widget(widget, &text);
                                    }
                                }
                            }
                            if ui.button("🌐 Web").on_hover_text("Open browser search").clicked() {
                                let text = "https://www.google.com".to_string();
                                img.url = Some(text.clone());
                                #[cfg(feature = "webengine")]
                                if let Some(widget) = &mut img.web_widget {
                                    crate::web_engine::navigate_widget(widget, &text);
                                }
                            }
                            if ui.button("📺 YT").on_hover_text("Open YouTube").clicked() {
                                let text = "https://www.youtube.com".to_string();
                                img.url = Some(text.clone());
                                #[cfg(feature = "webengine")]
                                if let Some(widget) = &mut img.web_widget {
                                    crate::web_engine::navigate_widget(widget, &text);
                                }
                            }
                        }
                        
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add(egui::Button::new(egui::RichText::new("✖").color(egui::Color32::RED))).on_hover_text("Delete Selected").clicked() {
                                if let Some(sel) = project.selected_object {
                                    match sel.object_type {
                                        crate::project::ObjectType::Stroke => { if sel.object_idx < layer.strokes.len() { layer.strokes.remove(sel.object_idx); } }
                                        crate::project::ObjectType::Text => { if sel.object_idx < layer.text_annotations.len() { layer.text_annotations.remove(sel.object_idx); } }
                                        crate::project::ObjectType::Image => { if sel.object_idx < layer.placed_images.len() { layer.placed_images.remove(sel.object_idx); } }
                                    }
                                    project.selected_object = None;
                                }
                            }
                        });
                    });
                });

                // Skew handles (mid-points of sides) — draw at bounds, hit-test at raw_bounds
                let draw_mids = [bounds.left_center(), bounds.right_center(), bounds.center_top(), bounds.center_bottom()];
                let hit_mids = [raw_bounds.left_center(), raw_bounds.right_center(), raw_bounds.center_top(), raw_bounds.center_bottom()];
                for m in draw_mids { painter.circle_filled(m, 3.0, egui::Color32::from_rgb(200, 200, 200)); }

                // Perspective handles (dots outside corners)
                let p_dist = 25.0;
                let draw_p_corners = [
                    bounds.left_top() + egui::vec2(-p_dist, -p_dist),
                    bounds.right_top() + egui::vec2(p_dist, -p_dist),
                    bounds.left_bottom() + egui::vec2(-p_dist, p_dist),
                    bounds.right_bottom() + egui::vec2(p_dist, p_dist),
                ];
                let hit_p_corners = [
                    raw_bounds.left_top() + egui::vec2(-p_dist, -p_dist),
                    raw_bounds.right_top() + egui::vec2(p_dist, -p_dist),
                    raw_bounds.left_bottom() + egui::vec2(-p_dist, p_dist),
                    raw_bounds.right_bottom() + egui::vec2(p_dist, p_dist),
                ];
                for pc in draw_p_corners {
                    painter.circle_filled(pc, 4.0, egui::Color32::from_rgb(100, 200, 255));
                    painter.circle_stroke(pc, 4.0, egui::Stroke::new(1.0, egui::Color32::BLACK));
                }

                // Transformation Handles (Resize)
                for corner in draw_corners {
                    painter.rect_filled(egui::Rect::from_center_size(corner, egui::vec2(8.0, 8.0)), 0.0, egui::Color32::WHITE);
                    painter.rect_stroke(egui::Rect::from_center_size(corner, egui::vec2(8.0, 8.0)), 0.0, egui::Stroke::new(1.0, egui::Color32::BLACK), egui::StrokeKind::Middle);
                }
                
                // Rotation handle: draw at bounds, hit-test at raw_bounds
                let draw_rot = bounds.center_top() - egui::vec2(0.0, 20.0);
                let hit_rot = raw_bounds.center_top() - egui::vec2(0.0, 20.0);
                painter.line_segment([bounds.center_top(), draw_rot], egui::Stroke::new(1.0, egui::Color32::WHITE));
                painter.circle_filled(draw_rot, 4.0, egui::Color32::WHITE);
                painter.circle_stroke(draw_rot, 4.0, egui::Stroke::new(1.0, egui::Color32::BLACK));

                if left_just_pressed && !*dragging_source_rect {
                    let mut hit = false;
                    if hit_rot.distance(pos) < 10.0 {
                        *line_start = Some(pos);
                        *drag_state = 1;
                        *initial_center = Some(raw_bounds.center());
                        *initial_layer = Some(layer.clone());
                        hit = true;
                    }
                    if !hit {
                        for (idx, pc) in hit_p_corners.iter().enumerate() {
                            if pc.distance(pos) < 10.0 {
                                *line_start = Some(pos);
                                *drag_state = 20 + idx;
                                *initial_center = Some(raw_bounds.center());
                                *initial_layer = Some(layer.clone());
                                hit = true; break;
                            }
                        }
                    }
                    if !hit {
                        for (idx, corner) in hit_corners.iter().enumerate() {
                            if egui::Rect::from_center_size(*corner, egui::vec2(12.0, 12.0)).contains(pos) {
                                *line_start = Some(pos);
                                *drag_state = 10 + idx;
                                *initial_bounds = Some(raw_bounds);
                                *initial_center = Some(raw_bounds.center());
                                *initial_layer = Some(layer.clone());
                                hit = true; break;
                            }
                        }
                    }
                    if !hit {
                        for (idx, &mp) in hit_mids.iter().enumerate() {
                            if egui::Rect::from_center_size(mp, egui::vec2(12.0, 12.0)).contains(pos) {
                                *line_start = Some(pos);
                                *drag_state = 2 + idx;
                                *initial_center = Some(raw_bounds.center());
                                *initial_layer = Some(layer.clone());
                                hit = true; break;
                            }
                        }
                    }
                    if !hit && raw_bounds.contains(pos) {
                        *line_start = Some(pos);
                        *drag_state = 0;
                        *initial_bounds = Some(raw_bounds);
                        *initial_layer = Some(layer.clone());
                    }
                    if hit || raw_bounds.contains(pos) { found_handle = true; }
                }
                
                if let Some(start) = *line_start {
                    if left_down {
                        if *dragging_source_rect {
                            // Dragging Mirror Source Rect
                            for img in &mut layer.placed_images {
                                if img.is_live && img.source_rect.is_some() {
                                    if start.x == -4.0 {
                                        // Resize handle
                                        let idx = start.y as usize;
                                        let ib = initial_bounds.unwrap();
                                        let ic = [ib.left_top(), ib.right_top(), ib.left_bottom(), ib.right_bottom()];
                                        let anchor = ic[3 - idx];
                                        let new_rect = egui::Rect::from_two_pos(anchor, pos);
                                        img.source_rect = Some([new_rect.min.x, new_rect.min.y, new_rect.width(), new_rect.height()]);
                                    } else {
                                        // Move whole rect
                                        let delta = pos - start;
                                        let ib = initial_bounds.unwrap();
                                        img.source_rect = Some([ib.min.x + delta.x, ib.min.y + delta.y, ib.width(), ib.height()]);
                                    }
                                }
                            }
                        } else if let Some(base_layer) = initial_layer.as_ref() {
                            // Stable Transformations using initial snapshot
                            *layer = base_layer.clone();
                            let canvas_rect = canvas_response.rect;

                            if *drag_state == 1 {
                                // Rotation
                                let center = initial_center.unwrap();
                                let angle = (pos - center).angle() - (start - center).angle();
                                rotate_layer(layer, center, angle);
                            } else if *drag_state >= 20 {
                                // Perspective
                                let p_idx = *drag_state - 20;
                                let delta = pos - start;
                                for img in &mut layer.placed_images { img.perspective[p_idx] += delta; }
                                for s in &mut layer.strokes { s.perspective[p_idx] += delta; }
                                for ann in &mut layer.text_annotations { ann.perspective[p_idx] += delta; }
                            } else if *drag_state >= 10 {
                                // Resize
                                let handle_idx = *drag_state - 10;
                                let ib = initial_bounds.unwrap();
                                let ic = [ib.left_top(), ib.right_top(), ib.left_bottom(), ib.right_bottom()];
                                let anchor = ic[3 - handle_idx];
                                let old_vec = ic[handle_idx] - anchor;
                                let new_vec = pos - anchor;
                                if old_vec.x.abs() > 1.0 && old_vec.y.abs() > 1.0 {
                                    scale_layer(layer, anchor, egui::vec2(new_vec.x / old_vec.x, new_vec.y / old_vec.y));
                                }
                            } else if *drag_state >= 2 && *drag_state <= 5 {
                                // Skew
                                let handle_idx = *drag_state - 2;
                                let delta = pos - start;
                                let mut skew_delta = egui::vec2(0.0, 0.0);
                                if handle_idx == 0 || handle_idx == 1 { // Left or Right center -> Skew Y
                                    skew_delta.y = -delta.y * 0.01;
                                } else { // Top or Bottom center -> Skew X
                                    skew_delta.x = -delta.x * 0.01;
                                }
                                skew_layer(layer, initial_center.unwrap(), skew_delta);
                            } else {
                                // Translate with clamping
                                let mut delta = pos - start;
                                
                                // Check current bounds and clamp delta to keep within canvas
                                if let Some(cur_bounds) = object_bounds(layer, sel.object_type, sel.object_idx) {
                                    let new_bounds = cur_bounds.translate(delta);
                                    if new_bounds.min.x < canvas_rect.min.x { delta.x += canvas_rect.min.x - new_bounds.min.x; }
                                    if new_bounds.max.x > canvas_rect.max.x { delta.x -= new_bounds.max.x - canvas_rect.max.x; }
                                    if new_bounds.min.y < canvas_rect.min.y { delta.y += canvas_rect.min.y - new_bounds.min.y; }
                                    if new_bounds.max.y > canvas_rect.max.y { delta.y -= new_bounds.max.y - canvas_rect.max.y; }
                                }

                                match sel.object_type {
                                    crate::project::ObjectType::Stroke => {
                                        if let Some(s) = layer.strokes.get_mut(sel.object_idx) {
                                            for p in &mut s.points { *p += delta; }
                                        }
                                    }
                                    crate::project::ObjectType::Text => {
                                        if let Some(t) = layer.text_annotations.get_mut(sel.object_idx) {
                                            t.position += delta;
                                        }
                                    }
                                    crate::project::ObjectType::Image => {
                                        if let Some(img) = layer.placed_images.get_mut(sel.object_idx) {
                                            img.position += delta;
                                        }
                                    }
                                }
                                *line_start = Some(start + delta); // Keep relative pos
                            }
                        }
                    }
                }

                if left_just_pressed && !found_handle && !ui.ctx().is_pointer_over_area() {
                    let is_double_click = ui.input(|i| i.pointer.button_double_clicked(egui::PointerButton::Primary));
                    let mut found_objects = Vec::new();
                    
                    for (l_idx, l) in project.layers.iter().enumerate() {
                        if !l.visible { continue; }
                        
                        // Hit test images
                        for (img_idx, img) in l.placed_images.iter().enumerate() {
                            let disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
                            let disp_h = img.display_size.unwrap_or([img.size[1] as f32, img.size[1] as f32])[1];
                            let img_rect = egui::Rect::from_min_size(img.position, egui::vec2(disp_w, disp_h));
                            if img_rect.contains(pos) {
                                found_objects.push(crate::project::SelectedObject {
                                    layer_idx: l_idx, object_type: crate::project::ObjectType::Image, object_idx: img_idx
                                });
                            }
                        }
                        
                        // Hit test text
                        for (txt_idx, txt) in l.text_annotations.iter().enumerate() {
                            let txt_rect = egui::Rect::from_min_size(txt.position, egui::vec2(txt.text.len() as f32 * txt.font_size * 0.6, txt.font_size));
                            if txt_rect.contains(pos) {
                                found_objects.push(crate::project::SelectedObject {
                                    layer_idx: l_idx, object_type: crate::project::ObjectType::Text, object_idx: txt_idx
                                });
                            }
                        }

                        // Hit test strokes
                        for (s_idx, s) in l.strokes.iter().enumerate() {
                            let hit = if s.points.len() < 2 {
                                s.points.iter().any(|p| p.distance(pos) < s.width + 10.0)
                            } else {
                                let mut hit_line = false;
                                for w in s.points.windows(2) {
                                    let (p0, p1) = (w[0], w[1]);
                                    let len_sq = p0.distance_sq(p1);
                                    if len_sq > 0.0 {
                                        let t = ((pos.x - p0.x) * (p1.x - p0.x) + (pos.y - p0.y) * (p1.y - p0.y)) / len_sq;
                                        let t = t.clamp(0.0, 1.0);
                                        let proj = p0 + (p1 - p0) * t;
                                        if pos.distance(proj) < s.width * 0.5 + 5.0 {
                                            hit_line = true; break;
                                        }
                                    }
                                }
                                hit_line
                            };
                            if hit {
                                found_objects.push(crate::project::SelectedObject {
                                    layer_idx: l_idx, object_type: crate::project::ObjectType::Stroke, object_idx: s_idx
                                });
                            }
                        }
                    }

                    if found_objects.is_empty() {
                        project.selected_object = None;
                    } else {
                        // Sort by layer index (highest first) to match visual order
                        found_objects.sort_by(|a, b| b.layer_idx.cmp(&a.layer_idx));

                        if is_double_click {
                            // Cycle through objects under cursor
                            let current = project.selected_object;
                            let mut next_idx = 0;
                            if let Some(curr) = current {
                                if let Some(i) = found_objects.iter().position(|o| o.layer_idx == curr.layer_idx && o.object_type == curr.object_type && o.object_idx == curr.object_idx) {
                                    // If we double-click the ALREADY selected text, edit it!
                                    if curr.object_type == crate::project::ObjectType::Text {
                                        let layer = &mut project.layers[curr.layer_idx];
                                        let txt = layer.text_annotations.remove(curr.object_idx);
                                        *pending_text = Some(crate::types::PendingText {
                                            position: txt.position,
                                            buffer: txt.text.clone(),
                                        });
                                        *active_tool = crate::overlay::Tool::Text;
                                        project.selected_object = None;
                                        return;
                                    }
                                    next_idx = (i + 1) % found_objects.len();
                                }
                            }
                            let next = found_objects[next_idx];
                            project.active_layer = next.layer_idx;
                            project.selected_object = Some(next);
                        } else {
                            // Single click: select topmost
                            let top = found_objects[0];
                            project.active_layer = top.layer_idx;
                            project.selected_object = Some(top);
                        }
                    }
                }

                if left_just_released {
                    *line_start = None;
                    *initial_layer = None;
                    *dragging_source_rect = false;
                    if active_layer_idx < project.layers.len() {
                        let layer = &project.layers[active_layer_idx];
                        if layer.strokes.is_empty() && layer.text_annotations.is_empty() && layer.placed_images.is_empty() {
                            if project.layers.len() > 1 { *remove_active_layer = true; }
                        }
                    }
                }
            }
        }
}

pub fn render(ctx: &mut ToolContext) {
    let project = &ctx.project;
    let ui = &ctx.ui;
    let painter = ui.painter_at(ctx.canvas_response.rect);
    let render_offset = ctx.render_offset;
    
    if let Some(sel) = project.selected_object {
        if sel.layer_idx < project.layers.len() {
            let layer = &project.layers[sel.layer_idx];
            if let Some(bounds) = layer_bounds(layer) {
                let bounds = bounds.translate(-render_offset);
                painter.rect_stroke(bounds.expand(4.0), 0.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 180, 255)), egui::StrokeKind::Middle);
                let handle_rect = egui::Rect::from_center_size(bounds.max + egui::vec2(4.0, 4.0), egui::vec2(8.0, 8.0));
                painter.rect_filled(handle_rect, 0.0, egui::Color32::WHITE);
                painter.rect_stroke(handle_rect, 0.0, egui::Stroke::new(1.0, egui::Color32::BLACK), egui::StrokeKind::Middle);
            }
        }
    }
}


