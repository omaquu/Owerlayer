import re

with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\ui\object_fx.rs', 'r', encoding='utf-8') as f:
    c = f.read()

# 1. Glow
# Original:
# ui.checkbox(&mut .glow, "Glow");
# if .glow {
#     ui.add(egui::DragValue::new(&mut .glow_strength).range(0.0..=100.0).prefix("Glow: "));
# }
glow_pattern = r'ui\.checkbox\(&mut \\.glow, "Glow"\);\s*if \\.glow \{\s*ui\.add\(egui::DragValue::new\(&mut \\.glow_strength\)\.range\(0\.0\.\.=100\.0\)\.prefix\("Glow: "\)\);\s*\}'
glow_repl = '''ui.checkbox(&mut .glow, "Glow");
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
                        // Missing closing brace to match original structure? No, original had ui.horizontal(|ui| { ... });.
                        // The pattern matches INSIDE the horizontal closure.
                        // Let's replace the whole horizontal block.
'''

c = re.sub(r'ui\.horizontal\(\|ui\|\s*\{\s*ui\.checkbox\(&mut \\.glow, "Glow"\);\s*if \\.glow \{\s*ui\.add\(egui::DragValue::new\(&mut \\.glow_strength\)\.range\(0\.0\.\.=100\.0\)\.prefix\("Glow: "\)\);\s*\}\s*\}\);', 
           '''ui.horizontal(|ui| {
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
                        }''', c)

# 2. Blur
# Original:
# if bl {
#     let mut val = .blur.max(0.2);
#     if ui.add(egui::DragValue::new(&mut val).range(0.2..=100.0)).changed() {
#         .blur = val;
#     }
# }
c = re.sub(r'let mut val = \\.blur\.max\(0\.2\);\s*if ui\.add\(egui::DragValue::new\(&mut val\)\.range\(0\.2\.\.=100\.0\)\)\.changed\(\)\s*\{\s*\\.blur = val;\s*\}',
           '''let mut val = .blur.max(0.2);
                                    if ui.add(egui::Slider::new(&mut val, 0.2..=100.0).step_by(0.2)).changed() {
                                        .blur = val.max(0.2);
                                    }''', c)

with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\ui\object_fx.rs', 'w', encoding='utf-8', newline='\n') as f:
    f.write(c)
print("object_fx.rs patched.")
