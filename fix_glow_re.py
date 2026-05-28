with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\ui\object_fx.rs', 'r', encoding='utf-8') as f:
    c = f.read()

import re

c = re.sub(
    r'ui\.checkbox\(&mut \\.glow, "Glow"\);\s*if \\.glow \{\s*ui\.add\(egui::DragValue::new\(&mut \\.glow_strength\)\.range\(0\.0\.\.=100\.0\)\.prefix\("Glow: "\)\);\s*\}\s*\n\s*\}\);',
    '''ui.checkbox(&mut .glow, "Glow");
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
                        }''',
    c
)

with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\ui\object_fx.rs', 'w', encoding='utf-8', newline='\n') as f:
    f.write(c)

print("regex replace done")
