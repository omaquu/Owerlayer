import os

with open("src/overlay.rs", "r", encoding="utf-8") as f:
    lines = f.readlines()

# photoshop_frame starts at 34 (idx 33)
# render_toolbar ends at 484 (idx 483)
toolbar_lines = lines[33:485]

new_overlay_lines = lines[:33] + lines[485:]

with open("src/ui/toolbar.rs", "w", encoding="utf-8") as f:
    f.write("use eframe::egui;\n")
    f.write("use crate::types::*;\n")
    f.write("use crate::project::Project;\n\n")
    f.writelines(toolbar_lines)

with open("src/overlay.rs", "w", encoding="utf-8") as f:
    f.writelines(new_overlay_lines)
