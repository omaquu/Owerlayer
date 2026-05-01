use eframe::egui;
use serde::{Deserialize, Serialize};
use rayon::prelude::*;

use crate::hotkey::{HotkeyBinding, detect_pressed_key};

// ──────────────────────────────────────────────────────────────
//  Data types
// ──────────────────────────────────────────────────────────────

pub struct MouseState {
    pub pos: egui::Pos2,
    #[allow(dead_code)]
    pub delta: egui::Vec2,
    pub left_down: bool,
    #[allow(dead_code)]
    pub left_just_pressed: bool,
    #[allow(dead_code)]
    pub left_just_released: bool,
}

impl MouseState {
    pub fn poll(prev_down: bool, prev_pos: egui::Pos2) -> Self {
        let (x, y, down) = crate::winapi_utils::poll_mouse();
        let pos = egui::pos2(x, y);
        Self {
            pos,
            delta: pos - prev_pos,
            left_down: down,
            left_just_pressed: down && !prev_down,
            left_just_released: !down && prev_down,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum ShapeType { Rect, Circle, Star, Heart }

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Tool {
    Move,
    Brush,
    Eraser,
    Text,
    Shape,
    Snip,
    Mirror,
    Cut,
    Pipette,
    Blur,
}

impl Tool {
    pub fn name(&self) -> &str {
        match self {
            Tool::Move => "Move Tool",
            Tool::Brush => "Brush Tool",
            Tool::Eraser => "Eraser Tool",
            Tool::Text => "Text Tool",
            Tool::Shape => "Shape Tool",
            Tool::Snip => "Snip Tool",
            Tool::Cut => "Cut Tool",
            Tool::Pipette => "Pipette Tool",
            Tool::Mirror => "Mirror Tool",
            Tool::Blur => "Blur Tool",
        }
    }

    pub fn shortcut(&self) -> &str {
        match self {
            Tool::Move => "V",
            Tool::Brush => "B",
            Tool::Eraser => "E",
            Tool::Text => "T",
            Tool::Shape => "U",
            Tool::Snip => "S",
            Tool::Cut => "C",
            Tool::Pipette => "I",
            Tool::Mirror => "M",
            Tool::Blur => "K",
        }
    }
}



#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BrushMode { Solid, Highlighter, Calligraphy, Spray }

#[derive(Clone, Serialize, Deserialize)]
pub struct Stroke {
    pub points: Vec<egui::Pos2>,
    pub color: [u8; 4],
    pub width: f32,
    pub kind: StrokeKind,
    pub brush_mode: BrushMode,
    #[serde(default)]
    pub background_color: Option<[u8; 4]>,
    #[serde(default)]
    pub shadow: bool,
    #[serde(default)]
    pub rotation: f32,
    #[serde(default)]
    pub flipped_h: bool,
    #[serde(default)]
    pub flipped_v: bool,
    #[serde(default)]
    pub perspective: [egui::Vec2; 4],
    #[serde(default)]
    pub brush_shape: BrushShape,
    #[serde(default)]
    pub skew: egui::Vec2,
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum StrokeKind { Freehand, Line, Arrow, Rect, Circle, Star, Heart, Blur }

#[derive(Clone, Serialize, Deserialize)]
pub struct TextAnnotation {
    pub position: egui::Pos2,
    pub text: String,
    pub color: [u8; 4],
    pub font_size: f32,
    pub monospace: bool,
    pub shadow: bool,
    pub outline: bool,
    #[serde(default)]
    pub stroke_width: f32,
    #[serde(default)]
    pub rotation: f32,
    #[serde(default)]
    pub flipped_h: bool,
    #[serde(default)]
    pub flipped_v: bool,
    #[serde(default)]
    pub perspective: [egui::Vec2; 4],
    #[serde(default)]
    pub font: TextFont,
    #[serde(default)]
    pub skew: egui::Vec2,
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum TextFont { Sans, Serif, Mono, Handwriting, Heading }

impl Default for TextFont { fn default() -> Self { Self::Sans } }

#[derive(Clone, Serialize, Deserialize)]
pub struct PlacedImage {
    pub id: usize,
    pub position: egui::Pos2,
    pub size: [usize; 2],
    #[serde(default)]
    pub display_size: Option<[f32; 2]>,
    #[serde(skip)]
    pub pixels: Vec<u8>, // RGBA pixels, we'll serialize to PNG in Phase 4
    #[serde(skip)]
    pub texture: Option<egui::TextureHandle>,
    #[serde(default)]
    pub shadow: bool,
    #[serde(default)]
    pub rotation: f32,
    #[serde(default)]
    pub flipped_h: bool,
    #[serde(default)]
    pub flipped_v: bool,
    #[serde(default)]
    pub skew: egui::Vec2,
    #[serde(default)]
    pub perspective: [egui::Vec2; 4],
    #[serde(default)]
    pub is_live: bool,
    #[serde(default)]
    pub source_rect: Option<[f32; 4]>,
    #[serde(default)]
    pub blur: f32,
    #[serde(skip)]
    pub mask: Option<Vec<u8>>,
    #[serde(skip)]
    pub last_capture: f64,
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SnipMode { Rect, Lasso, Polygon }

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BrushShape { Round, Square }

pub struct PendingText {
    pub position: egui::Pos2,
    pub buffer: String,
}

// ──────────────────────────────────────────────────────────────
//  Settings
// ──────────────────────────────────────────────────────────────

fn default_menu_hue() -> f32 { 90.0 }
fn default_menu_opacity() -> f32 { 0.75 }

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum EraserMode { Stroke, Pixel }

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CutMode { Rect, Lasso, Polygon, MagicWand }

impl Default for CutMode { fn default() -> Self { Self::Rect } }

#[derive(Clone, Serialize, Deserialize)]
pub struct Settings {
    pub hotkey: HotkeyBinding,
    pub pen_color: [u8; 4],
    pub pen_width: f32,
    pub font_size: f32,
    pub keep_ui_visible: bool,
    pub toggle_mode: bool,
    pub accent_color: [u8; 4],
    #[serde(default = "default_menu_hue")]
    pub menu_hue: f32,
    #[serde(default = "default_menu_opacity")]
    pub menu_opacity: f32,
    #[serde(default)]
    pub brush_mode: BrushMode,
    #[serde(default)]
    pub eraser_mode: EraserMode,
    #[serde(default)]
    pub text_monospace: bool,
    #[serde(default)]
    pub text_shadow: bool,
    #[serde(default)]
    pub text_outline: bool,
    #[serde(default)]
    pub text_stroke_width: f32,
    #[serde(default)]
    pub cut_mode: CutMode,
    #[serde(default = "default_bg_color")]
    pub background_color: [u8; 4],
    #[serde(default)]
    pub shape_type: ShapeType,
    #[serde(default)]
    pub shape_fill: bool,
    #[serde(default)]
    pub stroke_width: f32,
    #[serde(default)]
    pub brush_shadow: bool,
    #[serde(default)]
    pub snip_shadow: bool,
    #[serde(default)]
    pub snip_mode: SnipMode,
    #[serde(default = "default_blur_strength")]
    pub blur_strength: f32,
    #[serde(default)]
    pub magic_wand_threshold: f32,
    #[serde(default)]
    pub ui_scale: f32,
    #[serde(default)]
    pub inverted_cut: bool,
    #[serde(default)]
    pub hide_edit_info: bool,
    #[serde(default)]
    pub background_image: Option<String>,
    #[serde(default)]
    pub brush_shape: BrushShape,
    #[serde(default)]
    pub hide_all: bool,
    #[serde(default)]
    pub auto_hide_seconds: f32,
    #[serde(default)]
    pub exclude_from_capture: bool,
    #[serde(default)]
    pub software_rendering: bool,
    #[serde(default = "default_toolbar_bg")]
    pub toolbar_bg_color: [u8; 4],
    #[serde(default)]
    pub text_font: TextFont,
}

fn default_toolbar_bg() -> [u8; 4] { [30, 30, 30, 220] }

fn default_blur_strength() -> f32 { 10.0 }

impl Default for SnipMode { fn default() -> Self { Self::Rect } }

impl Default for BrushShape { fn default() -> Self { Self::Round } }

// Default values for serialization
fn default_stroke_width() -> f32 { 3.0 }

fn default_bg_color() -> [u8; 4] { [0, 0, 0, 0] }

impl Default for BrushMode { fn default() -> Self { Self::Solid } }
impl Default for EraserMode { fn default() -> Self { Self::Stroke } }
impl Default for ShapeType { fn default() -> Self { Self::Rect } }

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: HotkeyBinding::default(),
            pen_color: [255, 255, 255, 255],
            pen_width: 3.0,
            font_size: 20.0,
            keep_ui_visible: false,
            toggle_mode: false,
            accent_color: [100, 140, 255, 255],
            menu_hue: 90.0,
            menu_opacity: 0.75,
            brush_mode: BrushMode::Solid,
            eraser_mode: EraserMode::Stroke,
            text_monospace: false,
            text_shadow: false,
            text_outline: false,
            text_stroke_width: 1.0,
            cut_mode: CutMode::Rect,
            background_color: [0, 0, 0, 0],
            stroke_width: default_stroke_width(),
            shape_type: ShapeType::Rect,
            shape_fill: false,
            brush_shadow: false,
            snip_shadow: false,
            snip_mode: SnipMode::Rect,
            blur_strength: default_blur_strength(),
            magic_wand_threshold: 10.0,
            ui_scale: 1.0,
            inverted_cut: false,
            hide_edit_info: false,
            background_image: None,
            brush_shape: BrushShape::Round,
            hide_all: false,
            auto_hide_seconds: 0.0,
            exclude_from_capture: false,
            software_rendering: false,
            toolbar_bg_color: default_toolbar_bg(),
            text_font: TextFont::Sans,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        if let Some(path) = Self::config_path() {
            if let Ok(data) = std::fs::read_to_string(&path) {
                if let Ok(s) = serde_json::from_str(&data) { return s; }
            }
        }
        Self::default()
    }
    pub fn save(&self) {
        if let Some(path) = Self::config_path() {
            if let Some(parent) = path.parent() { let _ = std::fs::create_dir_all(parent); }
            if let Ok(data) = serde_json::to_string_pretty(self) { let _ = std::fs::write(&path, data); }
        }
    }
    fn config_path() -> Option<std::path::PathBuf> {
        directories::ProjectDirs::from("com", "omaquu", "owerlayer")
            .map(|d| d.config_dir().join("settings.json"))
    }
}

// ──────────────────────────────────────────────────────────────
//  Color helpers
// ──────────────────────────────────────────────────────────────

#[allow(dead_code)]
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let h = ((h % 360.0) + 360.0) % 360.0;
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    (((r + m) * 255.0) as u8, ((g + m) * 255.0) as u8, ((b + m) * 255.0) as u8)
}

fn photoshop_frame(settings: &Settings) -> egui::Frame {
    egui::Frame::window(&egui::Style::default())
        .fill(color32(&settings.toolbar_bg_color))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(60)))
        .corner_radius(egui::CornerRadius::same(10))
}

fn color32(c: &[u8; 4]) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])
}

fn apply_box_blur(pixels: &mut [u8], width: usize, height: usize, radius: usize) {
    let radius = radius.min(15).min(width / 2).min(height / 2);
    if radius == 0 || width == 0 || height == 0 { return; }

    let pixels_per_row = width * 4;
    let mut intermediate = pixels.to_vec();
    
    // Horizontal
    intermediate.par_chunks_exact_mut(pixels_per_row).for_each(|row| {
        let row_copy = row.to_vec();
        for x in 0..width {
            let mut r = 0u32; let mut g = 0u32; let mut b = 0u32; let mut a = 0u32;
            let mut count = 0;
            for i in -(radius as i32)..=(radius as i32) {
                let nx = x as i32 + i;
                if nx >= 0 && nx < width as i32 {
                    let idx = nx as usize * 4;
                    r += row_copy[idx] as u32;
                    g += row_copy[idx + 1] as u32;
                    b += row_copy[idx + 2] as u32;
                    a += row_copy[idx + 3] as u32;
                    count += 1;
                }
            }
            let idx = x * 4;
            row[idx] = (r / count) as u8;
            row[idx + 1] = (g / count) as u8;
            row[idx + 2] = (b / count) as u8;
            row[idx + 3] = (a / count) as u8;
        }
    });

    // Vertical (intermediate -> pixels)
    let intermediate_shared = &intermediate;
    pixels.par_chunks_exact_mut(pixels_per_row).enumerate().for_each(|(y, row)| {
        for x in 0..width {
            let mut r = 0u32; let mut g = 0u32; let mut b = 0u32; let mut a = 0u32;
            let mut count = 0;
            for i in -(radius as i32)..=(radius as i32) {
                let ny = y as i32 + i;
                if ny >= 0 && ny < height as i32 {
                    let idx = (ny as usize * width + x) * 4;
                    r += intermediate_shared[idx] as u32;
                    g += intermediate_shared[idx + 1] as u32;
                    b += intermediate_shared[idx + 2] as u32;
                    a += intermediate_shared[idx + 3] as u32;
                    count += 1;
                }
            }
            let idx = x * 4;
            row[idx] = (r / count) as u8;
            row[idx + 1] = (g / count) as u8;
            row[idx + 2] = (b / count) as u8;
            row[idx + 3] = (a / count) as u8;
        }
    });
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
            painter.line_segment([center + egui::vec2(-7.0, 7.0), center + egui::vec2(-2.0, 2.0)], stroke);
            let rect = egui::Rect::from_center_size(center + egui::vec2(2.0, -2.0), egui::vec2(6.0, 6.0));
            painter.rect_stroke(rect, 2.0, stroke, egui::StrokeKind::Middle);
            painter.line_segment([center + egui::vec2(3.0, -3.0), center + egui::vec2(5.0, -5.0)], stroke);
        }
        Tool::Eraser => {
            let p1 = center + egui::vec2(-6.0, 2.0);
            let p2 = center + egui::vec2(0.0, -4.0);
            let p3 = center + egui::vec2(6.0, 2.0);
            let p4 = center + egui::vec2(0.0, 8.0);
            painter.add(egui::Shape::line(vec![p1, p2, p3, p4, p1], stroke));
            painter.line_segment([center + egui::vec2(-3.0, -1.0), center + egui::vec2(3.0, 5.0)], stroke);
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
            // Draw scissors icon
            painter.circle_stroke(center + egui::vec2(-4.0, 4.0), 2.5, stroke);
            painter.circle_stroke(center + egui::vec2(4.0, 4.0), 2.5, stroke);
            painter.line_segment([center + egui::vec2(-2.5, 2.0), center + egui::vec2(6.0, -6.0)], stroke);
            painter.line_segment([center + egui::vec2(2.5, 2.0), center + egui::vec2(-6.0, -6.0)], stroke);
        }
        Tool::Pipette => {
            painter.line_segment([center - egui::vec2(6.0, 6.0), center + egui::vec2(6.0, 6.0)], stroke);
            painter.circle_stroke(center - egui::vec2(4.0, 4.0), 3.0, stroke);
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
    }
}

fn tool_btn_custom(ui: &mut egui::Ui, tool: Tool, active: bool) -> egui::Response {
    let size = egui::vec2(28.0, 28.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let bg = if active { 
        egui::Color32::from_rgb(60, 120, 200) 
    } else if response.hovered() { 
        egui::Color32::from_rgb(70, 70, 80) 
    } else { 
        egui::Color32::from_rgb(60, 60, 60) 
    };
    ui.painter().rect_filled(rect, 4.0, bg);
    ui.painter().rect_stroke(rect, 4.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(40, 40, 40)), egui::StrokeKind::Middle);
    
    let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink(4.0)));
    draw_tool_icon(&mut child_ui, tool, 16.0, active);
    
    response.on_hover_text(format!("{} ({})", tool.name(), tool.shortcut()))
}

pub fn render_photoshop_panel(
    ctx: &egui::Context,
    active_tool: &mut Tool,
    settings: &mut Settings,
    show_settings_panel: &mut bool,
    show_layers_panel: &mut bool,
    show_exit_dialog: &mut bool,
    _project: &mut crate::project::Project,
) {
    let main_tools = vec![
        Tool::Move, Tool::Brush, Tool::Eraser, Tool::Text, Tool::Shape, Tool::Snip, Tool::Cut, Tool::Pipette, Tool::Mirror, Tool::Blur,
    ];
    
    let hide_icon = if settings.hide_all { "👁" } else { "👓" };

    let window_id = egui::Id::new("photoshop_panel");
    let window_rect = ctx.memory(|mem| mem.area_rect(window_id)).unwrap_or(egui::Rect::from_min_size(egui::pos2(20.0, 50.0), egui::vec2(40.0, 40.0)));
    let screen_width = ctx.screen_rect().width();
    let is_vertical = window_rect.left() < 150.0 || window_rect.right() > screen_width - 150.0;

    let frame = photoshop_frame(settings);

    let mut win = egui::Window::new("photoshop_panel")
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .movable(true)
        .default_pos(egui::pos2(20.0, 50.0))
        .frame(frame);
    if is_vertical { win = win.min_width(160.0); } // Wider for two columns
    
    win.show(ctx, |ui| {
            if is_vertical {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(30.0);
                        if ui.add(egui::Button::new(hide_icon)).on_hover_text("Hide/Show All Drawings").clicked() {
                            settings.hide_all = !settings.hide_all;
                        }
                        ui.separator();
                        for tool in main_tools {
                            let is_selected = *active_tool == tool;
                            if tool_btn_custom(ui, tool, is_selected).clicked() { *active_tool = tool; }
                        }
                        ui.separator();
                        if ui.button("⚙").on_hover_text("Settings").clicked() { *show_settings_panel = !*show_settings_panel; }
                        if ui.button("📁").on_hover_text("Layers").clicked() { *show_layers_panel = !*show_layers_panel; }
                        if ui.add(egui::Button::new("✖").fill(egui::Color32::from_rgb(180, 50, 50))).on_hover_text("Exit").clicked() { *show_exit_dialog = true; }
                    });

                    ui.add(egui::Separator::default().vertical());

                    ui.vertical(|ui| {
                        ui.set_width(30.0);
                        render_tool_options(ui, active_tool, settings, true);
                    });
                });
            } else {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new(hide_icon)).on_hover_text("Hide/Show All Drawings").clicked() {
                        settings.hide_all = !settings.hide_all;
                    }
                    ui.separator();
                    for tool in &main_tools {
                        let is_selected = *active_tool == *tool;
                        if tool_btn_custom(ui, *tool, is_selected).clicked() { *active_tool = *tool; }
                    }
                    ui.separator();
                    if ui.button("⚙").clicked() { *show_settings_panel = !*show_settings_panel; }
                    if ui.button("📁").clicked() { *show_layers_panel = !*show_layers_panel; }
                    if ui.button("✖").clicked() { *show_exit_dialog = true; }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    render_tool_options(ui, active_tool, settings, false);
                });
            });
        }
    });
}

fn render_tool_options(ui: &mut egui::Ui, active_tool: &Tool, settings: &mut Settings, is_vertical: bool) {
    ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
    
    // Common colors
    ui.vertical(|ui| {
        let mut fg = color32(&settings.pen_color);
        if ui.color_edit_button_srgba(&mut fg).on_hover_text("Pen Color").changed() { settings.pen_color = [fg.r(), fg.g(), fg.b(), fg.a()]; }
        let mut bg = color32(&settings.background_color);
        if ui.color_edit_button_srgba(&mut bg).on_hover_text("Fill Color").changed() { settings.background_color = [bg.r(), bg.g(), bg.b(), bg.a()]; }
    });

    match active_tool {
        Tool::Brush | Tool::Eraser => {
            ui.add(egui::DragValue::new(&mut settings.pen_width).range(1.0..=100.0).prefix("Size: "));
            
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut settings.brush_shape, BrushShape::Round, "○");
                    ui.selectable_value(&mut settings.brush_shape, BrushShape::Square, "□");
                });

            if *active_tool == Tool::Brush {
                if is_vertical {
                    ui.vertical(|ui| {
                        ui.selectable_value(&mut settings.brush_mode, BrushMode::Solid, "Solid");
                        ui.selectable_value(&mut settings.brush_mode, BrushMode::Highlighter, "High");
                        ui.selectable_value(&mut settings.brush_mode, BrushMode::Calligraphy, "Calli");
                        ui.selectable_value(&mut settings.brush_mode, BrushMode::Spray, "Spray");
                    });
                } else {
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut settings.brush_mode, BrushMode::Solid, "Solid");
                        ui.selectable_value(&mut settings.brush_mode, BrushMode::Highlighter, "High");
                        ui.selectable_value(&mut settings.brush_mode, BrushMode::Calligraphy, "Calli");
                        ui.selectable_value(&mut settings.brush_mode, BrushMode::Spray, "Spray");
                    });
                }
                if is_vertical {
                    let text = egui::RichText::new("S").size(16.0).strong();
                    let btn = ui.add(egui::Button::new(text).min_size(egui::vec2(28.0, 24.0)));
                    let rect = btn.rect;
                    
                    // Draw the permanent shadow behind "S"
                    ui.painter().text(rect.center() + egui::vec2(1.0, 1.0), egui::Align2::CENTER_CENTER, "S", egui::FontId::proportional(16.0), egui::Color32::BLACK);
                    
                    if btn.on_hover_text("Brush Shadow").clicked() { settings.brush_shadow = !settings.brush_shadow; }
                    
                    if settings.brush_shadow {
                        // Use the accent color to show it's enabled
                        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "S", egui::FontId::proportional(16.0), color32(&settings.accent_color));
                    } else {
                        // Standard white when disabled (but shadow remains)
                        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "S", egui::FontId::proportional(16.0), egui::Color32::WHITE);
                    }
                } else {
                    ui.toggle_value(&mut settings.brush_shadow, "Shadow");
                }
            } else {
                if is_vertical {
                    ui.vertical(|ui| {
                        ui.selectable_value(&mut settings.eraser_mode, EraserMode::Stroke, "Stroke");
                        ui.selectable_value(&mut settings.eraser_mode, EraserMode::Pixel, "Pixel");
                    });
                } else {
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut settings.eraser_mode, EraserMode::Stroke, "Stroke");
                        ui.selectable_value(&mut settings.eraser_mode, EraserMode::Pixel, "Pixel");
                    });
                }
            }
        }
        Tool::Text => {
            ui.add(egui::DragValue::new(&mut settings.font_size).range(10.0..=200.0).prefix("Size: "));
            if is_vertical {
                ui.vertical(|ui| {
                    ui.selectable_value(&mut settings.text_font, TextFont::Sans, "Sans");
                    ui.selectable_value(&mut settings.text_font, TextFont::Serif, "Serif");
                    ui.selectable_value(&mut settings.text_font, TextFont::Mono, "Mono");
                    ui.selectable_value(&mut settings.text_font, TextFont::Handwriting, "Hand");
                    ui.selectable_value(&mut settings.text_font, TextFont::Heading, "Head");
                });
            } else {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut settings.text_font, TextFont::Sans, "Sans");
                    ui.selectable_value(&mut settings.text_font, TextFont::Serif, "Serif");
                    ui.selectable_value(&mut settings.text_font, TextFont::Mono, "Mono");
                    ui.selectable_value(&mut settings.text_font, TextFont::Handwriting, "Hand");
                    ui.selectable_value(&mut settings.text_font, TextFont::Heading, "Head");
                });
            }
            if is_vertical {
                if ui.add(egui::Button::new("S⬒")).on_hover_text("Shadow").clicked() {
                    settings.text_shadow = !settings.text_shadow;
                }
                if ui.add(egui::Button::new("O⬔")).on_hover_text("Outline").clicked() {
                    settings.text_outline = !settings.text_outline;
                }
            } else {
                ui.toggle_value(&mut settings.text_shadow, "Shadow");
                ui.toggle_value(&mut settings.text_outline, "Outline");
            }
        }
        Tool::Shape => {
            if is_vertical {
                ui.vertical(|ui| {
                    ui.selectable_value(&mut settings.shape_type, ShapeType::Rect, "Rect");
                    ui.selectable_value(&mut settings.shape_type, ShapeType::Circle, "Circ");
                    ui.selectable_value(&mut settings.shape_type, ShapeType::Star, "Star");
                    ui.selectable_value(&mut settings.shape_type, ShapeType::Heart, "Heart");
                });
            } else {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut settings.shape_type, ShapeType::Rect, "Rect");
                    ui.selectable_value(&mut settings.shape_type, ShapeType::Circle, "Circ");
                    ui.selectable_value(&mut settings.shape_type, ShapeType::Star, "Star");
                    ui.selectable_value(&mut settings.shape_type, ShapeType::Heart, "Heart");
                });
            }
            ui.checkbox(&mut settings.shape_fill, "Fill");
        }
        Tool::Snip => {
            if is_vertical {
                ui.vertical(|ui| {
                    ui.selectable_value(&mut settings.snip_mode, SnipMode::Rect, "Rect");
                    ui.selectable_value(&mut settings.snip_mode, SnipMode::Lasso, "Lasso");
                    ui.selectable_value(&mut settings.snip_mode, SnipMode::Polygon, "Poly");
                });
            } else {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut settings.snip_mode, SnipMode::Rect, "Rect");
                    ui.selectable_value(&mut settings.snip_mode, SnipMode::Lasso, "Lasso");
                    ui.selectable_value(&mut settings.snip_mode, SnipMode::Polygon, "Poly");
                });
            }
            if is_vertical {
                if ui.add(egui::Button::new("S⬒")).on_hover_text("Shadow").clicked() {
                    settings.snip_shadow = !settings.snip_shadow;
                }
                ui.add(egui::DragValue::new(&mut settings.blur_strength).range(0.0..=100.0).prefix("Blur: "));
            } else {
                ui.toggle_value(&mut settings.snip_shadow, "Shadow");
                ui.add(egui::DragValue::new(&mut settings.blur_strength).range(0.0..=100.0).prefix("Blur: "));
            }
        }
        Tool::Cut => {
            if is_vertical {
                ui.vertical(|ui| {
                    ui.selectable_value(&mut settings.cut_mode, CutMode::Rect, "Rect");
                    ui.selectable_value(&mut settings.cut_mode, CutMode::Lasso, "Lasso");
                    ui.selectable_value(&mut settings.cut_mode, CutMode::Polygon, "Poly");
                    ui.selectable_value(&mut settings.cut_mode, CutMode::MagicWand, "Wand");
                });
            } else {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut settings.cut_mode, CutMode::Rect, "Rect");
                    ui.selectable_value(&mut settings.cut_mode, CutMode::Lasso, "Lasso");
                    ui.selectable_value(&mut settings.cut_mode, CutMode::Polygon, "Poly");
                    ui.selectable_value(&mut settings.cut_mode, CutMode::MagicWand, "Wand");
                });
            }
            ui.checkbox(&mut settings.inverted_cut, "Invert");
            if settings.cut_mode == CutMode::MagicWand {
                ui.add(egui::DragValue::new(&mut settings.magic_wand_threshold).range(0.0..=100.0).prefix("Wand: "));
            }
        }
        Tool::Blur => {
            ui.add(egui::DragValue::new(&mut settings.blur_strength).range(1.0..=100.0).prefix("Blur Strength: "));
            ui.label("Drag to create live blur area");
        }
        _ => {
            ui.label(egui::RichText::new(active_tool.name()).size(12.0).color(egui::Color32::from_gray(150)));
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
) {
    render_photoshop_panel(ctx, active_tool, settings, show_settings_panel, show_layers_panel, show_exit_dialog, project);
}


// ──────────────────────────────────────────────────────────────
//  Settings window
// ──────────────────────────────────────────────────────────────

pub fn render_settings_window(
    ctx: &egui::Context,
    settings: &mut Settings,
    show: &mut bool,
    clear_all: &mut bool,
    listening_for_hotkey: &mut bool,
) {
    let accent = color32(&settings.accent_color);
    let frame = photoshop_frame(settings);

    egui::Window::new(egui::RichText::new("Settings").color(accent).size(16.0))
        .open(show)
        .resizable(false)
        .collapsible(true)
        .default_width(280.0)
        .frame(frame)
        .show(ctx, |ui| {
            ui.style_mut().visuals.widgets.inactive.bg_fill = egui::Color32::from_rgba_premultiplied(255, 255, 255, 8);
            ui.style_mut().visuals.widgets.hovered.bg_fill  = egui::Color32::from_rgba_premultiplied(255, 255, 255, 18);
            ui.style_mut().visuals.widgets.active.bg_fill   = egui::Color32::from_rgba_premultiplied(accent.r(), accent.g(), accent.b(), 60);

            // ── Hotkey ──
            section_heading(ui, "Hotkey", accent);
            ui.label("Hold this key to enter edit mode:");
            ui.add_space(4.0);

            if *listening_for_hotkey {
                ui.add(
                    egui::Button::new(egui::RichText::new("Press any key...").size(14.0).color(egui::Color32::from_rgb(255, 220, 80)))
                        .fill(egui::Color32::from_rgba_premultiplied(80, 60, 10, 180))
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(255, 220, 80)))
                        .corner_radius(egui::CornerRadius::same(8))
                        .min_size(egui::vec2(200.0, 32.0)),
                );
                if let Some(binding) = detect_pressed_key() {
                    if binding.vk_code == 0x1B { *listening_for_hotkey = false; }
                    else { settings.hotkey = binding; *listening_for_hotkey = false; }
                }
                ctx.request_repaint();
            } else {
                let btn = ui.add(
                    egui::Button::new(egui::RichText::new(format!("  {}  ", settings.hotkey.display_name())).size(14.0))
                        .fill(egui::Color32::from_rgba_premultiplied(255, 255, 255, 8))
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgba_premultiplied(255, 255, 255, 40)))
                        .corner_radius(egui::CornerRadius::same(8))
                        .min_size(egui::vec2(200.0, 32.0)),
                );
                if btn.on_hover_text("Click to rebind").clicked() { *listening_for_hotkey = true; }
            }

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label("UI Scale:");
                ui.add(egui::Slider::new(&mut settings.ui_scale, 0.5..=2.5).show_value(true));
            });

            ui.add_space(12.0);
            ui.add(egui::Separator::default().spacing(6.0));

            // ── Activation ──
            section_heading(ui, "Activation", accent);
            ui.checkbox(&mut settings.toggle_mode, "Toggle mode (tap to toggle)");
            ui.label(egui::RichText::new(
                if settings.toggle_mode { "Press hotkey once to enter edit, again to exit." }
                else { "Hold hotkey to draw. Release = pass-through." }
            ).size(11.0).color(egui::Color32::from_gray(120)));

            ui.add_space(6.0);
            ui.checkbox(&mut settings.keep_ui_visible, "Keep toolbar visible in pass-through");
            ui.checkbox(&mut settings.inverted_cut, "Inverted Cut (keep inside, remove outside)");
            ui.checkbox(&mut settings.hide_edit_info, "Hide Edit Mode Info Text");
            
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Auto-hide drawings (s):");
                ui.add(egui::DragValue::new(&mut settings.auto_hide_seconds).range(0.0..=3600.0));
            });
            ui.label(egui::RichText::new("0 = Never hide automatically").size(10.0).color(egui::Color32::GRAY));

            ui.add_space(4.0);
            if ui.checkbox(&mut settings.exclude_from_capture, "Exclude from capture (Fix Mirror loop)").on_hover_text("Hides this window from OBS, Discord, and Mirror captures. Turn OFF if you want OBS to record the overlay.").changed() {
                crate::winapi_utils::set_capture_exclusion(settings.exclude_from_capture);
            }

            ui.add_space(12.0);
            ui.add(egui::Separator::default().spacing(6.0));

            // ── Compatibility ──
            section_heading(ui, "Compatibility", accent);
            ui.checkbox(&mut settings.software_rendering, "Use Software Rendering (Requires Restart)");
            ui.label(egui::RichText::new("Use this if you experience flickering or transparency issues on some GPUs.").size(10.0).color(egui::Color32::GRAY));




            // ── Accent color ──
            section_heading(ui, "Accent Color", accent);
            let mut ac = color32(&settings.accent_color);
            if ui.color_edit_button_srgba(&mut ac).changed() {
                settings.accent_color = [ac.r(), ac.g(), ac.b(), ac.a()];
            }

            ui.add_space(8.0);
            section_heading(ui, "Toolbar Background", accent);
            let mut tbg = color32(&settings.toolbar_bg_color);
            if ui.color_edit_button_srgba(&mut tbg).changed() {
                settings.toolbar_bg_color = [tbg.r(), tbg.g(), tbg.b(), tbg.a()];
            }

            ui.add_space(12.0);
            ui.add(egui::Separator::default().spacing(6.0));

            // ── Background Image ──
            section_heading(ui, "Background Image", accent);
            ui.horizontal(|ui| {
                if ui.button("📁 Select Image").clicked() {
                    if let Some(path) = rfd::FileDialog::new().add_filter("images", &["png", "jpg", "jpeg", "webp"]).pick_file() {
                        settings.background_image = Some(path.to_string_lossy().to_string());
                    }
                }
                if settings.background_image.is_some() {
                    if ui.button("🗑").clicked() { settings.background_image = None; }
                }
            });

            ui.add_space(12.0);
            ui.add(egui::Separator::default().spacing(6.0));

            // ── Actions ──
            ui.horizontal(|ui| {
                if ui.add(
                    egui::Button::new(egui::RichText::new("Clear All").size(13.0).color(egui::Color32::from_rgb(255, 100, 100)))
                        .fill(egui::Color32::from_rgba_premultiplied(255, 60, 60, 25))
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgba_premultiplied(255, 80, 80, 100)))
                        .corner_radius(egui::CornerRadius::same(8)),
                ).clicked() { *clear_all = true; }

                if ui.add(
                    egui::Button::new(egui::RichText::new("Save").size(13.0).color(egui::Color32::from_rgb(100, 220, 120)))
                        .fill(egui::Color32::from_rgba_premultiplied(60, 200, 80, 25))
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgba_premultiplied(60, 200, 80, 100)))
                        .corner_radius(egui::CornerRadius::same(8)),
                ).clicked() { settings.save(); }
            });

            ui.add_space(12.0);
            let kofi_resp = ui.add(egui::Button::new(egui::RichText::new("☕ Donate on Ko-Fi").size(16.0).strong().color(egui::Color32::WHITE))
                .fill(egui::Color32::from_rgb(41, 171, 226))
                .min_size(egui::vec2(ui.available_width(), 40.0))
                .corner_radius(egui::CornerRadius::same(8)));
            if kofi_resp.clicked() {
                ctx.open_url(egui::OpenUrl::new_tab("https://ko-fi.com/owerlayer"));
            }
        });
}

pub fn render_layers_window(
    ctx: &egui::Context,
    project: &mut crate::project::Project,
    settings: &Settings,
    show: &mut bool,
) {
    let frame = photoshop_frame(settings);

    egui::Window::new(egui::RichText::new("Layers").color(egui::Color32::from_rgb(180, 180, 200)).size(16.0))
        .open(show)
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .default_width(320.0)
        .frame(frame)
        .show(ctx, |ui| {
            ui.style_mut().visuals.widgets.inactive.bg_fill = egui::Color32::from_rgba_premultiplied(255, 255, 255, 8);
            
            ui.horizontal(|ui| {
                if ui.button("📄").on_hover_text("New Project").clicked() {
                    *project = crate::project::Project::new("New Project");
                }
                if ui.button("💾").on_hover_text("Save Project").clicked() { project.save(); }
                if ui.button("📂").on_hover_text("Load Project").clicked() {
                    if let Some(p) = crate::project::Project::load(&project.name) {
                        *project = p;
                    }
                }
                ui.add(egui::TextEdit::singleline(&mut project.name).desired_width(80.0));
            });

            ui.add_space(8.0);

            let mut layer_to_remove = None;
            let mut layer_to_move_up = None;
            let mut layer_to_move_down = None;
            let total_layers = project.layers.len();

            for i in (0..total_layers).rev() {
                let is_active = project.active_layer == i;
                let layer = &mut project.layers[i];
                
                if layer.placed_images.iter().any(|img| img.is_live) {
                    ui.ctx().request_repaint();
                }

                let bg_color = if is_active {
                    egui::Color32::from_rgba_premultiplied(60, 120, 200, 100)
                } else {
                    egui::Color32::TRANSPARENT
                };

                let row_frame = egui::Frame::NONE.fill(bg_color).inner_margin(egui::Margin::symmetric(4, 2)).corner_radius(egui::CornerRadius::same(4));
                
                row_frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // Drag handle
                        let (handle_rect, handle_response) = ui.allocate_exact_size(egui::vec2(12.0, 20.0), egui::Sense::drag());
                        ui.painter().text(handle_rect.center(), egui::Align2::CENTER_CENTER, "⠿", egui::FontId::proportional(14.0), egui::Color32::from_gray(100));
                        
                        if handle_response.dragged() {
                            let delta_y = handle_response.drag_delta().y;
                            if delta_y < -5.0 && i > 0 { layer_to_move_up = Some(i); ui.ctx().request_repaint(); }
                            else if delta_y > 5.0 && i < total_layers - 1 { layer_to_move_down = Some(i); ui.ctx().request_repaint(); }
                        }

                        ui.checkbox(&mut layer.visible, "");
                        
                        let name_resp = ui.add(egui::TextEdit::singleline(&mut layer.name).desired_width(60.0).frame(false));
                        if name_resp.clicked() { project.active_layer = i; }
                        
                        let mut opacity_percent = (layer.opacity * 100.0) as i32;
                        if ui.add(egui::DragValue::new(&mut opacity_percent).range(0..=100).suffix("%")).changed() {
                            layer.opacity = opacity_percent as f32 / 100.0;
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("🗑").on_hover_text("Remove Layer").clicked() { 
                                layer_to_remove = Some(i); 
                            }
                            if ui.button("⬇").on_hover_text("Move to Bottom").clicked() && i > 0 { layer_to_move_down = Some(i); }
                            if ui.button("⬆").on_hover_text("Move to Top").clicked() && i < total_layers - 1 { layer_to_move_up = Some(i); }
                        });
                    });
                    ui.horizontal(|ui| {
                        ui.add_space(20.0);
                        if ui.toggle_value(&mut layer.shadow, "Shadow").clicked() {}
                        ui.add(egui::Separator::default().vertical());
                        if ui.toggle_value(&mut layer.outline, "Outline").clicked() {}
                    });
                });

                // Selection logic
                let row_resp = ui.interact(ui.available_rect_before_wrap(), egui::Id::new(("layer_row", i)), egui::Sense::click());
                if row_resp.clicked() { project.active_layer = i; }
            }

            if let Some(idx) = layer_to_remove {
                project.layers.remove(idx);
                if project.layers.is_empty() {
                    project.layers.push(crate::project::Layer::new("Layer 1"));
                }
                if project.active_layer >= project.layers.len() {
                    project.active_layer = project.layers.len() - 1;
                }
            }
            if let Some(idx) = layer_to_move_up {
                project.layers.swap(idx, idx + 1);
                project.active_layer = idx + 1;
            }
            if let Some(idx) = layer_to_move_down {
                project.layers.swap(idx, idx - 1);
                project.active_layer = idx - 1;
            }

            ui.horizontal(|ui| {
                if ui.button("➕ Add Layer").clicked() {
                    project.layers.push(crate::project::Layer::new(&format!("Layer {}", project.layers.len() + 1)));
                    project.active_layer = project.layers.len() - 1;
                }
                if ui.button("🔗 Merge").clicked() {
                    if project.layers.len() > 1 {
                        let mut base = project.layers.remove(0);
                        for mut l in project.layers.drain(..) {
                            base.strokes.append(&mut l.strokes);
                            base.text_annotations.append(&mut l.text_annotations);
                            base.placed_images.append(&mut l.placed_images);
                        }
                        project.layers.push(base);
                        project.active_layer = 0;
                    }
                }
            });
        });
}

fn section_heading(ui: &mut egui::Ui, text: &str, accent: egui::Color32) {
    ui.add_space(4.0);
    ui.label(egui::RichText::new(text).size(14.0).strong().color(accent));
    ui.add_space(2.0);
}

// ──────────────────────────────────────────────────────────────
//  Mode indicator
// ──────────────────────────────────────────────────────────────

pub fn render_mode_indicator(ctx: &egui::Context, edit_mode: bool, hotkey_name: &str, toggle_mode: bool, settings: &Settings) {
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
                    ui.label(egui::RichText::new("🦉").size(14.0));
                    ui.add(egui::Label::new(egui::RichText::new(hint).size(12.0).color(egui::Color32::from_gray(200))).truncate());
                });
            });
        });
}

// ──────────────────────────────────────────────────────────────
//  Canvas
// ──────────────────────────────────────────────────────────────

fn layer_bounds(layer: &crate::project::Layer) -> Option<egui::Rect> {
    let mut rect: Option<egui::Rect> = None;
    let mut extend = |pos: egui::Pos2| {
        if let Some(r) = &mut rect { r.extend_with(pos); }
        else { rect = Some(egui::Rect::from_min_size(pos, egui::vec2(0.0, 0.0))); }
    };

    for img in &layer.placed_images {
        let disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
        let disp_h = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[1];
        
        let mut found_tight = false;
        if !img.pixels.is_empty() || img.mask.is_some() {
            let mut min_x = img.size[0]; let mut max_x = 0;
            let mut min_y = img.size[1]; let mut max_y = 0;
            let mut has_pixels = false;
            for y in 0..img.size[1] {
                for x in 0..img.size[0] {
                    let idx = y * img.size[0] + x;
                    let alpha = if let Some(m) = &img.mask { m[idx] } else { img.pixels[idx * 4 + 3] };
                    if alpha > 10 {
                        if x < min_x { min_x = x; } if x > max_x { max_x = x; }
                        if y < min_y { min_y = y; } if y > max_y { max_y = y; }
                        has_pixels = true;
                    }
                }
            }
            if has_pixels {
                let px = img.position.x + (min_x as f32 / img.size[0] as f32) * disp_w;
                let py = img.position.y + (min_y as f32 / img.size[1] as f32) * disp_h;
                let pw = ((max_x - min_x + 1) as f32 / img.size[0] as f32) * disp_w;
                let ph = ((max_y - min_y + 1) as f32 / img.size[1] as f32) * disp_h;
                extend(egui::pos2(px, py));
                extend(egui::pos2(px + pw, py + ph));
                found_tight = true;
            }
        }
        
        if !found_tight {
            extend(img.position);
            extend(img.position + egui::vec2(disp_w, disp_h));
        }
    }
    for s in &layer.strokes {
        for p in &s.points { extend(*p); }
    }
    for ann in &layer.text_annotations {
        extend(ann.position);
        extend(ann.position + egui::vec2(ann.text.len() as f32 * ann.font_size * 0.6, ann.font_size * 1.2));
    }
    rect
}

fn translate_layer(layer: &mut crate::project::Layer, delta: egui::Vec2) {
    for img in &mut layer.placed_images { img.position += delta; }
    for s in &mut layer.strokes {
        for p in &mut s.points { *p += delta; }
    }
    for ann in &mut layer.text_annotations { ann.position += delta; }
}

fn scale_layer(layer: &mut crate::project::Layer, center: egui::Pos2, scale: egui::Vec2) {
    // Prevent collapsing to zero
    if scale.x.abs() < 0.01 || scale.y.abs() < 0.01 { return; }

    for img in &mut layer.placed_images {
        let rel = img.position - center;
        img.position = center + egui::vec2(rel.x * scale.x, rel.y * scale.y);
        
        let mut disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
        let mut disp_h = img.display_size.unwrap_or([img.size[1] as f32, img.size[1] as f32])[1];
        
        disp_w = (disp_w * scale.x).abs().max(4.0);
        disp_h = (disp_h * scale.y).abs().max(4.0);
        img.display_size = Some([disp_w, disp_h]);
    }
    for s in &mut layer.strokes {
        for p in &mut s.points {
            let rel = *p - center;
            *p = center + egui::vec2(rel.x * scale.x, rel.y * scale.y);
        }
        s.width = (s.width * (scale.x.abs() + scale.y.abs()) / 2.0).max(0.1);
    }
    for ann in &mut layer.text_annotations {
        let rel = ann.position - center;
        ann.position = center + egui::vec2(rel.x * scale.x, rel.y * scale.y);
        ann.font_size = (ann.font_size * (scale.x.abs() + scale.y.abs()) / 2.0).max(1.0);
    }
}

fn is_inside_poly(poly: &[egui::Pos2], p: egui::Pos2) -> bool {
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        if ((poly[i].y > p.y) != (poly[j].y > p.y)) &&
           (p.x < (poly[j].x - poly[i].x) * (p.y - poly[i].y) / (poly[j].y - poly[i].y) + poly[i].x) {
            inside = !inside;
        }
        j = i;
    }
    inside
}

pub fn render_canvas(
    ui: &mut egui::Ui,
    active_tool: &Tool,
    settings: &mut Settings,
    project: &mut crate::project::Project,
    current_stroke: &mut Vec<egui::Pos2>,
    line_start: &mut Option<egui::Pos2>,
    initial_bounds: &mut Option<egui::Rect>,
    initial_center: &mut Option<egui::Pos2>,
    initial_layer: &mut Option<crate::project::Layer>,
    initial_active_layer_idx: &mut usize,
    dragging_source_rect: &mut bool,
    pending_text: &mut Option<PendingText>,
    last_tool_used: &mut Option<Tool>,
    edit_mode: bool,
    mouse: &MouseState,
    can_draw: bool,
) {
    let rect = ui.available_rect_before_wrap();
    // Use click_and_drag to consume events and prevent eframe from initiating a modal window drag (which causes freezing)
    let (canvas_response, painter) = ui.allocate_painter(rect.size(), egui::Sense::click_and_drag());

    // ── Hide All Logic ──
    if settings.hide_all { 
        // We still allow drawing if edit_mode is true? 
        // User said: "same button should show the once hid when clicking it"
        // Usually, hide all means visually hidden.
    }

    // ── Layers Rendering ──
    for (i, layer) in project.layers.iter_mut().enumerate().filter(|(_, l)| l.visible) {
        if settings.hide_all { continue; }
        let is_active = project.active_layer == i;
        let l_op = layer.opacity;
        
        // Placed Images
        // Shadows for images
        for img in layer.placed_images.iter() {
            if img.shadow || settings.snip_shadow {
                let disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
                let disp_h = img.display_size.unwrap_or([img.size[1] as f32, img.size[1] as f32])[1];
                let shadow_rect = egui::Rect::from_min_size(img.position + egui::vec2(6.0, 6.0), egui::vec2(disp_w, disp_h));
                painter.rect_filled(shadow_rect, 0.0, egui::Color32::from_black_alpha((100.0 * l_op) as u8));
            }
        }

        for img in layer.placed_images.iter_mut() {
            if img.is_live {
                let ppp = ui.ctx().pixels_per_point();
                let (sx, sy) = if let Some(src) = img.source_rect {
                    ((src[0] * ppp) as i32, (src[1] * ppp) as i32)
                } else {
                    ((img.position.x * ppp) as i32, (img.position.y * ppp) as i32)
                };
                let sw = img.size[0] as i32;
                let sh = img.size[1] as i32;
                if let Some(mut p) = crate::winapi_utils::capture_screen_rect(sx, sy, sw, sh) {
                    if img.blur > 0.1 {
                        apply_box_blur(&mut p, sw as usize, sh as usize, img.blur as usize);
                    }
                    
                    if let Some(mask) = &img.mask {
                        p.par_chunks_exact_mut(4).enumerate().for_each(|(i, pixel)| {
                            if mask[i] == 0 { pixel[3] = 0; }
                        });
                    }
                    
                    img.pixels = p;
                    img.texture = None;
                }
            }

            if img.texture.is_none() && !img.pixels.is_empty() {
                let color_image = egui::ColorImage::from_rgba_unmultiplied(img.size, &img.pixels);
                img.texture = Some(ui.ctx().load_texture(
                    format!("snip_{}_{}", layer.name, img.id),
                    color_image,
                    egui::TextureOptions::LINEAR,
                ));
            }
            
            if let Some(tex) = &img.texture {
                let disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
                let disp_h = img.display_size.unwrap_or([img.size[1] as f32, img.size[1] as f32])[1];
                let center = img.position + egui::vec2(disp_w * 0.5, disp_h * 0.5);

                let mut uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                if img.flipped_h { std::mem::swap(&mut uv.min.x, &mut uv.max.x); }
                if img.flipped_v { std::mem::swap(&mut uv.min.y, &mut uv.max.y); }

                if layer.shadow || img.shadow {
                    let mut s_mesh = egui::Mesh::with_texture(tex.id());
                    s_mesh.add_rect_with_uv(egui::Rect::from_min_size(egui::pos2(-disp_w*0.5, -disp_h*0.5), egui::vec2(disp_w, disp_h)), uv, egui::Color32::from_black_alpha((100.0 * l_op) as u8));
                    transform_mesh(&mut s_mesh, center + egui::vec2(3.0, 3.0), img.rotation, img.skew, img.perspective);
                    painter.add(egui::Shape::mesh(s_mesh));
                }

                if layer.outline {
                    let mut o_mesh = egui::Mesh::with_texture(tex.id());
                    o_mesh.add_rect_with_uv(egui::Rect::from_min_size(egui::pos2(-disp_w*0.5-1.5, -disp_h*0.5-1.5), egui::vec2(disp_w+3.0, disp_h+3.0)), uv, egui::Color32::from_white_alpha((200.0 * l_op) as u8));
                    transform_mesh(&mut o_mesh, center, img.rotation, img.skew, img.perspective);
                    painter.add(egui::Shape::mesh(o_mesh));
                }

                let mut mesh = egui::Mesh::with_texture(tex.id());
                mesh.add_rect_with_uv(egui::Rect::from_min_size(egui::pos2(-disp_w*0.5, -disp_h*0.5), egui::vec2(disp_w, disp_h)), uv, egui::Color32::from_white_alpha((255.0 * l_op) as u8));
                transform_mesh(&mut mesh, center, img.rotation, img.skew, img.perspective);
                painter.add(egui::Shape::mesh(mesh));
            }
        }

        // Completed strokes
        for s in layer.strokes.iter() {
            let mut c = color32(&s.color);
            c = egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (c.a() as f32 * l_op) as u8);
            
            if s.brush_mode == BrushMode::Highlighter {
                c = egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (100.0 * l_op) as u8);
            }

            let draw_stroke = |p: &egui::Painter, col: egui::Color32, offset: egui::Vec2| {
                match s.kind {
                    StrokeKind::Freehand => {
                        if s.points.len() >= 2 {
                            let mut pts: Vec<_> = s.points.iter().map(|&pt| pt + offset).collect();
                            
                            // Apply transformations if any
                            if s.rotation != 0.0 || s.skew != egui::Vec2::ZERO || s.perspective.iter().any(|&v| v != egui::Vec2::ZERO) {
                                // Find center for transformation
                                let mut min = egui::pos2(f32::MAX, f32::MAX);
                                let mut max = egui::pos2(f32::MIN, f32::MIN);
                                for &p in &pts {
                                    min.x = min.x.min(p.x); min.y = min.y.min(p.y);
                                    max.x = max.x.max(p.x); max.y = max.y.max(p.y);
                                }
                                let center = min + (max - min) * 0.5;
                                let sin = s.rotation.sin();
                                let cos = s.rotation.cos();
                                let size = max - min;

                                for p in &mut pts {
                                    let tx = if size.x > 0.0 { (p.x - min.x) / size.x } else { 0.5 };
                                    let ty = if size.y > 0.0 { (p.y - min.y) / size.y } else { 0.5 };
                                    let p_offset = 
                                        s.perspective[0] * (1.0 - tx) * (1.0 - ty) +
                                        s.perspective[1] * tx * (1.0 - ty) +
                                        s.perspective[2] * (1.0 - tx) * ty +
                                        s.perspective[3] * tx * ty;
                                    
                                    let rel_p = *p - center;
                                    let px = rel_p.x + p_offset.x + rel_p.y * s.skew.x;
                                    let py = rel_p.y + p_offset.y + rel_p.x * s.skew.y;
                                    
                                    p.x = center.x + px * cos - py * sin;
                                    p.y = center.y + py * cos + px * sin;
                                }
                            }

                            if s.brush_mode == BrushMode::Spray {
                                for pt in &pts {
                                    let mut seed = (pt.x * 1337.0 + pt.y * 7331.0) as u32;
                                    for _ in 0..12 {
                                        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
                                        let rx = ((seed % 100) as f32 - 50.0) / 50.0 * s.width;
                                        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
                                        let ry = ((seed % 100) as f32 - 50.0) / 50.0 * s.width;
                                        p.circle_filled(*pt + egui::vec2(rx, ry), 1.0, col);
                                    }
                                }
                            } else if s.brush_mode == BrushMode::Calligraphy {
                                let angle = std::f32::consts::PI / 4.0;
                                let perp = egui::vec2(angle.cos(), angle.sin()) * s.width * 0.5;
                                for window in pts.windows(2) {
                                    p.add(egui::Shape::convex_polygon(
                                        vec![window[0] - perp, window[0] + perp, window[1] + perp, window[1] - perp],
                                        col, egui::Stroke::NONE,
                                    ));
                                }
                            } else {
                                if s.brush_shape == BrushShape::Square {
                                    for window in pts.windows(2) {
                                        let dir = (window[1] - window[0]).normalized();
                                        let perp = egui::vec2(-dir.y, dir.x) * s.width * 0.5;
                                        p.add(egui::Shape::convex_polygon(
                                            vec![window[0] - perp, window[0] + perp, window[1] + perp, window[1] - perp],
                                            col, egui::Stroke::NONE,
                                        ));
                                    }
                                } else {
                                    p.add(egui::Shape::line(pts, egui::Stroke::new(s.width, col)));
                                }
                            }
                        }
                    }
                    StrokeKind::Line => { if s.points.len() == 2 { p.line_segment([s.points[0]+offset, s.points[1]+offset], egui::Stroke::new(s.width, col)); } }
                    StrokeKind::Arrow => { if s.points.len() == 2 { draw_arrow(p, s.points[0]+offset, s.points[1]+offset, s.width, col); } }
                    StrokeKind::Rect => {
                        if s.points.len() == 2 {
                            let rect = egui::Rect::from_two_pos(s.points[0]+offset, s.points[1]+offset);
                            if offset == egui::Vec2::ZERO { // Only fill on main pass
                                if let Some(bg) = s.background_color {
                                    let mut bg_col = color32(&bg);
                                    bg_col = egui::Color32::from_rgba_unmultiplied(bg_col.r(), bg_col.g(), bg_col.b(), (bg_col.a() as f32 * l_op) as u8);
                                    p.rect_filled(rect, 0.0, bg_col);
                                }
                            }
                            p.rect_stroke(rect, 0.0, egui::Stroke::new(s.width, col), egui::StrokeKind::Middle);
                        }
                    }
                    StrokeKind::Circle => {
                        if s.points.len() == 2 {
                            let center = s.points[0] + offset;
                            let radius = s.points[0].distance(s.points[1]);
                            if offset == egui::Vec2::ZERO {
                                if let Some(bg) = s.background_color {
                                    let mut bg_col = color32(&bg);
                                    bg_col = egui::Color32::from_rgba_unmultiplied(bg_col.r(), bg_col.g(), bg_col.b(), (bg_col.a() as f32 * l_op) as u8);
                                    p.circle_filled(center, radius, bg_col);
                                }
                            }
                            p.circle_stroke(center, radius, egui::Stroke::new(s.width, col));
                        }
                    }
                    _ => {} // Other shapes skipped for brief
                }
            };

            if layer.shadow || s.shadow {
                draw_stroke(&painter, egui::Color32::from_black_alpha((128.0 * l_op) as u8), egui::vec2(2.0, 2.0));
            }
            if layer.outline {
                let o_col = if c.r() as u32 + c.g() as u32 + c.b() as u32 > 382 { egui::Color32::BLACK } else { egui::Color32::WHITE };
                draw_stroke(&painter, o_col, egui::vec2(1.0, 0.0));
                draw_stroke(&painter, o_col, egui::vec2(-1.0, 0.0));
                draw_stroke(&painter, o_col, egui::vec2(0.0, 1.0));
                draw_stroke(&painter, o_col, egui::vec2(0.0, -1.0));
            }
            draw_stroke(&painter, c, egui::Vec2::ZERO);
        }

        // Text annotations
        for ann in layer.text_annotations.iter() {
            let font = match ann.font {
                TextFont::Sans => egui::FontId::proportional(ann.font_size),
                TextFont::Serif => egui::FontId::new(ann.font_size, egui::FontFamily::Name("serif".into())),
                TextFont::Mono => egui::FontId::monospace(ann.font_size),
                TextFont::Handwriting => egui::FontId::new(ann.font_size, egui::FontFamily::Name("handwriting".into())),
                TextFont::Heading => egui::FontId::new(ann.font_size, egui::FontFamily::Name("heading".into())),
            };
            let mut c = color32(&ann.color);
            c = egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (c.a() as f32 * l_op) as u8);
            
            let outline_col = if c.r() as u32 + c.g() as u32 + c.b() as u32 > 382 { 
                egui::Color32::from_black_alpha((255.0 * l_op) as u8)
            } else { 
                egui::Color32::from_white_alpha((255.0 * l_op) as u8)
            };
            
            if layer.shadow || ann.shadow || settings.text_shadow {
                painter.text(ann.position + egui::vec2(2.0, 2.0), egui::Align2::LEFT_TOP, &ann.text, font.clone(), egui::Color32::from_black_alpha((150.0 * l_op) as u8));
            }

            if layer.outline || ann.outline || settings.text_outline {
                let sw = if ann.stroke_width > 0.0 { ann.stroke_width } else { 1.0 };
                painter.text(ann.position + egui::vec2(sw, sw), egui::Align2::LEFT_TOP, &ann.text, font.clone(), outline_col);
                painter.text(ann.position + egui::vec2(-sw, -sw), egui::Align2::LEFT_TOP, &ann.text, font.clone(), outline_col);
                painter.text(ann.position + egui::vec2(sw, -sw), egui::Align2::LEFT_TOP, &ann.text, font.clone(), outline_col);
                painter.text(ann.position + egui::vec2(-sw, sw), egui::Align2::LEFT_TOP, &ann.text, font.clone(), outline_col);
            }
            
            painter.text(ann.position, egui::Align2::LEFT_TOP, &ann.text, font, c);
        }

        // Bounding box for Move tool
        if *active_tool == Tool::Move && is_active {
            if let Some(bounds) = layer_bounds(layer) {
                painter.rect_stroke(bounds.expand(4.0), 0.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 180, 255)), egui::StrokeKind::Middle);
                let handle_rect = egui::Rect::from_center_size(bounds.max + egui::vec2(4.0, 4.0), egui::vec2(8.0, 8.0));
                painter.rect_filled(handle_rect, 0.0, egui::Color32::WHITE);
                painter.rect_stroke(handle_rect, 0.0, egui::Stroke::new(1.0, egui::Color32::BLACK), egui::StrokeKind::Middle);
            }
        }
    }

    // ── Live preview ──
    let mut pen_c = color32(&settings.pen_color);
    if settings.brush_mode == BrushMode::Highlighter {
        pen_c = egui::Color32::from_rgba_premultiplied(pen_c.r() / 2, pen_c.g() / 2, pen_c.b() / 2, 80);
    }
    
    if current_stroke.len() >= 2 && matches!(*active_tool, Tool::Brush) {
        if settings.brush_mode == BrushMode::Calligraphy {
            let angle = std::f32::consts::PI / 4.0;
            let perp = egui::vec2(angle.cos(), angle.sin()) * settings.pen_width * 0.5;
            for window in current_stroke.windows(2) {
                let p1 = window[0];
                let p2 = window[1];
                painter.add(egui::Shape::convex_polygon(
                    vec![p1 - perp, p1 + perp, p2 + perp, p2 - perp],
                    pen_c,
                    egui::Stroke::NONE,
                ));
            }
        } else {
            painter.add(egui::Shape::line(current_stroke.clone(), egui::Stroke::new(settings.pen_width, pen_c)));
        }
    }
    if let Some(start) = *line_start {
        if matches!(active_tool, Tool::Shape) {
            let rect = egui::Rect::from_two_pos(start, mouse.pos);
            let _bg = color32(&settings.background_color);
            match settings.shape_type {
                ShapeType::Rect => {
                    let bg_c = color32(&settings.background_color);
                    painter.rect_filled(rect, 0.0, egui::Color32::from_rgba_unmultiplied(bg_c.r(), bg_c.g(), bg_c.b(), bg_c.a()/2)); // Live preview with half opacity
                    painter.rect_stroke(rect, 0.0, egui::Stroke::new(settings.stroke_width, pen_c), egui::StrokeKind::Middle);
                }
                ShapeType::Circle => {
                    let radius = start.distance(mouse.pos);
                    let bg_c = color32(&settings.background_color);
                    painter.circle_filled(start, radius, egui::Color32::from_rgba_unmultiplied(bg_c.r(), bg_c.g(), bg_c.b(), bg_c.a()/2));
                    painter.circle_stroke(start, radius, egui::Stroke::new(settings.stroke_width, pen_c));
                }
                ShapeType::Star => {
                    let radius = start.distance(mouse.pos);
                    let bg_c = color32(&settings.background_color);
                    draw_star(&painter, start, radius, pen_c, Some(egui::Color32::from_rgba_unmultiplied(bg_c.r(), bg_c.g(), bg_c.b(), bg_c.a()/2)), settings.stroke_width);
                }
                ShapeType::Heart => {
                    let radius = start.distance(mouse.pos);
                    let bg_c = color32(&settings.background_color);
                    draw_heart(&painter, start, radius, pen_c, Some(egui::Color32::from_rgba_unmultiplied(bg_c.r(), bg_c.g(), bg_c.b(), bg_c.a()/2)), settings.stroke_width);
                }
                _ => {}
            }
        } else if matches!(active_tool, Tool::Snip) {
            // Draw a dashed rect for snip preview
            let rect = egui::Rect::from_two_pos(start, mouse.pos);
            painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, egui::Color32::WHITE), egui::StrokeKind::Middle);
            painter.rect_stroke(rect.expand(1.0), 0.0, egui::Stroke::new(1.0, egui::Color32::BLACK), egui::StrokeKind::Middle);
            painter.rect_filled(rect, 0.0, egui::Color32::from_black_alpha(40));
        }
    }

    // ── Pending text cursor ──
    if let Some(pending) = pending_text.as_ref() {
        let time = ui.input(|i| i.time);
        let blink = (time * 3.0).sin() > 0.0;
        let font = if settings.text_monospace { egui::FontId::monospace(settings.font_size) } else { egui::FontId::proportional(settings.font_size) };
        if !pending.buffer.is_empty() {
            if settings.text_outline {
                let c = pen_c;
                let outline_col = if c.r() as u32 + c.g() as u32 + c.b() as u32 > 382 { egui::Color32::BLACK } else { egui::Color32::WHITE };
                painter.text(pending.position + egui::vec2(1.0, 1.0), egui::Align2::LEFT_TOP, &pending.buffer, font.clone(), outline_col);
                painter.text(pending.position + egui::vec2(-1.0, -1.0), egui::Align2::LEFT_TOP, &pending.buffer, font.clone(), outline_col);
                painter.text(pending.position + egui::vec2(1.0, -1.0), egui::Align2::LEFT_TOP, &pending.buffer, font.clone(), outline_col);
                painter.text(pending.position + egui::vec2(-1.0, 1.0), egui::Align2::LEFT_TOP, &pending.buffer, font.clone(), outline_col);
            } else if settings.text_shadow {
                painter.text(pending.position + egui::vec2(2.0, 2.0), egui::Align2::LEFT_TOP, &pending.buffer, font.clone(), egui::Color32::from_black_alpha(150));
            }
            painter.text(pending.position, egui::Align2::LEFT_TOP, &pending.buffer, font.clone(), pen_c);
        }
        if blink {
            let tw = if pending.buffer.is_empty() { 0.0 } else {
                ui.fonts(|f| f.layout_no_wrap(pending.buffer.clone(), font.clone(), pen_c).rect.width())
            };
            let cs = if pending.buffer.is_empty() { "Type here... |" } else { "|" };
            painter.text(
                egui::pos2(pending.position.x + tw, pending.position.y),
                egui::Align2::LEFT_TOP, cs, font,
                if pending.buffer.is_empty() { egui::Color32::from_rgba_unmultiplied(150, 150, 150, 140) } else { pen_c },
            );
        }
        ui.ctx().request_repaint();
    }

    // Use egui's native logical coordinates for precision drawing
    let pos = ui.input(|i| i.pointer.hover_pos()).unwrap_or(mouse.pos);
    let left_down = ui.input(|i| i.pointer.primary_down());
    let left_just_pressed = ui.input(|i| i.pointer.primary_pressed());
    let left_just_released = ui.input(|i| i.pointer.primary_released());

    // Auto-create new layer if using a different tool for the first time
    // We only do this if we are using a drawing tool: Snip, Text.
    let is_auto_layer_tool = matches!(*active_tool, Tool::Snip | Tool::Text | Tool::Blur);
    if left_just_pressed && can_draw && edit_mode && is_auto_layer_tool {
        let needs_new_layer = match last_tool_used {
            Some(t) => *t != *active_tool,
            None => true,
        };
        if needs_new_layer {
            // Check if current layer is completely empty and named "Layer 1"
            let is_empty_start = project.layers.len() == 1 
                && project.layers[0].strokes.is_empty() 
                && project.layers[0].text_annotations.is_empty() 
                && project.layers[0].placed_images.is_empty();
                
            if is_empty_start {
                project.layers[0].name = active_tool.name().to_string();
            } else {
                project.layers.push(crate::project::Layer::new(active_tool.name()));
                project.active_layer = project.layers.len() - 1;
            }
            *last_tool_used = Some(*active_tool);
        }
    }

    let snip_created = false;
    
    // Layer selection for Move tool
    if left_just_pressed && can_draw && edit_mode && matches!(active_tool, Tool::Move) {
        let mut hit_idx = None;
        for (i, l) in project.layers.iter().enumerate().rev() {
            if let Some(b) = layer_bounds(l) {
                if b.contains(pos) {
                    hit_idx = Some(i);
                    break;
                }
            }
        }
        if let Some(idx) = hit_idx {
            project.active_layer = idx;
        }
    }

    let mut remove_active_layer = false;
    let active_layer = project.get_active_layer_mut();
    if !edit_mode || !can_draw || active_layer.is_none() || ui.ctx().is_pointer_over_area() { return; }
    let layer = active_layer.unwrap();

    match active_tool {
        Tool::Brush => {
            if left_down {
                // Reject glitchy points (0,0) or huge jumps
                if pos.x < 1.0 && pos.y < 1.0 { return; }

                if let Some(last) = current_stroke.last().cloned() {
                    let dist = last.distance(pos);
                    if dist > 200.0 { 
                        current_stroke.clear();
                        current_stroke.push(pos);
                    } else if dist > 0.5 {
                        let steps = (dist / 0.5) as usize;
                        for s in 1..=steps {
                            let t = s as f32 / steps as f32;
                            current_stroke.push(last.lerp(pos, t));
                        }
                    }
                } else { current_stroke.push(pos); }
            }
            if left_just_released && !current_stroke.is_empty() {
                layer.strokes.push(Stroke {
                    points: current_stroke.clone(),
                    color: settings.pen_color, width: settings.pen_width,
                    kind: StrokeKind::Freehand, brush_mode: settings.brush_mode,
                    background_color: Some(settings.background_color),
                    shadow: settings.brush_shadow,
                    rotation: 0.0, flipped_h: false, flipped_v: false,
                    perspective: [egui::Vec2::ZERO; 4],
                    brush_shape: settings.brush_shape,
                    skew: egui::Vec2::ZERO,
                });
                current_stroke.clear();
            }
        }
        Tool::Shape => {
            if left_just_pressed { *line_start = Some(pos); }
            if left_just_released {
                if let Some(start) = line_start.take() {
                    let kind = match settings.shape_type {
                        ShapeType::Rect => StrokeKind::Rect,
                        ShapeType::Circle => StrokeKind::Circle,
                        ShapeType::Star => StrokeKind::Star,
                        ShapeType::Heart => StrokeKind::Heart,
                    };
                    layer.strokes.push(Stroke {
                        points: vec![start, pos],
                        color: settings.pen_color, width: settings.pen_width,
                        kind, brush_mode: settings.brush_mode,
                        background_color: Some(settings.background_color),
                        shadow: settings.brush_shadow,
                        rotation: 0.0, flipped_h: false, flipped_v: false,
                        perspective: [egui::Vec2::ZERO; 4],
                        brush_shape: settings.brush_shape,
                        skew: egui::Vec2::ZERO,
                    });
                }
            }
        }
        Tool::Snip => {
            let mode = settings.snip_mode;
            if mode == SnipMode::Rect {
                if left_just_pressed { *line_start = Some(pos); }
                if let Some(start) = *line_start {
                    let rect = egui::Rect::from_two_pos(start, pos);
                    painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, egui::Color32::WHITE), egui::StrokeKind::Middle);
                }
                if left_just_released {
                    if let Some(start) = line_start.take() {
                        let rect = egui::Rect::from_two_pos(start, pos);
                        let x = rect.min.x.max(0.0);
                        let y = rect.min.y.max(0.0);
                        let w = rect.width();
                        let h = rect.height();
                        if w > 5.0 && h > 5.0 {
                            let ppp = ui.ctx().pixels_per_point();
                            if let Some(pixels) = crate::winapi_utils::capture_screen_rect((x * ppp) as i32, (y * ppp) as i32, (w * ppp) as i32, (h * ppp) as i32) {
                                layer.placed_images.push(PlacedImage {
                                    id: layer.placed_images.len(), position: rect.min,
                                    size: [(w * ppp) as usize, (h * ppp) as usize], display_size: Some([w, h]),
                                    pixels, texture: None, shadow: settings.snip_shadow,
                                    rotation: 0.0, flipped_h: false, flipped_v: false, skew: egui::vec2(0.0, 0.0),
                                    perspective: [egui::Vec2::ZERO; 4],
                                    is_live: false,
                                    source_rect: None,
                                    blur: 0.0,
                                    mask: None,
                                    last_capture: 0.0,
                                });
                            }
                        }
                    }
                }
            } else if mode == SnipMode::Lasso {
                if left_down { current_stroke.push(pos); }
                if !current_stroke.is_empty() {
                    painter.add(egui::Shape::line(current_stroke.clone(), egui::Stroke::new(1.0, egui::Color32::WHITE)));
                }
                if left_just_released && current_stroke.len() > 3 {
                    let bounds = egui::Rect::from_points(&current_stroke);
                    let ppp = ui.ctx().pixels_per_point();
                    let sx = (bounds.min.x.max(0.0) * ppp) as i32;
                    let sy = (bounds.min.y.max(0.0) * ppp) as i32;
                    let sw = (bounds.width() * ppp) as i32;
                    let sh = (bounds.height() * ppp) as i32;
                    
                    if sw > 5 && sh > 5 {
                        if let Some(mut pixels) = crate::winapi_utils::capture_screen_rect(sx, sy, sw, sh) {
                            let poly: Vec<egui::Pos2> = current_stroke.iter().map(|p| egui::pos2(p.x - bounds.min.x, p.y - bounds.min.y)).collect();
                            for py in 0..sh as usize {
                                for px in 0..sw as usize {
                                    let lp = egui::pos2(px as f32 / ppp, py as f32 / ppp);
                                    if !is_inside_poly(&poly, lp) {
                                        let idx = (py * sw as usize + px) * 4;
                                        pixels[idx+3] = 0;
                                    }
                                }
                            }
                            layer.placed_images.push(PlacedImage {
                                id: layer.placed_images.len(), position: bounds.min,
                                size: [sw as usize, sh as usize], display_size: Some([bounds.width(), bounds.height()]),
                                pixels, texture: None, shadow: settings.snip_shadow,
                                rotation: 0.0, flipped_h: false, flipped_v: false, skew: egui::vec2(0.0, 0.0),
                                perspective: [egui::Vec2::ZERO; 4],
                                is_live: false,
                                source_rect: None,
                                blur: 0.0,
                                mask: None,
                                last_capture: 0.0,
                            });
                        }
                    }
                    current_stroke.clear();
                }
            } else if mode == SnipMode::Polygon {
                if left_just_pressed { current_stroke.push(pos); }
                let right_clicked = ui.input(|i| i.pointer.secondary_pressed());
                let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                let close_to_start = current_stroke.len() > 2 && pos.distance(current_stroke[0]) < 15.0 && left_just_pressed;
                
                if (right_clicked || enter_pressed || close_to_start) && !current_stroke.is_empty() {
                    let bounds = egui::Rect::from_points(&current_stroke);
                    let ppp = ui.ctx().pixels_per_point();
                    let sx = (bounds.min.x.max(0.0) * ppp) as i32;
                    let sy = (bounds.min.y.max(0.0) * ppp) as i32;
                    let sw = (bounds.width() * ppp) as i32;
                    let sh = (bounds.height() * ppp) as i32;
                    
                    if sw > 5 && sh > 5 {
                        if let Some(mut pixels) = crate::winapi_utils::capture_screen_rect(sx, sy, sw, sh) {
                            let poly: Vec<egui::Pos2> = current_stroke.iter().map(|p| egui::pos2(p.x - bounds.min.x, p.y - bounds.min.y)).collect();
                            for py in 0..sh as usize {
                                for px in 0..sw as usize {
                                    let lp = egui::pos2(px as f32 / ppp, py as f32 / ppp);
                                    if !is_inside_poly(&poly, lp) {
                                        let idx = (py * sw as usize + px) * 4;
                                        pixels[idx+3] = 0;
                                    }
                                }
                            }
                            layer.placed_images.push(PlacedImage {
                                id: layer.placed_images.len(), position: bounds.min,
                                size: [sw as usize, sh as usize], display_size: Some([bounds.width(), bounds.height()]),
                                pixels, texture: None, shadow: settings.snip_shadow,
                                rotation: 0.0, flipped_h: false, flipped_v: false, skew: egui::vec2(0.0, 0.0),
                                perspective: [egui::Vec2::ZERO; 4],
                                is_live: false,
                                source_rect: None,
                                blur: 0.0,
                                mask: None,
                                last_capture: 0.0,
                            });
                        }
                    }
                    current_stroke.clear();
                }
                if !current_stroke.is_empty() {
                    let mut pts = current_stroke.clone();
                    pts.push(pos);
                    painter.add(egui::Shape::line(pts, egui::Stroke::new(1.0, egui::Color32::WHITE)));
                }
            }
        }
        Tool::Cut => {
            let mode = settings.cut_mode;
            let mut cut_rect: Option<egui::Rect> = None;
            let mut cut_poly: Option<Vec<egui::Pos2>> = None;

            if mode == CutMode::Lasso {
                if left_down { 
                    current_stroke.push(pos); 
                }
                if !current_stroke.is_empty() {
                    painter.add(egui::Shape::line(current_stroke.clone(), egui::Stroke::new(1.0, egui::Color32::WHITE)));
                }
                if left_just_released && !current_stroke.is_empty() {
                    cut_poly = Some(current_stroke.clone());
                    current_stroke.clear();
                }
            } else if mode == CutMode::Rect {
                if left_just_pressed { *line_start = Some(pos); }
                if let Some(start) = *line_start {
                    let rect = egui::Rect::from_two_pos(start, pos);
                    painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, egui::Color32::WHITE), egui::StrokeKind::Middle);
                }
                if left_just_released {
                    if let Some(start) = line_start.take() {
                        let rect = egui::Rect::from_two_pos(start, pos);
                        if rect.width() > 2.0 && rect.height() > 2.0 { cut_rect = Some(rect); }
                    }
                }
            } else if mode == CutMode::Polygon {
                if left_just_pressed { current_stroke.push(pos); }
                let right_clicked = ui.input(|i| i.pointer.secondary_pressed());
                let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                let close_to_start = current_stroke.len() > 2 && pos.distance(current_stroke[0]) < 15.0 && left_just_pressed;
                if (right_clicked || enter_pressed || close_to_start) && !current_stroke.is_empty() {
                    cut_poly = Some(current_stroke.clone());
                    current_stroke.clear();
                }
                if !current_stroke.is_empty() {
                    let mut pts = current_stroke.clone();
                    pts.push(pos);
                    painter.add(egui::Shape::line(pts, egui::Stroke::new(1.0, egui::Color32::WHITE)));
                }
            } else if mode == CutMode::MagicWand {
                if left_just_pressed {
                    for img in &mut layer.placed_images {
                        let disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
                        let disp_h = img.display_size.unwrap_or([img.size[1] as f32, img.size[1] as f32])[1];
                        let img_rect = egui::Rect::from_min_size(img.position, egui::vec2(disp_w, disp_h));
                        
                        if img_rect.contains(pos) {
                            let px = ((pos.x - img.position.x) * (img.size[0] as f32 / disp_w)) as i32;
                            let py = ((pos.y - img.position.y) * (img.size[1] as f32 / disp_h)) as i32;
                            
                            if px >= 0 && px < img.size[0] as i32 && py >= 0 && py < img.size[1] as i32 {
                                let start_idx = (py as usize * img.size[0] + px as usize) * 4;
                                let start_color = [img.pixels[start_idx], img.pixels[start_idx+1], img.pixels[start_idx+2], img.pixels[start_idx+3]];
                                
                                if start_color[3] > 0 {
                                    magic_wand_flood_fill(img, px, py, start_color, settings.magic_wand_threshold);
                                    img.texture = None; 
                                }
                            }
                        }
                    }
                }
            }

            if let Some(rect) = cut_rect {
                painter.rect_filled(rect, 0.0, egui::Color32::from_white_alpha(180));
                for img in &mut layer.placed_images {
                    let disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
                    let disp_h = img.display_size.unwrap_or([img.size[1] as f32, img.size[1] as f32])[1];
                    let img_rect = egui::Rect::from_min_size(img.position, egui::vec2(disp_w, disp_h));
                    if img_rect.intersects(rect) {
                        let mut modified = false;
                        if img.is_live && img.mask.is_none() {
                            img.mask = Some(vec![255; img.size[0] * img.size[1]]);
                        }

                        for py in 0..img.size[1] {
                            for px in 0..img.size[0] {
                                let local_pos = img.position + egui::vec2(px as f32 * (disp_w / img.size[0] as f32), py as f32 * (disp_h / img.size[1] as f32));
                                let inside = rect.contains(local_pos);
                                let should_erase = if settings.inverted_cut { !inside } else { inside };
                                if should_erase {
                                    let idx = py * img.size[0] + px;
                                    if img.is_live {
                                        let mask = img.mask.as_mut().unwrap();
                                        if mask[idx] != 0 { mask[idx] = 0; modified = true; }
                                    } else {
                                        let b_idx = idx * 4;
                                        if img.pixels[b_idx + 3] != 0 { img.pixels[b_idx + 3] = 0; modified = true; }
                                    }
                                }
                            }
                        }
                        if modified { img.texture = None; }
                    }
                }
            } else if let Some(poly) = cut_poly {
                painter.add(egui::Shape::convex_polygon(poly.clone(), egui::Color32::from_white_alpha(180), egui::Stroke::NONE));
                for img in &mut layer.placed_images {
                    let disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
                    let disp_h = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[1];
                    let mut modified = false;
                    if img.is_live && img.mask.is_none() {
                        img.mask = Some(vec![255; img.size[0] * img.size[1]]);
                    }

                    for py in 0..img.size[1] {
                        for px in 0..img.size[0] {
                            let local_pos = img.position + egui::vec2(px as f32 * (disp_w / img.size[0] as f32), py as f32 * (disp_h / img.size[1] as f32));
                            let inside = is_inside_poly(&poly, local_pos);
                            let should_erase = if settings.inverted_cut { !inside } else { inside };
                            if should_erase {
                                let idx = py * img.size[0] + px;
                                if img.is_live {
                                    let mask = img.mask.as_mut().unwrap();
                                    if mask[idx] != 0 { mask[idx] = 0; modified = true; }
                                } else {
                                    let b_idx = idx * 4;
                                    if img.pixels[b_idx+3] != 0 { img.pixels[b_idx+3] = 0; modified = true; }
                                }
                            }
                        }
                    }
                    if modified { img.texture = None; }
                }
            }
        }
        Tool::Pipette => {
            if left_down {
                if let Some(pixels) = crate::winapi_utils::capture_screen_rect(pos.x as i32, pos.y as i32, 1, 1) {
                    settings.pen_color = [pixels[0], pixels[1], pixels[2], 255];
                }
            }
        }
        Tool::Mirror => {
            if left_just_pressed { *line_start = Some(pos); }
            if let Some(start) = *line_start {
                let rect = egui::Rect::from_two_pos(start, pos);
                painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, color32(&settings.accent_color)), egui::StrokeKind::Middle);
                painter.rect_filled(rect, 0.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 20));
            }
            if left_just_released {
                if let Some(start) = line_start.take() {
                    let rect = egui::Rect::from_two_pos(start, pos);
                    let w = rect.width();
                    let h = rect.height();
                    if w > 4.0 && h > 4.0 {
                        // Create Mirror layer if not exists
                        let mut mirror_layer_idx = None;
                        for (idx, l) in project.layers.iter().enumerate() {
                            if l.name == "Mirrors" { mirror_layer_idx = Some(idx); break; }
                        }
                        let l_idx = if let Some(idx) = mirror_layer_idx {
                            idx
                        } else {
                            project.layers.push(crate::project::Layer::new("Mirrors"));
                            project.layers.len() - 1
                        };
                        let target_layer = &mut project.layers[l_idx];

                        let ppp = ui.ctx().pixels_per_point();
                        target_layer.placed_images.push(PlacedImage {
                            id: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as usize,
                            position: rect.min,
                            size: [(w * ppp) as usize, (h * ppp) as usize], display_size: Some([w, h]),
                            pixels: Vec::new(), texture: None, shadow: settings.snip_shadow,
                            rotation: 0.0, flipped_h: false, flipped_v: false, skew: egui::vec2(0.0, 0.0),
                            perspective: [egui::Vec2::ZERO; 4],
                            is_live: true,
                            source_rect: Some([rect.min.x, rect.min.y, w, h]),
                            blur: 0.0,
                            mask: None,
                            last_capture: 0.0,
                        });
                    }
                }
            }
        }
        Tool::Blur => {
            if left_just_pressed { *line_start = Some(pos); }
            if let Some(start) = *line_start {
                let rect = egui::Rect::from_two_pos(start, pos);
                painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, egui::Color32::WHITE), egui::StrokeKind::Middle);
                painter.rect_filled(rect, 0.0, egui::Color32::from_black_alpha(40));
            }
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

                        let ppp = ui.ctx().pixels_per_point();
                        target_layer.placed_images.push(PlacedImage {
                            id: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as usize,
                            position: rect.min,
                            size: [(w * ppp) as usize, (h * ppp) as usize], display_size: Some([w, h]),
                            pixels: Vec::new(), texture: None, shadow: false,
                            rotation: 0.0, flipped_h: false, flipped_v: false, skew: egui::vec2(0.0, 0.0),
                            perspective: [egui::Vec2::ZERO; 4],
                            is_live: true,
                            source_rect: None,
                            blur: settings.blur_strength,
                            mask: None,
                            last_capture: 0.0,
                        });
                    }
                }
            }
        }
        Tool::Text => {
            if canvas_response.double_clicked() {
                for (idx, ann) in layer.text_annotations.iter().enumerate() {
                    let rect = egui::Rect::from_min_size(ann.position, egui::vec2(ann.text.len() as f32 * ann.font_size * 0.6, ann.font_size * 1.2));
                    if rect.contains(pos) {
                        *pending_text = Some(PendingText { position: ann.position, buffer: ann.text.clone() });
                        layer.text_annotations.remove(idx);
                        break;
                    }
                }
            } else if left_just_pressed {
                if let Some(p) = pending_text.take() {
                    if !p.buffer.is_empty() {
                        layer.text_annotations.push(TextAnnotation {
                            position: p.position, text: p.buffer,
                            color: settings.pen_color, font_size: settings.font_size,
                            monospace: settings.text_monospace, shadow: settings.text_shadow, outline: settings.text_outline,
                            stroke_width: settings.text_stroke_width,
                            rotation: 0.0, flipped_h: false, flipped_v: false,
                            perspective: [egui::Vec2::ZERO; 4],
                            font: settings.text_font,
                            skew: egui::Vec2::ZERO,
                        });
                    }
                } else {
                    *pending_text = Some(PendingText { position: pos, buffer: String::new() });
                }
            }
        }
        Tool::Move => {
            if let Some(bounds) = layer_bounds(layer) {
                let corners = [
                    bounds.left_top(), bounds.right_top(),
                    bounds.left_bottom(), bounds.right_bottom(),
                ];
                
                // --- Highlight Mirror Source Rect ---
                for img in &layer.placed_images {
                    if img.is_live && img.source_rect.is_some() {
                        let src = img.source_rect.unwrap();
                        let src_rect = egui::Rect::from_min_size(egui::pos2(src[0], src[1]), egui::vec2(src[2], src[3]));
                        painter.rect_stroke(src_rect, 0.0, egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 100, 0)), egui::StrokeKind::Middle);
                        painter.text(src_rect.left_top() - egui::vec2(0.0, 10.0), egui::Align2::LEFT_BOTTOM, "Source", egui::FontId::proportional(10.0), egui::Color32::from_rgb(255, 100, 0));
                        
                        // Handles for source rect
                        let s_corners = [src_rect.left_top(), src_rect.right_top(), src_rect.left_bottom(), src_rect.right_bottom()];
                        for (idx, &sc) in s_corners.iter().enumerate() {
                            let handle_rect = egui::Rect::from_center_size(sc, egui::vec2(8.0, 8.0));
                            painter.rect_filled(handle_rect, 0.0, egui::Color32::from_rgb(255, 150, 50));
                            if left_just_pressed && handle_rect.contains(pos) {
                                *line_start = Some(egui::pos2(-4.0, idx as f32)); // Move source rect handle
                                *initial_bounds = Some(src_rect);
                                *dragging_source_rect = true;
                            }
                        }
                        if left_just_pressed && src_rect.contains(pos) && !*dragging_source_rect {
                            *line_start = Some(pos);
                            *initial_bounds = Some(src_rect);
                            *dragging_source_rect = true;
                        }
                    }
                }

                // Draw selection rect
                painter.rect_stroke(bounds, 0.0, egui::Stroke::new(1.0, egui::Color32::WHITE), egui::StrokeKind::Middle);
                
                // --- Transformation Buttons ---
                let top_btns_rect = egui::Rect::from_min_size(bounds.left_top() - egui::vec2(0.0, 32.0), egui::vec2(200.0, 30.0));
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(top_btns_rect), |ui| {
                    ui.horizontal(|ui| {
                        ui.visuals_mut().widgets.inactive.bg_fill = egui::Color32::from_black_alpha(150);
                        if ui.button("⬌").on_hover_text("Flip Horizontal").clicked() {
                            let center = bounds.center().x;
                            for img in &mut layer.placed_images { img.position.x = center - (img.position.x - center) - (img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0]); img.flipped_h = !img.flipped_h; }
                            for ann in &mut layer.text_annotations { ann.position.x = center - (ann.position.x - center) - (ann.text.len() as f32 * ann.font_size * 0.6); }
                            for s in &mut layer.strokes { for p in &mut s.points { p.x = center - (p.x - center); } }
                        }
                        if ui.button("⬍").on_hover_text("Flip Vertical").clicked() {
                            let center = bounds.center().y;
                            for img in &mut layer.placed_images { img.position.y = center - (img.position.y - center) - (img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[1]); img.flipped_v = !img.flipped_v; }
                            for ann in &mut layer.text_annotations { ann.position.y = center - (ann.position.y - center) - ann.font_size; }
                            for s in &mut layer.strokes { for p in &mut s.points { p.y = center - (p.y - center); } }
                        }
                        
                        // Blur Slider for selected layer
                        let mut has_live = false;
                        let mut blur_val = 0.0;
                        for img in &layer.placed_images { if img.is_live { has_live = true; blur_val = img.blur; break; } }
                        if has_live {
                            if ui.add(egui::DragValue::new(&mut blur_val).range(0.0..=100.0).prefix("B: ")).changed() {
                                for img in &mut layer.placed_images { if img.is_live { img.blur = blur_val; } }
                            }
                        }

                        if ui.add(egui::Button::new(egui::RichText::new("🗑").color(egui::Color32::RED))).clicked() {
                            layer.strokes.clear();
                            layer.text_annotations.clear();
                            layer.placed_images.clear();
                        }
                    });
                });

                // Skew handles (mid-points of sides)
                let mids = [bounds.left_center(), bounds.right_center(), bounds.center_top(), bounds.center_bottom()];
                for m in mids { painter.circle_filled(m, 3.0, egui::Color32::from_rgb(200, 200, 200)); }

                // Transformation Handles
                for corner in corners {
                    painter.rect_filled(egui::Rect::from_center_size(corner, egui::vec2(8.0, 8.0)), 0.0, egui::Color32::WHITE);
                    painter.rect_stroke(egui::Rect::from_center_size(corner, egui::vec2(8.0, 8.0)), 0.0, egui::Stroke::new(1.0, egui::Color32::BLACK), egui::StrokeKind::Middle);
                }
                
                // Rotation handle
                let rot_handle = bounds.center_top() - egui::vec2(0.0, 20.0);
                painter.line_segment([bounds.center_top(), rot_handle], egui::Stroke::new(1.0, egui::Color32::WHITE));
                painter.circle_filled(rot_handle, 4.0, egui::Color32::WHITE);
                painter.circle_stroke(rot_handle, 4.0, egui::Stroke::new(1.0, egui::Color32::BLACK));

                if left_just_pressed && !*dragging_source_rect {
                    let mut hit = false;
                    if rot_handle.distance(pos) < 10.0 {
                        *line_start = Some(egui::pos2(-1.0, -1.0)); // Rotation
                        *initial_center = Some(bounds.center());
                        *initial_layer = Some(layer.clone());
                        hit = true;
                    }
                    if !hit {
                        for (idx, corner) in corners.iter().enumerate() {
                            if egui::Rect::from_center_size(*corner, egui::vec2(12.0, 12.0)).contains(pos) {
                                *line_start = Some(egui::pos2(idx as f32, -1.0)); // Resize
                                *initial_bounds = Some(bounds);
                                *initial_center = Some(bounds.center());
                                *initial_layer = Some(layer.clone());
                                hit = true; break;
                            }
                        }
                    }
                    if !hit {
                        for (idx, &mp) in mids.iter().enumerate() {
                            if egui::Rect::from_center_size(mp, egui::vec2(12.0, 12.0)).contains(pos) {
                                *line_start = Some(egui::pos2(idx as f32, -2.0)); // Skew
                                *initial_center = Some(bounds.center());
                                *initial_layer = Some(layer.clone());
                                hit = true; break;
                            }
                        }
                    }
                    if !hit && bounds.contains(pos) {
                        *line_start = Some(pos); // Move
                        *initial_bounds = Some(bounds);
                        *initial_layer = Some(layer.clone());
                    }
                }
                
                if let Some(start) = *line_start {
                    if left_down {
                        if *dragging_source_rect {
                            // Dragging Mirror Source Rect
                            for img in &mut layer.placed_images {
                                if img.is_live && img.source_rect.is_some() {
                                    if start.x == -4.0 {
                                        // Resize handle
                                        let idx = start.y as usize;
                                        let ib = initial_bounds.unwrap();
                                        let ic = [ib.left_top(), ib.right_top(), ib.left_bottom(), ib.right_bottom()];
                                        let anchor = ic[3 - idx];
                                        let new_rect = egui::Rect::from_two_pos(anchor, pos);
                                        img.source_rect = Some([new_rect.min.x, new_rect.min.y, new_rect.width(), new_rect.height()]);
                                    } else {
                                        // Move whole rect
                                        let delta = pos - start;
                                        let ib = initial_bounds.unwrap();
                                        img.source_rect = Some([ib.min.x + delta.x, ib.min.y + delta.y, ib.width(), ib.height()]);
                                    }
                                }
                            }
                        } else if let Some(base_layer) = initial_layer.as_ref() {
                            // Stable Transformations using initial snapshot
                            *layer = base_layer.clone();
                            if start.x == -1.0 && start.y == -1.0 {
                                // Rotation
                                let center = initial_center.unwrap();
                                let angle = (pos - center).angle() - (start - center).angle();
                                rotate_layer(layer, center, angle);
                            } else if start.y == -1.0 {
                                // Resize
                                let handle_idx = start.x as usize;
                                let ib = initial_bounds.unwrap();
                                let ic = [ib.left_top(), ib.right_top(), ib.left_bottom(), ib.right_bottom()];
                                let anchor = ic[3 - handle_idx];
                                let old_vec = ic[handle_idx] - anchor;
                                let new_vec = pos - anchor;
                                if old_vec.x.abs() > 1.0 && old_vec.y.abs() > 1.0 {
                                    scale_layer(layer, anchor, egui::vec2(new_vec.x / old_vec.x, new_vec.y / old_vec.y));
                                }
                            } else if start.y == -2.0 {
                                // Skew
                                let delta = pos - start;
                                skew_layer(layer, initial_center.unwrap(), delta * 0.01);
                            } else {
                                // Translate
                                translate_layer(layer, pos - start);
                            }
                        }
                    }
                }
            }
            if left_just_released {
                *line_start = None;
                *initial_layer = None;
                *dragging_source_rect = false;
                if layer.strokes.is_empty() && layer.text_annotations.is_empty() && layer.placed_images.is_empty() {
                    if project.layers.len() > 1 { remove_active_layer = true; }
                }
            }
        }
        Tool::Eraser => {
            if left_down {
                let r = 20.0_f32 + settings.pen_width;
                if settings.eraser_mode == EraserMode::Stroke {
                    layer.strokes.retain(|s| !s.points.iter().any(|p| p.distance(pos) < r));
                } else {
                    // Split mode
                    let mut new_strokes = Vec::new();
                    let mut keep_strokes = Vec::new();
                    
                    for s in layer.strokes.drain(..) {
                        if s.kind != StrokeKind::Freehand {
                            if s.points.iter().any(|p| p.distance(pos) < r) { continue; }
                            keep_strokes.push(s);
                            continue;
                        }
                        
                        let mut current_segment = Vec::new();
                        for p in s.points {
                            if p.distance(pos) < r {
                                if current_segment.len() >= 2 {
                                    new_strokes.push(Stroke {
                                        points: current_segment.clone(),
                                        color: s.color, width: s.width, kind: s.kind, brush_mode: s.brush_mode,
                                        background_color: s.background_color,
                                        shadow: s.shadow,
                                        rotation: s.rotation, flipped_h: s.flipped_h, flipped_v: s.flipped_v,
                                        perspective: [egui::Vec2::ZERO; 4],
                                        brush_shape: s.brush_shape,
                                        skew: s.skew,
                                    });
                                }
                                current_segment.clear();
                            } else {
                                current_segment.push(p);
                            }
                        }
                        if current_segment.len() >= 2 {
                            new_strokes.push(Stroke {
                                points: current_segment,
                                color: s.color, width: s.width, kind: s.kind, brush_mode: s.brush_mode,
                                background_color: s.background_color,
                                shadow: s.shadow,
                                rotation: s.rotation, flipped_h: s.flipped_h, flipped_v: s.flipped_v,
                                perspective: [egui::Vec2::ZERO; 4],
                                brush_shape: s.brush_shape,
                                skew: s.skew,
                            });
                        }
                    }
                    layer.strokes.extend(keep_strokes);
                    layer.strokes.extend(new_strokes);
                }
                layer.text_annotations.retain(|ann| ann.position.distance(pos) > r);

                // Pixel erasing for snips
                for img in &mut layer.placed_images {
                    let disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
                    let disp_h = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[1];
                    let img_rect = egui::Rect::from_min_size(img.position, egui::vec2(disp_w, disp_h));
                    
                    if img_rect.expand(r).contains(pos) {
                        let mut modified = false;
                        if img.is_live && img.mask.is_none() {
                            img.mask = Some(vec![255; img.size[0] * img.size[1]]);
                        }
                        
                        for py in 0..img.size[1] {
                            for px in 0..img.size[0] {
                                let local_pos = img.position + egui::vec2(
                                    px as f32 * (disp_w / img.size[0] as f32),
                                    py as f32 * (disp_h / img.size[1] as f32)
                                );
                                if local_pos.distance(pos) < r {
                                    let idx = py * img.size[0] + px;
                                    if img.is_live {
                                        let mask = img.mask.as_mut().unwrap();
                                        if mask[idx] != 0 { mask[idx] = 0; modified = true; }
                                    } else {
                                        let b_idx = idx * 4;
                                        if img.pixels[b_idx + 3] != 0 {
                                            img.pixels[b_idx + 3] = 0;
                                            modified = true;
                                        }
                                    }
                                }
                            }
                        }
                        if modified {
                            img.texture = None; // Force re-upload
                        }
                    }
                }
            }
        }
    }

    if remove_active_layer {
        project.layers.remove(project.active_layer);
        project.active_layer = project.active_layer.saturating_sub(1);
    }

    if snip_created {
        project.layers.push(crate::project::Layer::new(&format!("Snip {}", project.layers.len() + 1)));
        project.active_layer = project.layers.len() - 1;
        let prev_idx = project.active_layer - 1;
        let snip = project.layers[prev_idx].placed_images.pop().unwrap();
        
        let snip_clone = snip.clone();
        std::thread::spawn(move || {
            if let Some(mut pics) = directories::UserDirs::new().and_then(|d| d.picture_dir().map(|p| p.to_path_buf())) {
                pics.push("Owerlayer");
                pics.push("Snips");
                let _ = std::fs::create_dir_all(&pics);
                let time_str = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                let path = pics.join(format!("snip_{}.png", time_str));
                let mut img_buf = image::RgbaImage::new(snip_clone.size[0] as u32, snip_clone.size[1] as u32);
                img_buf.copy_from_slice(&snip_clone.pixels);
                let _ = img_buf.save(path);
            }
        });
        
        project.layers.last_mut().unwrap().placed_images.push(snip);
    }
}

fn draw_arrow(painter: &egui::Painter, from: egui::Pos2, to: egui::Pos2, width: f32, color: egui::Color32) {
    painter.line_segment([from, to], egui::Stroke::new(width, color));
    let dir = (to - from).normalized();
    let perp = egui::vec2(-dir.y, dir.x);
    let head_len = (width * 5.0).max(12.0);
    let p1 = to - dir * head_len + perp * head_len * 0.5;
    let p2 = to - dir * head_len - perp * head_len * 0.5;
    painter.add(egui::Shape::line(vec![p1, to, p2], egui::Stroke::new(width, color)));
}

fn draw_star(painter: &egui::Painter, center: egui::Pos2, radius: f32, color: egui::Color32, bg_color: Option<egui::Color32>, width: f32) {
    let mut points = Vec::new();
    let num_points = 5;
    let outer_radius = radius;
    let inner_radius = radius * 0.4;
    use std::f32::consts::PI;
    for i in 0..num_points * 2 {
        let r = if i % 2 == 0 { outer_radius } else { inner_radius };
        let angle = i as f32 * PI / (num_points as f32) - PI / 2.0;
        points.push(center + egui::vec2(angle.cos() * r, angle.sin() * r));
    }
    
    if let Some(bg) = bg_color {
        let mut mesh = egui::Mesh::default();
        let color_idx = mesh.vertices.len() as u32;
        mesh.vertices.push(egui::epaint::Vertex { pos: center, uv: egui::Pos2::ZERO, color: bg });
        for i in 0..num_points * 2 {
            mesh.vertices.push(egui::epaint::Vertex { pos: points[i as usize], uv: egui::Pos2::ZERO, color: bg });
            let next_idx = (i + 1) % (num_points * 2);
            mesh.indices.push(color_idx);
            mesh.indices.push(color_idx + 1 + i as u32);
            mesh.indices.push(color_idx + 1 + next_idx as u32);
        }
        painter.add(egui::Shape::mesh(mesh));
    }
    
    points.push(points[0]);
    painter.add(egui::Shape::line(points, egui::Stroke::new(width, color)));
}

fn draw_heart(painter: &egui::Painter, center: egui::Pos2, radius: f32, color: egui::Color32, bg_color: Option<egui::Color32>, width: f32) {
    let mut points = Vec::new();
    let steps = 30;
    use std::f32::consts::PI;
    for i in 0..steps {
        let t = i as f32 * 2.0 * PI / steps as f32;
        let x = 16.0 * t.sin().powi(3);
        let y = -(13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos());
        let scale = radius / 16.0;
        points.push(center + egui::vec2(x * scale, y * scale));
    }
    
    if let Some(bg) = bg_color {
        let mut mesh = egui::Mesh::default();
        let color_idx = mesh.vertices.len() as u32;
        mesh.vertices.push(egui::epaint::Vertex { pos: center, uv: egui::Pos2::ZERO, color: bg });
        for i in 0..steps {
            mesh.vertices.push(egui::epaint::Vertex { pos: points[i as usize], uv: egui::Pos2::ZERO, color: bg });
            let next_idx = (i + 1) % steps;
            mesh.indices.push(color_idx);
            mesh.indices.push(color_idx + 1 + i as u32);
            mesh.indices.push(color_idx + 1 + next_idx as u32);
        }
        painter.add(egui::Shape::mesh(mesh));
    }
    
    points.push(points[0]);
    painter.add(egui::Shape::line(points, egui::Stroke::new(width, color)));
}

fn magic_wand_flood_fill(img: &mut PlacedImage, start_x: i32, start_y: i32, target_color: [u8; 4], threshold: f32) {
    let w = img.size[0] as i32;
    let h = img.size[1] as i32;
    let mut stack = vec![(start_x, start_y)];
    let mut visited = vec![false; (w * h) as usize];
    
    let color_diff = |c1: [u8; 4], c2: [u8; 4]| -> f32 {
        let dr = (c1[0] as f32 - c2[0] as f32).abs();
        let dg = (c1[1] as f32 - c2[1] as f32).abs();
        let db = (c1[2] as f32 - c2[2] as f32).abs();
        (dr + dg + db) / 3.0
    };

    while let Some((x, y)) = stack.pop() {
        if x < 0 || x >= w || y < 0 || y >= h { continue; }
        let idx = (y * w + x) as usize;
        if visited[idx] { continue; }
        visited[idx] = true;

        let pixel_idx = idx * 4;
        let current_color = [img.pixels[pixel_idx], img.pixels[pixel_idx+1], img.pixels[pixel_idx+2], img.pixels[pixel_idx+3]];
        
        if current_color[3] > 0 && color_diff(current_color, target_color) <= threshold {
            img.pixels[pixel_idx + 3] = 0; // Erase
            stack.push((x + 1, y));
            stack.push((x - 1, y));
            stack.push((x, y + 1));
            stack.push((x, y - 1));
        }
    }
}

fn transform_mesh(mesh: &mut egui::Mesh, center: egui::Pos2, rotation: f32, skew: egui::Vec2, perspective: [egui::Vec2; 4]) {
    if mesh.vertices.is_empty() { return; }
    let mut min = egui::pos2(f32::MAX, f32::MAX);
    let mut max = egui::pos2(f32::MIN, f32::MIN);
    for v in &mesh.vertices {
        min.x = min.x.min(v.pos.x); min.y = min.y.min(v.pos.y);
        max.x = max.x.max(v.pos.x); max.y = max.y.max(v.pos.y);
    }
    let size = max - min;
    let sin = rotation.sin();
    let cos = rotation.cos();

    for v in &mut mesh.vertices {
        let p = v.pos;
        let tx = if size.x > 0.0 { (p.x - min.x) / size.x } else { 0.5 };
        let ty = if size.y > 0.0 { (p.y - min.y) / size.y } else { 0.5 };
        
        let p_offset = 
            perspective[0] * (1.0 - tx) * (1.0 - ty) + // TL
            perspective[1] * tx * (1.0 - ty) +         // TR
            perspective[2] * (1.0 - tx) * ty +         // BL
            perspective[3] * tx * ty;                  // BR
            
        let rel_p = p - egui::Pos2::ZERO; // p is already relative to the mesh local origin
        let px = rel_p.x + p_offset.x + rel_p.y * skew.x;
        let py = rel_p.y + p_offset.y + rel_p.x * skew.y;
        
        v.pos.x = center.x + px * cos - py * sin;
        v.pos.y = center.y + py * cos + px * sin;
    }
}

fn rotate_layer(layer: &mut crate::project::Layer, center: egui::Pos2, angle: f32) {
    let sin = angle.sin();
    let cos = angle.cos();
    let rot = |p: egui::Pos2| {
        let rel = p - center;
        center + egui::vec2(rel.x * cos - rel.y * sin, rel.y * cos + rel.x * sin)
    };
    for img in &mut layer.placed_images { img.position = rot(img.position); img.rotation += angle; }
    for s in &mut layer.strokes {
        for p in &mut s.points { *p = rot(*p); }
        s.rotation += angle;
    }
    for ann in &mut layer.text_annotations { ann.position = rot(ann.position); ann.rotation += angle; }
}

fn skew_layer(layer: &mut crate::project::Layer, center: egui::Pos2, skew_delta: egui::Vec2) {
    let skew_p = |p: egui::Pos2| {
        let rel = p - center;
        center + egui::vec2(rel.x + rel.y * skew_delta.x, rel.y + rel.x * skew_delta.y)
    };
    for img in &mut layer.placed_images { img.position = skew_p(img.position); img.skew += skew_delta; }
    for s in &mut layer.strokes {
        for p in &mut s.points { *p = skew_p(*p); }
        s.skew += skew_delta;
    }
    for ann in &mut layer.text_annotations { ann.position = skew_p(ann.position); ann.skew += skew_delta; }
}
