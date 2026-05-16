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
                
                // Helper to render filter controls
                let mut render_filters = |ui: &mut egui::Ui, grayscale: &mut bool, invert: &mut bool, sepia: &mut bool, glow: &mut bool, glow_strength: &mut f32, blur: &mut f32, blur_effect: &mut crate::overlay::BlurEffect| {
                    ui.horizontal(|ui| {
                        ui.checkbox(grayscale, "Grayscale");
                        ui.checkbox(invert, "Invert");
                        ui.checkbox(sepia, "Sepia");
                    });
                    ui.horizontal(|ui| {
                        ui.checkbox(glow, "Glow");
                        if *glow {
                            ui.add(egui::DragValue::new(glow_strength).range(0.0..=100.0).prefix("Glow: "));
                        }
                    });
                    ui.horizontal(|ui| {
                        let mut bl = *blur > 0.0;
                        if ui.checkbox(&mut bl, "Blur").changed() {
                            *blur = if bl { 10.0 } else { 0.0 };
                        }
                        if *blur > 0.0 {
                            ui.add(egui::DragValue::new(blur).range(0.0..=100.0));
                        }
                    });
                    if *blur > 0.0 {
                        ui.horizontal(|ui| {
                            ui.selectable_value(blur_effect, crate::overlay::BlurEffect::Gaussian, "Gaus");
                            ui.selectable_value(blur_effect, crate::overlay::BlurEffect::Pixelate, "Pix");
                            ui.selectable_value(blur_effect, crate::overlay::BlurEffect::Glitch, "VHS");
                        });
                    }
                };

                match sel.object_type {
                    crate::types::ObjectType::Image => {
                        if sel.object_idx < layer.placed_images.len() {
                            let img = &mut layer.placed_images[sel.object_idx];
                            ui.checkbox(&mut img.shadow, "Shadow");
                            ui.checkbox(&mut img.outline, "Outline");
                            render_filters(ui, &mut img.grayscale, &mut img.invert, &mut img.sepia, &mut img.glow, &mut img.glow_strength, &mut img.blur, &mut img.blur_effect);
                        }
                    }
                    crate::types::ObjectType::Text => {
                        if sel.object_idx < layer.text_annotations.len() {
                            let ann = &mut layer.text_annotations[sel.object_idx];
                            ui.checkbox(&mut ann.shadow, "Shadow");
                            ui.checkbox(&mut ann.outline, "Outline");
                            render_filters(ui, &mut ann.grayscale, &mut ann.invert, &mut ann.sepia, &mut ann.glow, &mut ann.glow_strength, &mut ann.blur, &mut ann.blur_effect);
                        }
                    }
                    crate::types::ObjectType::Stroke => {
                        if sel.object_idx < layer.strokes.len() {
                            let s = &mut layer.strokes[sel.object_idx];
                            ui.checkbox(&mut s.shadow, "Shadow");
                            ui.checkbox(&mut s.outline, "Outline");
                            render_filters(ui, &mut s.grayscale, &mut s.invert, &mut s.sepia, &mut s.glow, &mut s.glow_strength, &mut s.blur, &mut s.blur_effect);
                        }
                    }
                }
            });
    }
}
