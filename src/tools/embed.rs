use eframe::egui;
use crate::overlay::*;
use crate::tools::ToolContext;

pub fn update(ctx: &mut ToolContext) {
    if *ctx.layer_prompt_open { return; }
    let left_just_pressed = ctx.mouse.left_just_pressed;
    let pos = ctx.mouse.pos;
    let ui = &mut *ctx.ui;
    let canvas_response = ctx.canvas_response;
    let painter = ui.painter_at(canvas_response.rect);

    painter.text(pos + egui::vec2(10.0, 10.0), egui::Align2::LEFT_TOP, "Click to place Embed Widget", egui::FontId::proportional(12.0), egui::Color32::WHITE);
    if left_just_pressed {
        ctx.auto_create_layer();
        let project = &mut *ctx.project;
        let active_tool = &mut *ctx.active_tool;
        let last_tool_used = &mut *ctx.last_tool_used;
        
        if project.active_layer < project.layers.len() {
            let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
            let mut new_img = PlacedImage::new(
                id,
                pos,
                [400, 300],
                vec![200; 400 * 300 * 4], // Gray placeholder
            );
            new_img.url = Some("about:blank".to_string());
            new_img.is_live = true;

            #[cfg(feature = "webengine")]
            if crate::web_engine::is_available() {
                if let Some(widget) = crate::web_engine::create_widget("about:blank", 400, 300) {
                    new_img.pixels = widget.pixels.clone();
                    new_img.web_widget = Some(std::sync::Arc::new(std::sync::Mutex::new(widget)));
                }
            }

            let layer = &mut project.layers[project.active_layer];
            layer.placed_images.push(new_img);
            layer.expanded = true;
            
            project.selected_object = Some(SelectedObject {
                layer_idx: project.active_layer,
                object_type: ObjectType::Image,
                object_idx: layer.placed_images.len() - 1,
            });
            *last_tool_used = Some(*active_tool);
            *active_tool = Tool::Move;
        }
    }
}
