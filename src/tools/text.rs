use eframe::egui;
use crate::types::*;
use crate::utils::*;
use crate::overlay::*;

use crate::tools::ToolContext;

/// Resolve a TextFont + size to an egui FontId.
/// Falls back to proportional (system default) for named fonts that may not be loaded.
pub fn resolve_font(font: TextFont, size: f32) -> egui::FontId {
    match font {
        TextFont::Sans        => egui::FontId::proportional(size),
        TextFont::Mono        => egui::FontId::monospace(size),
        // Named families may not be registered; fall back to proportional gracefully
        TextFont::Serif       => egui::FontId::new(size, egui::FontFamily::Name("serif".into())),
        TextFont::Handwriting => egui::FontId::new(size, egui::FontFamily::Name("handwriting".into())),
        TextFont::Heading     => egui::FontId::new(size, egui::FontFamily::Name("heading".into())),
        TextFont::Custom      => egui::FontId::proportional(size),
    }
}

pub fn update(ctx: &mut ToolContext) {
    if ctx.mouse.left_just_pressed {
        ctx.auto_create_layer();
    }
    if ctx.project.layers.is_empty() { return; }
    let project  = &mut *ctx.project;
    let settings = &mut *ctx.settings;
    let mouse    = ctx.mouse;
    let pending_text    = &mut *ctx.pending_text;
    let ui              = &mut *ctx.ui;
    let canvas_response = ctx.canvas_response;
    let pos      = mouse.pos;
    let left_just_pressed = mouse.left_just_pressed;
    let active_layer_idx  = project.active_layer;

    let layer = &mut project.layers[active_layer_idx];

    // ── Commit: pending_text is finalized in main.rs; we handle commit here only for
    //    consistency (main.rs takes care of Enter/Escape). Nothing else to do. ──

    // ── Double-click to re-edit an existing annotation ──
    if canvas_response.double_clicked() {
        for (idx, ann) in layer.text_annotations.iter().enumerate() {
            // Estimate bounding box — use font_size * ~0.6 per char width
            let approx_width = ann.text.chars().count() as f32 * ann.font_size * 0.55 + 10.0;
            let rect = egui::Rect::from_min_size(
                ann.position,
                egui::vec2(approx_width, ann.font_size * 1.4),
            );
            if rect.contains(pos) {
                // Restore the annotation's settings into the toolbar
                settings.pen_color    = ann.color;
                settings.text_font    = ann.font;
                settings.font_size    = ann.font_size;
                settings.text_shadow  = ann.shadow;
                settings.text_outline = ann.outline;
                settings.text_wave_warp = ann.wave_warp;
                *pending_text = Some(PendingText { position: ann.position, buffer: ann.text.clone() });
                layer.text_annotations.remove(idx);
                return; // Re-editing takes precedence
            }
        }
    }

    // ── Paint over existing text to apply current settings ──
    let mut painted_over = false;
    if mouse.left_down && pending_text.is_none() {
        for ann in layer.text_annotations.iter_mut() {
            let approx_width = ann.text.chars().count() as f32 * ann.font_size * 0.55 + 10.0;
            let rect = egui::Rect::from_min_size(
                ann.position,
                egui::vec2(approx_width, ann.font_size * 1.4),
            );
            if rect.contains(pos) {
                ann.font = settings.text_font;
                ann.color = settings.pen_color;
                ann.font_size = settings.font_size;
                ann.shadow = settings.text_shadow;
                ann.outline = settings.text_outline;
                ann.wave_warp = settings.text_wave_warp;
                painted_over = true;
            }
        }
    }

    if left_just_pressed && pending_text.is_none() && !painted_over {
        // Start new text entry at click position
        *pending_text = Some(PendingText { position: pos, buffer: String::new() });
    }
}

// ──────────────────────────────────────────────────────────────
//  Layer text rendering
// ──────────────────────────────────────────────────────────────

pub fn draw_layer_text(
    p: &egui::Painter,
    layer: &crate::project::Layer,
    render_offset: egui::Vec2,
    l_op: f32,
    settings: &Settings,
    time: f32,
) {
    for ann in layer.text_annotations.iter() {
        if !ann.visible { continue; }

        let font = resolve_font(ann.font, ann.font_size);

        let mut c = color32(&ann.color);
        c = egui::Color32::from_rgba_unmultiplied(
            c.r(), c.g(), c.b(),
            (c.a() as f32 * l_op * ann.opacity) as u8,
        );

        let outline_col = if c.r() as u32 + c.g() as u32 + c.b() as u32 > 382 {
            egui::Color32::from_black_alpha((255.0 * l_op * ann.opacity) as u8)
        } else {
            egui::Color32::from_white_alpha((255.0 * l_op * ann.opacity) as u8)
        };

        let shadow_col = egui::Color32::from_black_alpha((150.0 * l_op * ann.opacity) as u8);

        if ann.wave_warp {
            // ── Wave warp: render character-by-character with sinusoidal vertical offset ──
            draw_wave_text(p, ann, &font, c, outline_col, shadow_col, render_offset, layer, settings, l_op, time);
        } else {
            // ── Normal rendering with proper mesh tessellation for transforms ──
            let text_shape = |color: egui::Color32, offset: egui::Vec2| {
                let galley = p.fonts(|f| f.layout_no_wrap(ann.text.clone(), font.clone(), color));
                egui::Shape::galley(ann.position + offset, galley, color)
            };

            let mut shapes = Vec::new();
            let sw = if ann.stroke_width > 0.0 { ann.stroke_width } else { 1.0 };
            
            if layer.shadow || ann.shadow || settings.text_shadow {
                shapes.push(egui::epaint::ClippedShape { clip_rect: egui::Rect::EVERYTHING, shape: text_shape(shadow_col, egui::vec2(2.0, 2.0)) });
            }
            if layer.outline || ann.outline || settings.text_outline {
                for off in [egui::vec2(sw, sw), egui::vec2(-sw, -sw), egui::vec2(sw, -sw), egui::vec2(-sw, sw)] {
                    shapes.push(egui::epaint::ClippedShape { clip_rect: egui::Rect::EVERYTHING, shape: text_shape(outline_col, off) });
                }
            }
            shapes.push(egui::epaint::ClippedShape { clip_rect: egui::Rect::EVERYTHING, shape: text_shape(c, egui::vec2(0.0, 0.0)) });

            let primitives = p.ctx().tessellate(shapes, p.ctx().pixels_per_point());
            
            let rect = egui::Rect::from_min_size(ann.position, egui::vec2(ann.text.len() as f32 * ann.font_size * 0.6, ann.font_size * 1.2));
            let center = rect.center();

            for primitive in primitives {
                if let egui::epaint::Primitive::Mesh(mut mesh) = primitive.primitive {
                    // Apply relative transform
                    for v in &mut mesh.vertices {
                        v.pos -= center.to_vec2();
                        if ann.flipped_h { v.pos.x = -v.pos.x; }
                        if ann.flipped_v { v.pos.y = -v.pos.y; }
                    }
                    
                    // Apply transform_mesh for rotation/skew/perspective/scale
                    transform_mesh(&mut mesh, center, ann.rotation, ann.skew, ann.perspective, ann.scale);
                    
                    // Apply render offset
                    for v in &mut mesh.vertices {
                        v.pos += render_offset;
                    }
                    p.add(egui::Shape::mesh(mesh));
                }
            }
        }
    }
}

/// Render text with a sine-wave vertical offset per character.
fn draw_wave_text(
    p: &egui::Painter,
    ann: &TextAnnotation,
    font: &egui::FontId,
    c: egui::Color32,
    outline_col: egui::Color32,
    shadow_col: egui::Color32,
    render_offset: egui::Vec2,
    layer: &crate::project::Layer,
    settings: &Settings,
    l_op: f32,
    time: f32,
) {
    let wave_amplitude = ann.font_size * 0.25;
    let wave_frequency = std::f32::consts::TAU / (ann.font_size * 3.5);
    // Approximate character width from font metrics (use ~0.55× font_size as monospace estimate)
    let char_w = ann.font_size * 0.55;
    let sw = if ann.stroke_width > 0.0 { ann.stroke_width } else { 1.0 };

    let rect = egui::Rect::from_min_size(ann.position, egui::vec2(ann.text.len() as f32 * char_w, ann.font_size * 1.2));
    let center = rect.center();

    for (i, ch) in ann.text.chars().enumerate() {
        let lx = i as f32 * char_w;
        let wave_y = wave_amplitude * (wave_frequency * (ann.position.x + lx) - time * 5.0).sin();
        
        let char_pos_raw = ann.position + egui::vec2(lx, wave_y);
        let transformed_pos = transform_point_complex(char_pos_raw, center, ann.rotation, ann.skew, ann.perspective, rect, ann.scale);
        
        let char_str: String = std::iter::once(ch).collect();
        let char_pos = transformed_pos + render_offset;

        if layer.shadow || ann.shadow || settings.text_shadow {
            p.add(egui::Shape::text(&p.fonts(|f| f.clone()), char_pos + egui::vec2(2.0, 2.0), egui::Align2::LEFT_TOP, &char_str, font.clone(), shadow_col));
        }
        if layer.outline || ann.outline || settings.text_outline {
            for off in [egui::vec2(sw, sw), egui::vec2(-sw, -sw), egui::vec2(sw, -sw), egui::vec2(-sw, sw)] {
                p.add(egui::Shape::text(&p.fonts(|f| f.clone()), char_pos + off, egui::Align2::LEFT_TOP, &char_str, font.clone(), outline_col));
            }
        }
        p.add(egui::Shape::text(&p.fonts(|f| f.clone()), char_pos, egui::Align2::LEFT_TOP, &char_str, font.clone(), c));
    }
    let _ = l_op; // used via colors already
}
