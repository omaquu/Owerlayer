with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\ui\object_fx.rs', 'r', encoding='utf-8') as f:
    c = f.read()

old_blur = '''                                if bl {
                                    let mut val = .blur.max(0.2);
                                    if ui.add(egui::DragValue::new(&mut val).range(0.2..=100.0)).changed() {
                                        .blur = val;
                                    }
                                }'''

new_blur = '''                                if bl {
                                    let mut val = .blur.max(0.2);
                                    if ui.add(egui::Slider::new(&mut val, 0.2..=100.0).step_by(0.2)).changed() {
                                        .blur = val;
                                    }
                                }'''

if old_blur in c:
    c = c.replace(old_blur, new_blur)
    with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\ui\object_fx.rs', 'w', encoding='utf-8', newline='\n') as f:
        f.write(c)
    print('object_fx.rs blur patched.')
else:
    print('Pattern not found in object_fx.rs for blur')
