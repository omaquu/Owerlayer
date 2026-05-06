use eframe::egui;
use crate::types::*;
use crate::utils::color32;
// use crate::project::Project;
use crate::ui::toolbar::photoshop_frame;
use crate::ui::settings_menu::section_heading;

pub fn render_filter_menu(
    ctx: &egui::Context,
    project: &mut crate::project::Project,
    settings: &Settings,
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

        egui::Window::new(format!("Layer Filters: {}", layer_name))
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .frame(frame)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Layer Filters").size(11.0).color(accent));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("✖").clicked() { *filters_open = None; }
                    });
                });
                ui.separator();
                
                let layer = &mut project.layers[idx];

                section_heading(ui, "Shadow / Glow", accent);
                ui.checkbox(&mut layer.shadow, "Enable Drop Shadow");
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
                });

                ui.add_space(8.0);
                section_heading(ui, "Outline / Stroke", accent);
                ui.checkbox(&mut layer.outline, "Enable Outline");
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
                ui.add_space(8.0);

                if ui.button("Close").clicked() {
                    *filters_open = None;
                }
            });
        
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            *filters_open = None;
        }
    }
}

