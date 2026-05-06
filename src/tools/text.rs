use eframe::egui;
use crate::types::*;
use crate::utils::*;
use crate::overlay::*;

use crate::tools::ToolContext;

pub fn update(ctx: &mut ToolContext) {
    if ctx.mouse.left_just_pressed {
        ctx.auto_create_layer();
    }
    let project = &mut *ctx.project;
    let settings = &mut *ctx.settings;
    let mouse = ctx.mouse;
    let pending_text = &mut *ctx.pending_text;
    let ui = &mut *ctx.ui;
    let canvas_response = ctx.canvas_response;
    let painter = ui.painter_at(canvas_response.rect);
    let pos = mouse.pos;
    let left_just_pressed = mouse.left_just_pressed;
    let active_layer_idx = project.active_layer;
    let render_offset = ctx.render_offset;

    let layer = &mut project.layers[active_layer_idx];
            
            let mut finished_text = None;
            if let Some(pending) = pending_text.as_mut() {
                let time = ui.input(|i| i.time);
                let blink = (time * 3.0).sin() > 0.0;
                let font = match settings.text_font {
                    TextFont::Sans => egui::FontId::proportional(settings.font_size),
                    TextFont::Serif => egui::FontId::new(settings.font_size, egui::FontFamily::Name("serif".into())),
                    TextFont::Mono => egui::FontId::monospace(settings.font_size),
                    TextFont::Handwriting => egui::FontId::new(settings.font_size, egui::FontFamily::Name("handwriting".into())),
                    TextFont::Heading => egui::FontId::new(settings.font_size, egui::FontFamily::Name("heading".into())),
                    TextFont::Custom => egui::FontId::proportional(settings.font_size),
                };

                ui.input(|i| {
                    for event in &i.events {
                        match event {
                            egui::Event::Text(text) => {
                                pending.buffer.push_str(text);
                            }
                            egui::Event::Key { key: egui::Key::Backspace, pressed: true, .. } => {
                                pending.buffer.pop();
                            }
                            egui::Event::Key { key: egui::Key::Enter, pressed: true, modifiers, .. } => {
                                if modifiers.shift {
                                    pending.buffer.push('\n');
                                } else {
                                    finished_text = Some(PendingText { position: pending.position, buffer: pending.buffer.clone() });
                                }
                            }
                            egui::Event::Key { key: egui::Key::Escape, pressed: true, .. } => {
                                finished_text = Some(PendingText { position: pending.position, buffer: pending.buffer.clone() });
                            }
                            _ => {}
                        }
                    }
                });

                if finished_text.is_none() {
                    let mut display_text = pending.buffer.clone();
                    if blink { display_text.push('|'); }
                    
                    let pen_c = color32(&settings.pen_color);
                    if settings.text_outline {
                        let c = pen_c;
                        let outline_col = if c.r() as u32 + c.g() as u32 + c.b() as u32 > 382 { egui::Color32::BLACK } else { egui::Color32::WHITE };
                        painter.text(pending.position - render_offset + egui::vec2(1.0, 1.0), egui::Align2::LEFT_TOP, &display_text, font.clone(), outline_col);
                        painter.text(pending.position - render_offset + egui::vec2(-1.0, -1.0), egui::Align2::LEFT_TOP, &display_text, font.clone(), outline_col);
                        painter.text(pending.position - render_offset + egui::vec2(1.0, -1.0), egui::Align2::LEFT_TOP, &display_text, font.clone(), outline_col);
                        painter.text(pending.position - render_offset + egui::vec2(-1.0, 1.0), egui::Align2::LEFT_TOP, &display_text, font.clone(), outline_col);
                    } else if settings.text_shadow {
                        painter.text(pending.position - render_offset + egui::vec2(2.0, 2.0), egui::Align2::LEFT_TOP, &display_text, font.clone(), egui::Color32::from_black_alpha(150));
                    }
                    painter.text(pending.position - render_offset, egui::Align2::LEFT_TOP, &display_text, font.clone(), pen_c);
                }
                ui.ctx().request_repaint();
            }
            
            // Commit finished text
            if let Some(p) = finished_text {
                if !p.buffer.is_empty() {
                    let mut ann = TextAnnotation::new(p.position, p.buffer, settings.pen_color, settings.font_size);
                    ann.monospace = settings.text_monospace;
                    ann.shadow = settings.text_shadow;
                    ann.outline = settings.text_outline;
                    ann.stroke_width = settings.text_stroke_width;
                    ann.font = settings.text_font;
                    layer.text_annotations.push(ann);
                }
                *pending_text = None;
            }

            if canvas_response.double_clicked() {
                for (idx, ann) in layer.text_annotations.iter().enumerate() {
                    let rect = egui::Rect::from_min_size(ann.position, egui::vec2(ann.text.len() as f32 * ann.font_size * 0.6, ann.font_size * 1.2));
                    if rect.contains(pos) {
                        *pending_text = Some(PendingText { position: ann.position, buffer: ann.text.clone() });
                        layer.text_annotations.remove(idx);
                        break;
                    }
                }
            } else if left_just_pressed && pending_text.is_none() {
                *pending_text = Some(PendingText { position: pos, buffer: String::new() });
            }

}

pub fn draw_layer_text(p: &egui::Painter, layer: &crate::project::Layer, render_offset: egui::Vec2, l_op: f32, settings: &Settings) {
    for ann in layer.text_annotations.iter() {
        let font = match ann.font {
            TextFont::Sans => egui::FontId::proportional(ann.font_size),
            TextFont::Serif => egui::FontId::new(ann.font_size, egui::FontFamily::Name("serif".into())),
            TextFont::Mono => egui::FontId::monospace(ann.font_size),
            TextFont::Handwriting => egui::FontId::new(ann.font_size, egui::FontFamily::Name("handwriting".into())),
            TextFont::Heading => egui::FontId::new(ann.font_size, egui::FontFamily::Name("heading".into())),
            TextFont::Custom => {
                egui::FontId::proportional(ann.font_size)
            }
        };
        let mut c = color32(&ann.color);
        c = egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (c.a() as f32 * l_op * ann.opacity) as u8);
        
        let outline_col = if c.r() as u32 + c.g() as u32 + c.b() as u32 > 382 { 
            egui::Color32::from_black_alpha((255.0 * l_op * ann.opacity) as u8)
        } else { 
            egui::Color32::from_white_alpha((255.0 * l_op * ann.opacity) as u8)
        };
        
        if layer.shadow || ann.shadow || settings.text_shadow {
            p.text(ann.position + render_offset + egui::vec2(2.0, 2.0), egui::Align2::LEFT_TOP, &ann.text, font.clone(), egui::Color32::from_black_alpha((150.0 * l_op * ann.opacity) as u8));
        }

        if layer.outline || ann.outline || settings.text_outline {
            let sw = if ann.stroke_width > 0.0 { ann.stroke_width } else { 1.0 };
            p.text(ann.position + render_offset + egui::vec2(sw, sw), egui::Align2::LEFT_TOP, &ann.text, font.clone(), outline_col);
            p.text(ann.position + render_offset + egui::vec2(-sw, -sw), egui::Align2::LEFT_TOP, &ann.text, font.clone(), outline_col);
            p.text(ann.position + render_offset + egui::vec2(sw, -sw), egui::Align2::LEFT_TOP, &ann.text, font.clone(), outline_col);
            p.text(ann.position + render_offset + egui::vec2(-sw, sw), egui::Align2::LEFT_TOP, &ann.text, font.clone(), outline_col);
        }
        
        p.text(ann.position + render_offset, egui::Align2::LEFT_TOP, &ann.text, font, c);
    }
}
