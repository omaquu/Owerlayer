use eframe::egui;
use crate::types::*;
// use crate::project::{Project, Layer};
use crate::ui::toolbar::photoshop_frame;

pub fn render_layers_window(
    ctx: &egui::Context,
    project: &mut crate::project::Project,
    settings: &mut Settings,
    active_tool: &mut crate::overlay::Tool,
    open: &mut bool,
    filters_open: &mut Option<usize>,
) {
    let frame = photoshop_frame(settings);

    egui::Window::new(egui::RichText::new("Layers").color(egui::Color32::from_rgb(180, 180, 200)).size(16.0))
        .open(open)
        .title_bar(false)
        .resizable(true)
        .default_width(320.0)
        .frame(frame)
        .show(ctx, |ui| {
            ui.style_mut().visuals.widgets.inactive.bg_fill = egui::Color32::from_rgba_premultiplied(255, 255, 255, 8);
            
            ui.horizontal(|ui| {
                if ui.button("📄").on_hover_text("New Project").clicked() {
                    *project = crate::project::Project::new("New Project");
                }
                if ui.button("💾").on_hover_text("Save Project").clicked() { project.save(); }
                if ui.button("📂").on_hover_text("Load Project").clicked() {
                    if let Some(p) = crate::project::Project::load(&project.name) {
                        *project = p;
                    }
                }
                ui.add(egui::TextEdit::singleline(&mut project.name).desired_width(80.0));
                
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
                let mut object_to_delete = None;
                let mut object_to_select = None;
                let object_to_clone: Option<(usize, crate::project::ObjectType, usize)> = None;
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
                        // Main layer row
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

                            // Drag handle for reordering
                            let drag_handle = ui.add(egui::Label::new("\u{2261}").sense(egui::Sense::drag()));
                            if drag_handle.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::Grab); }
                            
                            if drag_handle.dragged() { 
                                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                                // Visual effect for dragging: show a ghost row following mouse
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
                            
                            // Visual hint for active/dragging
                            if is_active && !drag_handle.dragged() {
                                ui.painter().rect_filled(drag_handle.rect.expand(2.0), 2.0, egui::Color32::from_rgba_premultiplied(200, 200, 200, 40));
                            }

                            ui.checkbox(&mut layer.visible, "");
                            
                            // Layer Preview Thumbnail
                            let (thumb_rect, _) = ui.allocate_exact_size(egui::vec2(24.0, 18.0), egui::Sense::hover());
                            ui.painter().rect_filled(thumb_rect, 2.0, egui::Color32::from_gray(40));
                            ui.painter().rect_stroke(thumb_rect, 2.0, egui::Stroke::new(1.0, egui::Color32::from_gray(60)), egui::StrokeKind::Middle);
                            
                            if let Some(img) = layer.placed_images.first() {
                                if let Some(tex) = &img.texture {
                                    ui.painter().image(tex.id(), thumb_rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                                }
                            } else if !layer.strokes.is_empty() {
                                // Draw a tiny squiggly line as a stroke preview
                                let c = thumb_rect.center();
                                ui.painter().line_segment([c - egui::vec2(5.0, 2.0), c + egui::vec2(5.0, 2.0)], egui::Stroke::new(1.0, egui::Color32::WHITE));
                            }
                            
                            // Layer name — clicking it selects the layer
                            if ui.selectable_label(is_active, &layer.name).clicked() {
                                project.active_layer = i;
                            }
                            
                            let mut opacity_percent = (layer.opacity * 100.0) as i32;
                            if ui.add(egui::DragValue::new(&mut opacity_percent).range(0..=100).suffix("%")).changed() {
                                layer.opacity = opacity_percent as f32 / 100.0;
                            }

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui: &mut egui::Ui| {
                                if ui.add(egui::Button::new(egui::RichText::new("🗑").color(egui::Color32::from_rgb(255, 50, 50))).frame(false)).on_hover_text("Delete Layer").clicked() { 
                                    layer_to_remove = Some(i); 
                                }
                                if ui.button("fx").clicked() { *filters_open = Some(i); }
                            });
                        });

                        // Child objects — auto-shown when expanded
                        if layer.expanded {
                            let obj_count = layer.placed_images.len() + layer.text_annotations.len() + layer.strokes.len();
                            if obj_count > 0 {
                                ui.indent(format!("layer_obj_{}", i), |ui: &mut egui::Ui| {
                                    for (img_idx, img) in layer.placed_images.iter_mut().enumerate() {
                                        ui.horizontal(|ui: &mut egui::Ui| {
                                            ui.checkbox(&mut img.visible, "");
                                            let is_sel = project.selected_object == Some(crate::project::SelectedObject { layer_idx: i, object_type: crate::project::ObjectType::Image, object_idx: img_idx });
                                            let label = if img.blur > 0.0 {
                                                format!("🔲 Blur {}", img_idx)
                                            } else if img.url.is_some() {
                                                let url = img.url.as_ref().unwrap();
                                                format!("🌐 {}", if url.len() > 15 { &url[..12] } else { url })
                                            } else if img.is_live && img.source_rect.is_some() {
                                                format!("🪞 Mirror {}", img_idx)
                                            } else if img.is_live {
                                                "🖼 Live".to_string()
                                            } else {
                                                format!("🖼 Image {}", img_idx)
                                            };
                                            let item_resp = ui.selectable_label(is_sel, label);
                                            if item_resp.clicked() {
                                                object_to_select = Some((i, crate::project::ObjectType::Image, img_idx));
                                            }
                                            if item_resp.double_clicked() {
                                                object_to_select = Some((i, crate::project::ObjectType::Image, img_idx));
                                                project.active_layer = i;
                                                *active_tool = crate::overlay::Tool::Move;
                                            }
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui: &mut egui::Ui| {
                                                if ui.add(egui::Button::new(egui::RichText::new("✖").color(egui::Color32::RED).size(10.0)).frame(false)).clicked() { object_to_delete = Some((i, crate::project::ObjectType::Image, img_idx)); }
                                                if ui.add(egui::Button::new(egui::RichText::new("fx").size(10.0)).frame(false)).clicked() {
                                                    object_to_select = Some((i, crate::project::ObjectType::Image, img_idx));
                                                    project.active_layer = i;
                                                    *active_tool = crate::overlay::Tool::Move;
                                                }
                                                let mut op_val = (img.opacity * 100.0) as i32;
                                                if ui.add(egui::DragValue::new(&mut op_val).range(0..=100).suffix("%")).changed() { img.opacity = op_val as f32 / 100.0; }
                                            });
                                        });
                                    }
                                    for (t_idx, ann) in layer.text_annotations.iter_mut().enumerate() {
                                        ui.horizontal(|ui: &mut egui::Ui| {
                                            ui.checkbox(&mut ann.visible, "");
                                            let is_sel = project.selected_object == Some(crate::project::SelectedObject { layer_idx: i, object_type: crate::project::ObjectType::Text, object_idx: t_idx });
                                            let name = format!("T \"{}\"", if ann.text.len() > 12 { &ann.text[..10] } else { &ann.text });
                                            let item_resp = ui.selectable_label(is_sel, name);
                                            if item_resp.clicked() {
                                                object_to_select = Some((i, crate::project::ObjectType::Text, t_idx));
                                            }
                                            if item_resp.double_clicked() {
                                                object_to_select = Some((i, crate::project::ObjectType::Text, t_idx));
                                                project.active_layer = i;
                                                *active_tool = crate::overlay::Tool::Move;
                                            }
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui: &mut egui::Ui| {
                                                if ui.add(egui::Button::new(egui::RichText::new("✖").color(egui::Color32::RED).size(10.0)).frame(false)).clicked() { object_to_delete = Some((i, crate::project::ObjectType::Text, t_idx)); }
                                                if ui.add(egui::Button::new(egui::RichText::new("fx").size(10.0)).frame(false)).clicked() {
                                                    object_to_select = Some((i, crate::project::ObjectType::Text, t_idx));
                                                    project.active_layer = i;
                                                    *active_tool = crate::overlay::Tool::Move;
                                                }
                                                let mut op_val = (ann.opacity * 100.0) as i32;
                                                if ui.add(egui::DragValue::new(&mut op_val).range(0..=100).suffix("%")).changed() { ann.opacity = op_val as f32 / 100.0; }
                                            });
                                        });
                                    }
                                    let mut freehand_indices = Vec::new();
                                    for (s_idx, s) in layer.strokes.iter_mut().enumerate() {
                                        if s.kind == crate::overlay::StrokeKind::Freehand {
                                            freehand_indices.push(s_idx);
                                            continue;
                                        }
                                        ui.horizontal(|ui: &mut egui::Ui| {
                                            ui.checkbox(&mut s.visible, "");
                                            let is_sel = project.selected_object == Some(crate::project::SelectedObject { layer_idx: i, object_type: crate::project::ObjectType::Stroke, object_idx: s_idx });
                                            let s_name = match s.kind {
                                                crate::overlay::StrokeKind::Rect => "✏ Rect",
                                                crate::overlay::StrokeKind::Circle => "✏ Circle",
                                                crate::overlay::StrokeKind::Blur => "✏ Blur",
                                                _ => "✏ Stroke",
                                            };
                                            let item_resp = ui.selectable_label(is_sel, format!("{} {}", s_name, s_idx));
                                            if item_resp.clicked() {
                                                object_to_select = Some((i, crate::project::ObjectType::Stroke, s_idx));
                                            }
                                            if item_resp.double_clicked() {
                                                object_to_select = Some((i, crate::project::ObjectType::Stroke, s_idx));
                                                project.active_layer = i;
                                                *active_tool = crate::overlay::Tool::Move;
                                            }
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui: &mut egui::Ui| {
                                                if ui.add(egui::Button::new(egui::RichText::new("✖").color(egui::Color32::RED).size(10.0)).frame(false)).clicked() { object_to_delete = Some((i, crate::project::ObjectType::Stroke, s_idx)); }
                                                if ui.add(egui::Button::new(egui::RichText::new("fx").size(10.0)).frame(false)).clicked() {
                                                    object_to_select = Some((i, crate::project::ObjectType::Stroke, s_idx));
                                                    project.active_layer = i;
                                                    *active_tool = crate::overlay::Tool::Move;
                                                }
                                                let mut op_val = (s.opacity * 100.0) as i32;
                                                if ui.add(egui::DragValue::new(&mut op_val).range(0..=100).suffix("%")).changed() { s.opacity = op_val as f32 / 100.0; }
                                            });
                                        });
                                    }
                                    if !freehand_indices.is_empty() {
                                        ui.horizontal(|ui: &mut egui::Ui| {
                                            let is_sel = project.selected_object.map_or(false, |sel| sel.layer_idx == i && sel.object_type == crate::project::ObjectType::Stroke && freehand_indices.contains(&sel.object_idx));
                                            if ui.selectable_label(is_sel, "🖌 Brush Strokes").clicked() {
                                                object_to_select = Some((i, crate::project::ObjectType::Stroke, freehand_indices[0]));
                                            }
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui: &mut egui::Ui| {
                                                if ui.add(egui::Button::new(egui::RichText::new("✖").color(egui::Color32::RED)).frame(false)).clicked() {
                                                    // We use a special marker index to indicate deleting all freehand strokes
                                                    object_to_delete = Some((i, crate::project::ObjectType::Stroke, usize::MAX));
                                                }
                                            });
                                        });
                                    }
                                });
                            }
                        }
                    });
                }

                if let Some(idx) = layer_to_remove {
                    project.layers.remove(idx);
                    project.selected_object = None;
                    if project.layers.is_empty() { project.layers.push(crate::project::Layer::new("Layer 1")); }
                    project.active_layer = project.active_layer.min(project.layers.len() - 1);
                }
                if let Some(idx) = layer_to_move_up {
                    if idx < total_layers - 1 {
                        project.layers.swap(idx, idx + 1);
                        if project.active_layer == idx { project.active_layer = idx + 1; }
                        else if project.active_layer == idx + 1 { project.active_layer = idx; }
                    }
                }
                if let Some(idx) = layer_to_move_down {
                    if idx > 0 {
                        project.layers.swap(idx, idx - 1);
                        if project.active_layer == idx { project.active_layer = idx - 1; }
                        else if project.active_layer == idx - 1 { project.active_layer = idx; }
                    }
                }
                if let Some((l_idx, obj_type, o_idx)) = object_to_delete {
                    match obj_type {
                        crate::project::ObjectType::Stroke => {
                            if o_idx == usize::MAX {
                                project.layers[l_idx].strokes.retain(|s| s.kind != crate::overlay::StrokeKind::Freehand);
                            } else {
                                project.layers[l_idx].strokes.remove(o_idx);
                            }
                        }
                        crate::project::ObjectType::Text => { project.layers[l_idx].text_annotations.remove(o_idx); }
                        crate::project::ObjectType::Image => { project.layers[l_idx].placed_images.remove(o_idx); }
                    }
                    project.selected_object = None;
                }
                if let Some((l_idx, obj_type, o_idx)) = object_to_clone {
                    match obj_type {
                        crate::project::ObjectType::Stroke => {
                            let mut cloned = project.layers[l_idx].strokes[o_idx].clone();
                            cloned.points.iter_mut().for_each(|p| { p.x += 10.0; p.y += 10.0; });
                            project.layers[l_idx].strokes.push(cloned);
                        }
                        crate::project::ObjectType::Image => {
                            let mut cloned = project.layers[l_idx].placed_images[o_idx].clone();
                            cloned.position.x += 10.0; cloned.position.y += 10.0;
                            cloned.id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
                            cloned.texture = None;
                            project.layers[l_idx].placed_images.push(cloned);
                        }
                        crate::project::ObjectType::Text => {
                            let mut cloned = project.layers[l_idx].text_annotations[o_idx].clone();
                            cloned.position.x += 10.0; cloned.position.y += 10.0;
                            project.layers[l_idx].text_annotations.push(cloned);
                        }
                    }
                }
                if let Some((l_idx, obj_type, o_idx)) = object_to_select {
                    project.active_layer = l_idx;
                    project.selected_object = Some(crate::project::SelectedObject { layer_idx: l_idx, object_type: obj_type, object_idx: o_idx });
                }
            });
        });
}

