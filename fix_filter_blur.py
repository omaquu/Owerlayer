with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\ui\filter_menu.rs', 'r', encoding='utf-8') as f:
    c = f.read()

old_blur = '''                ui.horizontal(|ui| {
                    let mut bl = layer.blur > 0.0;
                    if ui.checkbox(&mut bl, "Blur").changed() {
                        layer.blur = if bl { 10.0 } else { 0.0 };
                    }
                    if bl {
                        let mut val = layer.blur.max(0.2);
                        if ui.add(egui::Slider::new(&mut val, 0.2..=100.0).step_by(0.2)).changed() {
                            layer.blur = val.max(0.2);
                        }
                    }
                });'''

new_blur = '''                ui.horizontal(|ui| {
                    let mut bl = layer.blur > 0.0;
                    if ui.checkbox(&mut bl, "Blur").changed() {
                        layer.blur = if bl { 10.0 } else { 0.0 };
                    }
                    if bl {
                        let mut val = layer.blur;
                        if ui.add(egui::DragValue::new(&mut val).speed(0.1).range(0.0..=100.0)).changed() {
                            layer.blur = val;
                        }
                    }
                });'''

c = c.replace(old_blur, new_blur)

with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\ui\filter_menu.rs', 'w', encoding='utf-8', newline='\n') as f:
    f.write(c)

print("filter_menu blur updated.")
