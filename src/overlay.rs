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

pub fn add_quad_with_custom_uv(
    mesh: &mut egui::Mesh,
    rect: egui::Rect,
    uv_lt: egui::Pos2,
    uv_rt: egui::Pos2,
    uv_rb: egui::Pos2,
    uv_lb: egui::Pos2,
    color: egui::Color32,
) {
    let idx = mesh.vertices.len() as u32;
    mesh.vertices.push(egui::epaint::Vertex { pos: rect.left_top(), uv: uv_lt, color });
    mesh.vertices.push(egui::epaint::Vertex { pos: rect.right_top(), uv: uv_rt, color });
    mesh.vertices.push(egui::epaint::Vertex { pos: rect.right_bottom(), uv: uv_rb, color });
    mesh.vertices.push(egui::epaint::Vertex { pos: rect.left_bottom(), uv: uv_lb, color });
    mesh.indices.extend([idx, idx + 1, idx + 2, idx, idx + 2, idx + 3]);
}

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
    if !edit_mode || settings.hide_edit_info || settings.hide_all { return; }
    let hint = if toggle_mode {
        format!("EDIT MODE  |  {} to exit  |  F12 reset UI to monitor 1", hotkey_name)
    } else {
        format!("EDIT MODE  |  release {} to exit  |  F12 reset UI", hotkey_name)
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
    perf_stats: &mut crate::types::AppPerfStats,
    volume_sessions: &mut [crate::volume_mixer::AudioSessionInfo],
    volume_mixer_cmd_tx: &std::sync::mpsc::Sender<crate::volume_mixer::MixerCommand>,
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

    // ── Grid Rendering ──
    if settings.show_grid && !settings.hide_all {
        let grid_size = settings.grid_size.max(10.0);
        let color = egui::Color32::from_rgba_unmultiplied(200, 200, 200, 50); // 20% opacity subtle grid
        let stroke = egui::Stroke::new(1.0, color);
        // Vertical lines
        let mut x = rect.min.x;
        while x <= rect.max.x {
            painter.line_segment([egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)], stroke);
            x += grid_size;
        }
        // Horizontal lines
        let mut y = rect.min.y;
        while y <= rect.max.y {
            painter.line_segment([egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)], stroke);
            y += grid_size;
        }
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
            if let Some(w_type) = img.widget_type {
                let disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
                let disp_h = img.display_size.unwrap_or([img.size[1] as f32, img.size[1] as f32])[1];
                let area_pos = img.position - render_offset;

                let apply_filters = |mut c: egui::Color32| -> egui::Color32 {
                    c = crate::utils::apply_color_effects(c, img.grayscale, img.invert, img.sepia, img.glow, img.glow_strength);
                    c.gamma_multiply(img.opacity)
                };

                let bg_color = apply_filters(crate::utils::color32(&settings.pen_color));
                let accent_color = apply_filters(crate::utils::color32(&settings.background_color));

                let adjust_brightness = |c: egui::Color32, amount: f32| -> egui::Color32 {
                    let mut r = c.r() as f32 + amount * 255.0;
                    let mut g = c.g() as f32 + amount * 255.0;
                    let mut b = c.b() as f32 + amount * 255.0;
                    egui::Color32::from_rgba_unmultiplied(
                        r.clamp(0.0, 255.0) as u8,
                        g.clamp(0.0, 255.0) as u8,
                        b.clamp(0.0, 255.0) as u8,
                        c.a(),
                    )
                };

                let hover_bg = adjust_brightness(bg_color, 0.1);
                let active_bg = adjust_brightness(bg_color, 0.2);
                let hover_accent = adjust_brightness(accent_color, 0.1);
                let active_accent = adjust_brightness(accent_color, 0.2);

                let mut frame = egui::Frame::window(ui.style())
                    .inner_margin(egui::Margin::same(if img.widget_type == Some(crate::types::WidgetType::Calculator) { 4 } else { 8 }))
                    .fill(if img.transparent_bg { egui::Color32::TRANSPARENT } else { bg_color })
                    .stroke(if img.transparent_bg && !img.outline { egui::Stroke::NONE } else { egui::Stroke::new(1.0, accent_color) });

                if img.outline {
                    let outline_color = apply_filters(crate::utils::color32(&img.outline_color));
                    frame = frame.stroke(egui::Stroke::new(img.outline_width.max(1.0), outline_color));
                }

                if img.shadow {
                    let shadow_color = apply_filters(crate::utils::color32(&img.shadow_color));
                    frame = frame.shadow(egui::Shadow {
                        offset: [img.shadow_offset[0] as i8, img.shadow_offset[1] as i8],
                        blur: img.shadow_blur.clamp(0.0, 255.0) as u8,
                        spread: img.shadow_spread.clamp(0.0, 255.0) as u8,
                        color: shadow_color,
                    });
                } else if img.glow {
                    let glow_color = apply_filters(crate::utils::color32(&img.glow_color));
                    frame = frame.shadow(egui::Shadow {
                        offset: [0, 0],
                        blur: (img.glow_strength * 2.0).clamp(0.0, 255.0) as u8,
                        spread: img.glow_spread.clamp(0.0, 255.0) as u8,
                        color: glow_color,
                    });
                } else {
                    frame = frame.shadow(egui::Shadow::NONE);
                }

                egui::Area::new(egui::Id::new(img.id))
                    .fixed_pos(area_pos)
                    .order(egui::Order::Foreground)
                    .show(ui.ctx(), |ui| {
                        let visuals = ui.visuals_mut();
                        visuals.widgets.inactive.bg_fill = bg_color;
                        visuals.widgets.inactive.weak_bg_fill = bg_color;
                        visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, accent_color);
                        visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, apply_filters(egui::Color32::WHITE));

                        visuals.widgets.hovered.bg_fill = hover_bg;
                        visuals.widgets.hovered.weak_bg_fill = hover_bg;
                        visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, hover_accent);
                        visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, apply_filters(egui::Color32::WHITE));

                        visuals.widgets.active.bg_fill = active_bg;
                        visuals.widgets.active.weak_bg_fill = active_bg;
                        visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, active_accent);
                        visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, apply_filters(egui::Color32::WHITE));

                        visuals.widgets.noninteractive.bg_fill = bg_color;
                        visuals.widgets.noninteractive.weak_bg_fill = bg_color;
                        visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, accent_color);
                        visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, apply_filters(egui::Color32::WHITE));

                        frame.show(ui, |ui| {
                            ui.set_width(disp_w);
                            ui.set_height(disp_h);
                            match w_type {
                                crate::types::WidgetType::Calculator => {
                                    let mut calc = img.calculator_state.clone().unwrap_or_default();
                                    
                                    // Dynamic scaling based on widget size
                                    let default_w: f32 = 140.0;
                                    let default_h: f32 = 180.0;
                                    let scale_x = (disp_w / default_w).max(0.3);
                                    let scale_y = (disp_h / default_h).max(0.3);
                                    let scale = scale_x.min(scale_y);
                                    let btn_size = (26.0 * scale).max(12.0);
                                    let btn_font = (12.0 * scale).max(6.0);
                                    let display_font = (14.0 * scale).max(7.0);
                                    let spacing = (4.0 * scale).max(1.0);

                                    let rect = ui.max_rect();
                                    let is_hovered = ui.rect_contains_pointer(rect);
                                    if is_hovered {
                                        ui.input(|i| {
                                            for event in &i.events {
                                                if let egui::Event::Text(t) = event {
                                                    for c in t.chars() {
                                                        if c.is_digit(10) || c == '.' {
                                                            calc.push_char(c);
                                                        } else if c == '+' || c == '-' || c == '*' || c == '/' {
                                                            calc.set_op(c);
                                                        } else if c == '=' || c == '\r' || c == '\n' {
                                                            calc.calculate();
                                                        }
                                                    }
                                                } else if let egui::Event::Key { key: egui::Key::Backspace, pressed: true, .. } = event {
                                                    calc.backspace();
                                                } else if let egui::Event::Key { key: egui::Key::Escape, pressed: true, .. } = event {
                                                    calc.clear();
                                                }
                                            }
                                        });
                                    }

                                    ui.vertical(|ui| {
                                        ui.spacing_mut().item_spacing = egui::vec2(spacing, spacing);
                                        ui.spacing_mut().button_padding = egui::vec2((2.0 * scale).max(0.5), (2.0 * scale).max(0.5));
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            ui.label(egui::RichText::new(&calc.display)
                                                .font(egui::FontId::monospace(display_font))
                                                .color(apply_filters(egui::Color32::WHITE)));
                                        });
                                        ui.separator();

                                        egui::Grid::new(format!("calc_grid_{}", img.id))
                                            .spacing(egui::vec2(spacing, spacing))
                                            .show(ui, |ui| {
                                                let btn = |ui: &mut egui::Ui, label: &str, bg: egui::Color32| {
                                                    let response = ui.add(egui::Button::new(
                                                        egui::RichText::new(label).font(egui::FontId::proportional(btn_font))
                                                            .color(apply_filters(egui::Color32::WHITE))
                                                    ).fill(bg).min_size(egui::vec2(btn_size, btn_size)));
                                                    response.clicked()
                                                };
                                                
                                                let gray = adjust_brightness(bg_color, 0.1);
                                                let dark_gray = bg_color;
                                                let orange = accent_color;

                                                if btn(ui, "C", gray) { calc.clear(); }
                                                if btn(ui, "±", gray) { calc.negate(); }
                                                if btn(ui, "%", gray) { calc.percent(); }
                                                if btn(ui, "÷", orange) { calc.set_op('/'); }
                                                ui.end_row();

                                                if btn(ui, "7", dark_gray) { calc.push_char('7'); }
                                                if btn(ui, "8", dark_gray) { calc.push_char('8'); }
                                                if btn(ui, "9", dark_gray) { calc.push_char('9'); }
                                                if btn(ui, "×", orange) { calc.set_op('*'); }
                                                ui.end_row();

                                                if btn(ui, "4", dark_gray) { calc.push_char('4'); }
                                                if btn(ui, "5", dark_gray) { calc.push_char('5'); }
                                                if btn(ui, "6", dark_gray) { calc.push_char('6'); }
                                                if btn(ui, "−", orange) { calc.set_op('-'); }
                                                ui.end_row();

                                                if btn(ui, "1", dark_gray) { calc.push_char('1'); }
                                                if btn(ui, "2", dark_gray) { calc.push_char('2'); }
                                                if btn(ui, "3", dark_gray) { calc.push_char('3'); }
                                                if btn(ui, "+", orange) { calc.set_op('+'); }
                                                ui.end_row();

                                                if btn(ui, "0", dark_gray) { calc.push_char('0'); }
                                                if btn(ui, ".", dark_gray) { calc.push_char('.'); }
                                                if btn(ui, "⌫", dark_gray) { calc.backspace(); }
                                                if btn(ui, "=", orange) { calc.calculate(); }
                                                ui.end_row();
                                            });
                                    });
                                    img.calculator_state = Some(calc);
                                }
                                crate::types::WidgetType::VolumeMixer => {
                                    let mixer_state = img.volume_mixer_state.clone().unwrap_or_default();
                                    if mixer_state.mode == crate::types::MixerMode::Single {
                                        let session_name = &mixer_state.target_session;
                                        let session_opt = volume_sessions.iter().find(|s| &s.name == session_name);
                                        let (current_volume, is_muted, pid) = if let Some(session) = session_opt {
                                            (session.volume, session.mute, session.pid)
                                        } else {
                                            (0.5, false, u32::MAX)
                                        };

                                        let (rect, response) = ui.allocate_exact_size(
                                            egui::vec2(disp_w, disp_h),
                                            egui::Sense::click_and_drag()
                                        );

                                        let mut volume = current_volume;
                                        let mut changed = false;

                                        if response.dragged() || response.clicked() {
                                            if let Some(mouse_pos) = ui.input(|i| i.pointer.hover_pos()) {
                                                if mixer_state.style == crate::types::SliderStyle::Knob {
                                                    if response.dragged() {
                                                        let delta = ui.input(|i| i.pointer.delta());
                                                        let speed = 0.005;
                                                        volume = (volume - delta.y * speed).clamp(0.0, 1.0);
                                                        changed = true;
                                                    } else if response.clicked() {
                                                        let center = rect.center();
                                                        let dx = mouse_pos.x - center.x;
                                                        let dy = mouse_pos.y - center.y;
                                                        let mut angle = dy.atan2(dx);
                                                        if angle < 0.0 { angle += 2.0 * std::f32::consts::PI; }
                                                        let angle_deg = angle.to_degrees();
                                                        let mut shifted = angle_deg - 135.0;
                                                        if shifted < 0.0 { shifted += 360.0; }
                                                        if shifted > 270.0 {
                                                            if shifted > 315.0 {
                                                                volume = 0.0;
                                                            } else {
                                                                volume = 1.0;
                                                            }
                                                        } else {
                                                            volume = shifted / 270.0;
                                                        }
                                                        changed = true;
                                                    }
                                                } else {
                                                    let val = if mixer_state.orientation == crate::types::SliderOrientation::Horizontal {
                                                        let margin = 12.0;
                                                        let width = rect.width() - 2.0 * margin;
                                                        if width > 0.0 {
                                                            ((mouse_pos.x - rect.left() - margin) / width).clamp(0.0, 1.0)
                                                        } else {
                                                            0.5
                                                        }
                                                    } else {
                                                        let margin = 12.0;
                                                        let height = rect.height() - 2.0 * margin;
                                                        if height > 0.0 {
                                                            (1.0 - (mouse_pos.y - rect.top() - margin) / height).clamp(0.0, 1.0)
                                                        } else {
                                                            0.5
                                                        }
                                                    };
                                                    volume = val;
                                                    changed = true;
                                                }
                                            }
                                        }

                                        if changed && volume != current_volume {
                                            let _ = volume_mixer_cmd_tx.send(crate::volume_mixer::MixerCommand::SetVolume { pid, volume });
                                            if let Some(session) = volume_sessions.iter_mut().find(|s| s.pid == pid) {
                                                session.volume = volume;
                                            }
                                        }

                                        if response.double_clicked() {
                                            let _ = volume_mixer_cmd_tx.send(crate::volume_mixer::MixerCommand::SetMute { pid, mute: !is_muted });
                                            if let Some(session) = volume_sessions.iter_mut().find(|s| s.pid == pid) {
                                                session.mute = !is_muted;
                                            }
                                        }

                                        let painter = ui.painter();
                                        let orientation = mixer_state.orientation;
                                        
                                        match mixer_state.style {
                                            crate::types::SliderStyle::SleekPill => {
                                                let radius = rect.height().min(rect.width()) * 0.5;
                                                if orientation == crate::types::SliderOrientation::Horizontal {
                                                    let track_rect = rect;
                                                    painter.rect_filled(track_rect, radius, adjust_brightness(bg_color, -0.05));
                                                    let fill_width = volume * (rect.width() - 2.0 * radius) + radius;
                                                    let fill_rect = egui::Rect::from_min_max(rect.min, egui::pos2(rect.min.x + fill_width + radius, rect.max.y));
                                                    painter.rect_filled(fill_rect.intersect(rect), radius, if is_muted { apply_filters(egui::Color32::from_gray(100)) } else { accent_color });
                                                    
                                                    let handle_center_x = rect.min.x + radius + volume * (rect.width() - 2.0 * radius);
                                                    let handle_rect = egui::Rect::from_center_size(
                                                        egui::pos2(handle_center_x, rect.center().y),
                                                        egui::vec2(24.0, rect.height() - 4.0)
                                                    );
                                                    painter.rect_filled(handle_rect, 6.0, adjust_brightness(bg_color, 0.2));
                                                    painter.rect_stroke(handle_rect, 6.0, egui::Stroke::new(1.0, accent_color), egui::StrokeKind::Middle);
                                                    
                                                    let gx = handle_rect.center().x;
                                                    let gy = handle_rect.center().y;
                                                    let g_stroke = egui::Stroke::new(1.5, adjust_brightness(accent_color, -0.1));
                                                    painter.line_segment([egui::pos2(gx - 4.0, gy - 6.0), egui::pos2(gx - 4.0, gy + 6.0)], g_stroke);
                                                    painter.line_segment([egui::pos2(gx, gy - 6.0), egui::pos2(gx, gy + 6.0)], g_stroke);
                                                    painter.line_segment([egui::pos2(gx + 4.0, gy - 6.0), egui::pos2(gx + 4.0, gy + 6.0)], g_stroke);
                                                } else {
                                                    let track_rect = rect;
                                                    painter.rect_filled(track_rect, radius, adjust_brightness(bg_color, -0.05));
                                                    let fill_height = volume * (rect.height() - 2.0 * radius) + radius;
                                                    let fill_rect = egui::Rect::from_min_max(egui::pos2(rect.min.x, rect.max.y - fill_height - radius), rect.max);
                                                    painter.rect_filled(fill_rect.intersect(rect), radius, if is_muted { apply_filters(egui::Color32::from_gray(100)) } else { accent_color });
                                                    
                                                    let handle_center_y = rect.max.y - radius - volume * (rect.height() - 2.0 * radius);
                                                    let handle_rect = egui::Rect::from_center_size(
                                                        egui::pos2(rect.center().x, handle_center_y),
                                                        egui::vec2(rect.width() - 4.0, 24.0)
                                                    );
                                                    painter.rect_filled(handle_rect, 6.0, adjust_brightness(bg_color, 0.2));
                                                    painter.rect_stroke(handle_rect, 6.0, egui::Stroke::new(1.0, accent_color), egui::StrokeKind::Middle);
                                                    
                                                    let gx = handle_rect.center().x;
                                                    let gy = handle_rect.center().y;
                                                    let g_stroke = egui::Stroke::new(1.5, adjust_brightness(accent_color, -0.1));
                                                    painter.line_segment([egui::pos2(gx - 6.0, gy - 4.0), egui::pos2(gx + 6.0, gy - 4.0)], g_stroke);
                                                    painter.line_segment([egui::pos2(gx - 6.0, gy), egui::pos2(gx + 6.0, gy)], g_stroke);
                                                    painter.line_segment([egui::pos2(gx - 6.0, gy + 4.0), egui::pos2(gx + 6.0, gy + 4.0)], g_stroke);
                                                }
                                            }
                                            crate::types::SliderStyle::ThinMetal => {
                                                if orientation == crate::types::SliderOrientation::Horizontal {
                                                    let cy = rect.center().y;
                                                    let track_line = egui::Rect::from_min_max(egui::pos2(rect.min.x + 10.0, cy - 1.5), egui::pos2(rect.max.x - 10.0, cy + 1.5));
                                                    painter.rect_filled(track_line, 1.0, adjust_brightness(bg_color, -0.1));
                                                    
                                                    let tick_y = cy + 12.0;
                                                    let num_ticks = 11;
                                                    for idx in 0..num_ticks {
                                                        let frac = idx as f32 / (num_ticks - 1) as f32;
                                                        let tx = rect.min.x + 10.0 + frac * (rect.width() - 20.0);
                                                        let tick_len = if idx % 5 == 0 { 6.0 } else { 3.0 };
                                                        painter.line_segment(
                                                            [egui::pos2(tx, tick_y), egui::pos2(tx, tick_y + tick_len)],
                                                            egui::Stroke::new(1.0, adjust_brightness(bg_color, 0.1))
                                                        );
                                                    }
                                                    
                                                    let handle_x = rect.min.x + 10.0 + volume * (rect.width() - 20.0);
                                                    let handle_rect = egui::Rect::from_center_size(egui::pos2(handle_x, cy), egui::vec2(16.0, 16.0));
                                                    painter.rect_filled(handle_rect, 3.0, apply_filters(egui::Color32::from_gray(200)));
                                                    painter.rect_stroke(handle_rect, 3.0, egui::Stroke::new(1.0, apply_filters(egui::Color32::from_gray(100))), egui::StrokeKind::Middle);
                                                    painter.circle_filled(handle_rect.center(), 3.0, if is_muted { apply_filters(egui::Color32::from_gray(100)) } else { accent_color });
                                                } else {
                                                    let cx = rect.center().x;
                                                    let track_line = egui::Rect::from_min_max(egui::pos2(cx - 1.5, rect.min.y + 10.0), egui::pos2(cx + 1.5, rect.max.y - 10.0));
                                                    painter.rect_filled(track_line, 1.0, adjust_brightness(bg_color, -0.1));
                                                    
                                                    let tick_x = cx - 12.0;
                                                    let num_ticks = 11;
                                                    for idx in 0..num_ticks {
                                                        let frac = idx as f32 / (num_ticks - 1) as f32;
                                                        let ty = rect.max.y - 10.0 - frac * (rect.height() - 20.0);
                                                        let tick_len = if idx % 5 == 0 { 6.0 } else { 3.0 };
                                                        painter.line_segment(
                                                            [egui::pos2(tick_x - tick_len, ty), egui::pos2(tick_x, ty)],
                                                            egui::Stroke::new(1.0, adjust_brightness(bg_color, 0.1))
                                                        );
                                                    }
                                                    
                                                    let handle_y = rect.max.y - 10.0 - volume * (rect.height() - 20.0);
                                                    let handle_rect = egui::Rect::from_center_size(egui::pos2(cx, handle_y), egui::vec2(16.0, 16.0));
                                                    painter.rect_filled(handle_rect, 3.0, apply_filters(egui::Color32::from_gray(200)));
                                                    painter.rect_stroke(handle_rect, 3.0, egui::Stroke::new(1.0, apply_filters(egui::Color32::from_gray(100))), egui::StrokeKind::Middle);
                                                    painter.circle_filled(handle_rect.center(), 3.0, if is_muted { apply_filters(egui::Color32::from_gray(100)) } else { accent_color });
                                                }
                                            }
                                            crate::types::SliderStyle::ThickTicks => {
                                                if orientation == crate::types::SliderOrientation::Horizontal {
                                                    let cy = rect.center().y;
                                                    let track_rect = egui::Rect::from_min_max(egui::pos2(rect.min.x + 10.0, cy - 4.0), egui::pos2(rect.max.x - 10.0, cy + 4.0));
                                                    painter.rect_filled(track_rect, 2.0, adjust_brightness(bg_color, -0.1));
                                                    let handle_x = rect.min.x + 10.0 + volume * (rect.width() - 20.0);
                                                    let fill_rect = egui::Rect::from_min_max(track_rect.min, egui::pos2(handle_x, track_rect.max.y));
                                                    painter.rect_filled(fill_rect, 2.0, if is_muted { apply_filters(egui::Color32::from_gray(100)) } else { accent_color });
                                                    
                                                    let tick_y = cy + 14.0;
                                                    let num_ticks = 6;
                                                    for idx in 0..num_ticks {
                                                        let frac = idx as f32 / (num_ticks - 1) as f32;
                                                        let tx = rect.min.x + 10.0 + frac * (rect.width() - 20.0);
                                                        painter.line_segment(
                                                            [egui::pos2(tx, tick_y), egui::pos2(tx, tick_y + 4.0)],
                                                            egui::Stroke::new(2.0, adjust_brightness(bg_color, 0.2))
                                                        );
                                                    }
                                                    
                                                    let handle_rect = egui::Rect::from_center_size(egui::pos2(handle_x, cy), egui::vec2(18.0, 18.0));
                                                    painter.rect_filled(handle_rect, 4.0, adjust_brightness(bg_color, 0.3));
                                                    painter.rect_stroke(handle_rect, 4.0, egui::Stroke::new(1.5, accent_color), egui::StrokeKind::Middle);
                                                } else {
                                                    let cx = rect.center().x;
                                                    let track_rect = egui::Rect::from_min_max(egui::pos2(cx - 4.0, rect.min.y + 10.0), egui::pos2(cx + 4.0, rect.max.y - 10.0));
                                                    painter.rect_filled(track_rect, 2.0, adjust_brightness(bg_color, -0.1));
                                                    let handle_y = rect.max.y - 10.0 - volume * (rect.height() - 20.0);
                                                    let fill_rect = egui::Rect::from_min_max(egui::pos2(track_rect.min.x, handle_y), track_rect.max);
                                                    painter.rect_filled(fill_rect, 2.0, if is_muted { apply_filters(egui::Color32::from_gray(100)) } else { accent_color });
                                                    
                                                    let tick_x = cx + 14.0;
                                                    let num_ticks = 6;
                                                    for idx in 0..num_ticks {
                                                        let frac = idx as f32 / (num_ticks - 1) as f32;
                                                        let ty = rect.max.y - 10.0 - frac * (rect.height() - 20.0);
                                                        painter.line_segment(
                                                            [egui::pos2(tick_x, ty), egui::pos2(tick_x + 4.0, ty)],
                                                            egui::Stroke::new(2.0, adjust_brightness(bg_color, 0.2))
                                                        );
                                                    }
                                                    
                                                    let handle_rect = egui::Rect::from_center_size(egui::pos2(cx, handle_y), egui::vec2(18.0, 18.0));
                                                    painter.rect_filled(handle_rect, 4.0, adjust_brightness(bg_color, 0.3));
                                                    painter.rect_stroke(handle_rect, 4.0, egui::Stroke::new(1.5, accent_color), egui::StrokeKind::Middle);
                                                }
                                            }
                                            crate::types::SliderStyle::GradientBar => {
                                                let radius = rect.height().min(rect.width()) * 0.5;
                                                if orientation == crate::types::SliderOrientation::Horizontal {
                                                    painter.rect_filled(rect, radius, adjust_brightness(bg_color, -0.1));
                                                    let fill_width = volume * (rect.width() - 2.0 * radius) + radius;
                                                    let fill_rect = egui::Rect::from_min_max(rect.min, egui::pos2(rect.min.x + fill_width + radius, rect.max.y)).intersect(rect);
                                                    
                                                    let start_color = if is_muted { apply_filters(egui::Color32::from_gray(80)) } else { accent_color };
                                                    let end_color = if is_muted { apply_filters(egui::Color32::from_gray(120)) } else { adjust_brightness(accent_color, 0.3) };
                                                    let mut mesh = egui::Mesh::default();
                                                    let idx = mesh.vertices.len() as u32;
                                                    mesh.vertices.push(egui::epaint::Vertex { pos: fill_rect.left_top(), uv: egui::pos2(0.0,0.0), color: start_color });
                                                    mesh.vertices.push(egui::epaint::Vertex { pos: fill_rect.right_top(), uv: egui::pos2(0.0,0.0), color: end_color });
                                                    mesh.vertices.push(egui::epaint::Vertex { pos: fill_rect.right_bottom(), uv: egui::pos2(0.0,0.0), color: end_color });
                                                    mesh.vertices.push(egui::epaint::Vertex { pos: fill_rect.left_bottom(), uv: egui::pos2(0.0,0.0), color: start_color });
                                                    mesh.indices.extend([idx, idx + 1, idx + 2, idx, idx + 2, idx + 3]);
                                                    painter.add(egui::Shape::mesh(mesh));
                                                    painter.rect_stroke(fill_rect, radius, egui::Stroke::new(1.0, start_color), egui::StrokeKind::Middle);
                                                    
                                                    let handle_center_x = rect.min.x + radius + volume * (rect.width() - 2.0 * radius);
                                                    let handle_rect = egui::Rect::from_center_size(
                                                        egui::pos2(handle_center_x, rect.center().y),
                                                        egui::vec2(12.0, rect.height() - 6.0)
                                                    );
                                                    painter.rect_filled(handle_rect, 4.0, apply_filters(egui::Color32::WHITE));
                                                    painter.rect_stroke(handle_rect, 4.0, egui::Stroke::new(1.0, apply_filters(egui::Color32::from_gray(150))), egui::StrokeKind::Middle);
                                                } else {
                                                    painter.rect_filled(rect, radius, adjust_brightness(bg_color, -0.1));
                                                    let fill_height = volume * (rect.height() - 2.0 * radius) + radius;
                                                    let fill_rect = egui::Rect::from_min_max(egui::pos2(rect.min.x, rect.max.y - fill_height - radius), rect.max).intersect(rect);
                                                    
                                                    let start_color = if is_muted { apply_filters(egui::Color32::from_gray(80)) } else { accent_color };
                                                    let end_color = if is_muted { apply_filters(egui::Color32::from_gray(120)) } else { adjust_brightness(accent_color, 0.3) };
                                                    let mut mesh = egui::Mesh::default();
                                                    let idx = mesh.vertices.len() as u32;
                                                    mesh.vertices.push(egui::epaint::Vertex { pos: fill_rect.left_top(), uv: egui::pos2(0.0,0.0), color: end_color });
                                                    mesh.vertices.push(egui::epaint::Vertex { pos: fill_rect.right_top(), uv: egui::pos2(0.0,0.0), color: end_color });
                                                    mesh.vertices.push(egui::epaint::Vertex { pos: fill_rect.right_bottom(), uv: egui::pos2(0.0,0.0), color: start_color });
                                                    mesh.vertices.push(egui::epaint::Vertex { pos: fill_rect.left_bottom(), uv: egui::pos2(0.0,0.0), color: start_color });
                                                    mesh.indices.extend([idx, idx + 1, idx + 2, idx, idx + 2, idx + 3]);
                                                    painter.add(egui::Shape::mesh(mesh));
                                                    painter.rect_stroke(fill_rect, radius, egui::Stroke::new(1.0, start_color), egui::StrokeKind::Middle);
                                                    
                                                    let handle_center_y = rect.max.y - radius - volume * (rect.height() - 2.0 * radius);
                                                    let handle_rect = egui::Rect::from_center_size(
                                                        egui::pos2(rect.center().x, handle_center_y),
                                                        egui::vec2(rect.width() - 6.0, 12.0)
                                                    );
                                                    painter.rect_filled(handle_rect, 4.0, apply_filters(egui::Color32::WHITE));
                                                    painter.rect_stroke(handle_rect, 4.0, egui::Stroke::new(1.0, apply_filters(egui::Color32::from_gray(150))), egui::StrokeKind::Middle);
                                                }
                                            }
                                            crate::types::SliderStyle::Knob => {
                                                let center = rect.center();
                                                let outer_radius = rect.width().min(rect.height()) * 0.45;
                                                let inner_radius = outer_radius * 0.8;
                                                
                                                painter.circle_stroke(center, outer_radius, egui::Stroke::new(3.0, adjust_brightness(bg_color, -0.2)));
                                                
                                                let start_angle = 135.0_f32.to_radians();
                                                let end_angle = 405.0_f32.to_radians();
                                                let target_angle = start_angle + volume * (end_angle - start_angle);
                                                
                                                let arc_steps = 30;
                                                let mut points = Vec::new();
                                                for idx in 0..=arc_steps {
                                                    let frac = idx as f32 / arc_steps as f32;
                                                    let angle = start_angle + frac * (target_angle - start_angle);
                                                    if angle <= target_angle {
                                                        let x = center.x + angle.cos() * outer_radius;
                                                        let y = center.y + angle.sin() * outer_radius;
                                                        points.push(egui::pos2(x, y));
                                                    }
                                                }
                                                if points.len() >= 2 {
                                                    painter.line(points, egui::Stroke::new(3.0, if is_muted { apply_filters(egui::Color32::from_gray(100)) } else { accent_color }));
                                                }
                                                
                                                painter.circle_filled(center, inner_radius, adjust_brightness(bg_color, 0.15));
                                                painter.circle_stroke(center, inner_radius, egui::Stroke::new(1.0, adjust_brightness(accent_color, -0.1)));
                                                
                                                let pointer_radius = inner_radius * 0.6;
                                                let px = center.x + target_angle.cos() * pointer_radius;
                                                let py = center.y + target_angle.sin() * pointer_radius;
                                                painter.circle_filled(egui::pos2(px, py), 2.5, apply_filters(egui::Color32::WHITE));
                                            }
                                        }

                                        let text_col = apply_filters(egui::Color32::WHITE);
                                        let percentage_text = format!("{}%", (volume * 100.0).round() as i32);
                                        
                                        if mixer_state.style == crate::types::SliderStyle::Knob {
                                            let center = rect.center();
                                            let outer_radius = rect.width().min(rect.height()) * 0.45;
                                            painter.text(
                                                egui::pos2(center.x, center.y + outer_radius + 10.0),
                                                egui::Align2::CENTER_CENTER,
                                                format!("{}: {}", session_name, percentage_text),
                                                egui::FontId::proportional(10.0),
                                                text_col
                                            );
                                        } else if orientation == crate::types::SliderOrientation::Horizontal {
                                            painter.text(
                                                egui::pos2(rect.center().x, rect.top() + 6.0),
                                                egui::Align2::CENTER_CENTER,
                                                format!("{}: {}", session_name, percentage_text),
                                                egui::FontId::proportional(10.0),
                                                text_col
                                            );
                                        } else {
                                            painter.text(
                                                egui::pos2(rect.center().x, rect.top() + 6.0),
                                                egui::Align2::CENTER_CENTER,
                                                session_name,
                                                egui::FontId::proportional(9.0),
                                                text_col
                                            );
                                            painter.text(
                                                egui::pos2(rect.center().x, rect.bottom() - 10.0),
                                                egui::Align2::CENTER_CENTER,
                                                percentage_text,
                                                egui::FontId::proportional(10.0),
                                                text_col
                                            );
                                        }
                                    } else {
                                        let show_all = settings.selected_mixer_apps.is_empty();
                                        
                                        ui.vertical(|ui| {
                                            ui.horizontal(|ui| {
                                                ui.label(egui::RichText::new("Volume Mixer").strong().color(accent_color));
                                                if ui.button("⟲").on_hover_text("Refresh Sessions").clicked() {
                                                    let _ = volume_mixer_cmd_tx.send(crate::volume_mixer::MixerCommand::ForcePoll);
                                                }
                                            });
                                            ui.separator();
                                            
                                            egui::ScrollArea::vertical().max_height(disp_h - 70.0).show(ui, |ui| {
                                                let mut rendered_any = false;
                                                for session in volume_sessions.iter_mut() {
                                                    let is_selected = settings.selected_mixer_apps.contains(&session.name);
                                                    if is_selected || show_all {
                                                        rendered_any = true;
                                                        ui.horizontal(|ui| {
                                                            ui.set_min_height(24.0);
                                                            let short_name = if session.name.len() > 10 {
                                                                format!("{}...", &session.name[..8])
                                                            } else {
                                                                session.name.clone()
                                                            };
                                                            ui.label(&short_name).on_hover_text(&session.name);
                                                            
                                                            let mut vol = session.volume;
                                                            if ui.add_sized([70.0, 16.0], egui::Slider::new(&mut vol, 0.0..=1.0).show_value(false)).changed() {
                                                                session.volume = vol;
                                                                let _ = volume_mixer_cmd_tx.send(crate::volume_mixer::MixerCommand::SetVolume { pid: session.pid, volume: vol });
                                                            }
                                                            
                                                            let mute_text = if session.mute { "🔇" } else { "🔊" };
                                                            if ui.small_button(mute_text).clicked() {
                                                                session.mute = !session.mute;
                                                                let _ = volume_mixer_cmd_tx.send(crate::volume_mixer::MixerCommand::SetMute { pid: session.pid, mute: session.mute });
                                                            }
                                                        });
                                                    }
                                                }
                                                if !rendered_any {
                                                    ui.label("No apps selected.");
                                                }
                                            });
                                            
                                            ui.separator();
                                            egui::CollapsingHeader::new("Visibility Settings").show(ui, |ui| {
                                                let mut active_app_names: Vec<String> = volume_sessions.iter().map(|s| s.name.clone()).collect();
                                                active_app_names.sort();
                                                active_app_names.dedup();
                                                
                                                for app_name in active_app_names {
                                                    let mut is_checked = settings.selected_mixer_apps.contains(&app_name);
                                                    if ui.checkbox(&mut is_checked, &app_name).changed() {
                                                        if is_checked {
                                                            if !settings.selected_mixer_apps.contains(&app_name) {
                                                                settings.selected_mixer_apps.push(app_name);
                                                            }
                                                        } else {
                                                            settings.selected_mixer_apps.retain(|x| x != &app_name);
                                                        }
                                                        settings.save();
                                                    }
                                                }
                                            });
                                        });
                                    }
                                }
                            }
                        });
                    });
                continue;
            }
            if img.is_live {
                let ppp = ui.ctx().pixels_per_point();
                let disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
                let disp_h = img.display_size.unwrap_or([img.size[1] as f32, img.size[1] as f32])[1];
                let center = img.position + egui::vec2(disp_w * 0.5, disp_h * 0.5);

                if img.url.is_none() {
                    let (wx, wy) = crate::winapi_utils::get_window_screen_pos();
                    let (ox, oy) = if settings.use_absolute_screen_coords { (0, 0) } else { (wx, wy) };
                    
                    let src_rect = if let Some(src) = img.source_rect {
                        if img.source_rotation != 0.0 || img.source_skew != egui::Vec2::ZERO || img.source_perspective != [egui::Vec2::ZERO; 4] || img.source_scale != egui::vec2(1.0, 1.0) {
                            let rect_untransformed = egui::Rect::from_min_size(egui::pos2(src[0], src[1]), egui::vec2(src[2], src[3]));
                            let src_center = rect_untransformed.center();
                            let p_arr = img.source_perspective;
                            let c0 = transform_point_complex(rect_untransformed.left_top(), src_center, img.source_rotation, img.source_skew, p_arr, rect_untransformed, img.source_scale);
                            let c1 = transform_point_complex(rect_untransformed.right_top(), src_center, img.source_rotation, img.source_skew, p_arr, rect_untransformed, img.source_scale);
                            let c2 = transform_point_complex(rect_untransformed.left_bottom(), src_center, img.source_rotation, img.source_skew, p_arr, rect_untransformed, img.source_scale);
                            let c3 = transform_point_complex(rect_untransformed.right_bottom(), src_center, img.source_rotation, img.source_skew, p_arr, rect_untransformed, img.source_scale);
                            
                            let min_x = c0.x.min(c1.x).min(c2.x).min(c3.x);
                            let max_x = c0.x.max(c1.x).max(c2.x).max(c3.x);
                            let min_y = c0.y.min(c1.y).min(c2.y).min(c3.y);
                            let max_y = c0.y.max(c1.y).max(c2.y).max(c3.y);
                            [min_x, min_y, max_x - min_x, max_y - min_y]
                        } else {
                            src
                        }
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
                        hwnd: match img.capture_source {
                            CaptureSource::Desktop => 0,
                            CaptureSource::Overlay => img.hwnd,
                            CaptureSource::Origin => img.target_hwnd as usize,
                        },
                        mask: img.mask.clone(),
                        mask_size: img.mask_size.unwrap_or(img.size),
                        exclude_from_capture: settings.exclude_from_capture,
                        snip_points: img.snip_points.clone(),
                        live_performance_mode: settings.live_performance_mode,
                    };
                    
                    _capture_thread.update_request(img.id, req);
                    
                    if let Some(res) = _capture_thread.get_frame(img.id) {
                        if img.gif_recorder.is_recording {
                            let now = std::time::Instant::now();
                            let start = *img.gif_recorder.start_time.get_or_insert(now);
                            let elapsed = now.duration_since(start).as_secs_f32();
                            let target = img.gif_recorder.target_duration_secs as f32;
                            
                            let should_sample = match img.gif_recorder.last_sample_time {
                                Some(last) => now.duration_since(last).as_millis() >= 100,
                                None => true,
                            };

                            if should_sample && elapsed <= target {
                                img.gif_recorder.last_sample_time = Some(now);
                                img.gif_recorder.frames.push((res.pixels.clone(), res.size));
                                img.gif_recorder.status = format!("🔴 Recording: {:.1}s / {}s", elapsed, img.gif_recorder.target_duration_secs);
                            }

                            if elapsed >= target {
                                img.gif_recorder.is_recording = false;
                                img.gif_recorder.status = "Recording finished - save dialog opened".into();
                                let frames_to_encode = std::mem::take(&mut img.gif_recorder.frames);
                                crate::utils::save_gif_recording(frames_to_encode);
                            }
                        }

                        img.size = res.size;
                        img.thumbnail_dirty = true;

                        let upload_start = std::time::Instant::now();

                        let color_image = if let Some(ci) = res.color_image {
                            img.pixels = res.pixels;
                            ci
                        } else {
                            img.pixels = res.pixels.clone();
                            let color_pixels: Vec<egui::Color32> = res.pixels
                                .chunks_exact(4)
                                .map(|c| egui::Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]))
                                .collect();
                            Arc::new(egui::ColorImage {
                                size: res.size,
                                pixels: color_pixels,
                            })
                        };

                        if let Some(tex) = &mut img.texture {
                            tex.set(egui::ImageData::Color(color_image), egui::TextureOptions::LINEAR);
                        } else {
                            img.texture = Some(ui.ctx().load_texture(
                                format!("snip_{}_{}_{}", layer.name, img.id, img.texture_version),
                                egui::ImageData::Color(color_image),
                                egui::TextureOptions::LINEAR,
                            ));
                        }
                        let upload_time = upload_start.elapsed().as_micros();

                        // Accumulate perf metrics
                        perf_stats.frame_count += 1;
                        perf_stats.wgc_gpu_copy_us_sum += res.perf.wgc_gpu_copy_us;
                        perf_stats.wgc_map_wait_us_sum += res.perf.wgc_map_wait_us;
                        perf_stats.wgc_pixel_swap_us_sum += res.perf.wgc_pixel_swap_us;
                        perf_stats.gdi_capture_us_sum += res.perf.gdi_capture_us;
                        perf_stats.thread_mask_effects_us_sum += res.perf.thread_mask_effects_us;
                        perf_stats.thread_color32_conv_us_sum += res.perf.thread_color32_conv_us;
                        perf_stats.thread_total_us_sum += res.perf.thread_total_us;
                        perf_stats.upload_us_sum += upload_time;
                    }
                }
            }

            // --- Thumbnail Update (Static Snapshot for Live Snip) ---
            if img.is_live && img.thumbnail_dirty && !img.pixels.is_empty() {
                if img.thumbnail_texture.is_none() {
                    let color_image = egui::ColorImage::from_rgba_unmultiplied(img.size, &img.pixels);
                    img.thumbnail_texture = Some(ui.ctx().load_texture(
                        format!("thumb_{}_{}_{}", layer.name, img.id, img.texture_version),
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
                        img.texture = Some(ui.ctx().load_texture(format!("gif_{}_{}", img.id, img.texture_version), color_image, Default::default()));
                    }
                }
                ui.ctx().request_repaint();
            } else if !img.pixels.is_empty() {
                // Static or Live Snip
                let should_update_texture = img.texture.is_none() // First load
                    || (!img.is_live && img.thumbnail_dirty);                       // New frame or static update ready



                if should_update_texture {
                    let color_image = egui::ColorImage::from_rgba_unmultiplied(img.size, &img.pixels);
                    if let Some(tex) = &mut img.texture {
                        tex.set(color_image, egui::TextureOptions::LINEAR);
                    } else {
                        img.texture = Some(ui.ctx().load_texture(
                            format!("snip_{}_{}_{}", layer.name, img.id, img.texture_version),
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
                    let size = img.mask_size.unwrap_or(img.size);
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
                    img.cached_mask_outline = None;
                }
            }
            
            if let Some(tex) = &img.texture {
                let disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
                let disp_h = img.display_size.unwrap_or([img.size[1] as f32, img.size[1] as f32])[1];
                let center = (img.position - render_offset) + egui::vec2(disp_w * 0.5, disp_h * 0.5);

                let mut uv_lt = egui::pos2(0.0, 0.0);
                let mut uv_rt = egui::pos2(1.0, 0.0);
                let mut uv_rb = egui::pos2(1.0, 1.0);
                let mut uv_lb = egui::pos2(0.0, 1.0);

                if let Some(src) = img.source_rect {
                    if img.source_rotation != 0.0 || img.source_skew != egui::Vec2::ZERO || img.source_perspective != [egui::Vec2::ZERO; 4] || img.source_scale != egui::vec2(1.0, 1.0) {
                        let rect_untransformed = egui::Rect::from_min_size(egui::pos2(src[0], src[1]), egui::vec2(src[2], src[3]));
                        let src_center = rect_untransformed.center();
                        let p_arr = img.source_perspective;
                        let c0 = transform_point_complex(rect_untransformed.left_top(), src_center, img.source_rotation, img.source_skew, p_arr, rect_untransformed, img.source_scale);
                        let c1 = transform_point_complex(rect_untransformed.right_top(), src_center, img.source_rotation, img.source_skew, p_arr, rect_untransformed, img.source_scale);
                        let c2 = transform_point_complex(rect_untransformed.left_bottom(), src_center, img.source_rotation, img.source_skew, p_arr, rect_untransformed, img.source_scale);
                        let c3 = transform_point_complex(rect_untransformed.right_bottom(), src_center, img.source_rotation, img.source_skew, p_arr, rect_untransformed, img.source_scale);
                        
                        let min_x = c0.x.min(c1.x).min(c2.x).min(c3.x);
                        let max_x = c0.x.max(c1.x).max(c2.x).max(c3.x);
                        let min_y = c0.y.min(c1.y).min(c2.y).min(c3.y);
                        let max_y = c0.y.max(c1.y).max(c2.y).max(c3.y);
                        let aabb = egui::Rect::from_min_max(egui::pos2(min_x, min_y), egui::pos2(max_x, max_y));
                        let sz = aabb.size();
                        if sz.x > 0.0 && sz.y > 0.0 {
                            uv_lt = egui::pos2((c0.x - aabb.min.x) / sz.x, (c0.y - aabb.min.y) / sz.y);
                            uv_rt = egui::pos2((c1.x - aabb.min.x) / sz.x, (c1.y - aabb.min.y) / sz.y);
                            uv_lb = egui::pos2((c2.x - aabb.min.x) / sz.x, (c2.y - aabb.min.y) / sz.y);
                            uv_rb = egui::pos2((c3.x - aabb.min.x) / sz.x, (c3.y - aabb.min.y) / sz.y);
                        }
                    }
                }

                let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));

                let mut final_scale = img.scale;
                if img.flipped_h { final_scale.x *= -1.0; }
                if img.flipped_v { final_scale.y *= -1.0; }

                let has_gl_effect = img.blur > 0.1 || img.grayscale || img.invert || img.sepia || img.glow || layer.grayscale || layer.invert || layer.sepia || layer.glow;
                let effect_pad = if img.blur > 0.1 || layer.blur > 0.1 {
                    let eff_strength = img.blur.max(layer.blur) * 0.2;
                    let active_effect = if img.blur > 0.1 { img.blur_effect } else { layer.blur_effect };
                    match active_effect {
                        BlurEffect::Glitch => eff_strength * 12.0,
                        BlurEffect::Gaussian => eff_strength * 5.0,
                        BlurEffect::Pixelate => eff_strength * 2.0,
                    }
                } else { 0.0 };

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

                // VHS/Glitch effect needs continuous repaints so u_time advances each frame
                if final_effect == 3 {
                    ui.ctx().request_repaint();
                }

                let chromatic_val = if img.chromatic_aberration > 0.0 { img.chromatic_aberration } else { layer.chromatic_aberration };
                let aa_image = img.antialias || layer.antialias;
                let aa_outline = img.antialias_outline || layer.antialias_outline;
                let aa_shadow = img.antialias_shadow || layer.antialias_shadow;

                // Draw helper closure
                // apply_filters = true only for the main image pass, NOT for shadow/outline/glow silhouette passes
                let draw_pass = |
                    is_shadow: bool,
                    pass_antialias: bool,
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
                    let mut pass_pad = if apply_filters { effect_pad } else if pass_blur_strength > 0.0 { pass_blur_strength.max(20.0) } else { 0.0 };
                    
                    if pass_antialias {
                        pass_pad += 6.0;
                    }

                    let effective_spread = spread + (if is_shadow { pass_blur_strength * 0.5 } else { 0.0 });
                    if effective_spread > 0.0 {
                        pass_pad += effective_spread;
                        let scale_x = 1.0 + (effective_spread / disp_w.max(1.0)) * 2.0;
                        let scale_y = 1.0 + (effective_spread / disp_h.max(1.0)) * 2.0;
                        draw_scale.x *= scale_x;
                        draw_scale.y *= scale_y;
                    }
                    
                    let mut uv_lt_p = uv_lt;
                    let mut uv_rt_p = uv_rt;
                    let mut uv_rb_p = uv_rb;
                    let mut uv_lb_p = uv_lb;
                    
                    if pass_pad > 0.0 {
                        // Extrapolate UVs outwards from the center of the UV quad
                        let uv_center = egui::pos2(
                            (uv_lt.x + uv_rt.x + uv_rb.x + uv_lb.x) * 0.25,
                            (uv_lt.y + uv_rt.y + uv_rb.y + uv_lb.y) * 0.25,
                        );
                        let scale_x = 1.0 + (pass_pad / disp_w.max(1.0)) * 2.0;
                        let scale_y = 1.0 + (pass_pad / disp_h.max(1.0)) * 2.0;
                        
                        let pad_uv = |p: egui::Pos2| {
                            let dx = p.x - uv_center.x;
                            let dy = p.y - uv_center.y;
                            egui::pos2(uv_center.x + dx * scale_x, uv_center.y + dy * scale_y)
                        };
                        uv_lt_p = pad_uv(uv_lt);
                        uv_rt_p = pad_uv(uv_rt);
                        uv_rb_p = pad_uv(uv_rb);
                        uv_lb_p = pad_uv(uv_lb);
                    }
                    
                    if pass_pad > 0.0 {
                        padded_w += pass_pad * 2.0;
                        padded_h += pass_pad * 2.0;
                        add_quad_with_custom_uv(
                            &mut mesh,
                            egui::Rect::from_min_size(egui::pos2(center.x + offset_x - padded_w*0.5, center.y + offset_y - padded_h*0.5), egui::vec2(padded_w, padded_h)),
                            uv_lt_p,
                            uv_rt_p,
                            uv_rb_p,
                            uv_lb_p,
                            egui::Color32::WHITE
                        );
                    } else {
                        add_quad_with_custom_uv(
                            &mut mesh,
                            egui::Rect::from_min_size(egui::pos2(center.x + offset_x - disp_w*0.5, center.y + offset_y - disp_h*0.5), egui::vec2(disp_w, disp_h)),
                            uv_lt_p,
                            uv_rt_p,
                            uv_rb_p,
                            uv_lb_p,
                            egui::Color32::WHITE
                        );
                    }
                    
                    transform_mesh(&mut mesh, center + egui::vec2(offset_x, offset_y), img.rotation, img.skew, img.perspective, draw_scale);

                    // For silhouette passes (shadow/outline/glow), only use GL when blur effect is active
                    // so the blur kernel can spread the silhouette halo. Otherwise use fast software path.
                    let use_gl = gl_renderer.is_some() && (apply_filters || final_effect > 0 || pass_blur_strength > 0.0 || spread > 0.0 || chromatic_val > 0.0 || aa_image || aa_outline || aa_shadow);

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
                                    renderer.render_effect(gl, gl_tex, gl_mask, actual_effect, actual_strength, res, time, pass_gray, pass_inv, pass_sepia, tint, is_shadow, pass_opacity, vertex_count, &mapped_vertices, if apply_filters { chromatic_val } else { 0.0 }, pass_antialias);
                                    
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
                        ([0, 0, 0, 255], [6.0, 6.0], 0.0)
                    };
                    let tint = [s_col_arr[0] as f32 / 255.0, s_col_arr[1] as f32 / 255.0, s_col_arr[2] as f32 / 255.0, s_col_arr[3] as f32 / 255.0];
                    draw_pass(true, aa_shadow, false, s_off[0], s_off[1], s_spread, if img.shadow { img.shadow_blur } else { layer.shadow_blur }, tint, l_op * img.opacity);
                }

                if layer.outline || img.outline {
                    let (o_col_arr, o_width) = if img.outline { (img.outline_color, img.outline_width) } else { (layer.outline_color, layer.outline_width) };
                    let tint = [o_col_arr[0] as f32 / 255.0, o_col_arr[1] as f32 / 255.0, o_col_arr[2] as f32 / 255.0, o_col_arr[3] as f32 / 255.0];
                    let steps = 8;
                    for i in 0..steps {
                        let angle = (i as f32) * std::f32::consts::TAU / (steps as f32);
                        let off_x = angle.cos() * o_width;
                        let off_y = angle.sin() * o_width;
                        draw_pass(true, aa_outline, false, off_x, off_y, 0.0, 0.0, tint, l_op * img.opacity);
                    }
                }

                // Render main image pass
                draw_pass(false, aa_image, true, 0.0, 0.0, 0.0, 0.0, [1.0, 1.0, 1.0, 1.0], l_op * img.opacity);

                // Render Glow over the image content
                let has_glow = layer.glow || img.glow;
                if has_glow {
                    let (g_col_arr, g_str, g_spread) = if img.glow { 
                        (img.glow_color, img.glow_strength, img.glow_spread) 
                    } else {
                        (layer.glow_color, layer.glow_strength, layer.glow_spread)
                    };
                    let alpha = (g_str / 100.0).clamp(0.0, 1.0);
                    let tint = [g_col_arr[0] as f32 / 255.0, g_col_arr[1] as f32 / 255.0, g_col_arr[2] as f32 / 255.0, g_col_arr[3] as f32 / 255.0 * alpha];
                    // Render glow silhouette over image
                    draw_pass(true, aa_outline, true, 0.0, 0.0, g_spread, 10.0, tint, l_op * img.opacity);
                }
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
                        mask: None,
                        mask_size: [0, 0],
                        exclude_from_capture: settings.exclude_from_capture,
                        snip_points: None,
                        live_performance_mode: settings.live_performance_mode,
                    };
                    _capture_thread.update_request(s.id, req);

                    if let Some(res) = _capture_thread.get_frame(s.id) {
                        let color_img = egui::ColorImage::from_rgba_unmultiplied(res.size, &res.pixels);
                        let upload_start = std::time::Instant::now();
                        if let Some(ref mut tex) = s.cached_texture {
                            tex.set(color_img, egui::TextureOptions::LINEAR);
                        } else {
                            s.cached_texture = Some(ui.ctx().load_texture(
                                format!("blur_stroke_{}", s.id),
                                color_img,
                                egui::TextureOptions::LINEAR,
                            ));
                        }
                        let upload_time = upload_start.elapsed().as_micros();

                        // Accumulate perf metrics
                        perf_stats.frame_count += 1;
                        perf_stats.wgc_gpu_copy_us_sum += res.perf.wgc_gpu_copy_us;
                        perf_stats.wgc_map_wait_us_sum += res.perf.wgc_map_wait_us;
                        perf_stats.wgc_pixel_swap_us_sum += res.perf.wgc_pixel_swap_us;
                        perf_stats.gdi_capture_us_sum += res.perf.gdi_capture_us;
                        perf_stats.thread_mask_effects_us_sum += res.perf.thread_mask_effects_us;
                        perf_stats.thread_color32_conv_us_sum += res.perf.thread_color32_conv_us;
                        perf_stats.thread_total_us_sum += res.perf.thread_total_us;
                        perf_stats.upload_us_sum += upload_time;
                    }
                    ui.ctx().request_repaint();
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
        pos,
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
    if rasterize_phase == 0 && !ctx.settings.hide_all {
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

        // Helper for drawing dashed path
        fn simplify_path(points: &[egui::Pos2], epsilon: f32) -> Vec<egui::Pos2> {
            if points.len() < 3 {
                return points.to_vec();
            }
            
            let mut dmax = 0.0;
            let mut index = 0;
            let end = points.len() - 1;
            
            let p_start = points[0];
            let p_end = points[end];
            let line_vec = p_end - p_start;
            let line_len_sq = line_vec.length_sq();
            
            for i in 1..end {
                let p = points[i];
                let dist = if line_len_sq < 1e-6 {
                    (p - p_start).length()
                } else {
                    let t = ((p - p_start).dot(line_vec) / line_len_sq).clamp(0.0, 1.0);
                    let projection = p_start + line_vec * t;
                    (p - projection).length()
                };
                
                if dist > dmax {
                    index = i;
                    dmax = dist;
                }
            }
            
            if dmax > epsilon {
                let mut rec_results1 = simplify_path(&points[..=index], epsilon);
                let rec_results2 = simplify_path(&points[index..], epsilon);
                if !rec_results1.is_empty() {
                    rec_results1.pop();
                }
                rec_results1.extend(rec_results2);
                rec_results1
            } else {
                vec![p_start, p_end]
            }
        }

        let draw_dashed_path = |painter: &egui::Painter, points: &[egui::Pos2], time: f64| {
            if points.len() < 2 { return; }
            let simplified = if points.len() > 10 {
                simplify_path(points, 2.0)
            } else {
                points.to_vec()
            };
            crate::utils::draw_dashed_path(painter, &simplified, time);
        };

        let draw_selection_shape = |painter: &egui::Painter, shape: &crate::types::SelectionShape, render_offset: egui::Vec2, time: f64| {
            match shape {
                crate::types::SelectionShape::Rect(rect) => {
                    let r = rect.translate(-render_offset);
                    let pts = vec![
                        r.left_top(),
                        r.right_top(),
                        r.right_bottom(),
                        r.left_bottom(),
                        r.left_top(),
                    ];
                    draw_dashed_path(painter, &pts, time);
                }
                crate::types::SelectionShape::Circle { center, radius } => {
                    let c = *center - render_offset;
                    let mut pts = Vec::with_capacity(61);
                    for i in 0..=60 {
                        let angle = i as f32 * std::f32::consts::TAU / 60.0;
                        pts.push(c + egui::vec2(angle.cos() * radius, angle.sin() * radius));
                    }
                    draw_dashed_path(painter, &pts, time);
                }
                crate::types::SelectionShape::Poly(pts) => {
                    if pts.len() >= 2 {
                        let mut closed_pts: Vec<egui::Pos2> = pts.iter().map(|&p| p - render_offset).collect();
                        closed_pts.push(pts[0] - render_offset);
                        draw_dashed_path(painter, &closed_pts, time);
                    }
                }
            }
        };

        let time = ctx.ui.input(|i| i.time);
        let painter = ctx.ui.painter_at(ctx.canvas_response.rect);

        // 1. Render marquee selection outline (marching ants) — only when marquee tools are active
        let show_marching_ants = matches!(*ctx.active_tool, Tool::Cut | Tool::Snip);
        if edit_mode && !ctx.settings.hide_all && show_marching_ants {
            if let Some(sel) = &ctx.project.marquee_selection {
                let bounds = sel.bounds();
                if bounds.width() > 0.1 && bounds.height() > 0.1 {
                    if sel.ops.is_empty() {
                        draw_selection_shape(&painter, &sel.shape, ctx.render_offset, time);
                    } else {
                        let max_dim = bounds.width().max(bounds.height());
                        let step = (max_dim / 150.0).clamp(3.0, 10.0);
                        
                        let pad = 6.0f32;
                        let min_x = bounds.min.x - pad;
                        let min_y = bounds.min.y - pad;
                        let max_x = bounds.max.x + pad;
                        let max_y = bounds.max.y + pad;
                        
                        let cols = ((max_x - min_x) / step).ceil() as i32 + 1;
                        let rows = ((max_y - min_y) / step).ceil() as i32 + 1;
                        
                        let mut grid = vec![false; (cols * rows) as usize];
                        for r in 0..rows {
                            let y = min_y + r as f32 * step;
                            for c in 0..cols {
                                let x = min_x + c as f32 * step;
                                let p = egui::pos2(x, y);
                                grid[(r * cols + c) as usize] = sel.contains(p);
                            }
                        }
                        
                        let mut edges = std::collections::HashSet::new();
                        let mut adjacency: std::collections::HashMap<(i32, i32), Vec<(i32, i32)>> = std::collections::HashMap::new();
                        
                        let mut add_edge = |p1: (i32, i32), p2: (i32, i32)| {
                            if p1 == p2 { return; }
                            let edge = if p1 < p2 { (p1, p2) } else { (p2, p1) };
                            if edges.insert(edge) {
                                adjacency.entry(p1).or_default().push(p2);
                                adjacency.entry(p2).or_default().push(p1);
                            }
                        };
                        
                        for r in 0..(rows - 1) {
                            for c in 0..(cols - 1) {
                                let tl = grid[(r * cols + c) as usize];
                                let tr = grid[(r * cols + (c + 1)) as usize];
                                let br = grid[((r + 1) * cols + (c + 1)) as usize];
                                let bl = grid[((r + 1) * cols + c) as usize];
                                
                                let index = ((tl as usize) << 3) | ((tr as usize) << 2) | ((br as usize) << 1) | bl as usize;
                                if index == 0 || index == 15 {
                                    continue;
                                }
                                
                                let m0 = (2 * c + 1, 2 * r);
                                let m1 = (2 * c + 2, 2 * r + 1);
                                let m2 = (2 * c + 1, 2 * r + 2);
                                let m3 = (2 * c, 2 * r + 1);
                                
                                match index {
                                    1 => add_edge(m2, m3),
                                    2 => add_edge(m1, m2),
                                    3 => add_edge(m1, m3),
                                    4 => add_edge(m0, m1),
                                    5 => {
                                        add_edge(m0, m3);
                                        add_edge(m1, m2);
                                    }
                                    6 => add_edge(m0, m2),
                                    7 => add_edge(m0, m3),
                                    8 => add_edge(m0, m3),
                                    9 => add_edge(m0, m2),
                                    10 => {
                                        add_edge(m0, m1);
                                        add_edge(m2, m3);
                                    }
                                    11 => add_edge(m0, m1),
                                    12 => add_edge(m1, m3),
                                    13 => add_edge(m1, m2),
                                    14 => add_edge(m2, m3),
                                    _ => {}
                                }
                            }
                        }
                        
                        let mut loops: Vec<Vec<egui::Pos2>> = Vec::new();
                        while !edges.is_empty() {
                            let &edge = edges.iter().next().unwrap();
                            edges.remove(&edge);
                            
                            let (start, mut current) = edge;
                            let mut path = vec![start, current];
                            
                            loop {
                                let mut next_opt = None;
                                if let Some(neighbors) = adjacency.get(&current) {
                                    for &n in neighbors {
                                        let test_edge = if current < n { (current, n) } else { (n, current) };
                                        if edges.contains(&test_edge) {
                                            next_opt = Some((n, test_edge));
                                            break;
                                        }
                                    }
                                }
                                
                                if let Some((next, e)) = next_opt {
                                    edges.remove(&e);
                                    path.push(next);
                                    current = next;
                                } else {
                                    let closing_edge = if current < start { (current, start) } else { (start, current) };
                                    if edges.contains(&closing_edge) {
                                        edges.remove(&closing_edge);
                                    }
                                    if path.last() != Some(&start) {
                                        path.push(start);
                                    }
                                    break;
                                }
                            }
                            
                            if path.len() >= 3 {
                                let mut mapped_path: Vec<egui::Pos2> = path.into_iter().map(|pt| {
                                    let screen_x = min_x + (pt.0 as f32 / 2.0) * step;
                                    let screen_y = min_y + (pt.1 as f32 / 2.0) * step;
                                    egui::pos2(screen_x - ctx.render_offset.x, screen_y - ctx.render_offset.y)
                                }).collect();
                                // Force-close the contour so the marching ants loop seamlessly
                                if mapped_path.len() >= 2 && mapped_path.first() != mapped_path.last() {
                                    mapped_path.push(mapped_path[0]);
                                }
                                loops.push(mapped_path);
                            }
                        }
                        
                        // Draw all loops simplified and crawled gracefully
                        for path in &loops {
                            draw_dashed_path(&painter, path, time);
                        }
                    }
                }
                ctx.ui.ctx().request_repaint();
            }
        }

        // 2. Draw show source outlines for all visible layers and placed images (solid orange line style)
        if edit_mode && !ctx.settings.hide_all {
            let mut has_show_source = false;
            for (layer_idx, layer) in ctx.project.layers.iter_mut().enumerate() {
                if !layer.visible { continue; }
                for (img_idx, img) in layer.placed_images.iter_mut().enumerate() {
                    let is_selected = if let Some(sel) = &ctx.project.selected_object {
                        sel.layer_idx == layer_idx && sel.object_type == crate::types::ObjectType::Image && sel.object_idx == img_idx
                    } else {
                        false
                    };
                    if is_selected && img.show_source_rect && img.source_rect.is_some() {
                        has_show_source = true;
                        let src = img.source_rect.unwrap();
                        let src_rect = egui::Rect::from_min_size(egui::pos2(src[0], src[1]), egui::vec2(src[2], src[3]));

                        let stroke_color = if is_selected {
                            egui::Color32::from_rgb(255, 140, 0) // Orange
                        } else {
                            egui::Color32::from_rgb(200, 100, 0) // Muted orange-gray
                        };

                        let stroke_width = if is_selected { 1.8f32 } else { 1.2f32 };
                        
                        if let Some(ref mask_ref) = img.mask {
                            if img.cached_mask_outline.is_none() {
                                let ppp = ctx.ui.ctx().pixels_per_point();
                                img.cached_mask_outline = Some(get_mask_outline(mask_ref, img.mask_size.unwrap_or(img.size), ppp));
                            }
                            if let Some(ref loops) = img.cached_mask_outline {
                                let src_center = src_rect.center();
                                let p_arr = img.source_perspective;
                                let mask_sz = img.mask_size.unwrap_or(img.size);
                                let mask_w = (mask_sz[0] as f32).max(1.0);
                                let mask_h = (mask_sz[1] as f32).max(1.0);
                                for path in loops {
                                    let mut current_path = Vec::with_capacity(path.len());
                                    for &p in path {
                                        let norm_x = p.x / mask_w;
                                        let norm_y = p.y / mask_h;
                                        let world_pt = egui::pos2(src_rect.min.x + norm_x * src_rect.width(), src_rect.min.y + norm_y * src_rect.height());
                                        let transformed = crate::utils::transform_point_complex(world_pt, src_center, img.source_rotation, img.source_skew, p_arr, src_rect, img.source_scale) - ctx.render_offset;
                                        current_path.push(transformed);
                                    }
                                    if !current_path.is_empty() {
                                        crate::utils::draw_dashed_path_color(&painter, &current_path, time, stroke_color, stroke_width);
                                    }
                                }
                            }
                        } else if let Some(ref local_pts) = img.snip_points {
                            let src_center = src_rect.center();
                            let p_arr = img.source_perspective;
                            let mut max_x = 0.0f32;
                            let mut max_y = 0.0f32;
                            for p in local_pts {
                                if !p.x.is_nan() && p.x > max_x { max_x = p.x; }
                                if !p.y.is_nan() && p.y > max_y { max_y = p.y; }
                            }
                            let bounds_w = max_x.max(1.0);
                            let bounds_h = max_y.max(1.0);

                            let mut current_path = Vec::new();
                            for p in local_pts {
                                if p.x.is_nan() || p.y.is_nan() {
                                    if !current_path.is_empty() {
                                        if current_path.len() >= 2 && current_path.first() != current_path.last() {
                                            current_path.push(*current_path.first().unwrap());
                                        }
                                        let simplified = crate::utils::simplify_path(&current_path, 0.5);
                                        let mut closed = simplified;
                                        if closed.len() >= 2 && closed.first() != closed.last() {
                                            closed.push(*closed.first().unwrap());
                                        }
                                        crate::utils::draw_dashed_path_color(&painter, &closed, time, stroke_color, stroke_width);
                                        current_path.clear();
                                    }
                                } else {
                                    let norm_x = p.x / bounds_w;
                                    let norm_y = p.y / bounds_h;
                                    let world_pt = egui::pos2(src_rect.min.x + norm_x * src_rect.width(), src_rect.min.y + norm_y * src_rect.height());
                                    let transformed = crate::utils::transform_point_complex(world_pt, src_center, img.source_rotation, img.source_skew, p_arr, src_rect, img.source_scale) - ctx.render_offset;
                                    current_path.push(transformed);
                                }
                            }
                            if !current_path.is_empty() {
                                if current_path.len() >= 2 && current_path.first() != current_path.last() {
                                    current_path.push(*current_path.first().unwrap());
                                }
                                let simplified = crate::utils::simplify_path(&current_path, 0.5);
                                let mut closed = simplified;
                                if closed.len() >= 2 && closed.first() != closed.last() {
                                    closed.push(*closed.first().unwrap());
                                }
                                crate::utils::draw_dashed_path_color(&painter, &closed, time, stroke_color, stroke_width);
                            }
                        } else {
                            let r = src_rect.translate(-ctx.render_offset);
                            let pts = if img.source_rotation != 0.0 || img.source_skew != egui::Vec2::ZERO || img.source_perspective != [egui::Vec2::ZERO; 4] || img.source_scale != egui::vec2(1.0, 1.0) {
                                let src_center = src_rect.center();
                                let p_arr = img.source_perspective;
                                let c0 = transform_point_complex(src_rect.left_top(), src_center, img.source_rotation, img.source_skew, p_arr, src_rect, img.source_scale) - ctx.render_offset;
                                let c1 = transform_point_complex(src_rect.right_top(), src_center, img.source_rotation, img.source_skew, p_arr, src_rect, img.source_scale) - ctx.render_offset;
                                let c2 = transform_point_complex(src_rect.left_bottom(), src_center, img.source_rotation, img.source_skew, p_arr, src_rect, img.source_scale) - ctx.render_offset;
                                let c3 = transform_point_complex(src_rect.right_bottom(), src_center, img.source_rotation, img.source_skew, p_arr, src_rect, img.source_scale) - ctx.render_offset;
                                vec![c0, c1, c3, c2, c0]
                            } else {
                                vec![
                                    r.left_top(),
                                    r.right_top(),
                                    r.right_bottom(),
                                    r.left_bottom(),
                                    r.left_top(),
                                ]
                            };
                            crate::utils::draw_dashed_path_color(&painter, &pts, time, stroke_color, stroke_width);
                        }
                    }
                }
            }
            if has_show_source {
                ctx.ui.ctx().request_repaint();
            }
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
        if project.layers.is_empty() {
            project.layers.push(crate::project::Layer::new("Layer 1"));
        }
        project.active_layer = project.active_layer.saturating_sub(1);
        project.selected_object = None; // Prevent crash by clearing selection to removed layer
    }

    
    if let Some(sel) = new_selection { project.selected_object = Some(sel); }
    if switch_to_move { *active_tool = Tool::Move; }
}

fn get_mask_outline(mask: &[u8], size: [usize; 2], _ppp: f32) -> Vec<Vec<egui::Pos2>> {
    let w = size[0];
    let h = size[1];
    if w == 0 || h == 0 { return Vec::new(); }

    let step = (w.max(h) as f32 / 400.0).max(1.0);
    let sample_cols = (w as f32 / step).ceil() as i32 + 1;
    let sample_rows = (h as f32 / step).ceil() as i32 + 1;

    let cols = sample_cols + 2;
    let rows = sample_rows + 2;

    let mut grid = vec![false; (cols * rows) as usize];
    for r in 0..sample_rows {
        let py = ((r as f32 * step).round() as usize).min(h - 1);
        for c in 0..sample_cols {
            let px = ((c as f32 * step).round() as usize).min(w - 1);
            grid[((r + 1) * cols + (c + 1)) as usize] = mask[py * w + px] > 127;
        }
    }

    let mut edges = std::collections::HashSet::new();
    let mut adjacency: std::collections::HashMap<(i32, i32), Vec<(i32, i32)>> = std::collections::HashMap::new();

    let mut add_edge = |p1: (i32, i32), p2: (i32, i32)| {
        if p1 == p2 { return; }
        let edge = if p1 < p2 { (p1, p2) } else { (p2, p1) };
        if edges.insert(edge) {
            adjacency.entry(p1).or_default().push(p2);
            adjacency.entry(p2).or_default().push(p1);
        }
    };

    for r in 0..(rows - 1) {
        for c in 0..(cols - 1) {
            let tl = grid[(r * cols + c) as usize];
            let tr = grid[(r * cols + (c + 1)) as usize];
            let br = grid[((r + 1) * cols + (c + 1)) as usize];
            let bl = grid[((r + 1) * cols + c) as usize];

            let index = ((tl as usize) << 3) | ((tr as usize) << 2) | ((br as usize) << 1) | bl as usize;
            if index == 0 || index == 15 {
                continue;
            }

            let m0 = (2 * c + 1, 2 * r);
            let m1 = (2 * c + 2, 2 * r + 1);
            let m2 = (2 * c + 1, 2 * r + 2);
            let m3 = (2 * c, 2 * r + 1);

            match index {
                1 => add_edge(m2, m3),
                2 => add_edge(m1, m2),
                3 => add_edge(m1, m3),
                4 => add_edge(m0, m1),
                5 => {
                    add_edge(m0, m3);
                    add_edge(m1, m2);
                }
                6 => add_edge(m0, m2),
                7 => add_edge(m0, m3),
                8 => add_edge(m0, m3),
                9 => add_edge(m0, m2),
                10 => {
                    add_edge(m0, m1);
                    add_edge(m2, m3);
                }
                11 => add_edge(m0, m1),
                12 => add_edge(m1, m3),
                13 => add_edge(m1, m2),
                14 => add_edge(m2, m3),
                _ => {}
            }
        }
    }

    let mut loops: Vec<Vec<egui::Pos2>> = Vec::new();
    while !edges.is_empty() {
        let &edge = edges.iter().next().unwrap();
        edges.remove(&edge);

        let (start, mut current) = edge;
        let mut path = vec![start, current];

        loop {
            let mut next_opt = None;
            if let Some(neighbors) = adjacency.get(&current) {
                for &n in neighbors {
                    let test_edge = if current < n { (current, n) } else { (n, current) };
                    if edges.contains(&test_edge) {
                        next_opt = Some((n, test_edge));
                        break;
                    }
                }
            }

            if let Some((next, e)) = next_opt {
                edges.remove(&e);
                path.push(next);
                current = next;
            } else {
                let closing_edge = if current < start { (current, start) } else { (start, current) };
                if edges.contains(&closing_edge) {
                    edges.remove(&closing_edge);
                }
                if path.last() != Some(&start) {
                    path.push(start);
                }
                break;
            }
        }

        if path.len() >= 3 {
            let total_cols = (cols - 2) as f32;
            let total_rows = (rows - 2) as f32;
            let mapped_path: Vec<egui::Pos2> = path.into_iter().map(|pt| {
                let norm_x = ((pt.0 as f32 - 2.0) / (2.0 * total_cols)).clamp(0.0, 1.0);
                let norm_y = ((pt.1 as f32 - 2.0) / (2.0 * total_rows)).clamp(0.0, 1.0);
                let lx = norm_x * w as f32;
                let ly = norm_y * h as f32;
                egui::pos2(lx, ly)
            }).collect();
            loops.push(mapped_path);
        }
    }
    loops
}

