with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\ui\object_fx.rs', 'r', encoding='utf-8') as f:
    c = f.read()

import re

# 1. Patch Glow
old_glow = '''                        ui.horizontal(|ui| {
                            ui.checkbox(&mut .glow, "Glow");
                            if .glow {
                                ui.add(egui::DragValue::new(&mut .glow_strength).range(0.0..=100.0).prefix("Glow: "));
                            }
                        });'''

new_glow = '''                        ui.horizontal(|ui| {
                            ui.checkbox(&mut .glow, "Glow");
                        });
                        if .glow {
                            ui.horizontal(|ui| {
                                ui.label("Color:");
                                let mut gc = egui::Color32::from_rgba_unmultiplied(.glow_color[0], .glow_color[1], .glow_color[2], .glow_color[3]);
                                if ui.color_edit_button_srgba(&mut gc).changed() {
                                    .glow_color = [gc.r(), gc.g(), gc.b(), gc.a()];
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Strength:");
                                ui.add(egui::Slider::new(&mut .glow_strength, 0.0..=100.0).suffix("%"));
                            });
                            ui.horizontal(|ui| {
                                ui.label("Spread:");
                                ui.add(egui::Slider::new(&mut .glow_spread, 0.0..=80.0));
                            });
                        }'''

if old_glow in c:
    c = c.replace(old_glow, new_glow)
    print("Glow patched.")
else:
    print("GLOW NOT FOUND!")

# 2. Patch Blur
old_blur = '''                                if bl {
                                    let mut val = .blur.max(0.2);
                                    if ui.add(egui::DragValue::new(&mut val).range(0.2..=100.0)).changed() {
                                        .blur = val;
                                    }
                                }'''

new_blur = '''                                if bl {
                                    let mut val = .blur.max(0.2);
                                    if ui.add(egui::Slider::new(&mut val, 0.2..=100.0).step_by(0.2)).changed() {
                                        .blur = val.max(0.2);
                                    }
                                }'''

if old_blur in c:
    c = c.replace(old_blur, new_blur)
    print("Blur patched.")
else:
    print("BLUR NOT FOUND!")

with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\ui\object_fx.rs', 'w', encoding='utf-8', newline='\n') as f:
    f.write(c)

