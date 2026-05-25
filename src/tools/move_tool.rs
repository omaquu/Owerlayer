use eframe::egui;
use crate::utils::*;
use crate::overlay::*;

use crate::tools::ToolContext;

pub fn update(ctx: &mut ToolContext) {
    let project = &mut *ctx.project;
    let settings = &mut *ctx.settings;
    let mouse = ctx.mouse;
    let active_tool = &mut *ctx.active_tool;
    let remove_active_layer = &mut *ctx.remove_active_layer;
    let _new_selection = &mut *ctx.new_selection;
    let _switch_to_move = &mut *ctx.switch_to_move;
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

                let mut click_consumed = false;

                let is_layer_mode = project.selected_object.is_none();
                let layer_idx = project.selected_object.map(|s| s.layer_idx).unwrap_or(project.active_layer);
                
                if layer_idx < project.layers.len() {
                    let layer = &mut project.layers[layer_idx];
                    let raw_bounds_opt = if let Some(sel) = project.selected_object {
                        crate::utils::object_bounds(layer, sel.object_type, sel.object_idx)
                    } else {
                        crate::utils::layer_bounds(layer)
                    };
                    
                    if let Some(raw_bounds) = raw_bounds_opt {
                        let bounds = raw_bounds.translate(-render_offset);
                        let _draw_corners = [
                            bounds.left_top(), bounds.right_top(),
                            bounds.left_bottom(), bounds.right_bottom(),
                        ];
                        let hit_corners = [
                            bounds.left_top(), bounds.right_top(),
                            bounds.left_bottom(), bounds.right_bottom(),
                        ];

                        #[cfg(feature = "webengine")]
                        if let Some(sel) = project.selected_object {
                            if sel.object_type == ObjectType::Image {
                            if sel.object_idx < layer.placed_images.len() {
                                let img = &mut layer.placed_images[sel.object_idx];
                                if let Some(widget_arc) = &img.web_widget {
                                    if let Ok(mut widget) = widget_arc.lock() {
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
                        }
                        }
                
                // --- Highlight Mirror Source Rect ---
                for (img_idx, img) in layer.placed_images.iter_mut().enumerate() {
                    if img.source_rect.is_some() {
                        let is_selected = project.selected_object == Some(SelectedObject { layer_idx, object_type: ObjectType::Image, object_idx: img_idx });
                        
                        if is_selected && img.show_source_rect {
                            let src = img.source_rect.unwrap();
                            let src_rect = egui::Rect::from_min_size(egui::pos2(src[0], src[1]), egui::vec2(src[2], src[3]));
                            
                            if let Some(ref local_pts) = img.snip_points {
                                let world_pts: Vec<egui::Pos2> = local_pts.iter().map(|p| egui::pos2(src_rect.min.x + p.x, src_rect.min.y + p.y)).collect();
                                painter.add(egui::Shape::line(world_pts, egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 100, 0))));
                            } else {
                                painter.rect_stroke(src_rect, 0.0, egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 100, 0)), egui::StrokeKind::Middle);
                            }
                            
                            painter.text(src_rect.left_top() - egui::vec2(0.0, 10.0), egui::Align2::LEFT_BOTTOM, "Source", egui::FontId::proportional(10.0), egui::Color32::from_rgb(255, 100, 0));
                            
                            // Handles for source rect
                            let s_corners = [src_rect.left_top(), src_rect.right_top(), src_rect.left_bottom(), src_rect.right_bottom()];
                            for (idx, &sc) in s_corners.iter().enumerate() {
                                let handle_rect = egui::Rect::from_center_size(sc, egui::vec2(12.0, 12.0));
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
                }


                // Draw selection rect — amber border for locked objects
                let obj_is_locked = project.selected_object.map_or(false, |sel| match sel.object_type {
                    ObjectType::Image => layer.placed_images.get(sel.object_idx).map_or(false, |o| o.locked),
                    ObjectType::Stroke => layer.strokes.get(sel.object_idx).map_or(false, |o| o.locked),
                    ObjectType::Text => layer.text_annotations.get(sel.object_idx).map_or(false, |o| o.locked),
                });
                let sel_rect_color = if obj_is_locked {
                    egui::Color32::from_rgb(255, 180, 0)
                } else {
                    egui::Color32::WHITE
                };
                painter.rect_stroke(bounds, 0.0, egui::Stroke::new(1.0, sel_rect_color), egui::StrokeKind::Middle);
                if obj_is_locked {
                    painter.text(
                        bounds.center_top() - egui::vec2(0.0, 12.0),
                        egui::Align2::CENTER_BOTTOM,
                        "🔒 Locked",
                        egui::FontId::proportional(11.0),
                        egui::Color32::from_rgb(255, 180, 0),
                    );
                }
                

                // --- Transformation Buttons ---
                let mut _sel_is_blur = false;
                let mut sel_is_embed = false;
                if let Some(sel) = project.selected_object {
                    _sel_is_blur = sel.object_type == ObjectType::Image
                        && sel.object_idx < layer.placed_images.len()
                        && (layer.placed_images[sel.object_idx].name == "Blur" || layer.placed_images[sel.object_idx].blur > 0.0);
                    sel_is_embed = sel.object_type == ObjectType::Image
                        && sel.object_idx < layer.placed_images.len()
                        && layer.placed_images[sel.object_idx].url.is_some();
                }

                // Use two rows: row 1 = URL bar (for embeds), row 2 = buttons
                let bar_height = if sel_is_embed { 52.0 } else { 32.0 };
                let top_btns_rect = egui::Rect::from_min_size(bounds.left_top() - egui::vec2(0.0, bar_height + 40.0), egui::vec2(bounds.width().max(320.0), bar_height));
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(top_btns_rect), |ui| {
                    ui.visuals_mut().widgets.inactive.bg_fill = egui::Color32::from_black_alpha(150);

                    // Row 1: URL input bar (embed widgets only)
                    if sel_is_embed {
                        if let Some(sel) = project.selected_object {
                            if sel.object_idx < layer.placed_images.len() {
                                let img = &mut layer.placed_images[sel.object_idx];
                                if let Some(ref mut url) = img.url {
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new("🌐").size(12.0));
                                        let edit_resp = ui.add(egui::TextEdit::singleline(url).desired_width(ui.available_width() - 10.0).hint_text("Enter URL..."));
                                        if edit_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                            #[cfg(feature = "webengine")]
                                            if let Some(widget_arc) = &img.web_widget {
                                                if let Ok(mut widget) = widget_arc.lock() {
                                                    crate::web_engine::navigate_widget(&mut widget, url);
                                                }
                                            }
                                        }
                                    });
                                }
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
                        if ui.button("⬌").on_hover_text(if project.selected_object.is_some() { "Flip Horizontal" } else { "Flip Layer Horizontal" }).clicked() {
                            if let Some(sel) = project.selected_object {
                                match sel.object_type {
                                    ObjectType::Image => { layer.placed_images[sel.object_idx].flipped_h = !layer.placed_images[sel.object_idx].flipped_h; }
                                    ObjectType::Stroke => { layer.strokes[sel.object_idx].flipped_h = !layer.strokes[sel.object_idx].flipped_h; }
                                    ObjectType::Text => { layer.text_annotations[sel.object_idx].flipped_h = !layer.text_annotations[sel.object_idx].flipped_h; }
                                }
                            } else {
                                for img in &mut layer.placed_images { img.flipped_h = !img.flipped_h; }
                                for s in &mut layer.strokes { s.flipped_h = !s.flipped_h; }
                                for ann in &mut layer.text_annotations { ann.flipped_h = !ann.flipped_h; }
                            }
                        }
                        if ui.button("⬍").on_hover_text(if project.selected_object.is_some() { "Flip Vertical" } else { "Flip Layer Vertical" }).clicked() {
                            if let Some(sel) = project.selected_object {
                                match sel.object_type {
                                    ObjectType::Image => { layer.placed_images[sel.object_idx].flipped_v = !layer.placed_images[sel.object_idx].flipped_v; }
                                    ObjectType::Stroke => { layer.strokes[sel.object_idx].flipped_v = !layer.strokes[sel.object_idx].flipped_v; }
                                    ObjectType::Text => { layer.text_annotations[sel.object_idx].flipped_v = !layer.text_annotations[sel.object_idx].flipped_v; }
                                }
                            } else {
                                for img in &mut layer.placed_images { img.flipped_v = !img.flipped_v; }
                                for s in &mut layer.strokes { s.flipped_v = !s.flipped_v; }
                                for ann in &mut layer.text_annotations { ann.flipped_v = !ann.flipped_v; }
                            }
                        }

                        ui.separator();

                        // Opacity
                        let mut op_val = layer.opacity * 100.0;
                        if ui.add(egui::DragValue::new(&mut op_val).range(0.0..=100.0).prefix("Op: ").suffix("%")).changed() {
                            layer.opacity = op_val / 100.0;
                        }

                        // HUD Blur slider
                        let mut bl_val = if let Some(sel) = project.selected_object {
                            match sel.object_type {
                                ObjectType::Image => layer.placed_images[sel.object_idx].blur,
                                ObjectType::Stroke => layer.strokes[sel.object_idx].blur,
                                ObjectType::Text => layer.text_annotations[sel.object_idx].blur,
                            }
                        } else {
                            layer.blur
                        };
                        ui.separator();
                        if ui.add(egui::DragValue::new(&mut bl_val).range(0.0..=300.0).prefix("Blur: ")).changed() {
                            if let Some(sel) = project.selected_object {
                                match sel.object_type {
                                    ObjectType::Image => layer.placed_images[sel.object_idx].blur = bl_val,
                                    ObjectType::Stroke => layer.strokes[sel.object_idx].blur = bl_val,
                                    ObjectType::Text => layer.text_annotations[sel.object_idx].blur = bl_val,
                                }
                            } else {
                                layer.blur = bl_val;
                            }
                        }

                        ui.separator();
                        
                        // Embed widget buttons — ONLY for embed objects with URLs
                        if sel_is_embed {
                            if let Some(sel) = project.selected_object {
                                ui.separator();
                                ui.horizontal(|ui| {
                                    let img = &mut layer.placed_images[sel.object_idx];
                                    if ui.button("📋 Paste").on_hover_text("Paste URL from clipboard").clicked() {
                                        if let Some(text) = crate::winapi_utils::get_clipboard_text() {
                                            img.url = Some(text.clone());
                                            #[cfg(feature = "webengine")]
                                            if let Some(widget_arc) = &img.web_widget {
                                                if let Ok(mut widget) = widget_arc.lock() {
                                                    crate::web_engine::navigate_widget(&mut widget, &text);
                                                }
                                            }
                                        }
                                    }
                                    if ui.button("🌐 Web").on_hover_text("Open browser search").clicked() {
                                        let text = "https://www.google.com".to_string();
                                        img.url = Some(text.clone());
                                        #[cfg(feature = "webengine")]
                                        if let Some(widget_arc) = &img.web_widget {
                                            if let Ok(mut widget) = widget_arc.lock() {
                                                crate::web_engine::navigate_widget(&mut widget, &text);
                                            }
                                        }
                                    }
                                    if ui.button("📺 YT").on_hover_text("Open YouTube").clicked() {
                                        let text = "https://www.youtube.com".to_string();
                                        img.url = Some(text.clone());
                                        #[cfg(feature = "webengine")]
                                        if let Some(widget_arc) = &img.web_widget {
                                            if let Ok(mut widget) = widget_arc.lock() {
                                                crate::web_engine::navigate_widget(&mut widget, &text);
                                            }
                                        }
                                    }
                                });
                            }
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let hover_text = if is_layer_mode { "Delete Layer" } else { "Delete Selected" };
                            if ui.add(egui::Button::new(egui::RichText::new("✖").color(egui::Color32::RED)).min_size(egui::vec2(24.0, 24.0))).on_hover_text(hover_text).clicked() {
                                if is_layer_mode {
                                    if settings.prompt_delete_layer {
                                        ui.ctx().memory_mut(|m| m.data.insert_temp(egui::Id::new("layer_to_delete"), layer_idx));
                                    } else {
                                        *remove_active_layer = true;
                                    }
                                } else if let Some(sel) = project.selected_object {
                                    match sel.object_type {
                                        ObjectType::Stroke => { if sel.object_idx < layer.strokes.len() { layer.strokes.remove(sel.object_idx); } }
                                        ObjectType::Text => { if sel.object_idx < layer.text_annotations.len() { layer.text_annotations.remove(sel.object_idx); } }
                                        ObjectType::Image => { if sel.object_idx < layer.placed_images.len() { layer.placed_images.remove(sel.object_idx); } }
                                    }
                                    project.selected_object = None;
                                }
                            }
                        });
                    });
                });

                // Skew handles (hit-test only, visuals drawn in render())
                let _draw_mids = [bounds.left_center(), bounds.right_center(), bounds.center_top(), bounds.center_bottom()];
                let hit_mids = [bounds.left_center(), bounds.right_center(), bounds.center_top(), bounds.center_bottom()];

                // Perspective handles (hit-test only, visuals drawn in render())
                let p_dist = 25.0;
                let hit_p_corners = [
                    bounds.left_top() + egui::vec2(-p_dist, -p_dist),
                    bounds.right_top() + egui::vec2(p_dist, -p_dist),
                    bounds.left_bottom() + egui::vec2(-p_dist, p_dist),
                    bounds.right_bottom() + egui::vec2(p_dist, p_dist),
                ];
                
                // Rotation handles
                let hit_rot_top = bounds.center_top() - egui::vec2(0.0, 25.0);
                let hit_rot_bot = bounds.center_bottom() + egui::vec2(0.0, 25.0);
                
                // Removed duplicate red garbage bin delete button logic

                if top_btns_rect.contains(pos) { return; }

                if left_just_pressed && !*dragging_source_rect {
                    // Locked objects: show selection box but block all transforms
                    let sel_is_locked = project.selected_object.map_or(false, |sel| match sel.object_type {
                        ObjectType::Image => layer.placed_images.get(sel.object_idx).map_or(false, |o| o.locked),
                        ObjectType::Stroke => layer.strokes.get(sel.object_idx).map_or(false, |o| o.locked),
                        ObjectType::Text => layer.text_annotations.get(sel.object_idx).map_or(false, |o| o.locked),
                    });

                    let mut hit = false;
                    if !sel_is_locked && (hit_rot_top.distance(pos) < 20.0 || hit_rot_bot.distance(pos) < 20.0) {
                        *line_start = Some(pos);
                        *drag_state = 1;
                        *initial_center = Some(raw_bounds.center());
                        *initial_layer = Some(layer.clone());
                        hit = true;
                    }
                    if !hit && !sel_is_locked {
                        for (idx, pc) in hit_p_corners.iter().enumerate() {
                            if pc.distance(pos) < 16.0 {
                                *line_start = Some(pos);
                                *drag_state = 20 + idx;
                                *initial_center = Some(raw_bounds.center());
                                *initial_layer = Some(layer.clone());
                                hit = true; break;
                            }
                        }
                    }
                    if !hit && !sel_is_locked {
                        for (idx, corner) in hit_corners.iter().enumerate() {
                            if egui::Rect::from_center_size(*corner, egui::vec2(20.0, 20.0)).contains(pos) {
                                *line_start = Some(pos);
                                *drag_state = 10 + idx;
                                *initial_bounds = Some(raw_bounds);
                                *initial_center = Some(raw_bounds.center());
                                *initial_layer = Some(layer.clone());
                                hit = true; break;
                            }
                        }
                    }
                    if !hit && !sel_is_locked {
                        for (idx, &mp) in hit_mids.iter().enumerate() {
                            if egui::Rect::from_center_size(mp, egui::vec2(20.0, 20.0)).contains(pos) {
                                *line_start = Some(pos);
                                *drag_state = 2 + idx;
                                *initial_center = Some(raw_bounds.center());
                                *initial_layer = Some(layer.clone());
                                hit = true; break;
                            }
                        }
                    }
                    if !hit && bounds.contains(pos) {
                        // Click INSIDE body → move this object (only if not locked)
                        let is_double_click = ui.input(|i| i.pointer.button_double_clicked(egui::PointerButton::Primary));
                        if is_double_click {
                            if let Some(sel) = project.selected_object {
                                if sel.object_type == ObjectType::Text && sel.object_idx < layer.text_annotations.len() {
                                    let txt = layer.text_annotations.remove(sel.object_idx);
                                    settings.pen_color      = txt.color;
                                    settings.text_font      = txt.font;
                                    settings.font_size      = txt.font_size;
                                    settings.text_shadow    = txt.shadow;
                                    settings.text_outline   = txt.outline;
                                    settings.text_wave_warp = txt.wave_warp;
                                    *pending_text = Some(crate::types::PendingText { position: txt.position, buffer: txt.text.clone() });
                                    *active_tool = crate::overlay::Tool::Text;
                                    project.selected_object = None;
                                    return;
                                }
                            }
                        }

                        if !sel_is_locked {
                            *line_start = Some(pos);
                            *drag_state = 0;
                            *initial_bounds = Some(raw_bounds);
                            *initial_layer = Some(layer.clone());
                        }
                        click_consumed = true;
                    } else if !hit {
                        // Click OUTSIDE selected object → only deselect if we are NOT clicking another object (handled below)
                        // project.selected_object = None; // Move this to the bottom block
                    }
                    if hit { click_consumed = true; }
                }
                
                if let Some(start) = *line_start {
                    if left_down {
                        let world_pos = pos + render_offset;
                        let world_start = start + render_offset;
                        if *dragging_source_rect {
                            // Dragging Source Rect (works for both live and static snips)
                            if let Some(sel) = project.selected_object {
                                if let ObjectType::Image = sel.object_type {
                                    let img = &mut project.layers[sel.layer_idx].placed_images[sel.object_idx];
                                    if img.source_rect.is_some() {
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
                            }
                        } else if let Some(base_layer) = initial_layer.as_ref() {
                            // Stable Transformations using initial snapshot
                            *layer = base_layer.clone();
                            let _canvas_rect = canvas_response.rect;

                            if *drag_state == 1 {
                                // Rotation
                                let center = initial_center.unwrap();
                                let mut angle = (world_pos - center).angle() - (world_start - center).angle();
                                
                                // Snap to 15 degree increments if Shift is held
                                if ui.ctx().input(|i| i.modifiers.shift) {
                                    let step = 15.0f32.to_radians();
                                    angle = (angle / step).round() * step;
                                }

                                if let Some(sel) = project.selected_object {
                                    match sel.object_type {
                                        ObjectType::Image => { layer.placed_images[sel.object_idx].rotation += angle; }
                                        ObjectType::Stroke => { layer.strokes[sel.object_idx].rotation += angle; }
                                        ObjectType::Text => { layer.text_annotations[sel.object_idx].rotation += angle; }
                                    }
                                } else {
                                    rotate_layer(layer, center, angle);
                                }
                            } else if *drag_state >= 20 {
                                // Perspective
                                let p_idx = *drag_state - 20;
                                let delta = pos - start;
                                if let Some(sel) = project.selected_object {
                                    match sel.object_type {
                                        ObjectType::Image => { layer.placed_images[sel.object_idx].perspective[p_idx] += delta; }
                                        ObjectType::Stroke => { layer.strokes[sel.object_idx].perspective[p_idx] += delta; }
                                        ObjectType::Text => { layer.text_annotations[sel.object_idx].perspective[p_idx] += delta; }
                                    }
                                } else {
                                    crate::utils::perspective_layer(layer, p_idx, delta);
                                }
                            } else if *drag_state >= 10 {
                                // Resize
                                let handle_idx = *drag_state - 10;
                                let ib = initial_bounds.unwrap();
                                let ic = [ib.left_top(), ib.right_top(), ib.left_bottom(), ib.right_bottom()];
                                let anchor = ic[3 - handle_idx];
                                let old_vec = ic[handle_idx] - anchor;
                                let new_vec = world_pos - anchor;
                                if old_vec.x.abs() > 1.0 && old_vec.y.abs() > 1.0 {
                                    let scale = egui::vec2(new_vec.x / old_vec.x, new_vec.y / old_vec.y);
                                    if let Some(sel) = project.selected_object {
                                        match sel.object_type {
                                            ObjectType::Image => {
                                                let img = &mut layer.placed_images[sel.object_idx];
                                                let mut ds = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32]);
                                                ds[0] *= scale.x; ds[1] *= scale.y;
                                                img.display_size = Some(ds);
                                                let rel = img.position - anchor;
                                                img.position = anchor + egui::vec2(rel.x * scale.x, rel.y * scale.y);
                                                
                                                if let Some(ref mut src) = img.source_rect {
                                                    let src_w = src[2] * scale.x;
                                                    let src_h = src[3] * scale.y;
                                                    let src_rel_x = src[0] - anchor.x;
                                                    let src_rel_y = src[1] - anchor.y;
                                                    src[0] = anchor.x + src_rel_x * scale.x;
                                                    src[1] = anchor.y + src_rel_y * scale.y;
                                                    src[2] = src_w;
                                                    src[3] = src_h;
                                                }
                                            }
                                            ObjectType::Stroke => {
                                                let s = &mut layer.strokes[sel.object_idx];
                                                s.scale.x *= scale.x;
                                                s.scale.y *= scale.y;
                                                let initial_c = initial_center.unwrap();
                                                let diff = (anchor + (initial_c - anchor) * scale) - initial_c;
                                                for p in &mut s.points { *p += diff; }
                                            }
                                            ObjectType::Text => {
                                                let t = &mut layer.text_annotations[sel.object_idx];
                                                t.scale.x *= scale.x;
                                                t.scale.y *= scale.y;
                                                let initial_c = initial_center.unwrap();
                                                let diff = (anchor + (initial_c - anchor) * scale) - initial_c;
                                                t.position += diff;
                                            }
                                        }
                                    } else {
                                        scale_layer(layer, anchor, scale);
                                    }
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
                                if let Some(sel) = project.selected_object {
                                    match sel.object_type {
                                        ObjectType::Image => { layer.placed_images[sel.object_idx].skew += skew_delta; }
                                        ObjectType::Stroke => { layer.strokes[sel.object_idx].skew += skew_delta; }
                                        ObjectType::Text => { layer.text_annotations[sel.object_idx].skew += skew_delta; }
                                    }
                                } else {
                                    skew_layer(layer, initial_center.unwrap(), skew_delta);
                                }
                            } else {
                                // Translate
                                let delta = pos - start;
                                if let Some(sel) = project.selected_object {
                                    match sel.object_type {
                                        ObjectType::Stroke => {
                                            if let Some(s) = layer.strokes.get_mut(sel.object_idx) {
                                                for p in &mut s.points { *p += delta; }
                                            }
                                        }
                                        ObjectType::Text => {
                                            if let Some(t) = layer.text_annotations.get_mut(sel.object_idx) {
                                                t.position += delta;
                                            }
                                        }
                                        ObjectType::Image => {
                                            if let Some(img) = layer.placed_images.get_mut(sel.object_idx) {
                                                img.position += delta;
                                                // source_rect intentionally NOT moved — stays fixed so snip can be placed independently
                                                img.thumbnail_dirty = true;
                                            }
                                        }
                                    }
                                } else {
                                    crate::utils::translate_layer(layer, delta);
                                }
                            }
                        }
                    }
                }
                





                if left_just_released {
                    if let Some(start) = *line_start {
                        if (pos - start).length_sq() > 1.0 || *drag_state > 0 {
                            let name = match *drag_state {
                                0 => "Move",
                                1 => "Rotate",
                                2..=5 => "Skew",
                                10..=13 => "Resize",
                                20..=23 => "Perspective",
                                _ => "Transform",
                            };
                            *ctx.request_history_push = Some(name.into());
                        }
                    }
                    // Re-capture for static snips when source rect drag ends
                    if *dragging_source_rect {
                        if let Some(sel) = project.selected_object {
                            if sel.object_type == ObjectType::Image {
                                let img = &mut project.layers[sel.layer_idx].placed_images[sel.object_idx];
                                if !img.is_live {
                                    if let Some(src) = img.source_rect {
                                        let ppp = ui.ctx().pixels_per_point();
                                        let (wx, wy) = crate::winapi_utils::get_window_screen_pos();
                                        let sx = (src[0] * ppp) as i32 + if settings.use_absolute_screen_coords { 0 } else { wx };
                                        let sy = (src[1] * ppp) as i32 + if settings.use_absolute_screen_coords { 0 } else { wy };
                                        let sw = (src[2] * ppp) as i32;
                                        let sh = (src[3] * ppp) as i32;
                                        if sw > 0 && sh > 0 {
                                            if let Some(mut pixels) = crate::tools::snip::capture_screen_rect_safe(settings, sx, sy, sw, sh) {
                                                // Apply mask if present
                                                if let Some(ref mask) = img.mask {
                                                    for (i, &m) in mask.iter().enumerate() {
                                                        if m == 0 && i * 4 + 3 < pixels.len() {
                                                            pixels[i * 4 + 3] = 0;
                                                        }
                                                    }
                                                }
                                                img.size = [src[2].round() as usize, src[3].round() as usize];
                                                img.pixels = pixels;
                                                img.texture = None;
                                                img.thumbnail_dirty = true;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    *line_start = None;
                    *initial_layer = None;
                    *dragging_source_rect = false;
                    if active_layer_idx < project.layers.len() {
                        let layer = &project.layers[active_layer_idx];
                        // Layer is empty, but we shouldn't automatically delete it here.
                    }
                }
                }
            }

    // Global click detection for unselected objects
    if left_just_pressed && !click_consumed {
        if ui.ctx().wants_pointer_input() { return; } 
        let is_double_click = ui.input(|i| i.pointer.button_double_clicked(egui::PointerButton::Primary));
        let world_pos = pos + render_offset;
        let mut found_objects = Vec::new();
        
        for (l_idx, l) in project.layers.iter().enumerate() {
            if !l.visible { continue; }
            
            // Hit test images
            for (img_idx, _img) in l.placed_images.iter().enumerate() {
                if let Some(rect) = crate::utils::object_bounds(l, ObjectType::Image, img_idx) {
                    if rect.contains(world_pos) {
                        found_objects.push(SelectedObject {
                            layer_idx: l_idx, object_type: ObjectType::Image, object_idx: img_idx
                        });
                    }
                }
            }
            
            // Hit test text — use exact_size when available for accurate hit rect
            for (txt_idx, _txt) in l.text_annotations.iter().enumerate() {
                if let Some(rect) = crate::utils::object_bounds(l, ObjectType::Text, txt_idx) {
                    if rect.contains(world_pos) {
                        found_objects.push(SelectedObject {
                            layer_idx: l_idx, object_type: ObjectType::Text, object_idx: txt_idx
                        });
                    }
                }
            }

            // Hit test strokes
            for (s_idx, s) in l.strokes.iter().enumerate() {
                let hit = if s.points.len() < 2 {
                    s.points.iter().any(|p| p.distance(world_pos) < s.width + 10.0)
                } else {
                    let mut hit_line = false;
                    for w in s.points.windows(2) {
                        let (p0, p1) = (w[0], w[1]);
                        let len_sq = p0.distance_sq(p1);
                        if len_sq > 0.0 {
                            let t = ((world_pos.x - p0.x) * (p1.x - p0.x) + (world_pos.y - p0.y) * (p1.y - p0.y)) / len_sq;
                            let t = t.clamp(0.0, 1.0);
                            let proj = p0 + (p1 - p0) * t;
                            if world_pos.distance(proj) < s.width * 0.5 + 5.0 {
                                hit_line = true; break;
                            }
                        }
                    }
                    hit_line
                };
                if hit {
                    found_objects.push(SelectedObject {
                        layer_idx: l_idx, object_type: ObjectType::Stroke, object_idx: s_idx
                    });
                }
            }
        }

        if found_objects.is_empty() {
            // Click empty space: ALWAYS deselect
            project.selected_object = None;
            if let Some(layer) = project.get_active_layer() {
                *line_start = Some(pos);
                *drag_state = 0;
                *initial_layer = Some(layer.clone());
            }
        } else if is_double_click {
            // Double click on object: select and focus
            found_objects.sort_by(|a, b| b.layer_idx.cmp(&a.layer_idx));
            let top = found_objects[0];

            // If double-clicking text, enter edit mode
            if top.object_type == ObjectType::Text {
                let layer = &mut project.layers[top.layer_idx];
                let txt = layer.text_annotations.remove(top.object_idx);
                // Restore annotation settings into toolbar
                settings.pen_color      = txt.color;
                settings.text_font      = txt.font;
                settings.font_size      = txt.font_size;
                settings.text_shadow    = txt.shadow;
                settings.text_outline   = txt.outline;
                settings.text_wave_warp = txt.wave_warp;
                *pending_text = Some(crate::types::PendingText { position: txt.position, buffer: txt.text.clone() });
                *active_tool = crate::overlay::Tool::Text;
                project.selected_object = None;
                return;
            }

            project.active_layer = top.layer_idx;
            project.selected_object = Some(top);

            // Also start dragging this object
            *line_start = Some(pos);
            *drag_state = 0;
            if let Some(layer) = project.get_active_layer() {
                *initial_bounds = crate::utils::object_bounds(layer, top.object_type, top.object_idx);
                *initial_layer = Some(layer.clone());
            }
        } else {
            // Single click on object: Select it!
            found_objects.sort_by(|a, b| b.layer_idx.cmp(&a.layer_idx));
            let top = found_objects[0];
            project.active_layer = top.layer_idx;
            project.selected_object = Some(top);
            
            *line_start = Some(pos);
            *drag_state = 0;
            if let Some(layer) = project.get_active_layer() {
                *initial_bounds = crate::utils::object_bounds(layer, top.object_type, top.object_idx);
                *initial_layer = Some(layer.clone());
            }
        }
    }
}

pub fn render(ctx: &mut ToolContext) {
    if !ctx.edit_mode { return; }
    
    let is_dragging = ctx.line_start.is_some();
    let has_selection = ctx.project.selected_object.is_some();
    if !ctx.canvas_response.hovered() && !is_dragging && !has_selection { return; }

    let project = &ctx.project;
    let ui = &ctx.ui;
    let painter = ui.painter_at(ctx.canvas_response.rect);
    let render_offset = ctx.render_offset;
    
    let is_layer_mode = project.selected_object.is_none();
    let active_layer_idx = project.active_layer;
    
    if active_layer_idx < project.layers.len() {
        let layer = &project.layers[active_layer_idx];
        
        // Determine bounds to draw handles for
        let active_bounds = if let Some(sel) = project.selected_object {
            crate::utils::object_bounds(layer, sel.object_type, sel.object_idx)
        } else {
            crate::utils::layer_bounds(layer)
        };

        if let Some(raw_b) = active_bounds {
            let b = raw_b.translate(-render_offset).expand(if is_layer_mode { 2.0 } else { 4.0 });
            let color = if is_layer_mode { egui::Color32::from_rgb(80, 180, 255) } else { egui::Color32::from_rgb(255, 200, 0) };
            
            // Draw main selection rect
            painter.rect_stroke(b, 0.0, egui::Stroke::new(1.5, color), egui::StrokeKind::Middle);

            // 1. Rotation handles
            let rot_p_top = b.center_top() - egui::vec2(0.0, 25.0);
            let rot_p_bot = b.center_bottom() + egui::vec2(0.0, 25.0);
            
            painter.line_segment([b.center_top(), rot_p_top], egui::Stroke::new(1.0, egui::Color32::GRAY));
            painter.circle_filled(rot_p_top, 5.0, egui::Color32::from_rgb(255, 200, 50));
            painter.circle_stroke(rot_p_top, 5.0, egui::Stroke::new(1.0, egui::Color32::BLACK));

            painter.line_segment([b.center_bottom(), rot_p_bot], egui::Stroke::new(1.0, egui::Color32::GRAY));
            painter.circle_filled(rot_p_bot, 5.0, egui::Color32::from_rgb(255, 200, 50));
            painter.circle_stroke(rot_p_bot, 5.0, egui::Stroke::new(1.0, egui::Color32::BLACK));

            // 2. Perspective handles (corners, slightly offset)
            let p_dist = 25.0;
            let draw_p_corners = [
                b.left_top() + egui::vec2(-p_dist, -p_dist),
                b.right_top() + egui::vec2(p_dist, -p_dist),
                b.left_bottom() + egui::vec2(-p_dist, p_dist),
                b.right_bottom() + egui::vec2(p_dist, p_dist),
            ];
            for pc in draw_p_corners {
                painter.circle_filled(pc, 4.0, egui::Color32::from_rgb(50, 150, 255));
                painter.circle_stroke(pc, 4.0, egui::Stroke::new(1.0, egui::Color32::BLACK));
            }

            // 3. Resize handles (corners)
            for corner in [b.left_top(), b.right_top(), b.left_bottom(), b.right_bottom()] {
                let r = egui::Rect::from_center_size(corner, egui::vec2(8.0, 8.0));
                painter.rect_filled(r, 0.0, egui::Color32::WHITE);
                painter.rect_stroke(r, 0.0, egui::Stroke::new(1.0, egui::Color32::BLACK), egui::StrokeKind::Middle);
            }

            // 4. Skew handles (mids)
            for mid in [b.left_center(), b.right_center(), b.center_top(), b.center_bottom()] {
                painter.circle_filled(mid, 3.0, egui::Color32::from_rgb(100, 255, 100));
                painter.circle_stroke(mid, 3.0, egui::Stroke::new(1.0, egui::Color32::BLACK));
            }

            // Removed duplicate red garbage bin button rendering
        }
    }
}


