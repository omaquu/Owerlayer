import re
with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\ui\layer_menu.rs', 'r', encoding='utf-8') as f:
    c = f.read()

macro_def = '''
macro_rules! has_fx {
    () => {
        .shadow || .glow || .outline || .blur > 0.0 || .grayscale || .invert || .sepia
    }
}
'''

# Insert macro at the top after imports
idx = c.find('pub fn render_layer_menu')
if idx != -1:
    c = c[:idx] + macro_def + '\n' + c[idx:]

# 1. Layer FX button
# if ui.button("fx").clicked() { *filters_open = Some(i); }
layer_btn_find = 'if ui.button("fx").clicked() { *filters_open = Some(i); }'
layer_btn_replace = '''let mut layer_fx_btn = egui::Button::new("fx");
                                if has_fx!(layer) { layer_fx_btn = layer_fx_btn.fill(egui::Color32::from_rgb(100, 140, 200)); }
                                if ui.add(layer_fx_btn).clicked() { *filters_open = Some(i); }'''
c = c.replace(layer_btn_find, layer_btn_replace)

# 2. Image FX button
c = re.sub(r'if ui\.add\(egui::Button::new\(egui::RichText::new\("fx"\)\.size\(10\.0\)\)\.frame\(settings\.fx_open == Some\(crate::types::SelectedObject \{ layer_idx: i, object_type: crate::types::ObjectType::Image, object_idx: img_idx \}\)\)\)\.clicked\(\) \{',
           r'''let is_open = settings.fx_open == Some(crate::types::SelectedObject { layer_idx: i, object_type: crate::types::ObjectType::Image, object_idx: img_idx });
                                                    let mut btn = egui::Button::new(egui::RichText::new("fx").size(10.0)).frame(is_open);
                                                    if has_fx!(img) { btn = btn.fill(egui::Color32::from_rgb(100, 140, 200)); }
                                                    if ui.add(btn).clicked() {''', c)

# 3. Text FX button
c = re.sub(r'if ui\.add\(egui::Button::new\(egui::RichText::new\("fx"\)\.size\(10\.0\)\)\.frame\(settings\.fx_open == Some\(crate::types::SelectedObject \{ layer_idx: i, object_type: crate::types::ObjectType::Text, object_idx: t_idx \}\)\)\)\.clicked\(\) \{',
           r'''let is_open = settings.fx_open == Some(crate::types::SelectedObject { layer_idx: i, object_type: crate::types::ObjectType::Text, object_idx: t_idx });
                                                    let mut btn = egui::Button::new(egui::RichText::new("fx").size(10.0)).frame(is_open);
                                                    if has_fx!(t) { btn = btn.fill(egui::Color32::from_rgb(100, 140, 200)); }
                                                    if ui.add(btn).clicked() {''', c)

# 4. Stroke FX button (Normal)
c = re.sub(r'if ui\.add\(egui::Button::new\(egui::RichText::new\("fx"\)\.size\(10\.0\)\)\.frame\(settings\.fx_open == Some\(crate::types::SelectedObject \{ layer_idx: i, object_type: crate::types::ObjectType::Stroke, object_idx: s_idx \}\)\)\)\.clicked\(\) \{',
           r'''let is_open = settings.fx_open == Some(crate::types::SelectedObject { layer_idx: i, object_type: crate::types::ObjectType::Stroke, object_idx: s_idx });
                                                    let mut btn = egui::Button::new(egui::RichText::new("fx").size(10.0)).frame(is_open);
                                                    if has_fx!(s) { btn = btn.fill(egui::Color32::from_rgb(100, 140, 200)); }
                                                    if ui.add(btn).clicked() {''', c)

# 5. Stroke FX button (Freehand group)
c = re.sub(r'if ui\.add\(egui::Button::new\(egui::RichText::new\("fx"\)\.size\(10\.0\)\)\.frame\(settings\.fx_open == Some\(crate::types::SelectedObject \{ layer_idx: i, object_type: crate::types::ObjectType::Stroke, object_idx: freehand_indices\[0\] \}\)\)\)\.clicked\(\) \{',
           r'''let is_open = settings.fx_open == Some(crate::types::SelectedObject { layer_idx: i, object_type: crate::types::ObjectType::Stroke, object_idx: freehand_indices[0] });
                                                    let mut btn = egui::Button::new(egui::RichText::new("fx").size(10.0)).frame(is_open);
                                                    let first_stroke = &layer.strokes[freehand_indices[0]];
                                                    if has_fx!(first_stroke) { btn = btn.fill(egui::Color32::from_rgb(100, 140, 200)); }
                                                    if ui.add(btn).clicked() {''', c)


with open(r'C:\Users\kuron\.gemini\antigravity\WorkSpace\Owerlayer\src\ui\layer_menu.rs', 'w', encoding='utf-8', newline='\n') as f:
    f.write(c)

print("layer_menu.rs patched for FX button color.")
