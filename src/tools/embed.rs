use eframe::egui;
use crate::types::*;
use crate::utils::*;
use crate::overlay::*;

use crate::tools::ToolContext;

pub fn update(ctx: &mut ToolContext) {
    let project = &mut *ctx.project;
    let active_tool = &mut *ctx.active_tool;
    let last_tool_used = &mut *ctx.last_tool_used;
    let ui = &mut *ctx.ui;
    let canvas_response = ctx.canvas_response;
    let painter = ui.painter_at(canvas_response.rect);
    let pos = ctx.mouse.pos;
    let left_just_pressed = ctx.mouse.left_just_pressed;

                painter.text(pos + egui::vec2(10.0, 10.0), egui::Align2::LEFT_TOP, "Click to place Embed Widget", egui::FontId::proportional(12.0), egui::Color32::WHITE);
                if left_just_pressed {
                    let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
                    let mut new_img = PlacedImage::new(
                        id,
                        pos,
                        [400, 300],
                        vec![200; 400 * 300 * 4], // Gray placeholder
                    );
                    new_img.url = Some("about:blank".to_string());
                    new_img.is_live = true;
                    project.layers[project.active_layer].placed_images.push(new_img);
                    
                    project.selected_object = Some(crate::project::SelectedObject {
                        layer_idx: project.active_layer,
                        object_type: crate::project::ObjectType::Image,
                        object_idx: project.layers[project.active_layer].placed_images.len() - 1,
                    });
                    *last_tool_used = Some(*active_tool);
                    *active_tool = Tool::Move;
                }

}
