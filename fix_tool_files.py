import os
import re

tools = [
    "brush", "shape", "snip", "cut", "mirror", "blur", "text", "embed", "move_tool", "eraser"
]

vars_to_deref = ["snip_created", "remove_active_layer", "switch_to_move", "active_tool", "last_tool_used", "new_selection", "pending_text", "initial_bounds", "initial_center", "initial_layer", "drag_state", "dragging_source_rect", "embed_trigger", "line_start"]

context_bindings = """
    let project = &mut *ctx.project;
    let settings = &mut *ctx.settings;
    let mouse = ctx.mouse;
    let current_stroke = &mut *ctx.current_stroke;
    let line_start = &mut *ctx.line_start;
    let active_tool = &mut *ctx.active_tool;
    let last_tool_used = &mut *ctx.last_tool_used;
    let remove_active_layer = &mut *ctx.remove_active_layer;
    let snip_created = &mut *ctx.snip_created;
    let new_selection = &mut *ctx.new_selection;
    let switch_to_move = &mut *ctx.switch_to_move;
    let embed_trigger = &mut *ctx.embed_trigger;
    let pending_text = &mut *ctx.pending_text;
    let initial_bounds = &mut *ctx.initial_bounds;
    let initial_center = &mut *ctx.initial_center;
    let initial_layer = &mut *ctx.initial_layer;
    let drag_state = &mut *ctx.drag_state;
    let dragging_source_rect = &mut *ctx.dragging_source_rect;
    let ui = &mut *ctx.ui;
    let canvas_response = ctx.canvas_response;
    let painter = ui.painter_at(canvas_response.rect);
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

overlay_imports = "use crate::overlay::{is_inside_poly, magic_wand_flood_fill, layer_bounds, translate_layer, scale_layer, rotate_layer, skew_layer};\n"

for tool in tools:
    path = f"src/tools/{tool}.rs"
    if not os.path.exists(path):
        continue
    
    with open(path, "r", encoding="utf-8") as f:
        content = f.read()
    
    # 1. Update imports
    if "use crate::overlay" in content:
        content = re.sub(r"use crate::overlay::\{.*?\};", overlay_imports, content)
    else:
        content = content.replace("use crate::types::*;\n", "use crate::types::*;\n" + overlay_imports)

    # 2. Fix assignments in the body
    for var in vars_to_deref:
        # Match 'var =' or 'var  =' but not '*var =' or '.var ='
        # Using word boundaries \b to avoid matching sub-words
        content = re.sub(rf"(?<![\*\.])\b{var}\b\s*=", f"*{var} =", content)

    # 3. Update context bindings
    content = re.sub(r"pub fn update\(ctx: &mut ToolContext\) \{.*?let frame_count = ctx.frame_count;", 
                     f"pub fn update(ctx: &mut ToolContext) {{\n{context_bindings}", content, flags=re.DOTALL)

    with open(path, "w", encoding="utf-8") as f:
        f.write(content)
