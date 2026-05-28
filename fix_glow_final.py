with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\ui\object_fx.rs', 'r', encoding='utf-8') as f:
    c = f.read()

# Find the exact glow block
glow_start = c.find('                        ui.horizontal(|ui| {\n                            ui.checkbox(&mut .glow, "Glow");')
if glow_start == -1:
    print('Failed to find glow start')
else:
    glow_end = c.find('\n                        });', glow_start) + len('\n                        });')
    
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
    
    c = c[:glow_start] + new_glow + c[glow_end:]
    
    with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\ui\object_fx.rs', 'w', encoding='utf-8', newline='\n') as f:
        f.write(c)
    print('Glow replaced successfully!')
