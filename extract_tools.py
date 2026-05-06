import os

with open("src/overlay.rs", "r", encoding="utf-8") as f:
    lines = f.readlines()

def extract_block(start_line_idx):
    brace_count = 0
    end_idx = start_line_idx
    started = False
    for i in range(start_line_idx, len(lines)):
        line = lines[i]
        if "{" in line:
            brace_count += line.count("{")
            started = True
        if "}" in line:
            brace_count -= line.count("}")
        if started and brace_count == 0:
            end_idx = i
            break
    return end_idx

tools = [
    ("Brush", 902),
    ("Shape", 928),
    ("Snip", 957),
    ("Cut", 1150),
    ("Mirror", 1293),
    ("Blur", 1478),
    ("Text", 1508),
    ("Embed", 1535),
    ("Move", 1558),
    ("Eraser", 2058)
]

os.makedirs("src/tools", exist_ok=True)

context_bindings = """
    let project = &mut ctx.project;
    let settings = &mut ctx.settings;
    let mouse = ctx.mouse;
    let current_stroke = &mut ctx.current_stroke;
    let line_start = &mut ctx.line_start;
    let active_tool = &mut ctx.active_tool;
    let last_tool_used = &mut ctx.last_tool_used;
    let remove_active_layer = &mut ctx.remove_active_layer;
    let snip_created = &mut ctx.snip_created;
    let new_selection = &mut ctx.new_selection;
    let switch_to_move = &mut ctx.switch_to_move;
    let embed_trigger = &mut ctx.embed_trigger;
    let ui = &mut ctx.ui;
    let painter = ui.painter_at(ctx.canvas_response.rect);
    let pos = mouse.pos;
    let left_down = mouse.left_down;
    let left_just_pressed = mouse.left_just_pressed;
    let left_just_released = mouse.left_just_released;
    let right_just_pressed = ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Secondary));
    let active_layer_idx = project.active_layer;
    let ppp = ui.ctx().pixels_per_point();
    let render_offset = ctx.render_offset;
    let frame_count = ctx.frame_count;
"""

for tool_name, start_idx in tools:
    end_idx = extract_block(start_idx)
    content = "".join(lines[start_idx+1:end_idx])
    
    file_name = tool_name.lower()
    if file_name == "move": file_name = "move_tool"
    
    with open(f"src/tools/{file_name}.rs", "w", encoding="utf-8") as f:
        f.write("use eframe::egui;\n")
        f.write("use crate::types::*;\n")
        f.write("use crate::tools::ToolContext;\n")
        if file_name == "blur" or file_name == "snip":
            f.write("use crate::ui::toolbar::{apply_box_blur, apply_pixelate, apply_vhs_glitch};\n")
        f.write("\npub fn update(ctx: &mut ToolContext) {\n")
        f.write(context_bindings)
        f.write(content)
        f.write("\n}\n")

# Now modify overlay.rs to remove the extracted code and replace with calls.
# We have to do it backwards to not mess up indices.
new_lines = lines.copy()

for tool_name, start_idx in reversed(tools):
    end_idx = extract_block(start_idx)
    file_name = tool_name.lower()
    if file_name == "move": file_name = "move_tool"
    
    replacement = f"            Tool::{tool_name} => {{\n                crate::tools::{file_name}::update(&mut ctx);\n            }}\n"
    new_lines[start_idx:end_idx+1] = [replacement]

with open("src/overlay.rs", "w", encoding="utf-8") as f:
    f.writelines(new_lines)
