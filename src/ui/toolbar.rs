use eframe::egui;
use crate::types::*;
// use crate::project::Project;
use rayon::prelude::*;

pub fn photoshop_frame(settings: &Settings) -> egui::Frame {
    egui::Frame::none()
        .fill(color32(&settings.toolbar_bg_color))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(60)))
        .corner_radius(egui::CornerRadius::same(10))
        .inner_margin(egui::Margin::symmetric(6, 6))
}

use crate::utils::color32;

pub fn apply_box_blur(pixels: &mut [u8], width: usize, height: usize, radius: usize) {
    let radius = radius.min(100).max(1);
    let copy = pixels.to_vec();
    for y in 0..height {
        for x in 0..width {
            let mut r = 0u32; let mut g = 0u32; let mut b = 0u32;
            let mut count = 0;
            for dy in -(radius as i32)..=(radius as i32) {
                let ny = y as i32 + dy;
                if ny >= 0 && ny < height as i32 {
                    for dx in -(radius as i32)..=(radius as i32) {
                        let nx = x as i32 + dx;
                        if nx >= 0 && nx < width as i32 {
                            let idx = (ny as usize * width + nx as usize) * 4;
                            r += copy[idx] as u32;
                            g += copy[idx + 1] as u32;
                            b += copy[idx + 2] as u32;
                            count += 1;
                        }
                    }
                }
            }
            let idx = (y * width + x) * 4;
            pixels[idx] = (r / count) as u8;
            pixels[idx + 1] = (g / count) as u8;
            pixels[idx + 2] = (b / count) as u8;
        }
    }
}

pub fn apply_pixelate(pixels: &mut [u8], width: usize, height: usize, scale: usize) {
    let scale = scale.max(1).min(64);
    if scale <= 1 { return; }
    
    for y in (0..height).step_by(scale) {
        for x in (0..width).step_by(scale) {
            let mut r = 0u32; let mut g = 0u32; let mut b = 0u32; let mut a = 0u32;
            let mut count = 0;
            
            for py in 0..scale {
                for px in 0..scale {
                    let nx = x + px;
                    let ny = y + py;
                    if nx < width && ny < height {
                        let idx = (ny * width + nx) * 4;
                        r += pixels[idx] as u32;
                        g += pixels[idx + 1] as u32;
                        b += pixels[idx + 2] as u32;
                        a += pixels[idx + 3] as u32;
                        count += 1;
                    }
                }
            }
            
            if count > 0 {
                let r = (r / count) as u8;
                let g = (g / count) as u8;
                let b = (b / count) as u8;
                let a = (a / count) as u8;
                
                for py in 0..scale {
                    for px in 0..scale {
                        let nx = x + px;
                        let ny = y + py;
                        if nx < width && ny < height {
                            let idx = (ny * width + nx) * 4;
                            pixels[idx] = r;
                            pixels[idx + 1] = g;
                            pixels[idx + 2] = b;
                            pixels[idx + 3] = a;
                        }
                    }
                }
            }
        }
    }
}

#[allow(dead_code)]
fn apply_diamond_blur(pixels: &mut [u8], width: usize, height: usize, radius: usize) {
    let radius = radius.min(16).max(1); // Reduced max radius for performance
    let copy = pixels.to_vec();
    
    pixels.par_chunks_exact_mut(width * 4).enumerate().for_each(|(y, row)| {
        for x in 0..width {
            let mut r = 0u32; let mut g = 0u32; let mut b = 0u32; let mut a = 0u32;
            let mut count = 0;
            
            // Optimized diamond kernel: |dx| + |dy| <= radius
            for dy in -(radius as i32)..=(radius as i32) {
                let ny = y as i32 + dy;
                if ny < 0 || ny >= height as i32 { continue; }
                
                let dx_max = radius as i32 - dy.abs();
                for dx in -dx_max..=dx_max {
                    let nx = x as i32 + dx;
                    if nx >= 0 && nx < width as i32 {
                        let idx = (ny as usize * width + nx as usize) * 4;
                        r += copy[idx] as u32;
                        g += copy[idx + 1] as u32;
                        b += copy[idx + 2] as u32;
                        a += copy[idx + 3] as u32;
                        count += 1;
                    }
                }
            }
            
            if count > 0 {
                let idx = x * 4;
                row[idx] = (r / count) as u8;
                row[idx + 1] = (g / count) as u8;
                row[idx + 2] = (b / count) as u8;
                row[idx + 3] = (a / count) as u8;
            }
        }
    });
}

pub fn apply_vhs_glitch(pixels: &mut [u8], width: usize, height: usize, intensity: f32) {
    let intensity = intensity.min(1.0).max(0.0);
    if intensity < 0.01 { return; }
    
    let copy = pixels.to_vec();
    let mut rng = 12345u64;
    
    for y in 0..height {
        rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        // Increased shift range for dramatic blur at 100%
        let shift_range = 60.0 * intensity;
        let shift = (((rng >> 32) as f32 / 4294967295.0) * shift_range) as i32 - (shift_range * 0.5) as i32;
        
        let row_idx = y * width * 4;
        for x in 0..width {
            let nx = (x as i32 + shift).clamp(0, width as i32 - 1) as usize;
            let target_idx = row_idx + x * 4;
            let source_idx = row_idx + nx * 4;
            
            let color_offset = (10.0 * intensity) as i32;
            let rx = (nx as i32 + color_offset).clamp(0, width as i32 - 1) as usize;
            let bx = (nx as i32 - color_offset).clamp(0, width as i32 - 1) as usize;
            
            pixels[target_idx] = copy[row_idx + rx * 4];
            pixels[target_idx + 1] = copy[source_idx + 1];
            pixels[target_idx + 2] = copy[row_idx + bx * 4 + 2];
            pixels[target_idx + 3] = copy[source_idx + 3];
            
            if (rng % 150) < (15.0 * intensity) as u64 {
                pixels[target_idx] = pixels[target_idx].saturating_add(40);
                pixels[target_idx + 1] = pixels[target_idx + 1].saturating_add(40);
                pixels[target_idx + 2] = pixels[target_idx + 2].saturating_add(40);
            }
        }
    }
}

// ──────────────────────────────────────────────────────────────
//  Tool button
// ──────────────────────────────────────────────────────────────

// ──────────────────────────────────────────────────────────────
//  Tool Icons & UI Helpers
// ──────────────────────────────────────────────────────────────

pub fn draw_tool_icon(ui: &mut egui::Ui, tool: Tool, _size: f32, is_selected: bool) {
    let painter = ui.painter();
    let rect = ui.available_rect_before_wrap();
    let center = rect.center();
    
    let icon_color = if is_selected {
        egui::Color32::from_rgb(80, 180, 255)
    } else {
        egui::Color32::from_rgb(200, 200, 200)
    };
    
    let stroke = egui::Stroke::new(1.5, icon_color);
    
    match tool {
        Tool::Move => {
            painter.line_segment([center - egui::vec2(0.0, 7.0), center + egui::vec2(0.0, 7.0)], stroke);
            painter.line_segment([center - egui::vec2(7.0, 0.0), center + egui::vec2(7.0, 0.0)], stroke);
            painter.line_segment([center - egui::vec2(0.0, 7.0), center + egui::vec2(-3.0, -4.0)], stroke);
            painter.line_segment([center - egui::vec2(0.0, 7.0), center + egui::vec2(3.0, -4.0)], stroke);
            painter.line_segment([center + egui::vec2(0.0, 7.0), center + egui::vec2(-3.0, 4.0)], stroke);
            painter.line_segment([center + egui::vec2(0.0, 7.0), center + egui::vec2(3.0, 4.0)], stroke);
            painter.line_segment([center - egui::vec2(7.0, 0.0), center + egui::vec2(-4.0, -3.0)], stroke);
            painter.line_segment([center - egui::vec2(7.0, 0.0), center + egui::vec2(-4.0, 3.0)], stroke);
            painter.line_segment([center + egui::vec2(7.0, 0.0), center + egui::vec2(4.0, -3.0)], stroke);
            painter.line_segment([center + egui::vec2(7.0, 0.0), center + egui::vec2(4.0, 3.0)], stroke);
        }
        Tool::Brush => {
            // Draw a more "paintbrush" like icon
            let p1 = center + egui::vec2(-6.0, 6.0);
            let p2 = center + egui::vec2(2.0, -2.0);
            painter.line_segment([p1, p2], stroke);
            // Brush head
            let head_center = p2 + egui::vec2(2.0, -2.0);
            painter.circle_stroke(head_center, 4.0, stroke);
            painter.line_segment([head_center + egui::vec2(-2.0, 2.0), head_center + egui::vec2(2.0, -2.0)], stroke);
        }
        Tool::Eraser => {
            let p1 = center + egui::vec2(-6.0, 2.0);
            let p2 = center + egui::vec2(0.0, -4.0);
            let p3 = center + egui::vec2(6.0, 2.0);
            let p4 = center + egui::vec2(0.0, 8.0);
            painter.add(egui::Shape::line(vec![p1, p2, p3, p4, p1], stroke));
            painter.line_segment([center + egui::vec2(-3.0, -1.0), center + egui::vec2(3.0, 5.0)], stroke);
        }
        Tool::PaintBucket => {
            // Photoshop-like Paint Bucket icon: a tilted bucket pouring a drop
            let p1 = center + egui::vec2(-4.0, -2.0);
            let p2 = center + egui::vec2(4.0, -6.0);
            let p3 = center + egui::vec2(7.0, 0.0);
            let p4 = center + egui::vec2(-1.0, 4.0);
            painter.add(egui::Shape::line(vec![p1, p2, p3, p4, p1], stroke));
            
            // Handle
            painter.line_segment([center + egui::vec2(-4.0, -2.0), center + egui::vec2(-6.0, -7.0)], stroke);
            painter.line_segment([center + egui::vec2(-6.0, -7.0), center + egui::vec2(2.0, -11.0)], stroke);
            painter.line_segment([center + egui::vec2(2.0, -11.0), center + egui::vec2(4.0, -6.0)], stroke);
            
            // Drop
            painter.circle_filled(center + egui::vec2(6.0, 6.0), 1.5, icon_color);
        }
        Tool::Snip => {
            painter.line_segment([center - egui::vec2(6.0, 6.0), center - egui::vec2(6.0, -8.0)], stroke);
            painter.line_segment([center - egui::vec2(8.0, -6.0), center + egui::vec2(6.0, -6.0)], stroke);
            painter.line_segment([center + egui::vec2(6.0, -6.0), center + egui::vec2(6.0, 8.0)], stroke);
            painter.line_segment([center - egui::vec2(6.0, 6.0), center + egui::vec2(8.0, 6.0)], stroke);
        }
        Tool::Text => {
            painter.line_segment([center - egui::vec2(6.0, -6.0), center + egui::vec2(6.0, -6.0)], stroke);
            painter.line_segment([center - egui::vec2(6.0, -6.0), center - egui::vec2(6.0, -3.0)], stroke);
            painter.line_segment([center + egui::vec2(6.0, -6.0), center + egui::vec2(6.0, -3.0)], stroke);
            painter.line_segment([center, center - egui::vec2(0.0, 6.0)], stroke);
            painter.line_segment([center, center + egui::vec2(0.0, 6.0)], stroke);
            painter.line_segment([center - egui::vec2(3.0, 6.0), center + egui::vec2(3.0, 6.0)], stroke);
        }
        Tool::Shape => {
            painter.rect_stroke(egui::Rect::from_center_size(center, egui::vec2(14.0, 10.0)), 0.0, stroke, egui::StrokeKind::Middle);
        }
        Tool::Cut => {
            // Draw a dashed rectangle marquee icon
            // Top side
            painter.line_segment([center + egui::vec2(-7.0, -5.0), center + egui::vec2(-3.0, -5.0)], stroke);
            painter.line_segment([center + egui::vec2(-1.0, -5.0), center + egui::vec2(3.0, -5.0)], stroke);
            painter.line_segment([center + egui::vec2(5.0, -5.0), center + egui::vec2(7.0, -5.0)], stroke);
            // Bottom side
            painter.line_segment([center + egui::vec2(-7.0, 5.0), center + egui::vec2(-3.0, 5.0)], stroke);
            painter.line_segment([center + egui::vec2(-1.0, 5.0), center + egui::vec2(3.0, 5.0)], stroke);
            painter.line_segment([center + egui::vec2(5.0, 5.0), center + egui::vec2(7.0, 5.0)], stroke);
            // Left side
            painter.line_segment([center + egui::vec2(-7.0, -5.0), center + egui::vec2(-7.0, -3.0)], stroke);
            painter.line_segment([center + egui::vec2(-7.0, -1.0), center + egui::vec2(-7.0, 1.0)], stroke);
            painter.line_segment([center + egui::vec2(-7.0, 3.0), center + egui::vec2(-7.0, 5.0)], stroke);
            // Right side
            painter.line_segment([center + egui::vec2(7.0, -5.0), center + egui::vec2(7.0, -3.0)], stroke);
            painter.line_segment([center + egui::vec2(7.0, -1.0), center + egui::vec2(7.0, 1.0)], stroke);
            painter.line_segment([center + egui::vec2(7.0, 3.0), center + egui::vec2(7.0, 5.0)], stroke);
        }
        Tool::Mirror => {
            painter.rect_stroke(egui::Rect::from_center_size(center, egui::vec2(14.0, 14.0)), 0.0, stroke, egui::StrokeKind::Middle);
            painter.line_segment([center - egui::vec2(7.0, 7.0), center + egui::vec2(7.0, 7.0)], stroke);
        }
        Tool::Blur => {
            painter.circle_stroke(center, 7.0, stroke);
            painter.line_segment([center - egui::vec2(5.0, 0.0), center + egui::vec2(5.0, 0.0)], stroke);
            painter.line_segment([center - egui::vec2(0.0, 5.0), center + egui::vec2(0.0, 5.0)], stroke);
        }
        Tool::Embed => {
            // Draw link/chain icon
            painter.circle_stroke(center - egui::vec2(3.0, 3.0), 3.0, stroke);
            painter.circle_stroke(center + egui::vec2(3.0, 3.0), 3.0, stroke);
            painter.line_segment([center - egui::vec2(2.0, 2.0), center + egui::vec2(2.0, 2.0)], stroke);
        }
    }
}

pub fn draw_pick_color_icon(ui: &mut egui::Ui, rect: egui::Rect, color: egui::Color32) {
    let painter = ui.painter();
    let center = rect.center();
    let stroke = egui::Stroke::new(1.0, color);
    
    painter.line_segment([center + egui::vec2(-5.0, 5.0), center + egui::vec2(-1.0, 1.0)], stroke);
    let r = egui::Rect::from_center_size(center + egui::vec2(2.0, -2.0), egui::vec2(4.0, 4.0));
    painter.rect_stroke(r, 1.0, stroke, egui::StrokeKind::Middle);
    painter.line_segment([center + egui::vec2(3.0, -3.0), center + egui::vec2(5.0, -5.0)], stroke);
}

pub fn tool_btn_custom(ui: &mut egui::Ui, tool: Tool, is_selected: bool) -> egui::Response {
    let size = egui::vec2(28.0, 28.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let bg = if is_selected { 
        egui::Color32::from_rgb(60, 120, 200) 
    } else if response.hovered() { 
        egui::Color32::from_rgb(70, 70, 80) 
    } else { 
        egui::Color32::from_rgb(60, 60, 60) 
    };
    ui.painter().rect_filled(rect, 4.0, bg);
    ui.painter().rect_stroke(rect, 4.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(40, 40, 40)), egui::StrokeKind::Middle);
    
    let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink(4.0)));
    draw_tool_icon(&mut child_ui, tool, 16.0, is_selected);
    
    response.on_hover_text(format!("{} ({})", tool.name(), tool.shortcut()))
}

pub fn render_photoshop_panel(
    ctx: &egui::Context,
    active_tool: &mut Tool,
    settings: &mut Settings,
    show_settings_panel: &mut bool,
    show_layers_panel: &mut bool,
    show_exit_dialog: &mut bool,
    project: &mut crate::project::Project,
    embed_url: &mut String,
    embed_trigger: &mut bool,
    show_history_panel: &mut bool,
    request_history_push: &mut Option<String>,
) {
    let main_tools = vec![
        Tool::Move, Tool::Brush, Tool::Eraser, Tool::PaintBucket, Tool::Text, Tool::Shape, Tool::Snip, Tool::Cut, Tool::Blur, Tool::Embed,
    ];
    
    let hide_icon = if settings.hide_all { "👁" } else { "👓" };
    let is_vertical = settings.is_vertical;

    let frame = photoshop_frame(settings);
    let mut win = egui::Window::new("photoshop_panel")
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .movable(true)
        .default_pos(settings.toolbar_pos)
        .pivot(egui::Align2::LEFT_TOP)
        .frame(frame);
    
    if is_vertical { win = win.min_width(160.0); }
    
    let win_resp = win.show(ctx, |ui| {
        if is_vertical {
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(32.0);
                    if ui.add(egui::Button::new(hide_icon).min_size(egui::vec2(28.0, 24.0))).on_hover_text("Hide UI").clicked() { settings.hide_all = !settings.hide_all; }
                    ui.separator();
                    for tool in &main_tools {
                        let is_selected = *active_tool == *tool;
                        if tool_btn_custom(ui, *tool, is_selected).clicked() { *active_tool = *tool; }
                    }
                    ui.separator();
                    if ui.add(egui::Button::new("📁").min_size(egui::vec2(28.0, 24.0))).on_hover_text("Layers").clicked() { *show_layers_panel = !*show_layers_panel; }
                    if ui.add(egui::Button::new("🕓").min_size(egui::vec2(28.0, 24.0))).on_hover_text("History").clicked() { *show_history_panel = !*show_history_panel; }
                    if ui.add(egui::Button::new("⚙").min_size(egui::vec2(28.0, 24.0))).on_hover_text("Settings").clicked() { *show_settings_panel = !*show_settings_panel; }
                    if ui.add(egui::Button::new(egui::RichText::new("✖").color(egui::Color32::from_rgb(180, 50, 50))).min_size(egui::vec2(28.0, 24.0))).on_hover_text("Exit").clicked() { *show_exit_dialog = true; }
                });
                ui.add(egui::Separator::default().vertical());
                ui.vertical(|ui| {
                    ui.set_width(120.0);
                    render_tool_options(ui, active_tool, settings, project, true, embed_url, embed_trigger, request_history_push);
                });
            });
        } else {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new(hide_icon).min_size(egui::vec2(28.0, 24.0))).on_hover_text("Hide UI").clicked() { settings.hide_all = !settings.hide_all; }
                    ui.separator();
                    for tool in &main_tools {
                        let is_selected = *active_tool == *tool;
                        if tool_btn_custom(ui, *tool, is_selected).clicked() { *active_tool = *tool; }
                    }
                    ui.separator();
                    if ui.add(egui::Button::new("📁").min_size(egui::vec2(28.0, 24.0))).on_hover_text("Layers").clicked() { *show_layers_panel = !*show_layers_panel; }
                    if ui.add(egui::Button::new("🕓").min_size(egui::vec2(28.0, 24.0))).on_hover_text("History").clicked() { *show_history_panel = !*show_history_panel; }
                    if ui.add(egui::Button::new("⚙").min_size(egui::vec2(28.0, 24.0))).on_hover_text("Settings").clicked() { *show_settings_panel = !*show_settings_panel; }
                    if ui.add(egui::Button::new(egui::RichText::new("✖").color(egui::Color32::from_rgb(180, 50, 50))).min_size(egui::vec2(28.0, 24.0))).on_hover_text("Exit").clicked() { *show_exit_dialog = true; }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.set_height(24.0);
                    render_tool_options(ui, active_tool, settings, project, false, embed_url, embed_trigger, request_history_push);
                });
            });
        }
    });

    if let Some(resp) = win_resp {
        if resp.response.dragged() {
            let layer_id = resp.response.layer_id;
            if let Some(rect) = ctx.memory(|m| m.area_rect(layer_id.id)) {
                settings.toolbar_pos = rect.min;
            }
        }
    }
}

pub fn render_tool_options(ui: &mut egui::Ui, active_tool: &mut Tool, settings: &mut Settings, project: &mut crate::project::Project, _is_vertical: bool, embed_url: &mut String, embed_trigger: &mut bool, request_history_push: &mut Option<String>) {
    ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
    
    if !matches!(active_tool, Tool::Move | Tool::Mirror | Tool::Embed) {
        ui.horizontal(|ui| {
            let mut fg = color32(&settings.pen_color);
            if ui.color_edit_button_srgba(&mut fg).on_hover_text("Pen Color").changed() { settings.pen_color = [fg.r(), fg.g(), fg.b(), fg.a()]; }

            let (rect, resp) = ui.allocate_at_least(egui::vec2(24.0, 24.0), egui::Sense::click());
            if resp.clicked() { settings.picking_stroke_color = true; }
            if resp.hovered() { ui.painter().rect_filled(rect, 4.0, egui::Color32::from_white_alpha(30)); }
            draw_pick_color_icon(ui, rect, egui::Color32::WHITE);
            resp.on_hover_text("Pick Color");
            
            if *active_tool == Tool::Shape {
                let mut bg = color32(&settings.background_color);
                if ui.color_edit_button_srgba(&mut bg).on_hover_text("Fill Color").changed() { settings.background_color = [bg.r(), bg.g(), bg.b(), bg.a()]; }
                
                let (rect2, resp2) = ui.allocate_at_least(egui::vec2(24.0, 24.0), egui::Sense::click());
                if resp2.clicked() { settings.picking_fill_color = true; }
                if resp2.hovered() { ui.painter().rect_filled(rect2, 4.0, egui::Color32::from_white_alpha(30)); }
                draw_pick_color_icon(ui, rect2, egui::Color32::LIGHT_GRAY);
                resp2.on_hover_text("Pick Fill");
            }
        });
        ui.add(egui::Separator::default().vertical());
    }

    match active_tool {
        Tool::Brush | Tool::Eraser => {
            if ui.add(egui::DragValue::new(&mut settings.pen_width).range(1.0..=100.0)).on_hover_text("Pen Width").changed() {
                settings.save();
            }
            ui.horizontal(|ui| {
                ui.selectable_value(&mut settings.brush_shape, BrushShape::Round, "○").on_hover_text("Round Brush");
                ui.selectable_value(&mut settings.brush_shape, BrushShape::Square, "□").on_hover_text("Square Brush");
            });
            if *active_tool == Tool::Brush {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut settings.brush_mode, BrushMode::Solid, "Solid").on_hover_text("Solid Mode");
                    ui.selectable_value(&mut settings.brush_mode, BrushMode::Highlighter, "High").on_hover_text("Highlighter (40% Opacity)");
                    ui.selectable_value(&mut settings.brush_mode, BrushMode::Spray, "Spray").on_hover_text("Spray Mode");
                    ui.selectable_value(&mut settings.brush_mode, BrushMode::Calligraphy, "Calli").on_hover_text("Calligraphy Mode");
                    ui.selectable_value(&mut settings.brush_mode, BrushMode::Real, "Real").on_hover_text("Real Brush Mode");
                });
                if settings.brush_mode == BrushMode::Spray {
                    ui.horizontal(|ui| {
                        ui.label("Density");
                        if ui.add(egui::Slider::new(&mut settings.spray_density, 5..=100).show_value(true)).on_hover_text("Spray dot count per point").changed() {
                            settings.save();
                        }
                    });
                }
                if settings.brush_mode == BrushMode::Highlighter {
                    ui.horizontal(|ui| {
                        ui.label("Opacity");
                        if ui.add(egui::Slider::new(&mut settings.highlight_opacity, 0.1..=1.0).show_value(true)).on_hover_text("Highlighter opacity").changed() {
                            settings.save();
                        }
                    });
                }
                ui.horizontal(|ui| {
                    ui.toggle_value(&mut settings.brush_arrow, " > ").on_hover_text("Arrow at the end Toggle");
                });
            }
            if *active_tool == Tool::Eraser {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut settings.eraser_mode, EraserMode::Stroke, "Stroke").on_hover_text("Erase entire stroke");
                    ui.selectable_value(&mut settings.eraser_mode, EraserMode::Pixel, "Pixel").on_hover_text("Erase pixels (WIP/Mask)");
                });
            }
        }
        Tool::Text => {
            ui.add(egui::DragValue::new(&mut settings.font_size).range(10.0..=200.0)).on_hover_text("Font Size");
            ui.horizontal_wrapped(|ui| {
                ui.toggle_value(&mut settings.text_wave_warp, "〜").on_hover_text("Wave Warp");
            });
            ui.add(egui::Separator::default().vertical());
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut settings.font_search_query).hint_text("Search fonts...").desired_width(80.0));
                egui::ComboBox::from_id_salt("font_family")
                    .selected_text(format!("{:?}", settings.text_font))
                    .show_ui(ui, |ui| {
                        let fonts = [TextFont::Sans, TextFont::Serif, TextFont::Mono, TextFont::Handwriting, TextFont::Heading, TextFont::Custom];
                        for f in fonts {
                            let name = format!("{:?}", f);
                            if settings.font_search_query.is_empty() || name.to_lowercase().contains(&settings.font_search_query.to_lowercase()) {
                                ui.selectable_value(&mut settings.text_font, f, name);
                            }
                        }
                    });
            });
        }
        Tool::PaintBucket => {
            ui.add(egui::DragValue::new(&mut settings.magic_wand_threshold).range(0.0..=100.0).prefix("Tolerance: "));
        }
        Tool::Shape => {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut settings.shape_type, ShapeType::Rect, "Rect");
                ui.selectable_value(&mut settings.shape_type, ShapeType::Circle, "Circ");
                ui.selectable_value(&mut settings.shape_type, ShapeType::Star, "Star");
                ui.selectable_value(&mut settings.shape_type, ShapeType::Heart, "Heart");
                ui.selectable_value(&mut settings.shape_type, ShapeType::Arrow, "Arrow");
                ui.selectable_value(&mut settings.shape_type, ShapeType::Poly, "Poly");
            });
        }
        Tool::Snip => {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut settings.snip_mode, SnipMode::Rect, "Rect");
                    ui.selectable_value(&mut settings.snip_mode, SnipMode::Circle, "Circ");
                    ui.selectable_value(&mut settings.snip_mode, SnipMode::Lasso, "Lasso");
                    ui.selectable_value(&mut settings.snip_mode, SnipMode::Polygon, "Poly");
                    ui.selectable_value(&mut settings.snip_mode, SnipMode::Star, "Star");
                    ui.selectable_value(&mut settings.snip_mode, SnipMode::Heart, "Heart");
                    ui.add(egui::Separator::default().vertical());
                    let static_sel = !settings.snip_live;
                    let live_sel = settings.snip_live;
                    let static_color = if static_sel { egui::Color32::from_rgb(100, 200, 255) } else { egui::Color32::from_gray(140) };
                    let live_color = if live_sel { egui::Color32::from_rgb(255, 150, 50) } else { egui::Color32::from_gray(140) };
                    if ui.add(egui::Button::new(egui::RichText::new("⏸ Static").color(static_color).strong()).selected(static_sel)).clicked() { settings.snip_live = false; }
                    if ui.add(egui::Button::new(egui::RichText::new("⏺ Live").color(live_color).strong()).selected(live_sel)).clicked() { settings.snip_live = true; }
                });
                ui.horizontal(|ui| {
                    ui.add(egui::Slider::new(&mut settings.blur_strength, 0.0..=300.0).prefix("Blur: "));
                    if settings.blur_strength > 0.1 {
                        ui.add(egui::Separator::default().vertical());
                        ui.selectable_value(&mut settings.blur_effect, BlurEffect::Gaussian, "Gaus");
                        ui.selectable_value(&mut settings.blur_effect, BlurEffect::Pixelate, "Pix");
                        ui.selectable_value(&mut settings.blur_effect, BlurEffect::Glitch, "VHS");
                    }
                });
            });
        }
        Tool::Cut => {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut settings.cut_mode, CutMode::Rect, "Rect");
                ui.selectable_value(&mut settings.cut_mode, CutMode::Circle, "Circ");
                ui.selectable_value(&mut settings.cut_mode, CutMode::Star, "Star");
                ui.selectable_value(&mut settings.cut_mode, CutMode::Heart, "Heart");
                ui.selectable_value(&mut settings.cut_mode, CutMode::Lasso, "Lasso");
                ui.selectable_value(&mut settings.cut_mode, CutMode::Polygon, "Poly");
                ui.selectable_value(&mut settings.cut_mode, CutMode::MagicWand, "Wand");
                ui.add(egui::Separator::default().vertical());
                ui.checkbox(&mut settings.inverted_cut, "Invert");
                if settings.cut_mode == CutMode::MagicWand {
                    ui.add(egui::DragValue::new(&mut settings.magic_wand_threshold).range(0.0..=100.0).prefix("Thresh: "));
                }
                if project.marquee_selection.is_some() {
                    ui.add(egui::Separator::default().vertical());
                    if ui.add(egui::Button::new(egui::RichText::new("✂ Snip").color(egui::Color32::from_rgb(100, 220, 100)).strong())).clicked() {
                        project.request_copy = true;
                    }
                }
            });
        }
        Tool::Blur => {
            ui.add(egui::DragValue::new(&mut settings.blur_strength).range(1.0..=300.0).prefix("Blur: "));
            ui.horizontal(|ui| {
                ui.selectable_value(&mut settings.blur_effect, BlurEffect::Gaussian, "Gaus");
                ui.selectable_value(&mut settings.blur_effect, BlurEffect::Pixelate, "Pix");
                ui.selectable_value(&mut settings.blur_effect, BlurEffect::Glitch, "VHS");
            });
            ui.horizontal(|ui| {
                ui.selectable_value(&mut settings.shape_type, ShapeType::Rect, "Rect");
                ui.selectable_value(&mut settings.shape_type, ShapeType::Circle, "Circ");
                ui.selectable_value(&mut settings.shape_type, ShapeType::Star, "Star");
                ui.selectable_value(&mut settings.shape_type, ShapeType::Heart, "Heart");
            });
        }
        Tool::Move => {
            ui.horizontal_wrapped(|ui| {
                if let Some(sel) = project.selected_object {
                    if ui.button(egui::RichText::new("✖").color(egui::Color32::RED)).on_hover_text("Delete Selected (X)").clicked() {
                        let layer = &mut project.layers[sel.layer_idx];
                        match sel.object_type {
                            ObjectType::Image => { if sel.object_idx < layer.placed_images.len() { layer.placed_images.remove(sel.object_idx); } }
                            ObjectType::Stroke => { if sel.object_idx < layer.strokes.len() { layer.strokes.remove(sel.object_idx); } }
                            ObjectType::Text => { if sel.object_idx < layer.text_annotations.len() { layer.text_annotations.remove(sel.object_idx); } }
                        }
                        project.selected_object = None;
                    }
                    ui.separator();
                    
                    if ui.button("⟳").on_hover_text("Rotate 90").clicked() {
                        let layer = &mut project.layers[sel.layer_idx];
                        match sel.object_type {
                            ObjectType::Image => { layer.placed_images[sel.object_idx].rotation += std::f32::consts::PI / 2.0; }
                            ObjectType::Stroke => { layer.strokes[sel.object_idx].rotation += std::f32::consts::PI / 2.0; }
                            ObjectType::Text => { layer.text_annotations[sel.object_idx].rotation += std::f32::consts::PI / 2.0; }
                        }
                        *request_history_push = Some("Rotate".into());
                    }
                    if ui.button("↔").on_hover_text("Flip H").clicked() {
                        let layer = &mut project.layers[sel.layer_idx];
                        match sel.object_type {
                            ObjectType::Image => { layer.placed_images[sel.object_idx].flipped_h = !layer.placed_images[sel.object_idx].flipped_h; }
                            ObjectType::Stroke => { layer.strokes[sel.object_idx].flipped_h = !layer.strokes[sel.object_idx].flipped_h; }
                            ObjectType::Text => { layer.text_annotations[sel.object_idx].flipped_h = !layer.text_annotations[sel.object_idx].flipped_h; }
                        }
                        *request_history_push = Some("Flip H".into());
                    }
                    if ui.button("↕").on_hover_text("Flip V").clicked() {
                        let layer = &mut project.layers[sel.layer_idx];
                        match sel.object_type {
                            ObjectType::Image => { layer.placed_images[sel.object_idx].flipped_v = !layer.placed_images[sel.object_idx].flipped_v; }
                            ObjectType::Stroke => { layer.strokes[sel.object_idx].flipped_v = !layer.strokes[sel.object_idx].flipped_v; }
                            ObjectType::Text => { layer.text_annotations[sel.object_idx].flipped_v = !layer.text_annotations[sel.object_idx].flipped_v; }
                        }
                        *request_history_push = Some("Flip V".into());
                    }
                    ui.separator();

                    // Opacity
                    ui.label("Op:");
                    let mut op = match sel.object_type {
                        ObjectType::Image => project.layers[sel.layer_idx].placed_images[sel.object_idx].opacity,
                        ObjectType::Stroke => project.layers[sel.layer_idx].strokes[sel.object_idx].opacity,
                        ObjectType::Text => project.layers[sel.layer_idx].text_annotations[sel.object_idx].opacity,
                    } * 100.0;
                    if ui.add(egui::DragValue::new(&mut op).range(0.0..=100.0).suffix("%")).changed() {
                        let final_op = op / 100.0;
                        match sel.object_type {
                            ObjectType::Image => project.layers[sel.layer_idx].placed_images[sel.object_idx].opacity = final_op,
                            ObjectType::Stroke => project.layers[sel.layer_idx].strokes[sel.object_idx].opacity = final_op,
                            ObjectType::Text => project.layers[sel.layer_idx].text_annotations[sel.object_idx].opacity = final_op,
                        }
                    }
                    ui.separator();

                    // Blur strength
                    // ui.label("Blur:");
                    let mut bl = match sel.object_type {
                        ObjectType::Image => project.layers[sel.layer_idx].placed_images[sel.object_idx].blur,
                        ObjectType::Stroke => project.layers[sel.layer_idx].strokes[sel.object_idx].blur,
                        ObjectType::Text => project.layers[sel.layer_idx].text_annotations[sel.object_idx].blur,
                    };
                    let mut bl_slider = bl.max(0.0);
                    if ui.add(egui::DragValue::new(&mut bl_slider).range(0.0..=300.0).prefix("Blur: ")).changed() {
                        match sel.object_type {
                            ObjectType::Image => project.layers[sel.layer_idx].placed_images[sel.object_idx].blur = bl_slider,
                            ObjectType::Stroke => project.layers[sel.layer_idx].strokes[sel.object_idx].blur = bl_slider,
                            ObjectType::Text => project.layers[sel.layer_idx].text_annotations[sel.object_idx].blur = bl_slider,
                        }
                    }
                    
                    // Blur styles when object is selected and has blur > 0.1:
                    let blur_val = match sel.object_type {
                        ObjectType::Image => project.layers[sel.layer_idx].placed_images[sel.object_idx].blur,
                        ObjectType::Stroke => project.layers[sel.layer_idx].strokes[sel.object_idx].blur,
                        ObjectType::Text => project.layers[sel.layer_idx].text_annotations[sel.object_idx].blur,
                    };
                    if blur_val > 0.1 {
                        ui.separator();
                        match sel.object_type {
                            ObjectType::Image => {
                                let img = &mut project.layers[sel.layer_idx].placed_images[sel.object_idx];
                                ui.selectable_value(&mut img.blur_effect, BlurEffect::Gaussian, "Gaus");
                                ui.selectable_value(&mut img.blur_effect, BlurEffect::Pixelate, "Pix");
                                ui.selectable_value(&mut img.blur_effect, BlurEffect::Glitch, "VHS");
                            }
                            ObjectType::Stroke => {
                                let s = &mut project.layers[sel.layer_idx].strokes[sel.object_idx];
                                ui.selectable_value(&mut s.blur_effect, BlurEffect::Gaussian, "Gaus");
                                ui.selectable_value(&mut s.blur_effect, BlurEffect::Pixelate, "Pix");
                                ui.selectable_value(&mut s.blur_effect, BlurEffect::Glitch, "VHS");
                            }
                            ObjectType::Text => {
                                let t = &mut project.layers[sel.layer_idx].text_annotations[sel.object_idx];
                                ui.selectable_value(&mut t.blur_effect, BlurEffect::Gaussian, "Gaus");
                                ui.selectable_value(&mut t.blur_effect, BlurEffect::Pixelate, "Pix");
                                ui.selectable_value(&mut t.blur_effect, BlurEffect::Glitch, "VHS");
                            }
                        }
                    }
                    ui.separator();
                    
                    if let ObjectType::Image = sel.object_type {
                        let img = &mut project.layers[sel.layer_idx].placed_images[sel.object_idx];
                        ui.horizontal(|ui| {
                            if ui.selectable_label(!img.is_live, "Static").clicked() { img.is_live = false; }
                            if ui.selectable_label(img.is_live, "Live").clicked() {
                                img.is_live = true;
                                if img.source_rect.is_none() {
                                    img.source_rect = Some([img.position.x, img.position.y, img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0], img.display_size.unwrap_or([img.size[1] as f32, img.size[1] as f32])[1]]);
                                    img.show_source_rect = true;
                                }
                            }
                            if img.is_live {
                                ui.checkbox(&mut img.show_source_rect, "Show Source");
                            }
                        });
                        ui.separator();
                    }
                    if ui.button("⎌").on_hover_text("Reset Transforms").clicked() {
                        let layer = &mut project.layers[sel.layer_idx];
                        match sel.object_type {
                            ObjectType::Image => { let img = &mut layer.placed_images[sel.object_idx]; img.rotation = 0.0; img.skew = egui::Vec2::ZERO; img.perspective = [egui::Vec2::ZERO; 4]; }
                            ObjectType::Stroke => { let s = &mut layer.strokes[sel.object_idx]; s.rotation = 0.0; s.skew = egui::Vec2::ZERO; s.perspective = [egui::Vec2::ZERO; 4]; }
                            ObjectType::Text => { let t = &mut layer.text_annotations[sel.object_idx]; t.rotation = 0.0; t.skew = egui::Vec2::ZERO; t.perspective = [egui::Vec2::ZERO; 4]; }
                        }
                        *request_history_push = Some("Reset Transforms".into());
                    }
                } else {
                    ui.label("Active Layer:");
                    let layer = &mut project.layers[project.active_layer];
                    let mut op = layer.opacity * 100.0;
                    if ui.add(egui::DragValue::new(&mut op).range(0.0..=100.0).prefix("Op: ").suffix("%")).changed() {
                        layer.opacity = op / 100.0;
                    }
                    if ui.button("⎌").on_hover_text("Reset Layer Transforms").clicked() {
                        crate::utils::translate_layer(layer, -crate::utils::layer_bounds(layer).map(|b| b.min.to_vec2()).unwrap_or(egui::Vec2::ZERO));
                    }
                }
            });
        }
        Tool::Embed => {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    if ui.button("?? YouTube").clicked() {
                        *embed_url = "https://www.youtube.com/watch?v=dQw4w9WgXcQ".to_string();
                        *embed_trigger = true;
                    }
                    if ui.button("?? Browser").clicked() {
                        *embed_url = "https://www.google.com".to_string();
                        *embed_trigger = true;
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Program:");
                    let mut selected_hwnd = None;
                    egui::ComboBox::from_id_salt("running_programs")
                        .selected_text("Select Program...")
                        .show_ui(ui, |ui| {
                            let windows = crate::winapi_utils::list_visible_windows();
                            for (hwnd, title) in windows {
                                if ui.selectable_label(false, &title).clicked() {
                                    selected_hwnd = Some(hwnd);
                                }
                            }
                        });
                    if let Some(hwnd) = selected_hwnd {
                        *embed_url = format!("window://{}", hwnd);
                        *embed_trigger = true;
                    }
                });

                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(embed_url).hint_text("URL...").desired_width(120.0));
                    if ui.button("Load").clicked() {
                        *embed_trigger = true;
                    }
                    if ui.button("+").on_hover_text("Save URL shortcut").clicked() {
                        if !embed_url.is_empty() {
                            let label = embed_url.chars().take(15).collect::<String>();
                            settings.saved_embed_urls.push((label, embed_url.clone()));
                            settings.save();
                        }
                    }
                });

                if !settings.saved_embed_urls.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        let mut to_remove = None;
                        for (idx, (label, url)) in settings.saved_embed_urls.iter().enumerate() {
                            ui.horizontal(|ui| {
                                if ui.button(label).clicked() {
                                    *embed_url = url.clone();
                                    *embed_trigger = true;
                                }
                                if ui.small_button("x").clicked() {
                                    to_remove = Some(idx);
                                }
                            });
                        }
                        if let Some(idx) = to_remove {
                            settings.saved_embed_urls.remove(idx);
                            settings.save();
                        }
                    });
                }
            });
        }
        Tool::Mirror => {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut settings.mirror_mode, MirrorMode::Rect, "Rect");
                ui.selectable_value(&mut settings.mirror_mode, MirrorMode::Circle, "Circ");
                ui.selectable_value(&mut settings.mirror_mode, MirrorMode::Lasso, "Lasso");
                ui.selectable_value(&mut settings.mirror_mode, MirrorMode::Window, "Win");
            });
        }
    }
}


pub fn render_toolbar(
    ctx: &egui::Context,
    active_tool: &mut Tool,
    settings: &mut Settings,
    show_settings_panel: &mut bool,
    show_layers_panel: &mut bool,
    show_exit_dialog: &mut bool,
    project: &mut crate::project::Project,
    embed_url: &mut String,
    embed_trigger: &mut bool,
    show_history_panel: &mut bool,
    request_history_push: &mut Option<String>,
) {
    render_photoshop_panel(ctx, active_tool, settings, show_settings_panel, show_layers_panel, show_exit_dialog, project, embed_url, embed_trigger, show_history_panel, request_history_push);
}

