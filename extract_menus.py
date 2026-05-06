import os

with open("src/overlay.rs", "r", encoding="utf-8") as f:
    lines = f.readlines()

settings_lines = lines[38:217] + lines[575:581]
layers_lines = lines[217:490]
filter_lines = lines[490:575]

new_overlay_lines = lines[:38] + lines[581:]

with open("src/ui/settings_menu.rs", "w", encoding="utf-8") as f:
    f.write("use eframe::egui;\n")
    f.write("use crate::types::*;\n")
    f.write("use crate::ui::toolbar::photoshop_frame;\n")
    f.write("use crate::hotkey::detect_pressed_key;\n\n")
    f.writelines(settings_lines)

with open("src/ui/layer_menu.rs", "w", encoding="utf-8") as f:
    f.write("use eframe::egui;\n")
    f.write("use crate::types::*;\n")
    f.write("use crate::project::{Project, Layer};\n")
    f.write("use crate::ui::toolbar::photoshop_frame;\n\n")
    f.writelines(layers_lines)

with open("src/ui/filter_menu.rs", "w", encoding="utf-8") as f:
    f.write("use eframe::egui;\n")
    f.write("use crate::types::*;\n")
    f.write("use crate::project::Project;\n")
    f.write("use crate::ui::toolbar::photoshop_frame;\n\n")
    f.writelines(filter_lines)

with open("src/overlay.rs", "w", encoding="utf-8") as f:
    f.writelines(new_overlay_lines)
