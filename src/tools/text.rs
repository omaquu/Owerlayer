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
        if ctx.project.get_active_layer().map_or(false, |l| l.locked) {
            *ctx.layer_prompt_open = true;
            return;
        }
        ctx.auto_create_layer();
    }
    
    if ctx.project.get_active_layer().map_or(false, |l| l.locked) {
        return;
    }
    if ctx.project.layers.is_empty() { return; }
    let project  = &mut *ctx.project;
    let settings = &mut *ctx.settings;
    let mouse    = ctx.mouse;
    let pending_text    = &mut *ctx.pending_text;
    let _ui              = &mut *ctx.ui;
    let canvas_response = ctx.canvas_response;
    let pos      = mouse.pos;
    let left_just_pressed = mouse.left_just_pressed;
    let active_layer_idx  = project.active_layer;

    let layer = if let Some(l) = project.get_active_layer_mut() {
        l
    } else {
        return;
    };

    // ── Commit: pending_text is finalized in main.rs; we handle commit here only for
    //    consistency (main.rs takes care of Enter/Escape). Nothing else to do. ──

    // ── Double-click to re-edit an existing annotation ──
    if canvas_response.double_clicked() {
        for (idx, ann) in layer.text_annotations.iter().enumerate() {
            let rect = egui::Rect::from_min_size(
                ann.position,
                egui::vec2(ann.exact_size[0], ann.exact_size[1]),
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
            let rect = egui::Rect::from_min_size(
                ann.position,
                egui::vec2(ann.exact_size[0], ann.exact_size[1]),
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

        let (o_col_arr, o_width) = if ann.outline { (ann.outline_color, ann.outline_width) } else { (layer.outline_color, layer.outline_width) };
        let outline_col = egui::Color32::from_rgba_unmultiplied(o_col_arr[0], o_col_arr[1], o_col_arr[2], (o_col_arr[3] as f32 * l_op * ann.opacity) as u8);

        let (s_col_arr, s_off) = if ann.shadow { (ann.shadow_color, ann.shadow_offset) } else { (layer.shadow_color, layer.shadow_offset) };
        let shadow_col = egui::Color32::from_rgba_unmultiplied(s_col_arr[0], s_col_arr[1], s_col_arr[2], (s_col_arr[3] as f32 * l_op * ann.opacity) as u8);

        let is_gray = ann.grayscale || layer.grayscale;
        let is_inv = ann.invert || layer.invert;
        let is_sepia = ann.sepia || layer.sepia;
        let is_glow = ann.glow || layer.glow;
        let glow_str = if ann.glow { ann.glow_strength } else { layer.glow_strength };

        if ann.wave_warp {
            // ── Wave warp: render character-by-character with sinusoidal vertical offset ──
            let wave_c = apply_color_effects(c, is_gray, is_inv, is_sepia, is_glow, glow_str);
            let wave_out = apply_color_effects(outline_col, is_gray, is_inv, is_sepia, is_glow, glow_str);
            let wave_shad = apply_color_effects(shadow_col, is_gray, is_inv, is_sepia, is_glow, glow_str);
            draw_wave_text(p, ann, &font, wave_c, wave_out, wave_shad, render_offset, layer, settings, l_op, time);
        } else {
            // ── Normal rendering with proper mesh tessellation for transforms ──
            let text_shape = |color: egui::Color32, offset: egui::Vec2| {
                let galley = p.fonts(|f| f.layout_no_wrap(ann.text.clone(), font.clone(), color));
                egui::Shape::galley(ann.position + offset, galley, color)
            };

            let mut shapes = Vec::new();
            let sw = if o_width > 0.0 { o_width } else if ann.stroke_width > 0.0 { ann.stroke_width } else { 1.0 };
            
            if layer.shadow || ann.shadow || settings.text_shadow {
                shapes.push(egui::epaint::ClippedShape { clip_rect: egui::Rect::EVERYTHING, shape: text_shape(shadow_col, egui::vec2(s_off[0], s_off[1])) });
            }
            if layer.outline || ann.outline || settings.text_outline {
                for off in [egui::vec2(sw, sw), egui::vec2(-sw, -sw), egui::vec2(sw, -sw), egui::vec2(-sw, sw)] {
                    shapes.push(egui::epaint::ClippedShape { clip_rect: egui::Rect::EVERYTHING, shape: text_shape(outline_col, off) });
                }
            }
            shapes.push(egui::epaint::ClippedShape { clip_rect: egui::Rect::EVERYTHING, shape: text_shape(c, egui::vec2(0.0, 0.0)) });

            let primitives = p.ctx().tessellate(shapes, p.ctx().pixels_per_point());
            
            let rect = egui::Rect::from_min_size(ann.position, egui::vec2(ann.exact_size[0], ann.exact_size[1]));
            let center = rect.center();

            for primitive in primitives {
                if let egui::epaint::Primitive::Mesh(mut mesh) = primitive.primitive {
                    // Apply relative transform
                    for v in &mut mesh.vertices {
                        v.pos -= center.to_vec2();
                    }
                    
                    // Apply transform_mesh for rotation/skew/perspective/scale
                    let mut final_scale = ann.scale;
                    if ann.flipped_h { final_scale.x *= -1.0; }
                    if ann.flipped_v { final_scale.y *= -1.0; }
                    transform_mesh(&mut mesh, center, ann.rotation, ann.skew, ann.perspective, final_scale);
                    
                    // Apply Filters
                    apply_mesh_filters(&mut mesh, is_gray, is_inv, is_sepia, is_glow, glow_str);
                    
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

    let rect = egui::Rect::from_min_size(ann.position, egui::vec2(ann.exact_size[0], ann.exact_size[1]));
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
