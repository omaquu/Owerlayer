use eframe::egui;
use crate::project::Project;
use crate::types::{Settings, ObjectType};
use crate::utils::color32;
use crate::ui::settings_menu::section_heading;

pub fn render_fx_window(ctx: &egui::Context, project: &mut Project, settings: &mut Settings) {
    if let Some(sel) = settings.fx_open {
        let frame = crate::ui::toolbar::photoshop_frame(settings);
        let accent = color32(&settings.accent_color);
        
        let mut request_rasterize = false;
        
        let win_resp = egui::Window::new("Object Effects")
            .title_bar(false)
            .resizable(true)
            .collapsible(false)
            .frame(frame)
            .default_pos(settings.object_fx_menu_pos)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Object FX").strong().color(accent));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("✖").clicked() { settings.fx_open = None; }
                    });
                });
                ui.separator();
                
                if sel.layer_idx >= project.layers.len() {
                    settings.fx_open = None;
                    return;
                }

                let layer = &mut project.layers[sel.layer_idx];

                macro_rules! render_object_fx {
                    ($obj:expr, $is_vector:expr) => {
                        section_heading(ui, "Shadow / Glow", accent);
                        ui.checkbox(&mut $obj.shadow, "Enable Drop Shadow");
                        if $obj.shadow {
                            ui.horizontal(|ui| {
                                ui.label("Distance:");
                                ui.add(egui::DragValue::new(&mut $obj.shadow_offset[0]).speed(0.1).prefix("X:"));
                                ui.add(egui::DragValue::new(&mut $obj.shadow_offset[1]).speed(0.1).prefix("Y:"));
                            });
                            ui.horizontal(|ui| {
                                ui.label("Color:");
                                let mut c = egui::Color32::from_rgba_unmultiplied($obj.shadow_color[0], $obj.shadow_color[1], $obj.shadow_color[2], $obj.shadow_color[3]);
                                if ui.color_edit_button_srgba(&mut c).changed() {
                                    $obj.shadow_color = [c.r(), c.g(), c.b(), c.a()];
                                }
                                ui.add_space(8.0);
                                ui.label("Spread:");
                                ui.add(egui::Slider::new(&mut $obj.shadow_blur, 0.0..=50.0));
                            });
                        }

                        ui.add_space(8.0);
                        section_heading(ui, "Outline / Stroke", accent);
                        ui.checkbox(&mut $obj.outline, "Enable Outline");
                        if $obj.outline {
                            ui.horizontal(|ui| {
                                ui.label("Thickness:");
                                ui.add(egui::Slider::new(&mut $obj.outline_width, 0.5..=20.0));
                            });
                            ui.horizontal(|ui| {
                                ui.label("Color:");
                                let mut c = egui::Color32::from_rgba_unmultiplied($obj.outline_color[0], $obj.outline_color[1], $obj.outline_color[2], $obj.outline_color[3]);
                                if ui.color_edit_button_srgba(&mut c).changed() {
                                    $obj.outline_color = [c.r(), c.g(), c.b(), c.a()];
                                }
                            });
                        }

                        ui.add_space(8.0);
                        section_heading(ui, "Opacity & Visibility", accent);
                        ui.horizontal(|ui| {
                            ui.label("Opacity:");
                            let mut op = $obj.opacity * 100.0;
                            if ui.add(egui::Slider::new(&mut op, 0.0..=100.0).suffix("%")).changed() {
                                $obj.opacity = op / 100.0;
                            }
                        });
                        ui.checkbox(&mut $obj.visible, "Visible");
                        ui.add_space(8.0);

                        section_heading(ui, "Color & Effects", accent);
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut $obj.grayscale, "Grayscale");
                            ui.checkbox(&mut $obj.invert, "Invert");
                            ui.checkbox(&mut $obj.sepia, "Sepia");
                        });
                        
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut $obj.glow, "Glow");
                        });
                        if $obj.glow {
                            ui.horizontal(|ui| {
                                ui.label("Color:");
                                let mut gc = egui::Color32::from_rgba_unmultiplied($obj.glow_color[0], $obj.glow_color[1], $obj.glow_color[2], $obj.glow_color[3]);
                                if ui.color_edit_button_srgba(&mut gc).changed() {
                                    $obj.glow_color = [gc.r(), gc.g(), gc.b(), gc.a()];
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Strength:");
                                ui.add(egui::Slider::new(&mut $obj.glow_strength, 0.0..=100.0).suffix("%"));
                            });
                        }
                        
                        if !$is_vector {
                            ui.horizontal(|ui| {
                                let mut bl = $obj.blur >= 0.0;
                                if ui.checkbox(&mut bl, "Blur").changed() {
                                    $obj.blur = if bl { 10.0 } else { -1.0 };
                                }
                                if bl {
                                    let mut val = $obj.blur;
                                    if ui.add(egui::DragValue::new(&mut val).range(0.0..=300.0)).changed() {
                                        $obj.blur = val;
                                    }
                                }
                            });
                            if $obj.blur >= 0.0 {
                                ui.horizontal(|ui| {
                                    ui.selectable_value(&mut $obj.blur_effect, crate::types::BlurEffect::Gaussian, "Gaus");
                                    ui.selectable_value(&mut $obj.blur_effect, crate::types::BlurEffect::Pixelate, "Pix");
                                    ui.selectable_value(&mut $obj.blur_effect, crate::types::BlurEffect::Glitch, "VHS");
                                });
                            }
                        } else {
                            ui.label(egui::RichText::new("Blur FX requires rasterized image.").size(10.0).color(egui::Color32::from_gray(120)));
                            if ui.button("Rasterize Object").clicked() {
                                request_rasterize = true;
                            }
                        }
                    };
                }

                match sel.object_type {
                    ObjectType::Image => {
                        if sel.object_idx < layer.placed_images.len() {
                            let img = &mut layer.placed_images[sel.object_idx];
                            render_object_fx!(img, false);
                        }
                    }
                    ObjectType::Text => {
                        if sel.object_idx < layer.text_annotations.len() {
                            let ann = &mut layer.text_annotations[sel.object_idx];
                            render_object_fx!(ann, true);
                        }
                    }
                    ObjectType::Stroke => {
                        if sel.object_idx < layer.strokes.len() {
                            let s = &mut layer.strokes[sel.object_idx];
                            render_object_fx!(s, true);
                        }
                    }
                }
            });
            
        if let Some(resp) = win_resp {
            if resp.response.dragged() {
                let id = resp.response.layer_id;
                if let Some(rect) = ctx.memory(|m| m.area_rect(id.id)) {
                    settings.object_fx_menu_pos = rect.min;
                }
            }
        }

        if request_rasterize {
            project.rasterize_request = Some(crate::types::RasterizeRequest {
                layer_idx: sel.layer_idx,
                object_idx: Some((sel.object_type, sel.object_idx)),
            });
        }
    }
}
