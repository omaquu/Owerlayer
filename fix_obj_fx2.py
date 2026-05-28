with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\ui\object_fx.rs', 'r', encoding='utf-8') as f:
    c = f.read()

# GLOW REPLACEMENT
idx = c.find('ui.checkbox(&mut .glow, "Glow");')
if idx != -1:
    start_idx = c.rfind('ui.horizontal(|ui| {', 0, idx)
    end_idx = c.find('});', idx) + 3
    if start_idx != -1 and end_idx != -1:
        old_glow = c[start_idx:end_idx]
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
        print("Replaced glow block.")

# BLUR REPLACEMENT
idx = c.find('let mut val = .blur.max(0.2);')
if idx != -1:
    end_idx = c.find('}', idx)
    end_idx = c.find('}', end_idx + 1) + 1
    if end_idx != -1:
        old_blur = c[idx:end_idx]
        new_blur = '''let mut val = .blur;
                                    if ui.add(egui::DragValue::new(&mut val).speed(0.1).range(0.0..=100.0)).changed() {
                                        .blur = val;
                                    }'''
        c = c.replace(old_blur, new_blur)
        print("Replaced blur block.")

with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\ui\object_fx.rs', 'w', encoding='utf-8', newline='\n') as f:
    f.write(c)
