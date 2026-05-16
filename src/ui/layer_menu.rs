use eframe::egui;
use crate::types::*;
use crate::ui::toolbar::photoshop_frame;

pub fn render_layers_window(
    ctx: &egui::Context,
    project: &mut crate::project::Project,
    settings: &mut Settings,
    active_tool: &mut crate::overlay::Tool,
    open: &mut bool,
    filters_open: &mut Option<usize>,
    load_picker_open: &mut bool,
) {
    let frame = photoshop_frame(settings);

    let win_resp = egui::Window::new(egui::RichText::new("Layers").color(egui::Color32::from_rgb(180, 180, 200)).size(16.0))
        .open(open)
        .title_bar(false)
        .resizable(true)
        .default_width(320.0)
        .default_pos(settings.layer_menu_pos)
        .frame(frame)
        .show(ctx, |ui| {
            ui.style_mut().visuals.widgets.inactive.bg_fill = egui::Color32::from_rgba_premultiplied(255, 255, 255, 8);
            
            ui.horizontal(|ui| {
                if ui.button("📄").on_hover_text("New Project").clicked() {
                    *project = crate::project::Project::new("New Project");
                }
                if ui.button("💾").on_hover_text("Save Project").clicked() { project.save(); }
                if ui.button("📂").on_hover_text("Load Project").clicked() {
                    *load_picker_open = !*load_picker_open;
                }
                if ui.add(egui::TextEdit::singleline(&mut project.name).desired_width(80.0)).changed() {
                    project.save();
                }
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("➕").on_hover_text("New Layer").clicked() {
                        project.layers.push(crate::project::Layer::new(&format!("Layer {}", project.layers.len() + 1)));
                        project.active_layer = project.layers.len() - 1;
                    }
                });
            });

            ui.add_space(8.0);

            egui::ScrollArea::vertical().show(ui, |ui: &mut egui::Ui| {
                let mut layer_to_remove = None;
                let mut layer_to_move_up = None;
                let mut layer_to_move_down = None;
                let mut layer_to_merge_down = None;
                let mut object_to_delete = None;
                let mut object_to_select = None;
                let mut object_to_move: Option<(usize, ObjectType, usize, i32)> = None;
                let mut object_to_clone: Option<(usize, ObjectType, usize)> = None;
                let total_layers = project.layers.len();

                for i in (0..total_layers).rev() {
                    let is_active = project.active_layer == i;
                    let layer = &mut project.layers[i];
                    
                    let bg_color = if is_active {
                        egui::Color32::from_rgba_premultiplied(60, 120, 200, 100)
                    } else {
                        egui::Color32::TRANSPARENT
                    };

                    let row_frame = egui::Frame::default().fill(bg_color).inner_margin(egui::Margin::symmetric(4, 2)).corner_radius(egui::CornerRadius::same(4));
                    
                    row_frame.show(ui, |ui: &mut egui::Ui| {
                        ui.horizontal(|ui: &mut egui::Ui| {
                            let obj_count = layer.placed_images.len() + layer.text_annotations.len() + layer.strokes.len();
                            if obj_count > 0 {
                                let icon = if layer.expanded { "▼" } else { "▶" };
                                if ui.button(egui::RichText::new(icon).size(10.0)).clicked() {
                                    layer.expanded = !layer.expanded;
                                }
                            } else {
                                ui.add_space(18.0);
                            }

                            let drag_handle = ui.add(egui::Label::new("\u{2261}").sense(egui::Sense::drag()));
                            if drag_handle.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::Grab); }
                            
                            if drag_handle.dragged() { 
                                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                                if let Some(mouse_pos) = ui.ctx().pointer_interact_pos() {
                                    let mut ghost_rect = drag_handle.rect;
                                    ghost_rect.set_center(mouse_pos);
                                    ui.painter().rect_filled(ghost_rect.expand(20.0), 4.0, egui::Color32::from_rgba_premultiplied(100, 100, 100, 150));
                                }
                            }
                            if drag_handle.drag_stopped() {
                                if let Some(pos) = ui.ctx().pointer_interact_pos() {
                                    let threshold = drag_handle.rect.height() * 0.7;
                                    if drag_handle.rect.top() > 0.0 && pos.y < drag_handle.rect.top() - threshold && i < total_layers - 1 {
                                        layer_to_move_up = Some(i);
                                    } else if drag_handle.rect.bottom() > 0.0 && pos.y > drag_handle.rect.bottom() + threshold && i > 0 {
                                        layer_to_move_down = Some(i);
                                    }
                                }
                            }
                            
                            if is_active && !drag_handle.dragged() {
                                ui.painter().rect_filled(drag_handle.rect.expand(2.0), 2.0, egui::Color32::from_rgba_premultiplied(200, 200, 200, 40));
                            }

                            ui.checkbox(&mut layer.visible, "");
                            
                            let (thumb_rect, _) = ui.allocate_exact_size(egui::vec2(24.0, 18.0), egui::Sense::hover());
                            ui.painter().rect_filled(thumb_rect, 2.0, egui::Color32::from_gray(40));
                            ui.painter().rect_stroke(thumb_rect, 2.0, egui::Stroke::new(1.0, egui::Color32::from_gray(60)), egui::StrokeKind::Middle);
                            
                            if let Some(tex) = layer.placed_images.first().and_then(|img| img.thumbnail_texture.as_ref().or(img.texture.as_ref())) {
                                ui.painter().image(tex.id(), thumb_rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                            } else if !layer.strokes.is_empty() {
                                let c = thumb_rect.center();
                                ui.painter().line_segment([c - egui::vec2(5.0, 2.0), c + egui::vec2(5.0, 2.0)], egui::Stroke::new(1.0, egui::Color32::WHITE));
                            }
                            
                            let name_resp = ui.add(egui::TextEdit::singleline(&mut layer.name).frame(is_active).desired_width(100.0));
                            if name_resp.gained_focus() || name_resp.clicked() {
                                project.active_layer = i;
                            }
                            
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui: &mut egui::Ui| {
                                if ui.add(egui::Button::new(egui::RichText::new("🗑").color(egui::Color32::from_rgb(255, 60, 60))).fill(egui::Color32::from_black_alpha(40))).on_hover_text("Delete Layer").clicked() { 
                                    layer_to_remove = Some(i); 
                                }
                                if ui.button("fx").clicked() { *filters_open = Some(i); }
                                if ui.add(egui::Button::new(egui::RichText::new("➕").color(egui::Color32::from_rgb(100, 255, 100))).fill(egui::Color32::from_black_alpha(40))).on_hover_text("Add New Object").clicked() {
                                    let obj_idx = layer.placed_images.len();
                                    let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
                                    let w = 200usize;
                                    let h = 150usize;
                                    let pixels = vec![0u8; w * h * 4]; // fully transparent
                                    let mut img = crate::types::PlacedImage::new(id, egui::pos2(500.0, 500.0), [w, h], pixels);
                                    img.name = format!("Object {}", obj_idx + 1);
                                    layer.placed_images.push(img);
                                    layer.expanded = true;
                                    object_to_select = Some((i, ObjectType::Image, obj_idx));
                                    project.active_layer = i;
                                    *active_tool = crate::overlay::Tool::Move;
                                }
                                if i > 0 && ui.button("⭳").on_hover_text("Merge Down").clicked() { layer_to_merge_down = Some(i); }
                                if i < total_layers - 1 && ui.button("⬆").clicked() { layer_to_move_up = Some(i); }
                                if i > 0 && ui.button("⬇").clicked() { layer_to_move_down = Some(i); }
                                
                                let mut opacity_percent = (layer.opacity * 100.0) as i32;
                                if ui.add(egui::DragValue::new(&mut opacity_percent).range(0..=100).suffix("%")).changed() {
                                    layer.opacity = opacity_percent as f32 / 100.0;
                                }
                            });
                        });

                        if layer.expanded {
                            let obj_count = layer.placed_images.len() + layer.text_annotations.len() + layer.strokes.len();
                            if obj_count > 0 {
                                ui.indent(format!("layer_obj_{}", i), |ui: &mut egui::Ui| {
                                    for (img_idx, img) in layer.placed_images.iter_mut().enumerate() {
                                        let is_sel = project.selected_object == Some(SelectedObject { layer_idx: i, object_type: ObjectType::Image, object_idx: img_idx });
                                        let bg_color = if is_sel { egui::Color32::from_rgba_unmultiplied(60, 120, 200, 100) } else { egui::Color32::TRANSPARENT };
                                        egui::Frame::NONE.fill(bg_color).inner_margin(egui::Margin::symmetric(4, 2)).corner_radius(4.0).show(ui, |ui| {
                                            ui.horizontal(|ui: &mut egui::Ui| {
                                                ui.checkbox(&mut img.visible, "");
                                                let mut name = img.name.clone();
                                                let name_resp = ui.add(egui::TextEdit::singleline(&mut name).frame(false).desired_width(100.0));
                                                if name_resp.changed() { img.name = name; }
                                                if name_resp.gained_focus() || name_resp.clicked() {
                                                    object_to_select = Some((i, ObjectType::Image, img_idx));
                                                }
                                                if name_resp.double_clicked() {
                                                    object_to_select = Some((i, ObjectType::Image, img_idx));
                                                    project.active_layer = i;
                                                    *active_tool = crate::overlay::Tool::Move;
                                                }
                                                ui.add_space(5.0);
                                                
                                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui: &mut egui::Ui| {
                                                    if ui.add(egui::Button::new(egui::RichText::new("✖").color(egui::Color32::RED).size(10.0)).frame(false)).clicked() { object_to_delete = Some((i, ObjectType::Image, img_idx)); }
                                                    if ui.add(egui::Button::new(egui::RichText::new("⬇").size(10.0)).frame(false)).clicked() { object_to_move = Some((i, ObjectType::Image, img_idx, -1)); }
                                                    if ui.add(egui::Button::new(egui::RichText::new("⬆").size(10.0)).frame(false)).clicked() { object_to_move = Some((i, ObjectType::Image, img_idx, 1)); }
                                                    if ui.add(egui::Button::new(egui::RichText::new("⎘").size(10.0)).frame(false)).on_hover_text("Clone").clicked() { object_to_clone = Some((i, ObjectType::Image, img_idx)); }
                                                    if ui.add(egui::Button::new(egui::RichText::new("fx").size(10.0)).frame(settings.fx_open == Some(crate::types::SelectedObject { layer_idx: i, object_type: crate::types::ObjectType::Image, object_idx: img_idx }))).clicked() {
                                                        let target = crate::types::SelectedObject { layer_idx: i, object_type: crate::types::ObjectType::Image, object_idx: img_idx };
                                                        if settings.fx_open == Some(target) { settings.fx_open = None; }
                                                        else { settings.fx_open = Some(target); }
                                                        object_to_select = Some((i, ObjectType::Image, img_idx));
                                                        project.active_layer = i;
                                                        *active_tool = crate::overlay::Tool::Move;
                                                    }
                                                    let mut op_val = (img.opacity * 100.0) as i32;
                                                    if ui.add(egui::DragValue::new(&mut op_val).range(0..=100).suffix("%")).changed() { img.opacity = op_val as f32 / 100.0; }
                                                });
                                            });
                                        });
                                    }
                                    for (t_idx, ann) in layer.text_annotations.iter_mut().enumerate() {
                                        let is_sel = project.selected_object == Some(SelectedObject { layer_idx: i, object_type: ObjectType::Text, object_idx: t_idx });
                                        let bg_color = if is_sel { egui::Color32::from_rgba_unmultiplied(60, 120, 200, 100) } else { egui::Color32::TRANSPARENT };
                                        egui::Frame::NONE.fill(bg_color).inner_margin(egui::Margin::symmetric(4, 2)).corner_radius(4.0).show(ui, |ui| {
                                            ui.horizontal(|ui: &mut egui::Ui| {
                                                ui.checkbox(&mut ann.visible, "");
                                                let mut name = ann.text.clone();
                                                let name_resp = ui.add(egui::TextEdit::singleline(&mut name).frame(false).desired_width(100.0));
                                                if name_resp.changed() { ann.text = name; }
                                                if name_resp.gained_focus() || name_resp.clicked() {
                                                    object_to_select = Some((i, ObjectType::Text, t_idx));
                                                }
                                                if name_resp.double_clicked() {
                                                    object_to_select = Some((i, ObjectType::Text, t_idx));
                                                    project.active_layer = i;
                                                    *active_tool = crate::overlay::Tool::Move;
                                                }
                                                ui.add_space(5.0);
                                                
                                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui: &mut egui::Ui| {
                                                    if ui.add(egui::Button::new(egui::RichText::new("✖").color(egui::Color32::RED).size(10.0)).frame(false)).clicked() { object_to_delete = Some((i, ObjectType::Text, t_idx)); }
                                                    if ui.add(egui::Button::new(egui::RichText::new("⎘").size(10.0)).frame(false)).on_hover_text("Clone").clicked() { object_to_clone = Some((i, ObjectType::Text, t_idx)); }
                                                    if ui.add(egui::Button::new(egui::RichText::new("⬇").size(10.0)).frame(false)).clicked() { object_to_move = Some((i, ObjectType::Text, t_idx, -1)); }
                                                    if ui.add(egui::Button::new(egui::RichText::new("⬆").size(10.0)).frame(false)).clicked() { object_to_move = Some((i, ObjectType::Text, t_idx, 1)); }
                                                    if ui.add(egui::Button::new(egui::RichText::new("fx").size(10.0)).frame(settings.fx_open == Some(crate::types::SelectedObject { layer_idx: i, object_type: crate::types::ObjectType::Text, object_idx: t_idx }))).clicked() {
                                                        let target = crate::types::SelectedObject { layer_idx: i, object_type: crate::types::ObjectType::Text, object_idx: t_idx };
                                                        if settings.fx_open == Some(target) { settings.fx_open = None; }
                                                        else { settings.fx_open = Some(target); }
                                                        object_to_select = Some((i, ObjectType::Text, t_idx));
                                                        project.active_layer = i;
                                                        *active_tool = crate::overlay::Tool::Move;
                                                    }
                                                    let mut op_val = (ann.opacity * 100.0) as i32;
                                                    if ui.add(egui::DragValue::new(&mut op_val).range(0..=100).suffix("%")).changed() { ann.opacity = op_val as f32 / 100.0; }
                                                });
                                            });
                                        });
                                    }
                                    let mut freehand_indices = Vec::new();
                                    for (s_idx, s) in layer.strokes.iter_mut().enumerate() {
                                        if s.kind == crate::overlay::StrokeKind::Freehand {
                                            freehand_indices.push(s_idx);
                                            continue;
                                        }
                                        let is_sel = project.selected_object == Some(SelectedObject { layer_idx: i, object_type: ObjectType::Stroke, object_idx: s_idx });
                                        let bg_color = if is_sel { egui::Color32::from_rgba_unmultiplied(60, 120, 200, 100) } else { egui::Color32::TRANSPARENT };
                                        egui::Frame::NONE.fill(bg_color).inner_margin(egui::Margin::symmetric(4, 2)).corner_radius(4.0).show(ui, |ui| {
                                            ui.horizontal(|ui: &mut egui::Ui| {
                                                ui.checkbox(&mut s.visible, "");
                                                let mut name = s.name.clone();
                                                let name_resp = ui.add(egui::TextEdit::singleline(&mut name).frame(false).desired_width(100.0));
                                                if name_resp.changed() { s.name = name; }
                                                if name_resp.gained_focus() || name_resp.clicked() {
                                                    object_to_select = Some((i, ObjectType::Stroke, s_idx));
                                                }
                                                if name_resp.double_clicked() {
                                                    object_to_select = Some((i, ObjectType::Stroke, s_idx));
                                                    project.active_layer = i;
                                                    *active_tool = crate::overlay::Tool::Move;
                                                }
                                                ui.add_space(5.0);
                                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui: &mut egui::Ui| {
                                                    if ui.add(egui::Button::new(egui::RichText::new("✖").color(egui::Color32::RED).size(10.0)).frame(false)).clicked() { object_to_delete = Some((i, ObjectType::Stroke, s_idx)); }
                                                    if ui.add(egui::Button::new(egui::RichText::new("⎘").size(10.0)).frame(false)).on_hover_text("Clone").clicked() { object_to_clone = Some((i, ObjectType::Stroke, s_idx)); }
                                                    if ui.add(egui::Button::new(egui::RichText::new("⬇").size(10.0)).frame(false)).clicked() { object_to_move = Some((i, ObjectType::Stroke, s_idx, -1)); }
                                                    if ui.add(egui::Button::new(egui::RichText::new("⬆").size(10.0)).frame(false)).clicked() { object_to_move = Some((i, ObjectType::Stroke, s_idx, 1)); }
                                                    if ui.add(egui::Button::new(egui::RichText::new("fx").size(10.0)).frame(settings.fx_open == Some(crate::types::SelectedObject { layer_idx: i, object_type: crate::types::ObjectType::Stroke, object_idx: s_idx }))).clicked() {
                                                        let target = crate::types::SelectedObject { layer_idx: i, object_type: crate::types::ObjectType::Stroke, object_idx: s_idx };
                                                        if settings.fx_open == Some(target) { settings.fx_open = None; }
                                                        else { settings.fx_open = Some(target); }
                                                        object_to_select = Some((i, ObjectType::Stroke, s_idx));
                                                        project.active_layer = i;
                                                        *active_tool = crate::overlay::Tool::Move;
                                                    }
                                                    let mut op_val = (s.opacity * 100.0) as i32;
                                                    if ui.add(egui::DragValue::new(&mut op_val).range(0..=100).suffix("%")).changed() { s.opacity = op_val as f32 / 100.0; }
                                                });
                                            });
                                        });
                                    }
                                    if !freehand_indices.is_empty() {
                                        ui.horizontal(|ui: &mut egui::Ui| {
                                            let is_sel = project.selected_object.map_or(false, |sel| sel.layer_idx == i && sel.object_type == ObjectType::Stroke && freehand_indices.contains(&sel.object_idx));
                                            let mut all_visible = layer.strokes[freehand_indices[0]].visible;
                                            if ui.checkbox(&mut all_visible, "").changed() {
                                                for &idx in &freehand_indices { layer.strokes[idx].visible = all_visible; }
                                            }
                                            if ui.selectable_label(is_sel, "🖌 Brush Strokes").clicked() {
                                                object_to_select = Some((i, ObjectType::Stroke, freehand_indices[0]));
                                            }
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui: &mut egui::Ui| {
                                                if ui.add(egui::Button::new(egui::RichText::new("✖").color(egui::Color32::RED).size(10.0)).frame(false)).on_hover_text("Delete All Strokes").clicked() {
                                                    object_to_delete = Some((i, ObjectType::Stroke, usize::MAX));
                                                }
                                                if ui.add(egui::Button::new(egui::RichText::new("⎘").size(10.0)).frame(false)).on_hover_text("Clone All").clicked() { object_to_clone = Some((i, ObjectType::Stroke, usize::MAX)); }
                                                if ui.add(egui::Button::new(egui::RichText::new("⬇").size(10.0)).frame(false)).clicked() { object_to_move = Some((i, ObjectType::Stroke, freehand_indices[0], -1)); }
                                                if ui.add(egui::Button::new(egui::RichText::new("⬆").size(10.0)).frame(false)).clicked() { object_to_move = Some((i, ObjectType::Stroke, freehand_indices[0], 1)); }
                                                if ui.add(egui::Button::new(egui::RichText::new("fx").size(10.0)).frame(settings.fx_open == Some(crate::types::SelectedObject { layer_idx: i, object_type: crate::types::ObjectType::Stroke, object_idx: freehand_indices[0] }))).clicked() {
                                                    let target = crate::types::SelectedObject { layer_idx: i, object_type: crate::types::ObjectType::Stroke, object_idx: freehand_indices[0] };
                                                    if settings.fx_open == Some(target) { settings.fx_open = None; }
                                                    else { settings.fx_open = Some(target); }
                                                    object_to_select = Some((i, ObjectType::Stroke, freehand_indices[0]));
                                                    project.active_layer = i;
                                                    *active_tool = crate::overlay::Tool::Move;
                                                }
                                                let mut op_val = (layer.strokes[freehand_indices[0]].opacity * 100.0) as i32;
                                                if ui.add(egui::DragValue::new(&mut op_val).range(0..=100).suffix("%")).changed() {
                                                    let new_op = op_val as f32 / 100.0;
                                                    for &idx in &freehand_indices { layer.strokes[idx].opacity = new_op; }
                                                }
                                            });
                                        });
                                    }
                                });
                            }
                        }
                    });
                }

                if let Some(idx) = layer_to_merge_down {
                    if idx > 0 {
                        let mut top = project.layers.remove(idx);
                        let bottom = &mut project.layers[idx - 1];
                        bottom.placed_images.append(&mut top.placed_images);
                        bottom.text_annotations.append(&mut top.text_annotations);
                        bottom.strokes.append(&mut top.strokes);
                        bottom.expanded = true;
                        project.active_layer = idx - 1;
                        project.selected_object = None;
                    }
                }
                if let Some(idx) = layer_to_remove {
                    project.layers.remove(idx);
                    project.selected_object = None;
                    if project.layers.is_empty() { project.layers.push(crate::project::Layer::new("Layer 1")); }
                    project.active_layer = project.active_layer.min(project.layers.len() - 1);
                    project.save();
                }
                if let Some(idx) = layer_to_move_up {
                    if idx < total_layers - 1 {
                        project.layers.swap(idx, idx + 1);
                        if project.active_layer == idx { project.active_layer = idx + 1; }
                        else if project.active_layer == idx + 1 { project.active_layer = idx; }
                        project.save();
                    }
                }
                if let Some(idx) = layer_to_move_down {
                    if idx > 0 {
                        project.layers.swap(idx, idx - 1);
                        if project.active_layer == idx { project.active_layer = idx - 1; }
                        else if project.active_layer == idx - 1 { project.active_layer = idx; }
                        project.save();
                    }
                }
                if let Some((l_idx, obj_type, o_idx)) = object_to_delete {
                    match obj_type {
                        ObjectType::Stroke => {
                            if o_idx == usize::MAX {
                                project.layers[l_idx].strokes.retain(|s| s.kind != crate::overlay::StrokeKind::Freehand);
                            } else {
                                project.layers[l_idx].strokes.remove(o_idx);
                            }
                        }
                        ObjectType::Text => { project.layers[l_idx].text_annotations.remove(o_idx); }
                        ObjectType::Image => { project.layers[l_idx].placed_images.remove(o_idx); }
                    }
                    project.selected_object = None;
                    project.save();
                }
                if let Some((l_idx, obj_type, o_idx)) = object_to_clone {
                    match obj_type {
                        ObjectType::Stroke => {
                            if o_idx == usize::MAX {
                                let mut freehand = project.layers[l_idx].strokes.iter().filter(|s| s.kind == crate::overlay::StrokeKind::Freehand).cloned().collect::<Vec<_>>();
                                for s in &mut freehand {
                                    s.points.iter_mut().for_each(|p| { p.x += 10.0; p.y += 10.0; });
                                }
                                project.layers[l_idx].strokes.append(&mut freehand);
                            } else {
                                let mut cloned = project.layers[l_idx].strokes[o_idx].clone();
                                cloned.points.iter_mut().for_each(|p| { p.x += 10.0; p.y += 10.0; });
                                project.layers[l_idx].strokes.push(cloned);
                            }
                        }
                        ObjectType::Image => {
                            let mut cloned = project.layers[l_idx].placed_images[o_idx].clone();
                            cloned.position.x += 10.0; cloned.position.y += 10.0;
                            cloned.id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
                            cloned.texture = None;
                            project.layers[l_idx].placed_images.push(cloned);
                        }
                        ObjectType::Text => {
                            let mut cloned = project.layers[l_idx].text_annotations[o_idx].clone();
                            cloned.position.x += 10.0; cloned.position.y += 10.0;
                            project.layers[l_idx].text_annotations.push(cloned);
                        }
                    }
                }
                if let Some((l_idx, obj_type, o_idx)) = object_to_select {
                    project.active_layer = l_idx;
                    project.selected_object = Some(SelectedObject { layer_idx: l_idx, object_type: obj_type, object_idx: o_idx });
                }
                if let Some((l_idx, obj_type, o_idx, dir)) = object_to_move {
                    let layer = &mut project.layers[l_idx];
                    match obj_type {
                        ObjectType::Image => {
                            let new_idx = (o_idx as i32 + dir).clamp(0, layer.placed_images.len() as i32 - 1) as usize;
                            if new_idx != o_idx { 
                                layer.placed_images.swap(o_idx, new_idx); 
                                project.selected_object = Some(SelectedObject { layer_idx: l_idx, object_type: obj_type, object_idx: new_idx });
                            }
                        }
                        ObjectType::Text => {
                            let new_idx = (o_idx as i32 + dir).clamp(0, layer.text_annotations.len() as i32 - 1) as usize;
                            if new_idx != o_idx { 
                                layer.text_annotations.swap(o_idx, new_idx); 
                                project.selected_object = Some(SelectedObject { layer_idx: l_idx, object_type: obj_type, object_idx: new_idx });
                            }
                        }
                        ObjectType::Stroke => {
                            let new_idx = (o_idx as i32 + dir).clamp(0, layer.strokes.len() as i32 - 1) as usize;
                            if new_idx != o_idx { 
                                layer.strokes.swap(o_idx, new_idx); 
                                project.selected_object = Some(SelectedObject { layer_idx: l_idx, object_type: obj_type, object_idx: new_idx });
                            }
                        }
                    }
                }

            });
        });

    if let Some(resp) = win_resp {
        if resp.response.dragged() {
            let layer_id = resp.response.layer_id;
            if let Some(rect) = ctx.memory(|m| m.area_rect(layer_id.id)) {
                settings.layer_menu_pos = rect.min;
            }
        }
    }
    
    crate::ui::object_fx::render_fx_window(ctx, project, settings);

    if *load_picker_open {
        egui::Window::new("Load Project")
            .collapsible(false)
            .resizable(true)
            .default_width(200.0)
            .frame(photoshop_frame(settings))
            .show(ctx, |ui| {
                let projects = crate::project::Project::list_projects();
                if projects.is_empty() {
                    ui.label("No projects found.");
                } else {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for name in projects {
                            ui.horizontal(|ui| {
                                if ui.selectable_label(project.name == name, &name).clicked() {
                                    if let Some(p) = crate::project::Project::load(&name) {
                                        *project = p;
                                        *load_picker_open = false;
                                    }
                                }
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.button("🗑").on_hover_text("Delete from disk").clicked() {
                                        crate::project::Project::delete(&name);
                                    }
                                });
                            });
                        }
                    });
                }
                ui.separator();
                if ui.button("Clear All Layers").clicked() {
                    project.layers.clear();
                    project.layers.push(crate::project::Layer::new("Layer 1"));
                    project.active_layer = 0;
                    project.selected_object = None;
                    project.save();
                }
                if ui.button("Close").clicked() {
                    *load_picker_open = false;
                }
            });
    }
}
