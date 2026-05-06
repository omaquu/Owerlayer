import os

with open("src/overlay.rs", "r", encoding="utf-8") as f:
    lines = f.readlines()

types_lines = lines[10:601]
overlay_top = lines[0:10]
overlay_bottom = lines[601:]

with open("src/types.rs", "w", encoding="utf-8") as f:
    f.write("use eframe::egui;\n")
    f.write("use serde::{Deserialize, Serialize};\n")
    f.write("use crate::hotkey::HotkeyBinding;\n\n")
    f.writelines(types_lines)

with open("src/overlay.rs", "w", encoding="utf-8") as f:
    f.writelines(overlay_top)
    f.write("pub use crate::types::*;\n")
    f.writelines(overlay_bottom)
