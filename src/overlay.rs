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
    pending_stroke: &mut Option<Stroke>,
    pending_text_to_add: &mut Option<TextAnnotation>,
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
    rasterize_phase: u8,
    rasterize_bbox: Option<[f32; 4]>,
    rasterize_capture: crate::rasterize::CaptureBuffer,
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
    let rasterize_req = project.rasterize_request;
    for (i, layer) in project.layers.iter_mut().enumerate().filter(|(_, l)| l.visible) {
        if settings.hide_all && rasterize_phase == 0 { continue; }
        // During rasterize capture, only render the target layer
        if rasterize_phase == 1 {
            if let Some(req) = &rasterize_req {
                if i != req.layer_idx { continue; }
            }
        }
        let _is_active = project.active_layer == i;
        let l_op = layer.opacity;
        
        // Placed Images
        let live_count = layer.placed_images.iter().filter(|img| img.is_live).count();

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
                } else if img.url.is_none() {
                    let (wx, wy) = crate::winapi_utils::get_window_screen_pos();
                    let (ox, oy) = if settings.use_absolute_screen_coords { (0, 0) } else { (wx, wy) };
                    
                    let src_rect = if let Some(src) = img.source_rect {
                        src
                    } else {
                        let mut dummy_mesh = egui::Mesh::default();
                        dummy_mesh.add_rect_with_uv(
                            egui::Rect::from_min_size(egui::pos2(center.x - disp_w*0.5, center.y - disp_h*0.5), egui::vec2(disp_w, disp_h)),
                            egui::Rect::from_min_max(egui::pos2(0.0,0.0), egui::pos2(1.0,1.0)),
                            egui::Color32::WHITE
                        );
                        transform_mesh(&mut dummy_mesh, center, img.rotation, img.skew, img.perspective, img.scale);
                        let mut min = egui::pos2(f32::MAX, f32::MAX);
                        let mut max = egui::pos2(f32::MIN, f32::MIN);
                        for v in &dummy_mesh.vertices {
                            min.x = min.x.min(v.pos.x); min.y = min.y.min(v.pos.y);
                            max.x = max.x.max(v.pos.x); max.y = max.y.max(v.pos.y);
                        }
                        let aabb = egui::Rect::from_min_max(min, max);
                        [aabb.min.x, aabb.min.y, aabb.width(), aabb.height()]
                    };

                    let req = crate::capture_thread::CaptureRequest {
                        id: img.id,
                        source_rect: src_rect,
                        ppp,
                        blur: img.blur,
                        blur_effect: img.blur_effect,
                        window_offset: (ox, oy),
                        use_absolute: settings.use_absolute_screen_coords,
                        hwnd: img.hwnd,
                    };
                    
                    _capture_thread.update_request(img.id, req);
                    
                    if let Some(res) = _capture_thread.get_frame(img.id) {
                        img.size = res.size;
                        img.pixels = res.pixels;
                        img.texture = None;
                        img.thumbnail_dirty = true;
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

                let has_gl_effect = img.blur > 0.1 || img.grayscale || img.invert || img.sepia || img.glow || layer.grayscale || layer.invert || layer.sepia || layer.glow;
                let effect_pad = 0.0;

                let is_gray = img.grayscale || layer.grayscale;
                let is_inv = img.invert || layer.invert;
                let is_sepia = img.sepia || layer.sepia;

                let strength = img.blur.max(0.0).max(layer.blur.max(0.0)) * 0.2;
                let res = [img.size[0] as f32, img.size[1] as f32];
                let time = ui.input(|i| i.time) as f32;
                let tex_id = tex.id();
                let mask_tex_id = img.mask_texture.as_ref().map(|t| t.id());
                
                let effect = match img.blur_effect {
                    BlurEffect::Gaussian => 1,
                    BlurEffect::Pixelate => 2,
                    BlurEffect::Glitch => 3,
                };
                let layer_effect = match layer.blur_effect {
                    BlurEffect::Gaussian => 1,
                    BlurEffect::Pixelate => 2,
                    BlurEffect::Glitch => 3,
                };
                let final_effect = if img.blur > 0.1 { effect } else if layer.blur > 0.1 { layer_effect } else { 0 };

                // Draw helper closure
                // apply_filters = true only for the main image pass, NOT for shadow/outline/glow silhouette passes
                let draw_pass = |
                    is_shadow: bool,
                    apply_filters: bool,
                    offset_x: f32, offset_y: f32,
                    spread: f32,
                    pass_blur_strength: f32,
                    tint: [f32; 4],
                    pass_opacity: f32
                | {
                    if pass_opacity <= 0.0 { return; }
                    
                    let mut mesh = egui::Mesh::with_texture(tex_id);
                    let mut draw_scale = final_scale;
                    
                    let mut padded_w = disp_w;
                    let mut padded_h = disp_h;
                    // Only add effect_pad for the main pass (apply_filters=true)
                    let mut pass_pad = if apply_filters { effect_pad } else if pass_blur_strength > 0.0 { pass_blur_strength.max(20.0) } else { 0.0 };
                    
                    if spread > 0.0 {
                        pass_pad += spread;
                        let scale_x = 1.0 + (spread / disp_w.max(1.0)) * 2.0;
                        let scale_y = 1.0 + (spread / disp_h.max(1.0)) * 2.0;
                        draw_scale.x *= scale_x;
                        draw_scale.y *= scale_y;
                    }
                    
                    if pass_pad > 0.0 {
                        padded_w += pass_pad * 2.0;
                        padded_h += pass_pad * 2.0;
                        let uv_width = uv.width();
                        let uv_height = uv.height();
                        let uv_min_x = uv.min.x - (pass_pad / disp_w.max(1.0)) * uv_width;
                        let uv_min_y = uv.min.y - (pass_pad / disp_h.max(1.0)) * uv_height;
                        let uv_max_x = uv.max.x + (pass_pad / disp_w.max(1.0)) * uv_width;
                        let uv_max_y = uv.max.y + (pass_pad / disp_h.max(1.0)) * uv_height;
                        let padded_uv = egui::Rect::from_min_max(egui::pos2(uv_min_x, uv_min_y), egui::pos2(uv_max_x, uv_max_y));
                        mesh.add_rect_with_uv(egui::Rect::from_min_size(egui::pos2(center.x + offset_x - padded_w*0.5, center.y + offset_y - padded_h*0.5), egui::vec2(padded_w, padded_h)), padded_uv, egui::Color32::WHITE);
                    } else {
                        mesh.add_rect_with_uv(egui::Rect::from_min_size(egui::pos2(center.x + offset_x - disp_w*0.5, center.y + offset_y - disp_h*0.5), egui::vec2(disp_w, disp_h)), uv, egui::Color32::WHITE);
                    }
                    
                    transform_mesh(&mut mesh, center + egui::vec2(offset_x, offset_y), img.rotation, img.skew, img.perspective, draw_scale);

                    // For silhouette passes (shadow/outline/glow), only use GL when blur effect is active
                    // so the blur kernel can spread the silhouette halo. Otherwise use fast software path.
                    let use_gl = gl_renderer.is_some() && (apply_filters || final_effect > 0 || pass_blur_strength > 0.0 || spread > 0.0);

                    if use_gl {
                        let renderer = gl_renderer.as_ref().unwrap().clone();
                        let mut vertices = Vec::new();
                        for &idx in &mesh.indices {
                            let v = &mesh.vertices[idx as usize];
                            vertices.push(v.pos.x);
                            vertices.push(v.pos.y);
                            vertices.push(v.uv.x);
                            vertices.push(v.uv.y);
                        }
                        let vertex_count = mesh.indices.len() as i32;
                        let paint_rect = mesh.calc_bounds().expand(2.0);
                        // Only apply color filters on the main image pass, not silhouettes
                        let pass_gray = apply_filters && is_gray;
                        let pass_inv  = apply_filters && is_inv;
                        let pass_sepia = apply_filters && is_sepia;

                        painter.add(egui::PaintCallback {
                            rect: paint_rect,
                            callback: std::sync::Arc::new(egui_glow::CallbackFn::new(move |_info, render_ctx: &egui_glow::Painter| {
                                let gl = render_ctx.gl();
                                let ppp = _info.pixels_per_point;
                                let screen_h_px = _info.screen_size_px[1] as f32;
                                
                                let x = (paint_rect.min.x * ppp).round() as i32;
                                let y = (screen_h_px - paint_rect.max.y * ppp).round() as i32;
                                let w = (paint_rect.width() * ppp).round() as i32;
                                let h = (paint_rect.height() * ppp).round() as i32;
                                
                                if w <= 0 || h <= 0 { return; }
                                
                                let mut mapped_vertices = Vec::with_capacity(vertices.len());
                                for i in (0..vertices.len()).step_by(4) {
                                    let vx = vertices[i];
                                    let vy = vertices[i+1];
                                    let gl_x = ((vx - paint_rect.min.x) / paint_rect.width().max(1.0)) * 2.0 - 1.0;
                                    let gl_y = 1.0 - ((vy - paint_rect.min.y) / paint_rect.height().max(1.0)) * 2.0;
                                    mapped_vertices.push(gl_x);
                                    mapped_vertices.push(gl_y);
                                    mapped_vertices.push(vertices[i+2]);
                                    mapped_vertices.push(vertices[i+3]);
                                }
                                
                                let gl_tex = match render_ctx.texture(tex_id) {
                                    Some(t) => t, None => return,
                                };
                                let gl_mask = mask_tex_id.and_then(|id| render_ctx.texture(id));

                                let mut old_viewport = [0i32; 4];
                                unsafe {
                                    gl.get_parameter_i32_slice(glow::VIEWPORT, &mut old_viewport);
                                    gl.viewport(x, y, w, h);
                                    
                                    let actual_effect = if pass_blur_strength > 0.0 { 1 } else { final_effect };
                                    let actual_strength = if pass_blur_strength > 0.0 { pass_blur_strength } else { strength };
                                    renderer.render_effect(gl, gl_tex, gl_mask, actual_effect, actual_strength, res, time, pass_gray, pass_inv, pass_sepia, tint, is_shadow, pass_opacity, vertex_count, &mapped_vertices);
                                    
                                    gl.viewport(old_viewport[0], old_viewport[1], old_viewport[2], old_viewport[3]);
                                }
                            })),
                        });
                    } else {
                        // Software rendering - flat tint color with texture alpha as mask
                        // For silhouette passes, color filters applied to the tint itself (so grayscale outline becomes gray)
                        let mut tint_col = if apply_filters {
                            crate::utils::apply_color_effects(
                                egui::Color32::from_rgba_unmultiplied((tint[0]*255.0) as u8, (tint[1]*255.0) as u8, (tint[2]*255.0) as u8, (tint[3]*255.0) as u8),
                                is_gray, is_inv, is_sepia, false, 0.0
                            )
                        } else {
                            egui::Color32::from_rgba_unmultiplied((tint[0]*255.0) as u8, (tint[1]*255.0) as u8, (tint[2]*255.0) as u8, (tint[3]*255.0) as u8)
                        };
                        let alpha = (tint_col.a() as f32 * pass_opacity).clamp(0.0, 255.0) as u8;
                        tint_col = egui::Color32::from_rgba_unmultiplied(tint_col.r(), tint_col.g(), tint_col.b(), alpha);
                        
                        for v in &mut mesh.vertices {
                            v.color = tint_col;
                        }
                        painter.add(egui::Shape::mesh(mesh));
                    }
                };

                let has_shadow = layer.shadow || img.shadow || settings.snip_shadow;
                if has_shadow {
                    let (s_col_arr, s_off, s_spread) = if img.shadow { 
                        (img.shadow_color, img.shadow_offset, img.shadow_spread) 
                    } else if layer.shadow {
                        (layer.shadow_color, layer.shadow_offset, layer.shadow_spread)
                    } else {
                        ([0, 0, 0, 100], [6.0, 6.0], 0.0)
                    };
                    let tint = [s_col_arr[0] as f32 / 255.0, s_col_arr[1] as f32 / 255.0, s_col_arr[2] as f32 / 255.0, s_col_arr[3] as f32 / 255.0];
                    draw_pass(true, true, s_off[0], s_off[1], s_spread, if img.shadow { img.shadow_blur } else { layer.shadow_blur }, tint, l_op * img.opacity);
                }

                if layer.outline || img.outline {
                    let (o_col_arr, o_width) = if img.outline { (img.outline_color, img.outline_width) } else { (layer.outline_color, layer.outline_width) };
                    let tint = [o_col_arr[0] as f32 / 255.0, o_col_arr[1] as f32 / 255.0, o_col_arr[2] as f32 / 255.0, o_col_arr[3] as f32 / 255.0];
                    let steps = 8;
                    for i in 0..steps {
                        let angle = (i as f32) * std::f32::consts::TAU / (steps as f32);
                        let off_x = angle.cos() * o_width;
                        let off_y = angle.sin() * o_width;
                        draw_pass(true, true, off_x, off_y, 0.0, 0.0, tint, l_op * img.opacity);
                    }
                }

                let has_glow = layer.glow || img.glow;
                if has_glow {
                    let (g_col_arr, g_str, g_spread) = if img.glow { 
                        (img.glow_color, img.glow_strength, img.glow_spread) 
                    } else {
                        (layer.glow_color, layer.glow_strength, layer.glow_spread)
                    };
                    let alpha = (g_str / 100.0).clamp(0.0, 1.0);
                    let tint = [g_col_arr[0] as f32 / 255.0, g_col_arr[1] as f32 / 255.0, g_col_arr[2] as f32 / 255.0, g_col_arr[3] as f32 / 255.0 * alpha];
                    // Render glow silhouette
                    draw_pass(true, true, 0.0, 0.0, g_spread, 10.0, tint, l_op * img.opacity);
                }

                draw_pass(false, true, 0.0, 0.0, 0.0, 0.0, [1.0, 1.0, 1.0, 1.0], l_op * img.opacity);
            }
        }

        // Completed strokes
        let skip_strokes = rasterize_phase == 1 && rasterize_req.map(|r| r.object_idx.is_some() && !matches!(r.object_idx, Some((crate::types::ObjectType::Stroke, _)))).unwrap_or(false);
        if !skip_strokes {
            crate::tools::brush::draw_layer_strokes(&painter, layer, -render_offset, l_op);

            // Render Blur strokes as live blurred screen captures
            for (s_idx, s) in layer.strokes.iter_mut().enumerate() {
                if !s.visible || s.kind != crate::types::StrokeKind::Blur || s.points.len() < 2 { continue; }
                let rect_world = egui::Rect::from_two_pos(s.points[0], s.points[1]);
                let rect_screen = rect_world.translate(-render_offset);
                let (wx, wy) = crate::winapi_utils::get_window_screen_pos();
                let (ox, oy) = if settings.use_absolute_screen_coords { (0, 0) } else { (wx, wy) };
                let sw = (rect_screen.width() * ppp).round() as i32;
                let sh = (rect_screen.height() * ppp).round() as i32;
                if sw > 2 && sh > 2 {
                    let req = crate::capture_thread::CaptureRequest {
                        id: s.id,
                        source_rect: [rect_screen.min.x, rect_screen.min.y, rect_screen.width(), rect_screen.height()],
                        ppp,
                        blur: s.blur.max(8.0),
                        blur_effect: s.blur_effect,
                        window_offset: (ox, oy),
                        use_absolute: settings.use_absolute_screen_coords,
                        hwnd: 0,
                    };
                    _capture_thread.update_request(s.id, req);

                    if let Some(res) = _capture_thread.get_frame(s.id) {
                        let color_img = egui::ColorImage::from_rgba_unmultiplied(res.size, &res.pixels);
                        if let Some(ref mut tex) = s.cached_texture {
                            tex.set(color_img, egui::TextureOptions::LINEAR);
                        } else {
                            s.cached_texture = Some(ui.ctx().load_texture(
                                format!("blur_stroke_{}", s.id),
                                color_img,
                                egui::TextureOptions::LINEAR,
                            ));
                        }
                    }
                    if let Some(ref tex) = s.cached_texture {
                        // Draw shadow if enabled
                        let has_shadow = layer.shadow || s.shadow;
                        if has_shadow {
                            let (s_col_arr, s_off, s_spread) = if s.shadow {
                                (s.shadow_color, s.shadow_offset, s.shadow_spread)
                            } else if layer.shadow {
                                (layer.shadow_color, layer.shadow_offset, layer.shadow_spread)
                            } else {
                                ([0, 0, 0, 100], [6.0, 6.0], 0.0)
                            };
                            let alpha = (l_op * s.opacity).clamp(0.0, 1.0);
                            let shadow_color = egui::Color32::from_rgba_unmultiplied(
                                s_col_arr[0], s_col_arr[1], s_col_arr[2], (s_col_arr[3] as f32 * alpha) as u8
                            );
                            let shadow_rect = rect_screen.translate(egui::vec2(s_off[0], s_off[1])).expand(s_spread);
                            painter.rect_filled(shadow_rect, 0.0, shadow_color);
                        }

                        // Draw glow if enabled
                        let has_glow = layer.glow || s.glow;
                        if has_glow {
                            let (g_col_arr, g_str, g_spread) = if s.glow {
                                (s.glow_color, s.glow_strength, s.glow_spread)
                            } else {
                                (layer.glow_color, layer.glow_strength, layer.glow_spread)
                            };
                            let alpha = ((g_str / 100.0) * l_op * s.opacity).clamp(0.0, 1.0);
                            let glow_color = egui::Color32::from_rgba_unmultiplied(
                                g_col_arr[0], g_col_arr[1], g_col_arr[2], (g_col_arr[3] as f32 * alpha) as u8
                            );
                            let glow_rect = rect_screen.expand(g_spread);
                            painter.rect_filled(glow_rect, 0.0, glow_color);
                        }

                        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                        let mut mesh = egui::Mesh::with_texture(tex.id());
                        let opacity_col = egui::Color32::from_white_alpha((255.0 * l_op * s.opacity) as u8);
                        mesh.add_rect_with_uv(rect_screen, uv, opacity_col);
                        painter.add(egui::Shape::mesh(mesh));
                    }
                }
            }
        }

        // Text annotations
        let skip_text = rasterize_phase == 1 && rasterize_req.map(|r| r.object_idx.is_some() && !matches!(r.object_idx, Some((crate::types::ObjectType::Text, _)))).unwrap_or(false);
        if !skip_text {
            crate::tools::text::draw_layer_text(&painter, layer, -render_offset, l_op, settings, ui.input(|i| i.time) as f32);
        }

    }

    if let Some(s) = pending_stroke.as_ref() {
        let pen_c = color32(&s.color);
        crate::tools::brush::draw_stroke(&painter, s, pen_c, -render_offset, s.width, 1.0);
    }

    // ── Rasterize capture callback ──
    if rasterize_phase == 1 {
        if let Some(bbox) = rasterize_bbox {
            let ppp = ui.ctx().pixels_per_point();
            let screen_h = ui.ctx().screen_rect().height();
            let cb = crate::rasterize::create_capture_callback(bbox, ppp, screen_h, rasterize_capture);
            painter.add(cb);
        }
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
        pending_stroke,
        pending_text_to_add,
        initial_bounds,
        initial_center,
        initial_layer,
        drag_state,
        dragging_source_rect,
        edit_mode,
        layer_prompt_open,
        request_history_push,
    };

    // ── Live preview (skip during rasterize) ──
    if rasterize_phase == 0 {
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
    }

    if rasterize_phase == 0 && edit_mode && can_draw && active_layer_idx < ctx.project.layers.len() {
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
            Tool::PaintBucket => {
                crate::tools::paint_bucket::update(&mut ctx);
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

