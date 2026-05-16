use eframe::egui;
use eframe::glow::{self, HasContext};
use std::sync::Arc;
// use serde::{Deserialize, Serialize};
use crate::utils::*;
use crate::tools::ToolContext;

// use crate::hotkey::{HotkeyBinding, detect_pressed_key};

// ──────────────────────────────────────────────────────────────
//  Data types
// ──────────────────────────────────────────────────────────────

pub use crate::types::*;
use crate::ui::toolbar::{apply_box_blur, apply_pixelate, apply_vhs_glitch};

// ──────────────────────────────────────────────────────────────
//  Color helpers
// ──────────────────────────────────────────────────────────────



// ──────────────────────────────────────────────────────────────
//  Settings window
// ──────────────────────────────────────────────────────────────
// ──────────────────────────────────────────────────────────────
//  Mode indicator
// ──────────────────────────────────────────────────────────────

pub fn render_mode_indicator(ctx: &egui::Context, edit_mode: bool, hotkey_name: &str, toggle_mode: bool, settings: &Settings, owl_icon: &Option<egui::TextureHandle>) {
    if !edit_mode || settings.hide_edit_info { return; }
    let hint = if toggle_mode {
        format!("EDIT MODE  |  press {} to exit", hotkey_name)
    } else {
        format!("EDIT MODE  |  release {} to exit", hotkey_name)
    };

    egui::Area::new(egui::Id::new("mode_indicator"))
        .anchor(egui::Align2::CENTER_TOP, [0.0, 8.0])
        .interactable(false)
        .show(ctx, |ui| {
            egui::Frame {
                fill: egui::Color32::from_rgba_premultiplied(18, 18, 24, 200),
                inner_margin: egui::Margin::symmetric(16, 5),
                corner_radius: egui::CornerRadius::same(16),
                stroke: egui::Stroke::new(1.0, egui::Color32::from_rgba_premultiplied(255, 255, 255, 25)),
                ..Default::default()
            }
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if let Some(tex) = owl_icon {
                        ui.add(egui::Image::new(tex).fit_to_exact_size(egui::vec2(16.0, 16.0)));
                    } else {
                        ui.label(egui::RichText::new("🦉").size(14.0));
                    }
                    ui.add(egui::Label::new(egui::RichText::new(hint).size(12.0).color(egui::Color32::from_gray(200))).truncate());
                });
            });
        });
}

// ──────────────────────────────────────────────────────────────
//  Canvas
// ──────────────────────────────────────────────────────────────


pub fn render_canvas(
    ui: &mut egui::Ui,
    active_tool: &mut Tool,
    settings: &mut Settings,
    project: &mut crate::project::Project,
    current_stroke: &mut Vec<egui::Pos2>,
    line_start: &mut Option<egui::Pos2>,
    initial_bounds: &mut Option<egui::Rect>,
    initial_center: &mut Option<egui::Pos2>,
    initial_layer: &mut Option<crate::project::Layer>,
    drag_state: &mut usize,
    dragging_source_rect: &mut bool,
    pending_text: &mut Option<PendingText>,
    last_tool_used: &mut Option<Tool>,
    edit_mode: bool,
    layer_prompt_open: &mut bool,
    request_history_push: &mut Option<String>,
    mouse: &MouseState,
    can_draw: bool,
    _embed_trigger: &mut bool,
    frame_count: u64,
    _capture_thread: &crate::capture_thread::CaptureThread,
    gl_renderer: Option<Arc<crate::gl_renderer::GLRenderer>>,
) {
    let rect = ui.available_rect_before_wrap();
    
    let ppp = ui.ctx().pixels_per_point();
    let (wx, wy) = crate::winapi_utils::get_window_screen_pos();
    let win_offset = if settings.use_absolute_screen_coords {
        egui::vec2(wx as f32 / ppp, wy as f32 / ppp)
    } else {
        egui::Vec2::ZERO
    };

    let render_offset = win_offset;

    // Use egui's native logical coordinates, translated to global if needed
    let mut pos = ui.input(|i| i.pointer.hover_pos()).unwrap_or(mouse.pos);
    if settings.use_absolute_screen_coords {
        pos += win_offset;
    }
    
    let _left_down = ui.input(|i| i.pointer.primary_down());
    let _left_just_pressed = ui.input(|i| i.pointer.primary_pressed());
    let _left_just_released = ui.input(|i| i.pointer.primary_released());

    // Use click_and_drag to consume events and prevent eframe from initiating a modal window drag (which causes freezing)
    let (canvas_response, painter) = ui.allocate_painter(rect.size(), egui::Sense::click_and_drag());

    // ── Window picking logic ──
    if settings.picking_window {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
        if mouse.left_just_pressed {
            let ppp = ui.ctx().pixels_per_point();
            let (wx, wy) = crate::winapi_utils::get_window_screen_pos();
            let sx = (mouse.pos.x * ppp) as i32 + wx;
            let sy = (mouse.pos.y * ppp) as i32 + wy;
            if let Some((hwnd, _title, _rect)) = crate::winapi_utils::get_window_at_point(sx, sy) {
                if let Some(sel) = project.selected_object {
                    if sel.object_type == ObjectType::Image {
                        project.layers[sel.layer_idx].placed_images[sel.object_idx].hwnd = hwnd;
                        project.layers[sel.layer_idx].placed_images[sel.object_idx].is_live = true;
                    }
                }
            }
            settings.picking_window = false;
        }
        return;
    }

    // ── Pipette picking logic ──
    if settings.picking_fill_color || settings.picking_stroke_color {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
        if mouse.left_just_pressed {
            let ppp = ui.ctx().pixels_per_point();
            let (wx, wy) = crate::winapi_utils::get_window_screen_pos();
            let sx = (mouse.pos.x * ppp) as i32 + wx;
            let sy = (mouse.pos.y * ppp) as i32 + wy;
            if let Some(pixels) = crate::winapi_utils::capture_screen_rect(sx, sy, 1, 1) {
                if settings.picking_fill_color {
                    settings.background_color = [pixels[0], pixels[1], pixels[2], 255];
                    settings.picking_fill_color = false;
                } else if settings.picking_stroke_color {
                    settings.pen_color = [pixels[0], pixels[1], pixels[2], 255];
                    settings.picking_stroke_color = false;
                }
            }
        }
        return; // Prevent other interactions
    }

    // ── Hide All Logic ──
    if settings.hide_all { 
        // We still allow drawing if edit_mode is true? 
        // User said: "same button should show the once hid when clicking it"
        // Usually, hide all means visually hidden.
    }


    // ── Layers Rendering ──
    for (i, layer) in project.layers.iter_mut().enumerate().filter(|(_, l)| l.visible) {
        if settings.hide_all { continue; }
        let _is_active = project.active_layer == i;
        let l_op = layer.opacity;
        
        // Placed Images
        // Shadows for images
        for img in layer.placed_images.iter() {
            if !img.visible { continue; }
            if img.shadow || settings.snip_shadow {
                let disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
                let disp_h = img.display_size.unwrap_or([img.size[1] as f32, img.size[1] as f32])[1];
                let shadow_rect = egui::Rect::from_min_size(img.position - render_offset + egui::vec2(6.0, 6.0), egui::vec2(disp_w, disp_h));
                painter.rect_filled(shadow_rect, 0.0, egui::Color32::from_black_alpha((100.0 * l_op) as u8));
            }
        }

        for img in layer.placed_images.iter_mut() {
            if !img.visible { continue; }
            if img.is_live {
                let ppp = ui.ctx().pixels_per_point();
                let disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
                let disp_h = img.display_size.unwrap_or([img.size[1] as f32, img.size[1] as f32])[1];
                let center = img.position + egui::vec2(disp_w * 0.5, disp_h * 0.5);

                // ── Window capture live mode (hwnd-based, rate-limited to ~10fps) ──
                if img.hwnd != 0 {
                    let now = ui.input(|i| i.time) as f32;
                    let elapsed = now - img.last_frame_time;
                    if elapsed >= 0.1 { // ~10fps cap to keep performance reasonable
                        img.last_frame_time = now;
                        if let Some((pixels, pw, ph)) = crate::winapi_utils::capture_window(img.hwnd) {
                            img.size = [pw, ph];
                            img.pixels = pixels;
                            img.texture = None; // force texture rebuild
                        }
                    }
                } else {
                // ── Screen-region live mode (blur/mirror) ──
                let (wx, wy) = crate::winapi_utils::get_window_screen_pos();
                let (ox, oy) = if settings.use_absolute_screen_coords { (0, 0) } else { (wx, wy) };
                
                let (sx, sy, sw, sh) = if let Some(src) = img.source_rect {
                    // src is logical points. Convert to physical pixels for capture.
                    (
                        ((src[0] * ppp).round()) as i32 + ox,
                        ((src[1] * ppp).round()) as i32 + oy,
                        ((src[2] * ppp).round()) as i32,
                        ((src[3] * ppp).round()) as i32
                    )
                } else {
                    let mut dummy_mesh = egui::Mesh::default();
                    dummy_mesh.add_rect_with_uv(egui::Rect::from_min_size(egui::pos2(center.x - disp_w*0.5, center.y - disp_h*0.5), egui::vec2(disp_w, disp_h)), egui::Rect::from_min_max(egui::pos2(0.0,0.0), egui::pos2(1.0,1.0)), egui::Color32::WHITE);
                    transform_mesh(&mut dummy_mesh, center, img.rotation, img.skew, img.perspective, img.scale);
                    let mut min = egui::pos2(f32::MAX, f32::MAX);
                    let mut max = egui::pos2(f32::MIN, f32::MIN);
                    for v in &dummy_mesh.vertices {
                        min.x = min.x.min(v.pos.x); min.y = min.y.min(v.pos.y);
                        max.x = max.x.max(v.pos.x); max.y = max.y.max(v.pos.y);
                    }
                    let aabb = egui::Rect::from_min_max(min, max);
                    let p_w = (aabb.width() * ppp).round() as i32;
                    let p_h = (aabb.height() * ppp).round() as i32;
                    (((aabb.min.x * ppp).round()) as i32 + ox, ((aabb.min.y * ppp).round()) as i32 + oy, p_w, p_h)
                };

                if sw > 0 && sh > 0 {
                    // Only capture every 3rd frame (or if empty) to reduce GPU/CPU load, especially with OBS
                    if frame_count % 3 == 0 || img.pixels.is_empty() {
                        // Temporarily exclude from capture to avoid feedback loop (white box)
                        if !settings.exclude_from_capture {
                            crate::winapi_utils::set_capture_exclusion(true);
                        }
                        
                        if let Some(mut p) = crate::winapi_utils::capture_screen_rect(sx, sy, sw, sh) {
                            if !settings.exclude_from_capture {
                                crate::winapi_utils::set_capture_exclusion(false);
                            }
                            
                            img.size = [sw as usize, sh as usize]; // Update physical size
                            
                            if img.pixels.is_empty() { img.thumbnail_dirty = true; }

                            if img.blur > 0.1 && gl_renderer.is_some() {
                                // We'll handle this on GPU during rendering
                                img.pixels = p;
                            } else if img.blur > 0.1 {
                                match img.blur_effect {
                                    BlurEffect::Gaussian => apply_box_blur(&mut p, sw as usize, sh as usize, img.blur as usize),
                                    BlurEffect::Pixelate => apply_pixelate(&mut p, sw as usize, sh as usize, (img.blur * ppp) as usize),
                                    BlurEffect::Glitch => apply_vhs_glitch(&mut p, sw as usize, sh as usize, img.blur as f32 / 100.0),
                                }
                                img.pixels = p;
                            } else {
                                img.pixels = p;
                            }
                        }
                    } else if !settings.exclude_from_capture {
                        crate::winapi_utils::set_capture_exclusion(false);
                    }
                }
            }
        }

            // --- Thumbnail Update (Static Snip) ---
            if img.is_live && img.thumbnail_dirty && !img.pixels.is_empty() {
                let color_image = egui::ColorImage::from_rgba_unmultiplied(img.size, &img.pixels);
                if let Some(tex) = &mut img.thumbnail_texture {
                    tex.set(color_image, egui::TextureOptions::LINEAR);
                } else {
                    img.thumbnail_texture = Some(ui.ctx().load_texture(
                        format!("thumb_{}_{}", layer.name, img.id),
                        color_image,
                        egui::TextureOptions::LINEAR,
                    ));
                }
            }

            // --- Main Texture Update ---
            if !img.frames.is_empty() {
                // GIF/Animated handling
                let time = ui.input(|i| i.time) as f32;
                if img.last_frame_time == 0.0 { img.last_frame_time = time; }
                let dt = time - img.last_frame_time;
                let current_duration = img.frame_durations[img.current_frame];
                
                if dt >= current_duration {
                    img.current_frame = (img.current_frame + 1) % img.frames.len();
                    img.last_frame_time = time;
                    let color_image = egui::ColorImage::from_rgba_unmultiplied(img.size, &img.frames[img.current_frame]);
                    if let Some(tex) = &mut img.texture {
                        tex.set(color_image, Default::default());
                    } else {
                        img.texture = Some(ui.ctx().load_texture(format!("gif_{}", img.id), color_image, Default::default()));
                    }
                }
                ui.ctx().request_repaint();
            } else if !img.pixels.is_empty() {
                // Static or Live Snip
                let should_update_texture = img.texture.is_none() // First load
                    || (img.is_live && frame_count % 3 == 0)      // Live feed update
                    || (!img.is_live && img.thumbnail_dirty);     // Static update (filters, etc.)

                if should_update_texture {
                    let color_image = egui::ColorImage::from_rgba_unmultiplied(img.size, &img.pixels);
                    if let Some(tex) = &mut img.texture {
                        tex.set(color_image, egui::TextureOptions::LINEAR);
                    } else {
                        img.texture = Some(ui.ctx().load_texture(
                            format!("snip_{}_{}", layer.name, img.id),
                            color_image,
                            egui::TextureOptions::LINEAR,
                        ));
                    }
                }
            }
            
            // Clear dirty flags AFTER both updates
            if img.thumbnail_dirty && !img.pixels.is_empty() {
                img.thumbnail_dirty = false;
            }

            if img.mask.is_some() && (img.mask_texture.is_none() || img.mask_dirty) {
                if let Some(mask) = &img.mask {
                    let size = img.size;
                    let mut mask_rgba = vec![255u8; size[0] * size[1] * 4];
                    for (i, &m) in mask.iter().enumerate() {
                        if i < mask_rgba.len() / 4 {
                            mask_rgba[i*4] = m;
                            mask_rgba[i*4+1] = m;
                            mask_rgba[i*4+2] = m;
                            mask_rgba[i*4+3] = 255;
                        }
                    }
                    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &mask_rgba);
                    img.mask_texture = Some(ui.ctx().load_texture(format!("mask_{}", img.id), color_image, egui::TextureOptions::LINEAR));
                    img.mask_dirty = false;
                }
            }
            
            if let Some(tex) = &img.texture {
                let disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
                let disp_h = img.display_size.unwrap_or([img.size[1] as f32, img.size[1] as f32])[1];
                let center = (img.position - render_offset) + egui::vec2(disp_w * 0.5, disp_h * 0.5);

                let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));

                let mut final_scale = img.scale;
                if img.flipped_h { final_scale.x *= -1.0; }
                if img.flipped_v { final_scale.y *= -1.0; }

                if layer.shadow || img.shadow {
                    let mut s_mesh = egui::Mesh::with_texture(tex.id());
                    s_mesh.add_rect_with_uv(egui::Rect::from_min_size(egui::pos2(center.x - disp_w*0.5, center.y - disp_h*0.5), egui::vec2(disp_w, disp_h)), uv, egui::Color32::from_black_alpha((100.0 * l_op) as u8));
                    transform_mesh(&mut s_mesh, center + egui::vec2(3.0, 3.0), img.rotation, img.skew, img.perspective, final_scale);
                    painter.add(egui::Shape::mesh(s_mesh));
                }

                if layer.outline || img.outline {
                    let mut o_mesh = egui::Mesh::with_texture(tex.id());
                    o_mesh.add_rect_with_uv(egui::Rect::from_min_size(egui::pos2(center.x - disp_w*0.5 - 1.5, center.y - disp_h*0.5 - 1.5), egui::vec2(disp_w+3.0, disp_h+3.0)), uv, egui::Color32::from_white_alpha((200.0 * l_op) as u8));
                    transform_mesh(&mut o_mesh, center, img.rotation, img.skew, img.perspective, final_scale);
                    painter.add(egui::Shape::mesh(o_mesh));
                }

                let mut mesh = egui::Mesh::with_texture(tex.id());
                let color = egui::Color32::from_white_alpha((255.0 * l_op * img.opacity) as u8);
                mesh.add_rect_with_uv(egui::Rect::from_min_size(egui::pos2(center.x - disp_w*0.5, center.y - disp_h*0.5), egui::vec2(disp_w, disp_h)), uv, color);
                transform_mesh(&mut mesh, center, img.rotation, img.skew, img.perspective, final_scale);

                // Fix: sample whole texture UVs correctly for live images
                if img.is_live && img.source_rect.is_none() {
                    let mut min = egui::pos2(f32::MAX, f32::MAX);
                    let mut max = egui::pos2(f32::MIN, f32::MIN);
                    for v in &mesh.vertices {
                        min.x = min.x.min(v.pos.x); min.y = min.y.min(v.pos.y);
                        max.x = max.x.max(v.pos.x); max.y = max.y.max(v.pos.y);
                    }
                    let aabb = egui::Rect::from_min_max(min, max);
                    for v in &mut mesh.vertices {
                        v.uv.x = if aabb.width() > 0.0 { (v.pos.x - aabb.min.x) / aabb.width() } else { 0.0 };
                        v.uv.y = if aabb.height() > 0.0 { (v.pos.y - aabb.min.y) / aabb.height() } else { 0.0 };
                    }
                }

                let has_gl_effect = img.blur > 0.1 || img.grayscale || img.invert || img.sepia || img.glow;
                if has_gl_effect && gl_renderer.is_some() {
                    let renderer = gl_renderer.as_ref().unwrap().clone();
                    let effect = match img.blur_effect {
                        BlurEffect::Gaussian => 1,
                        BlurEffect::Pixelate => 2,
                        BlurEffect::Glitch => 3,
                    };
                    let strength = img.blur;
                    let res = [img.size[0] as f32, img.size[1] as f32];
                    let time = ui.input(|i| i.time) as f32;
                    let tex_id = tex.id();
                    let mask_tex_id = img.mask_texture.as_ref().map(|t| t.id());
                    
                    let mut vertices = Vec::new();
                    for v in &mesh.vertices {
                        // Map egui screen coords to GL coords (-1 to 1)
                        let screen_size = ui.ctx().screen_rect().size();
                        let gl_x = (v.pos.x / screen_size.x) * 2.0 - 1.0;
                        let gl_y = 1.0 - (v.pos.y / screen_size.y) * 2.0;
                        vertices.push(gl_x);
                        vertices.push(gl_y);
                        vertices.push(v.uv.x);
                        vertices.push(v.uv.y);
                    }

                    let paint_rect = mesh.calc_bounds();
                    let effect = img.blur_effect as i32;
                    let strength = img.blur;
                    let grayscale = img.grayscale;
                    let invert = img.invert;
                    let sepia = img.sepia;
                    let glow = img.glow;
                    let glow_strength = img.glow_strength;

                    painter.add(egui::PaintCallback {
                        rect: paint_rect,
                        callback: Arc::new(egui_glow::CallbackFn::new(move |_info, render_ctx: &egui_glow::Painter| {
                            let gl = render_ctx.gl();
                            
                            // Get actual GL texture IDs
                            let gl_tex = match tex_id {
                                egui::TextureId::Managed(id) => glow::NativeTexture(std::num::NonZeroU32::new(id as u32).unwrap()),
                                _ => return,
                            };
                            let gl_mask = mask_tex_id.and_then(|id| match id {
                                egui::TextureId::Managed(mid) => Some(glow::NativeTexture(std::num::NonZeroU32::new(mid as u32).unwrap())),
                                _ => None,
                            });

                            unsafe {
                                // Update vertices for this specific quad
                                gl.bind_buffer(glow::ARRAY_BUFFER, Some(renderer.vertex_buffer));
                                gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytemuck::cast_slice(&vertices), glow::DYNAMIC_DRAW);

                                renderer.render_effect(gl, gl_tex, gl_mask, effect, strength, res, time, grayscale, invert, sepia, glow, glow_strength);
                            }
                        })),
                    });
                } else {
                    painter.add(egui::Shape::mesh(mesh));
                }
            }
        }

        // Completed strokes
        crate::tools::brush::draw_layer_strokes(&painter, layer, -render_offset, l_op);

        // Text annotations
        crate::tools::text::draw_layer_text(&painter, layer, -render_offset, l_op, settings, ui.input(|i| i.time) as f32);

    }

    // ── Pending text cursor ──
    if let Some(pending) = pending_text.as_ref() {
        let time  = ui.input(|i| i.time);
        let blink = (time * 3.0).sin() > 0.0;
        let font  = crate::tools::text::resolve_font(settings.text_font, settings.font_size);

        let mut display_text = pending.buffer.clone();
        if blink { display_text.push('|'); }

        let pen_c = color32(&settings.pen_color);
        let draw_pos = pending.position - render_offset;

        if settings.text_wave_warp {
            let wave_amplitude = settings.font_size * 0.25;
            let wave_frequency = std::f32::consts::TAU / (settings.font_size * 3.5);
            let char_w = settings.font_size * 0.6;
            
            for (i, ch) in display_text.chars().enumerate() {
                let x = draw_pos.x + i as f32 * char_w;
                let wave_y = draw_pos.y + wave_amplitude * (wave_frequency * x - time as f32 * 5.0).sin();
                let char_str: String = std::iter::once(ch).collect();
                painter.text(egui::pos2(x, wave_y), egui::Align2::LEFT_TOP, &char_str, font.clone(), pen_c);
            }
        } else {
            if settings.text_outline {
                let c = pen_c;
                let outline_col = if c.r() as u32 + c.g() as u32 + c.b() as u32 > 382 {
                    egui::Color32::BLACK
                } else {
                    egui::Color32::WHITE
                };
                for off in [
                    egui::vec2(1.0, 1.0), egui::vec2(-1.0, -1.0),
                    egui::vec2(1.0, -1.0), egui::vec2(-1.0, 1.0),
                ] {
                    painter.text(draw_pos + off, egui::Align2::LEFT_TOP, &display_text, font.clone(), outline_col);
                }
            } else if settings.text_shadow {
                painter.text(draw_pos + egui::vec2(2.0, 2.0), egui::Align2::LEFT_TOP, &display_text, font.clone(), egui::Color32::from_black_alpha(150));
            }
            painter.text(draw_pos, egui::Align2::LEFT_TOP, &display_text, font, pen_c);
        }
        ui.ctx().request_repaint();
    }

    let mut remove_active_layer = false;
    let mut snip_created = false;
    let mut new_selection = None;
    let mut switch_to_move = false;
    let active_layer_idx = project.active_layer;

    let mut ctx = ToolContext {
        ui,
        project,
        settings,
        mouse,
        active_tool,
        last_tool_used,
        current_stroke,
        line_start,
        frame_count: frame_count as u64,
        render_offset,
        canvas_response: &canvas_response,
        remove_active_layer: &mut remove_active_layer,
        snip_created: &mut snip_created,
        new_selection: &mut new_selection,
        switch_to_move: &mut switch_to_move,
        embed_trigger: _embed_trigger,
        pending_text,
        initial_bounds,
        initial_center,
        initial_layer,
        drag_state,
        dragging_source_rect,
        edit_mode,
        layer_prompt_open,
        request_history_push,
    };

    // ── Live preview ──
    match *ctx.active_tool {
        Tool::Brush => crate::tools::brush::render_preview(&mut ctx),
        Tool::Shape => crate::tools::shape::render_preview(&mut ctx),
        Tool::Snip => crate::tools::snip::render_preview(&mut ctx),
        Tool::Blur => crate::tools::blur::render_preview(&mut ctx),
        _ => {}
    }

    // Bounding box for Move tool
    if *ctx.active_tool == Tool::Move {
        crate::tools::move_tool::render(&mut ctx);
    }






    if edit_mode && can_draw && active_layer_idx < ctx.project.layers.len() {
        match *ctx.active_tool {
            Tool::Brush => {
                crate::tools::brush::update(&mut ctx);
            }
            Tool::Shape => {
                crate::tools::shape::update(&mut ctx);
            }
            Tool::Snip => {
                crate::tools::snip::update(&mut ctx);
            }
            Tool::Cut => {
                crate::tools::cut::update(&mut ctx);
            }
            Tool::Mirror => {
                crate::tools::mirror::update(&mut ctx);
            }
            Tool::Blur => {
                crate::tools::blur::update(&mut ctx);
            }
            Tool::Text => {
                crate::tools::text::update(&mut ctx);
            }
            Tool::Embed => {
                crate::tools::embed::update(&mut ctx);
            }
            Tool::Move => {
                crate::tools::move_tool::update(&mut ctx);
            }
            Tool::Eraser => {
                crate::tools::eraser::update(&mut ctx);
            }
        }

    }

    if remove_active_layer {
        project.layers.remove(project.active_layer);
        project.active_layer = project.active_layer.saturating_sub(1);
        project.selected_object = None; // Prevent crash by clearing selection to removed layer
    }

    
    if let Some(sel) = new_selection { project.selected_object = Some(sel); }
    if switch_to_move { *active_tool = Tool::Move; }
}

