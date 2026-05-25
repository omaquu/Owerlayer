use eframe::egui;
use crate::project::Project;
use crate::types::{Settings, BlurEffect};
use crate::utils::color32;
use crate::ui::toolbar::photoshop_frame;
use crate::ui::settings_menu::section_heading;

pub fn render_filter_menu(
    ctx: &egui::Context,
    project: &mut Project,
    settings: &mut Settings,
    filters_open: &mut Option<usize>,
) {
    if let Some(idx) = *filters_open {
        if idx >= project.layers.len() {
            *filters_open = None;
            return;
        }

        let accent = color32(&settings.accent_color);
        let frame = photoshop_frame(settings);
        let layer_name = project.layers[idx].name.clone();

        let win_resp = egui::Window::new(format!("Layer Filters: {}", layer_name))
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .default_pos(settings.filter_menu_pos)
            .frame(frame)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Layer FX").size(11.0).color(accent));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("✖").clicked() { *filters_open = None; }
                    });
                });
                ui.separator();

                let layer = &mut project.layers[idx];

                // --- Shadow ---
                section_heading(ui, "Shadow / Glow", accent);
                ui.checkbox(&mut layer.shadow, "Enable Drop Shadow");
                if layer.shadow {
                    ui.horizontal(|ui| {
                        ui.label("Distance:");
                        ui.add(egui::DragValue::new(&mut layer.shadow_offset[0]).speed(0.1).prefix("X:"));
                        ui.add(egui::DragValue::new(&mut layer.shadow_offset[1]).speed(0.1).prefix("Y:"));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Color:");
                        let mut c = egui::Color32::from_rgba_unmultiplied(layer.shadow_color[0], layer.shadow_color[1], layer.shadow_color[2], layer.shadow_color[3]);
                        if ui.color_edit_button_srgba(&mut c).changed() {
                            layer.shadow_color = [c.r(), c.g(), c.b(), c.a()];
                        }
                        ui.add_space(8.0);
                        ui.label("Spread:");
                        ui.add(egui::Slider::new(&mut layer.shadow_blur, 0.0..=50.0));
                    });
                }

                // --- Outline ---
                ui.add_space(8.0);
                section_heading(ui, "Outline / Stroke", accent);
                ui.checkbox(&mut layer.outline, "Enable Outline");
                if layer.outline {
                    ui.horizontal(|ui| {
                        ui.label("Thickness:");
                        ui.add(egui::Slider::new(&mut layer.outline_width, 0.5..=20.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Color:");
                        let mut c = egui::Color32::from_rgba_unmultiplied(layer.outline_color[0], layer.outline_color[1], layer.outline_color[2], layer.outline_color[3]);
                        if ui.color_edit_button_srgba(&mut c).changed() {
                            layer.outline_color = [c.r(), c.g(), c.b(), c.a()];
                        }
                    });
                }

                // --- Opacity & Visibility ---
                ui.add_space(8.0);
                section_heading(ui, "Opacity & Visibility", accent);
                ui.horizontal(|ui| {
                    ui.label("Opacity:");
                    let mut op = layer.opacity * 100.0;
                    if ui.add(egui::Slider::new(&mut op, 0.0..=100.0).suffix("%")).changed() {
                        layer.opacity = op / 100.0;
                    }
                });
                ui.checkbox(&mut layer.visible, "Visible");

                // --- Color & Effects ---
                ui.add_space(8.0);
                section_heading(ui, "Color & Effects", accent);
                ui.horizontal(|ui| {
                    ui.checkbox(&mut layer.grayscale, "Grayscale");
                    ui.checkbox(&mut layer.invert, "Invert");
                    ui.checkbox(&mut layer.sepia, "Sepia");
                });

                // Glow
                ui.horizontal(|ui| {
                    ui.checkbox(&mut layer.glow, "Glow");
                });
                if layer.glow {
                    ui.horizontal(|ui| {
                        ui.label("Color:");
                        let mut gc = egui::Color32::from_rgba_unmultiplied(layer.glow_color[0], layer.glow_color[1], layer.glow_color[2], layer.glow_color[3]);
                        if ui.color_edit_button_srgba(&mut gc).changed() {
                            layer.glow_color = [gc.r(), gc.g(), gc.b(), gc.a()];
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Strength:");
                        ui.add(egui::Slider::new(&mut layer.glow_strength, 0.0..=100.0).suffix("%"));
                    });
                }

                // Blur
                ui.horizontal(|ui| {
                    let mut bl = layer.blur >= 0.0;
                    if ui.checkbox(&mut bl, "Blur").changed() {
                        layer.blur = if bl { 10.0 } else { -1.0 };
                    }
                    if bl {
                        let mut val = layer.blur;
                        if ui.add(egui::DragValue::new(&mut val).range(0.0..=300.0)).changed() {
                            layer.blur = val;
                        }
                    }
                });
                if layer.blur >= 0.0 {
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut layer.blur_effect, BlurEffect::Gaussian, "Gaus");
                        ui.selectable_value(&mut layer.blur_effect, BlurEffect::Pixelate, "Pix");
                        ui.selectable_value(&mut layer.blur_effect, BlurEffect::Glitch, "VHS");
                    });
                }

                ui.add_space(8.0);
                if ui.button("Close").clicked() {
                    *filters_open = None;
                }
            });

        if let Some(resp) = win_resp {
            if resp.response.dragged() {
                let id = resp.response.layer_id;
                if let Some(rect) = ctx.memory(|m| m.area_rect(id.id)) {
                    settings.filter_menu_pos = rect.min;
                }
            }
        }

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            *filters_open = None;
        }
    }
}

