with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\ui\object_fx.rs', 'r', encoding='utf-8') as f:
    c = f.read()

import re

glow_match = re.search(r'(ui\.horizontal\(\|ui\|\s*\{\s*ui\.checkbox\(&mut \\.glow, "Glow"\);\s*if \\.glow\s*\{\s*ui\.add\(egui::DragValue::new\(&mut \\.glow_strength\)\.range\(0\.0\.\.=100\.0\)\.prefix\("Glow: "\)\);\s*\}\s*\}\);)', c)
if glow_match:
    old_glow = glow_match.group(1)
    new_glow = '''ui.horizontal(|ui| {
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
                                ui.add(egui::DragValue::new(&mut .glow_strength).range(0.0..=100.0).prefix("Glow: "));
                            });
                            ui.horizontal(|ui| {
                                ui.label("Spread:");
                                ui.add(egui::Slider::new(&mut .glow_spread, 0.0..=80.0));
                            });
                        }'''
    c = c.replace(old_glow, new_glow)
    print("Replaced glow block in object_fx.rs")
else:
    print("Failed to match glow block in object_fx.rs")

blur_match = re.search(r'(let mut val = \\.blur\.max\(0\.2\);\s*if ui\.add\(egui::DragValue::new\(&mut val\)\.range\(0\.2\.\.=100\.0\)\)\.changed\(\)\s*\{\s*\\.blur = val;\s*\})', c)
if blur_match:
    old_blur = blur_match.group(1)
    new_blur = '''let mut val = .blur;
                                    if ui.add(egui::DragValue::new(&mut val).speed(0.1).range(0.0..=100.0)).changed() {
                                        .blur = val;
                                    }'''
    c = c.replace(old_blur, new_blur)
    print("Replaced blur block in object_fx.rs")
else:
    print("Failed to match blur block in object_fx.rs")

with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\ui\object_fx.rs', 'w', encoding='utf-8', newline='\n') as f:
    f.write(c)

