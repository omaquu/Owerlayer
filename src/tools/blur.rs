use eframe::egui;
use crate::types::*;
use crate::utils::*;
use crate::overlay::*;

use crate::tools::ToolContext;
use crate::ui::toolbar::{apply_box_blur, apply_pixelate, apply_vhs_glitch};

pub fn update(ctx: &mut ToolContext) {
    let project = &mut *ctx.project;
    let settings = &mut *ctx.settings;
    let mouse = ctx.mouse;
    let line_start = &mut *ctx.line_start;
    let ui = &mut *ctx.ui;
    let canvas_response = ctx.canvas_response;
    let painter = ui.painter_at(canvas_response.rect);
    let pos = mouse.pos;
    let left_just_pressed = mouse.left_just_pressed;
    let left_just_released = mouse.left_just_released;
    let render_offset = ctx.render_offset;

            if left_just_pressed { *line_start = Some(pos); }
            if left_just_released {
                if let Some(start) = line_start.take() {
                    let rect = egui::Rect::from_two_pos(start, pos);
                    let w = rect.width();
                    let h = rect.height();
                    if w > 4.0 && h > 4.0 {
                        let mut blur_layer_idx = None;
                        for (idx, l) in project.layers.iter().enumerate() {
                            if l.name == "Blurs" { blur_layer_idx = Some(idx); break; }
                        }
                        let l_idx = if let Some(idx) = blur_layer_idx {
                            idx
                        } else {
                            project.layers.push(crate::project::Layer::new("Blurs"));
                            project.layers.len() - 1
                        };
                        let target_layer = &mut project.layers[l_idx];

                        let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
                        let mut img = PlacedImage::new(id, rect.min, [rect.width() as usize, rect.height() as usize], Vec::new());
                        img.is_live = true;
                        img.blur = settings.blur_strength;
                        img.blur_effect = settings.blur_effect;
                        target_layer.placed_images.push(img);
                    }
                }
            }

}

pub fn render_preview(ctx: &mut ToolContext) {
    let start = match ctx.line_start { Some(s) => *s, None => return };
    let pos = ctx.mouse.pos;
    let render_offset = ctx.render_offset;
    let painter = ctx.ui.painter_at(ctx.canvas_response.rect);
    
    let rect = egui::Rect::from_two_pos(start, pos).translate(-render_offset);
    painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, egui::Color32::WHITE), egui::StrokeKind::Middle);
    painter.rect_filled(rect, 0.0, egui::Color32::from_black_alpha(40));
}
