use eframe::egui;
use crate::project::Project;
use crate::types::Settings;

pub fn render_fx_window(ctx: &egui::Context, project: &mut Project, settings: &mut Settings) {
    if let Some(sel) = settings.fx_open {
        let frame = crate::ui::toolbar::photoshop_frame(settings);
        egui::Window::new("Object Effects")
            .title_bar(false)
            .resizable(true)
            .collapsible(false)
            .frame(frame)
            .default_pos(settings.layer_menu_pos + egui::vec2(-260.0, 0.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Object FX").strong().color(egui::Color32::from_rgb(180, 180, 255)));
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
                match sel.object_type {
                    crate::types::ObjectType::Image => {
                        if sel.object_idx < layer.placed_images.len() {
                            let img = &mut layer.placed_images[sel.object_idx];
                            ui.checkbox(&mut img.shadow, "Shadow");
                            ui.checkbox(&mut img.outline, "Outline");
                            let mut bl = img.blur > 0.0;
                            if ui.checkbox(&mut bl, "Blur").changed() {
                                img.blur = if bl { 10.0 } else { 0.0 };
                            }
                            if img.blur > 0.0 {
                                ui.add(egui::DragValue::new(&mut img.blur).range(0.0..=100.0).prefix("Strength: "));
                                ui.horizontal(|ui| {
                                    ui.selectable_value(&mut img.blur_effect, crate::overlay::BlurEffect::Gaussian, "Gaus");
                                    ui.selectable_value(&mut img.blur_effect, crate::overlay::BlurEffect::Pixelate, "Pix");
                                    ui.selectable_value(&mut img.blur_effect, crate::overlay::BlurEffect::Glitch, "VHS");
                                });
                            }
                        }
                    }
                    crate::types::ObjectType::Text => {
                        if sel.object_idx < layer.text_annotations.len() {
                            let ann = &mut layer.text_annotations[sel.object_idx];
                            ui.checkbox(&mut ann.shadow, "Shadow");
                            ui.checkbox(&mut ann.outline, "Outline");
                        }
                    }
                    crate::types::ObjectType::Stroke => {
                        if sel.object_idx < layer.strokes.len() {
                            let s = &mut layer.strokes[sel.object_idx];
                            ui.checkbox(&mut s.shadow, "Shadow");
                            ui.checkbox(&mut s.outline, "Outline");
                        }
                    }
                }
            });
    }
}
