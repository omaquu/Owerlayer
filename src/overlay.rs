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
    pub fn poll(prev_down: bool, prev_pos: egui::Pos2, ppp: f32) -> Self {
        let (x_px, y_px, down) = crate::winapi_utils::poll_mouse();
        // window is at [-1, -1] screen pixels. 
        // egui_point = (screen_pixel - window_origin_pixel) / ppp
        let pos = egui::pos2((x_px + 1.0) / ppp, (y_px + 1.0) / ppp);
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
    Blur,
    Embed,
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
            Tool::Mirror => "Mirror Tool",
            Tool::Blur => "Blur Tool",
            Tool::Embed => "Embed Tool",
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
            Tool::Mirror => "M",
            Tool::Blur => "K",
            Tool::Embed => "E",
        }
    }
}



#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BrushMode { Solid, Highlighter, Calligraphy, Spray }

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum MirrorMode { Rect, Circle, Lasso, Poly, Window }
impl Default for MirrorMode { fn default() -> Self { Self::Rect } }

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
    #[serde(default)]
    pub outline: bool,
    #[serde(default = "default_opacity")]
    pub opacity: f32,
    pub outline_color: [u8; 4],
    pub outline_width: f32,
    pub shadow_color: [u8; 4],
    pub shadow_offset: [f32; 2],
    pub shadow_blur: f32,
}

impl Stroke {
    pub fn new(
        points: Vec<egui::Pos2>,
        color: [u8; 4],
        width: f32,
        kind: StrokeKind,
        brush_mode: BrushMode,
        background_color: Option<[u8; 4]>,
        shadow: bool,
        brush_shape: BrushShape,
        outline: bool,
    ) -> Self {
        Self {
            points,
            color,
            width,
            kind,
            brush_mode,
            background_color,
            shadow,
            brush_shape,
            outline,
            opacity: 1.0,
            rotation: 0.0,
            flipped_h: false,
            flipped_v: false,
            perspective: [egui::Vec2::ZERO; 4],
            skew: egui::Vec2::ZERO,
            outline_color: [0, 0, 0, 0],
            outline_width: 0.0,
            shadow_color: [0, 0, 0, 0],
            shadow_offset: [0.0, 0.0],
            shadow_blur: 0.0,
        }
    }
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
    #[serde(default = "default_opacity")]
    pub opacity: f32,
}

impl TextAnnotation {
    pub fn new(position: egui::Pos2, text: String, color: [u8; 4], font_size: f32) -> Self {
        Self {
            position, text, color, font_size,
            monospace: false, shadow: false, outline: false,
            stroke_width: 1.0,
            rotation: 0.0, flipped_h: false, flipped_v: false,
            perspective: [egui::Vec2::ZERO; 4],
            font: TextFont::Sans,
            skew: egui::Vec2::ZERO,
            opacity: 1.0,
        }
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum TextFont { Sans, Serif, Mono, Handwriting, Heading, Custom }

impl Default for TextFont { fn default() -> Self { Self::Sans } }

#[derive(Serialize, Deserialize)]
pub struct PlacedImage {
    pub id: usize,
    pub position: egui::Pos2,
    pub size: [usize; 2],
    pub display_size: Option<[f32; 2]>,
    pub pixels: Vec<u8>,
    #[serde(skip)]
    pub texture: Option<egui::TextureHandle>,
    pub shadow: bool,
    pub rotation: f32,
    pub flipped_h: bool,
    pub flipped_v: bool,
    #[serde(default)]
    pub perspective: [egui::Vec2; 4],
    #[serde(default)]
    pub skew: egui::Vec2,
    #[serde(default = "default_opacity")]
    pub opacity: f32,
    pub is_live: bool,
    pub source_rect: Option<[f32; 4]>,
    pub url: Option<String>,
    #[serde(default)]
    pub blur: f32,
    #[serde(default)]
    pub blur_effect: BlurEffect,
    #[serde(skip)]
    pub mask: Option<Vec<u8>>,
    pub show_source_rect: bool,
    #[serde(skip)]
    pub frames: Vec<Vec<u8>>,
    #[serde(skip)]
    pub frame_durations: Vec<f32>,
    #[serde(skip)]
    pub current_frame: usize,
    #[serde(skip)]
    pub last_frame_time: f32,
    pub hwnd: usize,
    #[cfg(feature = "webengine")]
    #[serde(skip)]
    pub web_widget: Option<crate::web_engine::WebWidget>,
}

impl Clone for PlacedImage {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            position: self.position,
            size: self.size,
            display_size: self.display_size,
            pixels: self.pixels.clone(),
            texture: None,
            shadow: self.shadow,
            rotation: self.rotation,
            flipped_h: self.flipped_h,
            flipped_v: self.flipped_v,
            perspective: self.perspective,
            skew: self.skew,
            opacity: self.opacity,
            is_live: self.is_live,
            source_rect: self.source_rect,
            url: self.url.clone(),
            blur: self.blur,
            blur_effect: self.blur_effect,
            mask: self.mask.clone(),
            show_source_rect: self.show_source_rect,
            frames: self.frames.clone(),
            frame_durations: self.frame_durations.clone(),
            current_frame: self.current_frame,
            last_frame_time: self.last_frame_time,
            hwnd: self.hwnd,
            #[cfg(feature = "webengine")]
            web_widget: None,
        }
    }
}

impl PlacedImage {
    pub fn new(id: usize, position: egui::Pos2, size: [usize; 2], pixels: Vec<u8>) -> Self {
        Self {
            id, position, size, pixels,
            display_size: Some([size[0] as f32, size[1] as f32]),
            texture: None, shadow: false, rotation: 0.0,
            flipped_h: false, flipped_v: false,
            perspective: [egui::Vec2::ZERO; 4],
            skew: egui::Vec2::ZERO,
            opacity: 1.0,
            is_live: false,
            source_rect: None,
            url: None,
            blur: 0.0,
            blur_effect: BlurEffect::Gaussian,
            mask: None,
            show_source_rect: false,
            frames: Vec::new(),
            frame_durations: Vec::new(),
            current_frame: 0,
            last_frame_time: 0.0,
            hwnd: 0,
            #[cfg(feature = "webengine")]
            web_widget: None,
        }
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SnipMode { Rect, Circle, Lasso, Polygon }

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
fn default_opacity() -> f32 { 1.0 }

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
    #[serde(default)]
    pub snip_live: bool,
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
    #[serde(default)]
    pub brush_outline: bool,
    #[serde(default)]
    pub multi_monitor: bool,
    #[serde(default)]
    pub experimental_features: bool,
    #[serde(default)]
    pub pin_layers_panel: bool,
    #[serde(default)]
    pub blur_effect: BlurEffect,
    #[serde(default)]
    pub custom_font_name: String,
    #[serde(skip)]
    pub font_search_query: String,
    #[serde(default)]
    pub window_capture_live: bool,
    #[serde(default)]
    pub use_absolute_screen_coords: bool,
    #[serde(skip)]
    pub picking_fill_color: bool,
    #[serde(skip)]
    pub picking_stroke_color: bool,
    #[serde(skip)]
    pub picking_window: bool,
    #[serde(default)]
    pub mirror_mode: MirrorMode,
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum BlurEffect { Gaussian, Pixelate, Glitch }

impl Default for BlurEffect { fn default() -> Self { Self::Gaussian } }

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
            snip_live: false,
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
            brush_outline: false,
            multi_monitor: true,
            experimental_features: false,
            pin_layers_panel: false,
            blur_effect: BlurEffect::Gaussian,
            custom_font_name: String::new(),
            font_search_query: String::new(),
            window_capture_live: false,
            use_absolute_screen_coords: false,
            picking_fill_color: false,
            picking_stroke_color: false,
            picking_window: false,
            mirror_mode: MirrorMode::default(),
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
    let radius = radius.min(64).min(width / 2).min(height / 2);
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

fn apply_pixelate(pixels: &mut [u8], width: usize, height: usize, scale: usize) {
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

fn apply_vhs_glitch(pixels: &mut [u8], width: usize, height: usize, intensity: f32) {
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
    project: &mut crate::project::Project,
    embed_url: &mut String,
    embed_trigger: &mut bool,
) {
    let main_tools = vec![
        Tool::Move, Tool::Brush, Tool::Eraser, Tool::Text, Tool::Shape, Tool::Snip, Tool::Cut, Tool::Blur, Tool::Embed,
    ];
    
    let hide_icon = if settings.hide_all { "👁" } else { "👓" };
    
    let window_id = egui::Id::new("photoshop_panel");
    let window_rect = ctx.memory(|mem| mem.area_rect(window_id)).unwrap_or(egui::Rect::from_min_size(egui::pos2(20.0, 50.0), egui::vec2(40.0, 40.0)));
    
    let last_pos = ctx.data(|d| d.get_temp::<egui::Pos2>(egui::Id::new("toolbar_last_pos")).unwrap_or(window_rect.min));
    let is_dragged = window_rect.min != last_pos && ctx.input(|i| i.pointer.primary_down());
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("toolbar_last_pos"), window_rect.min));
    
    let mut is_vertical = ctx.data(|d| d.get_temp::<bool>(egui::Id::new("toolbar_is_vertical")).unwrap_or(false));
    
    if is_dragged {
        if let Some(mouse_pos) = ctx.input(|i| i.pointer.hover_pos()) {
            let ppp = ctx.pixels_per_point();
            let (v_x, _) = crate::winapi_utils::get_virtual_origin();
            let mouse_x_phys = mouse_pos.x * ppp + v_x;
            
            #[cfg(windows)]
            {
                let monitor_rects = crate::winapi_utils::get_monitor_rects();
                let mut snapped = false;
                for m in monitor_rects {
                    // Snap to ANY vertical edge of ANY monitor
                    if (mouse_x_phys - m.left as f32).abs() < 50.0 || (mouse_x_phys - m.right as f32).abs() < 50.0 {
                        is_vertical = true;
                        snapped = true;
                        break;
                    }
                }
                if !snapped {
                    is_vertical = false;
                }
            }
            ctx.data_mut(|d| d.insert_temp(egui::Id::new("toolbar_is_vertical"), is_vertical));
        }
    }
    
    let frame = photoshop_frame(settings);

    let mut win = egui::Window::new("photoshop_panel")
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .movable(true)
        .constrain(false)
        .default_pos(egui::pos2(20.0, 50.0))
        .frame(frame);
    if is_vertical { win = win.min_width(220.0); } // Wider for two columns
    
    win.show(ctx, |ui| {
        if is_vertical {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(40.0);
                    if ui.add(egui::Button::new(hide_icon).min_size(egui::vec2(28.0, 24.0))).on_hover_text("Hide/Show All Drawings").clicked() {
                        settings.hide_all = !settings.hide_all;
                    }
                    ui.separator();
                    for tool in &main_tools {
                        let is_selected = *active_tool == *tool;
                        if tool_btn_custom(ui, *tool, is_selected).clicked() { *active_tool = *tool; }
                    }
                    ui.separator();
                    if ui.add(egui::Button::new("📁").min_size(egui::vec2(28.0, 24.0))).on_hover_text("Layers").clicked() { *show_layers_panel = !*show_layers_panel; }
                    if ui.add(egui::Button::new("↔").min_size(egui::vec2(28.0, 24.0))).on_hover_text("Toggle Vertical/Horizontal").clicked() {
                        is_vertical = !is_vertical;
                        ctx.data_mut(|d| d.insert_temp(egui::Id::new("toolbar_is_vertical"), is_vertical));
                    }
                    if ui.add(egui::Button::new("⚙").min_size(egui::vec2(28.0, 24.0))).on_hover_text("Settings").clicked() { *show_settings_panel = !*show_settings_panel; }
                    if ui.add(egui::Button::new(egui::RichText::new("✖").color(egui::Color32::from_rgb(180, 50, 50))).min_size(egui::vec2(28.0, 24.0))).on_hover_text("Exit").clicked() { *show_exit_dialog = true; }
                });

                ui.add(egui::Separator::default().vertical());

                ui.vertical(|ui| {
                    ui.set_width(140.0);
                    render_tool_options(ui, active_tool, settings, project, true, embed_url, embed_trigger);
                });
            });
        } else {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new(hide_icon).min_size(egui::vec2(28.0, 24.0))).on_hover_text("Hide/Show All Drawings").clicked() {
                        settings.hide_all = !settings.hide_all;
                    }
                    ui.separator();
                    for tool in &main_tools {
                        let is_selected = *active_tool == *tool;
                        if tool_btn_custom(ui, *tool, is_selected).clicked() { *active_tool = *tool; }
                    }
                    ui.separator();
                    if ui.add(egui::Button::new("📁").min_size(egui::vec2(28.0, 24.0))).on_hover_text("Layers Panel").clicked() { *show_layers_panel = !*show_layers_panel; }
                    if ui.add(egui::Button::new("↕").min_size(egui::vec2(28.0, 24.0))).on_hover_text("Toggle Vertical/Horizontal").clicked() {
                        is_vertical = !is_vertical;
                        ctx.data_mut(|d| d.insert_temp(egui::Id::new("toolbar_is_vertical"), is_vertical));
                    }
                    if ui.add(egui::Button::new("📁+").min_size(egui::vec2(28.0, 24.0))).on_hover_text("New Layer").clicked() {
                        project.layers.push(crate::project::Layer::new(&format!("Layer {}", project.layers.len() + 1)));
                        project.active_layer = project.layers.len() - 1;
                    }
                    if ui.add(egui::Button::new("⚙").min_size(egui::vec2(28.0, 24.0))).clicked() { *show_settings_panel = !*show_settings_panel; }
                    if ui.add(egui::Button::new(egui::RichText::new("✖").color(egui::Color32::from_rgb(180, 50, 50))).min_size(egui::vec2(28.0, 24.0))).clicked() { *show_exit_dialog = true; }
                });
                
                // Hide second row for tools with no options
                let has_options = match active_tool {
                    Tool::Move => project.selected_object.is_some(),
                    Tool::Mirror => true,
                    _ => true,
                };
                if has_options {
                    ui.separator();
                    ui.horizontal(|ui| {
                        render_tool_options(ui, active_tool, settings, project, false, embed_url, embed_trigger);
                    });
                }
            });
        }
    });
}

fn render_tool_options(ui: &mut egui::Ui, active_tool: &mut Tool, settings: &mut Settings, project: &mut crate::project::Project, is_vertical: bool, embed_url: &mut String, embed_trigger: &mut bool) {
    ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
    
    if !matches!(active_tool, Tool::Move | Tool::Mirror | Tool::Embed) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    let mut fg = color32(&settings.pen_color);
                    if ui.color_edit_button_srgba(&mut fg).on_hover_text("Pen Color").changed() { settings.pen_color = [fg.r(), fg.g(), fg.b(), fg.a()]; }
                    if ui.button("▨").on_hover_text("Pick Screen Color (Pen)").clicked() { settings.picking_fill_color = false; settings.picking_stroke_color = true; }
                });
                ui.horizontal(|ui| {
                    let mut bg = color32(&settings.background_color);
                    if ui.color_edit_button_srgba(&mut bg).on_hover_text("Fill Color").changed() { settings.background_color = [bg.r(), bg.g(), bg.b(), bg.a()]; }
                    if ui.button("▨").on_hover_text("Pick Screen Color (Fill)").clicked() { settings.picking_stroke_color = false; settings.picking_fill_color = true; }
                });
            });
        });
        ui.add(egui::Separator::default().vertical());
    }

    match active_tool {
        Tool::Brush | Tool::Eraser => {
            ui.add(egui::DragValue::new(&mut settings.pen_width).range(1.0..=100.0));
            
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
            
            // Font search box
            let search_resp = ui.add(
                egui::TextEdit::singleline(&mut settings.font_search_query)
                    .hint_text("🔍 Font...")
                    .desired_width(if is_vertical { 90.0 } else { 80.0 })
            );
            
            // Show dropdown when search has text
            if !settings.font_search_query.is_empty() {
                let query = settings.font_search_query.to_lowercase();
                let all_fonts = [
                    "Arial", "Arial Black", "Comic Sans MS", "Courier New",
                    "Georgia", "Impact", "Lucida Console", "Lucida Sans Unicode",
                    "Microsoft Sans Serif", "Palatino Linotype", "Tahoma",
                    "Times New Roman", "Trebuchet MS", "Verdana",
                    "Calibri", "Cambria", "Candara", "Consolas", "Constantia",
                    "Corbel", "Franklin Gothic Medium", "Gill Sans MT",
                    "Segoe UI", "Segoe Print", "Segoe Script",
                    "Book Antiqua", "Bookman Old Style", "Century Gothic",
                    "Garamond", "Rockwell",
                ];
                let matches: Vec<&str> = all_fonts.iter().copied()
                    .filter(|f| f.to_lowercase().contains(&query))
                    .take(8)
                    .collect();
                    
                if !matches.is_empty() {
                    egui::popup::popup_below_widget(ui, search_resp.id, &search_resp, egui::PopupCloseBehavior::CloseOnClickOutside, |ui| {
                        ui.set_min_width(120.0);
                        for font_name in matches {
                            if ui.selectable_label(settings.custom_font_name == font_name, font_name).clicked() {
                                settings.custom_font_name = font_name.to_string();
                                settings.text_font = TextFont::Custom;
                                settings.font_search_query.clear();
                                ui.memory_mut(|m| m.close_popup());
                            }
                        }
                    });
                    ui.memory_mut(|m| m.open_popup(search_resp.id));
                }
            }
            
            // Current font label when Custom is selected
            if settings.text_font == TextFont::Custom && !settings.custom_font_name.is_empty() {
                ui.label(egui::RichText::new(&settings.custom_font_name).size(10.0).color(egui::Color32::from_gray(180)));
            }
            
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
            ui.horizontal(|ui| {
                ui.toggle_value(&mut settings.text_shadow, " S ");
                ui.toggle_value(&mut settings.text_outline, " O ");
            });
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
            ui.separator();
        }
        Tool::Snip => {
            if is_vertical {
                ui.vertical(|ui| {
                    ui.selectable_value(&mut settings.snip_mode, SnipMode::Rect, "Rect");
                    ui.selectable_value(&mut settings.snip_mode, SnipMode::Circle, "Circ");
                    ui.selectable_value(&mut settings.snip_mode, SnipMode::Lasso, "Lasso");
                    ui.selectable_value(&mut settings.snip_mode, SnipMode::Polygon, "Poly");
                });
                ui.separator();
                ui.vertical(|ui| {
                    ui.selectable_value(&mut settings.snip_live, false, "📷 Snap");
                    ui.selectable_value(&mut settings.snip_live, true, "📡 Live");
                });
            } else {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut settings.snip_mode, SnipMode::Rect, "Rect");
                    ui.selectable_value(&mut settings.snip_mode, SnipMode::Circle, "Circ");
                    ui.selectable_value(&mut settings.snip_mode, SnipMode::Lasso, "Lasso");
                    ui.selectable_value(&mut settings.snip_mode, SnipMode::Polygon, "Poly");
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut settings.snip_live, false, "📷 Snap");
                    ui.selectable_value(&mut settings.snip_live, true, "📡 Live");
                });
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
            ui.toggle_value(&mut settings.inverted_cut, "Inv.");
            if settings.cut_mode == CutMode::MagicWand {
                ui.add(egui::DragValue::new(&mut settings.magic_wand_threshold).range(0.0..=100.0).prefix("Wand: "));
            }
        }
        Tool::Mirror => {
            if is_vertical {
                ui.vertical(|ui| {
                    ui.selectable_value(&mut settings.mirror_mode, MirrorMode::Rect, "Rect");
                    ui.selectable_value(&mut settings.mirror_mode, MirrorMode::Circle, "Circ");
                    ui.selectable_value(&mut settings.mirror_mode, MirrorMode::Lasso, "Lasso");
                    ui.selectable_value(&mut settings.mirror_mode, MirrorMode::Poly, "Poly");
                    ui.selectable_value(&mut settings.mirror_mode, MirrorMode::Window, "🖥 Win");
                });
            } else {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut settings.mirror_mode, MirrorMode::Rect, "Rect");
                    ui.selectable_value(&mut settings.mirror_mode, MirrorMode::Circle, "Circ");
                    ui.selectable_value(&mut settings.mirror_mode, MirrorMode::Lasso, "Lasso");
                    ui.selectable_value(&mut settings.mirror_mode, MirrorMode::Poly, "Poly");
                    ui.selectable_value(&mut settings.mirror_mode, MirrorMode::Window, "🖥 Win");
                });
            }
            if settings.mirror_mode == MirrorMode::Window {
                ui.separator();
                ui.label(egui::RichText::new("🖥 Window Capture").size(10.0).color(egui::Color32::from_gray(180)));
                if is_vertical {
                    ui.vertical(|ui| {
                        ui.selectable_value(&mut settings.window_capture_live, false, "📷 Snap");
                        ui.add_enabled_ui(settings.experimental_features, |ui| {
                            let resp = ui.selectable_value(&mut settings.window_capture_live, true, "📡 Live");
                            if !settings.experimental_features {
                                resp.on_hover_text("Requires Experimental Features to be enabled in Settings");
                            }
                        });
                    });
                } else {
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut settings.window_capture_live, false, "📷 Snap");
                        ui.add_enabled_ui(settings.experimental_features, |ui| {
                            let resp = ui.selectable_value(&mut settings.window_capture_live, true, "📡 Live");
                            if !settings.experimental_features {
                                resp.on_hover_text("Requires Experimental Features to be enabled in Settings");
                            }
                        });
                    });
                }
            }
            ui.label("Drag area to mirror");
        }
        Tool::Blur => {
            ui.add(egui::DragValue::new(&mut settings.blur_strength).range(1.0..=100.0).prefix("Strength: "));
            if is_vertical {
                ui.vertical(|ui| {
                    ui.selectable_value(&mut settings.blur_effect, BlurEffect::Gaussian, "Blur");
                    ui.selectable_value(&mut settings.blur_effect, BlurEffect::Pixelate, "Pixel");
                    ui.selectable_value(&mut settings.blur_effect, BlurEffect::Glitch, "VHS");
                });
            } else {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut settings.blur_effect, BlurEffect::Gaussian, "Blur");
                    ui.selectable_value(&mut settings.blur_effect, BlurEffect::Pixelate, "Pixel");
                    ui.selectable_value(&mut settings.blur_effect, BlurEffect::Glitch, "VHS");
                });
            }
            ui.label("Drag to create effect area");
        }
        Tool::Embed => {
            ui.add_enabled_ui(settings.experimental_features, |ui| {
                let btn_size = egui::vec2(28.0, 28.0);
                let resp = ui.add(egui::TextEdit::singleline(embed_url).hint_text("URL or Path").desired_width(if is_vertical { 100.0 } else { 120.0 }));
                if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    *embed_trigger = true;
                }
                if ui.add(egui::Button::new("📋").min_size(btn_size)).on_hover_text("Paste URL").clicked() {
                    if let Some(text) = crate::winapi_utils::get_clipboard_text() {
                        *embed_url = text;
                        *embed_trigger = true;
                    }
                }
                if ui.add(egui::Button::new("🌐").min_size(btn_size)).on_hover_text("Web URL").clicked() {
                    *embed_url = "https://".to_string();
                }
                if ui.add(egui::Button::new("📺").min_size(btn_size)).on_hover_text("YouTube").clicked() {
                    *embed_url = "https://www.youtube.com/watch?v=".to_string();
                }
                if ui.add(egui::Button::new("📁").min_size(btn_size)).on_hover_text("Local File").clicked() {
                    crate::winapi_utils::set_topmost(false);
                    if let Some(path) = rfd::FileDialog::new().add_filter("Media", &["png", "jpg", "jpeg", "gif", "webp"]).pick_file() {
                        *embed_url = path.to_string_lossy().to_string();
                        *embed_trigger = true;
                    }
                    crate::winapi_utils::set_topmost(true);
                }
                if ui.add(egui::Button::new("📄").min_size(btn_size)).on_hover_text("Blank Widget").clicked() {
                    *embed_url = "about:blank".to_string();
                    *embed_trigger = true;
                }
            });
            if !settings.experimental_features {
                ui.label(egui::RichText::new("Requires Experimental Features").size(9.0).color(egui::Color32::GOLD));
            }
        }
        Tool::Move => {
            if let Some(sel) = project.selected_object {
                ui.horizontal(|ui| {
                    if ui.button("🗑").on_hover_text("Delete Selected").clicked() {
                        let layer = &mut project.layers[sel.layer_idx];
                        match sel.object_type {
                            crate::project::ObjectType::Image => { 
                                if sel.object_idx < layer.placed_images.len() { layer.placed_images.remove(sel.object_idx); }
                            }
                            crate::project::ObjectType::Stroke => { 
                                if sel.object_idx < layer.strokes.len() { layer.strokes.remove(sel.object_idx); }
                            }
                            crate::project::ObjectType::Text => { 
                                if sel.object_idx < layer.text_annotations.len() { layer.text_annotations.remove(sel.object_idx); }
                            }
                        }
                        project.selected_object = None;
                    }
                    if ui.button("📁+").on_hover_text("Move to New Layer").clicked() {
                        let layer_idx = sel.layer_idx;
                        let obj_idx = sel.object_idx;
                        let obj_exists = match sel.object_type {
                            crate::project::ObjectType::Image => true,
                            crate::project::ObjectType::Stroke => true,
                            crate::project::ObjectType::Text => true,
                        };
                        
                        if obj_exists {
                            let new_layer = crate::project::Layer::new(&format!("New Layer {}", project.layers.len() + 1));
                            project.layers.push(new_layer);
                            let new_idx = project.layers.len() - 1;
                            
                            // Move logic
                            let new_obj_idx = match sel.object_type {
                                crate::project::ObjectType::Image => {
                                    let img = project.layers[layer_idx].placed_images.remove(obj_idx);
                                    project.layers[new_idx].placed_images.push(img);
                                    project.layers[new_idx].placed_images.len() - 1
                                }
                                crate::project::ObjectType::Stroke => {
                                    let s = project.layers[layer_idx].strokes.remove(obj_idx);
                                    project.layers[new_idx].strokes.push(s);
                                    project.layers[new_idx].strokes.len() - 1
                                }
                                crate::project::ObjectType::Text => {
                                    let t = project.layers[layer_idx].text_annotations.remove(obj_idx);
                                    project.layers[new_idx].text_annotations.push(t);
                                    project.layers[new_idx].text_annotations.len() - 1
                                }
                            };
                            project.active_layer = new_idx;
                            project.selected_object = Some(crate::project::SelectedObject {
                                layer_idx: new_idx,
                                object_type: sel.object_type,
                                object_idx: new_obj_idx,
                            });
                        }
                    }

                    if sel.object_type == crate::project::ObjectType::Image {
                        let layer = &mut project.layers[sel.layer_idx];
                        let img = &mut layer.placed_images[sel.object_idx];
                        ui.separator();
                        ui.selectable_value(&mut img.is_live, true, "Live");
                        ui.selectable_value(&mut img.is_live, false, "Snap");
                        if img.is_live {
                            if ui.button("🔗 Pick").on_hover_text("Capture specific window").clicked() {
                                settings.picking_window = true;
                            }
                        }
                    }

                    // Depth controls
                    ui.separator();
                    if ui.button("⤒").on_hover_text("Bring to Front").clicked() {
                        let layer = &mut project.layers[sel.layer_idx];
                        match sel.object_type {
                            crate::project::ObjectType::Image => { let obj = layer.placed_images.remove(sel.object_idx); layer.placed_images.push(obj); project.selected_object = Some(crate::project::SelectedObject { layer_idx: sel.layer_idx, object_type: sel.object_type, object_idx: layer.placed_images.len() - 1 }); }
                            crate::project::ObjectType::Stroke => { let obj = layer.strokes.remove(sel.object_idx); layer.strokes.push(obj); project.selected_object = Some(crate::project::SelectedObject { layer_idx: sel.layer_idx, object_type: sel.object_type, object_idx: layer.strokes.len() - 1 }); }
                            crate::project::ObjectType::Text => { let obj = layer.text_annotations.remove(sel.object_idx); layer.text_annotations.push(obj); project.selected_object = Some(crate::project::SelectedObject { layer_idx: sel.layer_idx, object_type: sel.object_type, object_idx: layer.text_annotations.len() - 1 }); }
                        }
                    }
                    if ui.button("⤓").on_hover_text("Send to Back").clicked() {
                        let layer = &mut project.layers[sel.layer_idx];
                        match sel.object_type {
                            crate::project::ObjectType::Image => { let obj = layer.placed_images.remove(sel.object_idx); layer.placed_images.insert(0, obj); project.selected_object = Some(crate::project::SelectedObject { layer_idx: sel.layer_idx, object_type: sel.object_type, object_idx: 0 }); }
                            crate::project::ObjectType::Stroke => { let obj = layer.strokes.remove(sel.object_idx); layer.strokes.insert(0, obj); project.selected_object = Some(crate::project::SelectedObject { layer_idx: sel.layer_idx, object_type: sel.object_type, object_idx: 0 }); }
                            crate::project::ObjectType::Text => { let obj = layer.text_annotations.remove(sel.object_idx); layer.text_annotations.insert(0, obj); project.selected_object = Some(crate::project::SelectedObject { layer_idx: sel.layer_idx, object_type: sel.object_type, object_idx: 0 }); }
                        }
                    }
                });
            }
            
            // Object list for active layer
            ui.separator();
            ui.label(egui::RichText::new("Objects:").size(10.0).color(egui::Color32::from_gray(120)));
            let active_idx = project.active_layer;
            if active_idx < project.layers.len() {
                let layer = &mut project.layers[active_idx];
                ui.horizontal(|ui| {
                    let mut to_delete = None;
                    for (i, img) in layer.placed_images.iter().enumerate() {
                        if ui.button(egui::RichText::new("🖼").size(10.0)).on_hover_text(img.url.as_deref().unwrap_or("Image")).clicked() {
                            project.selected_object = Some(crate::project::SelectedObject { layer_idx: active_idx, object_type: crate::project::ObjectType::Image, object_idx: i });
                        }
                        if ui.button(egui::RichText::new("x").color(egui::Color32::RED).size(8.0)).clicked() {
                            to_delete = Some((crate::project::ObjectType::Image, i));
                        }
                    }
                    for (i, ann) in layer.text_annotations.iter().enumerate() {
                        let label = if ann.text.len() > 5 { format!("{}...", &ann.text[..5]) } else { ann.text.clone() };
                        if ui.button(egui::RichText::new(format!("T: {}", label)).size(10.0)).clicked() {
                            project.selected_object = Some(crate::project::SelectedObject { layer_idx: active_idx, object_type: crate::project::ObjectType::Text, object_idx: i });
                        }
                        if ui.button(egui::RichText::new("x").color(egui::Color32::RED).size(8.0)).clicked() {
                            to_delete = Some((crate::project::ObjectType::Text, i));
                        }
                    }
                    if let Some((kind, idx)) = to_delete {
                        match kind {
                            crate::project::ObjectType::Image => { layer.placed_images.remove(idx); }
                            crate::project::ObjectType::Text => { layer.text_annotations.remove(idx); }
                            _ => {}
                        }
                        project.selected_object = None;
                    }
                });
            }
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
) {
    render_photoshop_panel(ctx, active_tool, settings, show_settings_panel, show_layers_panel, show_exit_dialog, project, embed_url, embed_trigger);
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
    owl_icon: &Option<egui::TextureHandle>,
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
            ui.checkbox(&mut settings.hide_edit_info, "Hide Edit Mode Info Text");
            if settings.experimental_features {
                ui.label("Warning: Web embeds may degrade performance.");
            }
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
            section_heading(ui, "Compatibility & Experimental", accent);
            ui.checkbox(&mut settings.software_rendering, "Use Software Rendering (Requires Restart)");
            ui.label(egui::RichText::new("Use this if you experience flickering or transparency issues on some GPUs.").size(10.0).color(egui::Color32::GRAY));

            ui.add_space(4.0);
            ui.checkbox(&mut settings.multi_monitor, "Multi-Monitor Mode (Requires Restart)");
            ui.label(egui::RichText::new("Enables drawing and snipping across all monitors.").size(10.0).color(egui::Color32::GRAY));

            ui.add_space(4.0);
            ui.checkbox(&mut settings.experimental_features, "Enable Experimental Features");
            ui.label(egui::RichText::new("Enables live webpage embedding and advanced effects.").size(10.0).color(egui::Color32::GOLD));
            
            ui.add_space(4.0);
            ui.checkbox(&mut settings.use_absolute_screen_coords, "Use Absolute Screen Coords");
            ui.label(egui::RichText::new("Fixes OBS capture offset on multi-monitor setups.").size(10.0).color(egui::Color32::GRAY));

            // ── Accent color ──
            ui.add_space(8.0);
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
            ui.add(egui::Separator::default().spacing(6.0));

            // ── About ──
            section_heading(ui, "About", accent);
            ui.horizontal(|ui| {
                if let Some(tex) = owl_icon {
                    ui.add(egui::Image::new(tex).max_width(32.0).max_height(32.0));
                }
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("Owerlayer").strong().size(14.0));
                    ui.label(egui::RichText::new("v0.4.2").size(11.0).color(egui::Color32::GRAY));
                    ui.label(egui::RichText::new("by omaquu").size(11.0).color(egui::Color32::GRAY));
                });
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
    settings: &mut Settings,
    open: &mut bool,
    filters_open: &mut Option<usize>,
) {
    let frame = photoshop_frame(settings);

    egui::Window::new(egui::RichText::new("Layers").color(egui::Color32::from_rgb(180, 180, 200)).size(16.0))
        .open(open)
        .title_bar(false)
        .resizable(true)
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
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("➕").on_hover_text("New Layer").clicked() {
                        project.layers.push(crate::project::Layer::new(&format!("Layer {}", project.layers.len() + 1)));
                        project.active_layer = project.layers.len() - 1;
                    }
                });
            });

            ui.add_space(8.0);

            egui::ScrollArea::vertical().show(ui, |ui: &mut egui::Ui| {
                let mut layer_to_remove = None;
                let mut layer_to_move_up = None;
                let mut layer_to_move_down = None;
                let mut object_to_delete = None;
                let mut object_to_select = None;
                let total_layers = project.layers.len();

                for i in (0..total_layers).rev() {
                    let is_active = project.active_layer == i;
                    let layer = &mut project.layers[i];
                    
                    let bg_color = if is_active {
                        egui::Color32::from_rgba_premultiplied(60, 120, 200, 100)
                    } else {
                        egui::Color32::TRANSPARENT
                    };

                    let row_frame = egui::Frame::default().fill(bg_color).inner_margin(egui::Margin::symmetric(4, 2)).corner_radius(egui::CornerRadius::same(4));
                    
                    row_frame.show(ui, |ui: &mut egui::Ui| {
                        // Main layer row
                        ui.horizontal(|ui: &mut egui::Ui| {
                            // Drag handle for reordering
                            let drag_handle = ui.add(egui::Label::new("\u{2261}").sense(egui::Sense::drag()));
                            if drag_handle.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::Grab); }
                            
                            if drag_handle.dragged() { 
                                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                                // Visual effect for dragging: show a ghost row following mouse
                                if let Some(mouse_pos) = ui.ctx().pointer_interact_pos() {
                                    let mut ghost_rect = drag_handle.rect;
                                    ghost_rect.set_center(mouse_pos);
                                    ui.painter().rect_filled(ghost_rect.expand(20.0), 4.0, egui::Color32::from_rgba_premultiplied(100, 100, 100, 150));
                                }
                            }
                            if drag_handle.drag_stopped() {
                                if let Some(pos) = ui.ctx().pointer_interact_pos() {
                                    let threshold = drag_handle.rect.height() * 0.7;
                                    if drag_handle.rect.top() > 0.0 && pos.y < drag_handle.rect.top() - threshold && i < total_layers - 1 {
                                        layer_to_move_up = Some(i);
                                    } else if drag_handle.rect.bottom() > 0.0 && pos.y > drag_handle.rect.bottom() + threshold && i > 0 {
                                        layer_to_move_down = Some(i);
                                    }
                                }
                            }
                            
                            // Visual hint for active/dragging
                            if is_active && !drag_handle.dragged() {
                                ui.painter().rect_filled(drag_handle.rect.expand(2.0), 2.0, egui::Color32::from_rgba_premultiplied(200, 200, 200, 40));
                            }

                            ui.checkbox(&mut layer.visible, "");
                            
                            // Layer Preview Thumbnail
                            let (thumb_rect, _) = ui.allocate_exact_size(egui::vec2(24.0, 18.0), egui::Sense::hover());
                            ui.painter().rect_filled(thumb_rect, 2.0, egui::Color32::from_gray(40));
                            ui.painter().rect_stroke(thumb_rect, 2.0, egui::Stroke::new(1.0, egui::Color32::from_gray(60)), egui::StrokeKind::Middle);
                            
                            if let Some(img) = layer.placed_images.first() {
                                if let Some(tex) = &img.texture {
                                    ui.painter().image(tex.id(), thumb_rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                                }
                            } else if !layer.strokes.is_empty() {
                                // Draw a tiny squiggly line as a stroke preview
                                let c = thumb_rect.center();
                                ui.painter().line_segment([c - egui::vec2(5.0, 2.0), c + egui::vec2(5.0, 2.0)], egui::Stroke::new(1.0, egui::Color32::WHITE));
                            }
                            
                            // Layer name — clicking it selects the layer
                            if ui.selectable_label(is_active, &layer.name).clicked() {
                                project.active_layer = i;
                            }
                            
                            let mut opacity_percent = (layer.opacity * 100.0) as i32;
                            if ui.add(egui::DragValue::new(&mut opacity_percent).range(0..=100).suffix("%")).changed() {
                                layer.opacity = opacity_percent as f32 / 100.0;
                            }

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui: &mut egui::Ui| {
                                if ui.add(egui::Button::new(egui::RichText::new("🗑").color(egui::Color32::from_rgb(255, 80, 80))).frame(false)).clicked() { 
                                    layer_to_remove = Some(i); 
                                }
                                if ui.button("fx").clicked() { *filters_open = Some(i); }
                            });
                        });

                        // Child objects — auto-shown when this is the active layer
                        if is_active {
                            let obj_count = layer.placed_images.len() + layer.text_annotations.len() + layer.strokes.len();
                            if obj_count > 0 {
                                ui.indent(format!("layer_obj_{}", i), |ui: &mut egui::Ui| {
                                    for (img_idx, img) in layer.placed_images.iter().enumerate() {
                                        ui.horizontal(|ui: &mut egui::Ui| {
                                            let is_sel = project.selected_object == Some(crate::project::SelectedObject { layer_idx: i, object_type: crate::project::ObjectType::Image, object_idx: img_idx });
                                            let label = if img.blur > 0.0 {
                                                format!("🔲 Blur {}", img_idx)
                                            } else if img.url.is_some() {
                                                let url = img.url.as_ref().unwrap();
                                                format!("🌐 {}", if url.len() > 15 { &url[..12] } else { url })
                                            } else if img.is_live && img.source_rect.is_some() {
                                                format!("🪞 Mirror {}", img_idx)
                                            } else if img.is_live {
                                                "🖼 Live".to_string()
                                            } else {
                                                format!("🖼 Image {}", img_idx)
                                            };
                                            if ui.selectable_label(is_sel, label).clicked() {
                                                object_to_select = Some((i, crate::project::ObjectType::Image, img_idx));
                                            }
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui: &mut egui::Ui| {
                                                if ui.small_button("🗑").clicked() { object_to_delete = Some((i, crate::project::ObjectType::Image, img_idx)); }
                                            });
                                        });
                                    }
                                    for (t_idx, ann) in layer.text_annotations.iter().enumerate() {
                                        ui.horizontal(|ui: &mut egui::Ui| {
                                            let is_sel = project.selected_object == Some(crate::project::SelectedObject { layer_idx: i, object_type: crate::project::ObjectType::Text, object_idx: t_idx });
                                            let name = format!("T \"{}\"", if ann.text.len() > 12 { &ann.text[..10] } else { &ann.text });
                                            if ui.selectable_label(is_sel, name).clicked() {
                                                object_to_select = Some((i, crate::project::ObjectType::Text, t_idx));
                                            }
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui: &mut egui::Ui| {
                                                if ui.small_button("🗑").clicked() { object_to_delete = Some((i, crate::project::ObjectType::Text, t_idx)); }
                                            });
                                        });
                                    }
                                    let mut freehand_indices = Vec::new();
                                    for (s_idx, s) in layer.strokes.iter().enumerate() {
                                        if s.kind == crate::overlay::StrokeKind::Freehand {
                                            freehand_indices.push(s_idx);
                                            continue;
                                        }
                                        ui.horizontal(|ui: &mut egui::Ui| {
                                            let is_sel = project.selected_object == Some(crate::project::SelectedObject { layer_idx: i, object_type: crate::project::ObjectType::Stroke, object_idx: s_idx });
                                            let s_name = match s.kind {
                                                crate::overlay::StrokeKind::Rect => "✏ Rect",
                                                crate::overlay::StrokeKind::Circle => "✏ Circle",
                                                crate::overlay::StrokeKind::Blur => "✏ Blur",
                                                _ => "✏ Stroke",
                                            };
                                            if ui.selectable_label(is_sel, format!("{} {}", s_name, s_idx)).clicked() {
                                                object_to_select = Some((i, crate::project::ObjectType::Stroke, s_idx));
                                            }
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui: &mut egui::Ui| {
                                                if ui.small_button("🗑").clicked() { object_to_delete = Some((i, crate::project::ObjectType::Stroke, s_idx)); }
                                            });
                                        });
                                    }
                                    if !freehand_indices.is_empty() {
                                        ui.horizontal(|ui: &mut egui::Ui| {
                                            let is_sel = project.selected_object.map_or(false, |sel| sel.layer_idx == i && sel.object_type == crate::project::ObjectType::Stroke && freehand_indices.contains(&sel.object_idx));
                                            if ui.selectable_label(is_sel, "🖌 Brush Strokes").clicked() {
                                                object_to_select = Some((i, crate::project::ObjectType::Stroke, freehand_indices[0]));
                                            }
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui: &mut egui::Ui| {
                                                if ui.small_button("🗑").clicked() {
                                                    // We use a special marker index to indicate deleting all freehand strokes
                                                    object_to_delete = Some((i, crate::project::ObjectType::Stroke, usize::MAX));
                                                }
                                            });
                                        });
                                    }
                                });
                            }
                        }
                    });
                }

                if let Some(idx) = layer_to_remove {
                    project.layers.remove(idx);
                    project.selected_object = None;
                    if project.layers.is_empty() { project.layers.push(crate::project::Layer::new("Layer 1")); }
                    project.active_layer = project.active_layer.min(project.layers.len() - 1);
                }
                if let Some(idx) = layer_to_move_up {
                    if idx < total_layers - 1 {
                        project.layers.swap(idx, idx + 1);
                        if project.active_layer == idx { project.active_layer = idx + 1; }
                        else if project.active_layer == idx + 1 { project.active_layer = idx; }
                    }
                }
                if let Some(idx) = layer_to_move_down {
                    if idx > 0 {
                        project.layers.swap(idx, idx - 1);
                        if project.active_layer == idx { project.active_layer = idx - 1; }
                        else if project.active_layer == idx - 1 { project.active_layer = idx; }
                    }
                }
                if let Some((l_idx, obj_type, o_idx)) = object_to_delete {
                    match obj_type {
                        crate::project::ObjectType::Stroke => {
                            if o_idx == usize::MAX {
                                project.layers[l_idx].strokes.retain(|s| s.kind != crate::overlay::StrokeKind::Freehand);
                            } else {
                                project.layers[l_idx].strokes.remove(o_idx);
                            }
                        }
                        crate::project::ObjectType::Text => { project.layers[l_idx].text_annotations.remove(o_idx); }
                        crate::project::ObjectType::Image => { project.layers[l_idx].placed_images.remove(o_idx); }
                    }
                    project.selected_object = None;
                }
                if let Some((l_idx, obj_type, o_idx)) = object_to_select {
                    project.active_layer = l_idx;
                    project.selected_object = Some(crate::project::SelectedObject { layer_idx: l_idx, object_type: obj_type, object_idx: o_idx });
                }
            });
        });
}

pub fn render_filter_menu(
    ctx: &egui::Context,
    project: &mut crate::project::Project,
    settings: &Settings,
    filters_open: &mut Option<usize>,
) {
    if let Some(idx) = *filters_open {
        if idx >= project.layers.len() {
            *filters_open = None;
            return;
        }

        let accent = color32(&settings.accent_color);
        let frame = photoshop_frame(settings);
        let layer_name = project.layers[idx].name.clone();

        egui::Window::new(format!("Layer Filters: {}", layer_name))
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .frame(frame)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Layer Filters").size(11.0).color(accent));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("✖").clicked() { *filters_open = None; }
                    });
                });
                ui.separator();
                
                let layer = &mut project.layers[idx];

                section_heading(ui, "Shadow / Glow", accent);
                ui.checkbox(&mut layer.shadow, "Enable Drop Shadow");
                ui.horizontal(|ui| {
                    ui.label("Distance:");
                    ui.add(egui::DragValue::new(&mut layer.shadow_offset[0]).speed(0.1).prefix("X:"));
                    ui.add(egui::DragValue::new(&mut layer.shadow_offset[1]).speed(0.1).prefix("Y:"));
                });
                ui.horizontal(|ui| {
                    ui.label("Color:");
                    let mut c = egui::Color32::from_rgba_unmultiplied(layer.shadow_color[0], layer.shadow_color[1], layer.shadow_color[2], layer.shadow_color[3]);
                    if ui.color_edit_button_srgba(&mut c).changed() {
                        layer.shadow_color = [c.r(), c.g(), c.b(), c.a()];
                    }
                });

                ui.add_space(8.0);
                section_heading(ui, "Outline / Stroke", accent);
                ui.checkbox(&mut layer.outline, "Enable Outline");
                ui.horizontal(|ui| {
                    ui.label("Thickness:");
                    ui.add(egui::Slider::new(&mut layer.outline_width, 0.5..=20.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Color:");
                    let mut c = egui::Color32::from_rgba_unmultiplied(layer.outline_color[0], layer.outline_color[1], layer.outline_color[2], layer.outline_color[3]);
                    if ui.color_edit_button_srgba(&mut c).changed() {
                        layer.outline_color = [c.r(), c.g(), c.b(), c.a()];
                    }
                });

                ui.add_space(8.0);
                section_heading(ui, "Opacity & Visibility", accent);
                ui.horizontal(|ui| {
                    ui.label("Opacity:");
                    let mut op = layer.opacity * 100.0;
                    if ui.add(egui::Slider::new(&mut op, 0.0..=100.0).suffix("%")).changed() {
                        layer.opacity = op / 100.0;
                    }
                });
                ui.checkbox(&mut layer.visible, "Visible");
                ui.add_space(8.0);

                if ui.button("Close").clicked() {
                    *filters_open = None;
                }
            });
        
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            *filters_open = None;
        }
    }
}

fn section_heading(ui: &mut egui::Ui, text: &str, accent: egui::Color32) {
    ui.add_space(4.0);
    ui.label(egui::RichText::new(text).size(14.0).strong().color(accent));
    ui.add_space(2.0);
}

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
        match s.kind {
            crate::overlay::StrokeKind::Circle | crate::overlay::StrokeKind::Star | crate::overlay::StrokeKind::Heart => {
                if s.points.len() >= 2 {
                    let center = s.points[0];
                    let radius = center.distance(s.points[1]);
                    extend(center - egui::vec2(radius, radius));
                    extend(center + egui::vec2(radius, radius));
                }
            }
            _ => {
                for p in &s.points { extend(*p); }
            }
        }
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
    mouse: &MouseState,
    can_draw: bool,
    _embed_trigger: &mut bool,
    frame_count: u64,
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
    
    let left_down = ui.input(|i| i.pointer.primary_down());
    let left_just_pressed = ui.input(|i| i.pointer.primary_pressed());
    let left_just_released = ui.input(|i| i.pointer.primary_released());

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
                    if sel.object_type == crate::project::ObjectType::Image {
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
        let is_active = project.active_layer == i;
        let l_op = layer.opacity;
        
        // Placed Images
        // Shadows for images
        for img in layer.placed_images.iter() {
            if img.shadow || settings.snip_shadow {
                let disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
                let disp_h = img.display_size.unwrap_or([img.size[1] as f32, img.size[1] as f32])[1];
                let shadow_rect = egui::Rect::from_min_size(img.position - render_offset + egui::vec2(6.0, 6.0), egui::vec2(disp_w, disp_h));
                painter.rect_filled(shadow_rect, 0.0, egui::Color32::from_black_alpha((100.0 * l_op) as u8));
            }
        }

        for img in layer.placed_images.iter_mut() {
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
                    dummy_mesh.add_rect_with_uv(egui::Rect::from_min_size(egui::pos2(-disp_w*0.5, -disp_h*0.5), egui::vec2(disp_w, disp_h)), egui::Rect::from_min_max(egui::pos2(0.0,0.0), egui::pos2(1.0,1.0)), egui::Color32::WHITE);
                    transform_mesh(&mut dummy_mesh, center, img.rotation, img.skew, img.perspective);
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
                            
                            if img.blur > 0.1 {
                                match img.blur_effect {
                                    BlurEffect::Gaussian => apply_box_blur(&mut p, sw as usize, sh as usize, img.blur as usize),
                                    BlurEffect::Pixelate => apply_pixelate(&mut p, sw as usize, sh as usize, (img.blur * ppp) as usize),
                                    BlurEffect::Glitch => apply_vhs_glitch(&mut p, sw as usize, sh as usize, img.blur as f32 / 100.0),
                                }
                            }
                            
                            if let Some(mask) = &img.mask {
                                p.par_chunks_exact_mut(4).enumerate().for_each(|(i, pixel)| {
                                    if i < mask.len() && mask[i] == 0 { pixel[3] = 0; }
                                });
                            }
                            
                            img.pixels = p;
                            img.texture = None;
                        } else if !settings.exclude_from_capture {
                            crate::winapi_utils::set_capture_exclusion(false);
                        }
                    }
                }

                } // end screen-region branch
            }

            if !img.frames.is_empty() {
                let time = ui.input(|i| i.time) as f32;
                if img.last_frame_time == 0.0 { img.last_frame_time = time; }
                let dt = time - img.last_frame_time;
                let current_duration = img.frame_durations[img.current_frame];
                
                if dt >= current_duration {
                    img.current_frame = (img.current_frame + 1) % img.frames.len();
                    img.last_frame_time = time;
                    // Update texture with new frame
                    let color_image = egui::ColorImage::from_rgba_unmultiplied(img.size, &img.frames[img.current_frame]);
                    img.texture = Some(ui.ctx().load_texture(format!("gif_{}", img.id), color_image, Default::default()));
                }
                ui.ctx().request_repaint();
            } else if img.texture.is_none() && !img.pixels.is_empty() {
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
                let center = (img.position - render_offset) + egui::vec2(disp_w * 0.5, disp_h * 0.5);

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
                let color = egui::Color32::from_white_alpha((255.0 * l_op * img.opacity) as u8);
                mesh.add_rect_with_uv(egui::Rect::from_min_size(egui::pos2(-disp_w*0.5, -disp_h*0.5), egui::vec2(disp_w, disp_h)), uv, color);
                transform_mesh(&mut mesh, center, img.rotation, img.skew, img.perspective);
                
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

            let draw_stroke = |p: &egui::Painter, col: egui::Color32, offset: egui::Vec2, w: f32| {
                match s.kind {
                    StrokeKind::Freehand => {
                        if s.points.len() >= 2 {
                            let mut pts: Vec<_> = s.points.iter().map(|&pt| pt - render_offset + offset).collect();
                            
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
                                        let rx = ((seed % 100) as f32 - 50.0) / 50.0 * w;
                                        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
                                        let ry = ((seed % 100) as f32 - 50.0) / 50.0 * w;
                                        p.circle_filled(*pt + egui::vec2(rx, ry), 1.0, col);
                                    }
                                }
                            } else if s.brush_mode == BrushMode::Calligraphy {
                                let angle = std::f32::consts::PI / 4.0;
                                let perp = egui::vec2(angle.cos(), angle.sin()) * w * 0.5;
                                for window in pts.windows(2) {
                                    draw_quad(
                                        p,
                                        window[0] - perp, window[0] + perp,
                                        window[1] + perp, window[1] - perp,
                                        col
                                    );
                                }
                            } else {
                                    if s.brush_shape == BrushShape::Square {
                                        for window in pts.windows(2) {
                                            let dir = (window[1] - window[0]).normalized();
                                            let perp = egui::vec2(-dir.y, dir.x) * w * 0.5;
                                            draw_quad(
                                                p,
                                                window[0] - perp, window[0] + perp,
                                                window[1] + perp, window[1] - perp,
                                                col
                                            );
                                        }
                                    } else {
                                        // Circle Brush: draw circles at each point for perfectly round appearance
                                        for pt in &pts {
                                            p.circle_filled(*pt, w * 0.5, col);
                                        }
                                    }
                                }
                            }
                        }
                    StrokeKind::Line => { if s.points.len() == 2 { p.line_segment([s.points[0]+offset, s.points[1]+offset], egui::Stroke::new(w, col)); } }
                    StrokeKind::Arrow => { if s.points.len() == 2 { draw_arrow(p, s.points[0]+offset, s.points[1]+offset, w, col); } }
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
                    StrokeKind::Star => {
                        if s.points.len() == 2 {
                            let center = s.points[0] + offset;
                            let radius = s.points[0].distance(s.points[1]);
                            draw_star(p, center, radius, col, s.background_color.as_ref().map(|bg| color32(bg)), s.width);
                        }
                    }
                    StrokeKind::Heart => {
                        if s.points.len() == 2 {
                            let center = s.points[0] + offset;
                            let radius = s.points[0].distance(s.points[1]);
                            draw_heart(p, center, radius, col, s.background_color.as_ref().map(|bg| color32(bg)), s.width);
                        }
                    }
                    _ => {}
                }
            };

            if layer.shadow || s.shadow {
                let s_col = if s.shadow {
                    if let Some(bg) = s.background_color {
                        let c = color32(&bg);
                        if c.a() > 0 { egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (128.0 * l_op) as u8) }
                        else { egui::Color32::from_black_alpha((128.0 * l_op) as u8) }
                    } else {
                        egui::Color32::from_black_alpha((128.0 * l_op) as u8)
                    }
                } else {
                    egui::Color32::from_rgba_unmultiplied(layer.shadow_color[0], layer.shadow_color[1], layer.shadow_color[2], (layer.shadow_color[3] as f32 * l_op) as u8)
                };
                let offset = if s.shadow { egui::vec2(2.0, 2.0) } else { egui::vec2(layer.shadow_offset[0], layer.shadow_offset[1]) };
                draw_stroke(&painter, s_col, offset, s.width);
            }
            if layer.outline || s.outline {
                let o_col = if s.outline {
                    let mut col = if c.r() as u32 + c.g() as u32 + c.b() as u32 > 382 { egui::Color32::BLACK } else { egui::Color32::WHITE };
                    if let Some(bg) = s.background_color {
                        let bg_c = color32(&bg);
                        if bg_c.a() > 0 { col = egui::Color32::from_rgba_unmultiplied(bg_c.r(), bg_c.g(), bg_c.b(), (255.0 * l_op) as u8); }
                    }
                    col
                } else {
                    egui::Color32::from_rgba_unmultiplied(layer.outline_color[0], layer.outline_color[1], layer.outline_color[2], (layer.outline_color[3] as f32 * l_op) as u8)
                };
                let o_width = if s.outline { s.width + 2.0 } else { s.width + layer.outline_width };
                draw_stroke(&painter, o_col, egui::Vec2::ZERO, o_width);
            }
            draw_stroke(&painter, c, egui::Vec2::ZERO, s.width);
        }

        // Text annotations
        for ann in layer.text_annotations.iter() {
            let font = match ann.font {
                TextFont::Sans => egui::FontId::proportional(ann.font_size),
                TextFont::Serif => egui::FontId::new(ann.font_size, egui::FontFamily::Name("serif".into())),
                TextFont::Mono => egui::FontId::monospace(ann.font_size),
                TextFont::Handwriting => egui::FontId::new(ann.font_size, egui::FontFamily::Name("handwriting".into())),
                TextFont::Heading => egui::FontId::new(ann.font_size, egui::FontFamily::Name("heading".into())),
                TextFont::Custom => {
                    // Use custom font name if stored in the annotation text (format: uses font family by name)
                    // We embed the font name via the settings at commit time in a custom_font_name field
                    // For now fall back to proportional - system font loading requires font data
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
                painter.text(ann.position - render_offset + egui::vec2(2.0, 2.0), egui::Align2::LEFT_TOP, &ann.text, font.clone(), egui::Color32::from_black_alpha((150.0 * l_op * ann.opacity) as u8));
            }

            if layer.outline || ann.outline || settings.text_outline {
                let sw = if ann.stroke_width > 0.0 { ann.stroke_width } else { 1.0 };
                painter.text(ann.position - render_offset + egui::vec2(sw, sw), egui::Align2::LEFT_TOP, &ann.text, font.clone(), outline_col);
                painter.text(ann.position - render_offset + egui::vec2(-sw, -sw), egui::Align2::LEFT_TOP, &ann.text, font.clone(), outline_col);
                painter.text(ann.position - render_offset + egui::vec2(sw, -sw), egui::Align2::LEFT_TOP, &ann.text, font.clone(), outline_col);
                painter.text(ann.position - render_offset + egui::vec2(-sw, sw), egui::Align2::LEFT_TOP, &ann.text, font.clone(), outline_col);
            }
            
            painter.text(ann.position - render_offset, egui::Align2::LEFT_TOP, &ann.text, font, c);
        }

        // Bounding box for Move tool
        if *active_tool == Tool::Move && is_active {
            if let Some(bounds) = layer_bounds(layer) {
                let bounds = bounds.translate(-render_offset);
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
        let pts: Vec<_> = current_stroke.iter().map(|&p| p - render_offset).collect();
        if settings.brush_mode == BrushMode::Calligraphy {
            let angle = std::f32::consts::PI / 4.0;
            let perp = egui::vec2(angle.cos(), angle.sin()) * settings.pen_width * 0.5;
            for window in pts.windows(2) {
                let p1 = window[0];
                let p2 = window[1];
                draw_quad(
                    &painter,
                    p1 - perp, p1 + perp,
                    p2 + perp, p2 - perp,
                    pen_c
                );
            }
        } else {
            painter.add(egui::Shape::line(pts, egui::Stroke::new(settings.pen_width, pen_c)));
        }
    }
    if let Some(start) = *line_start {
        if matches!(active_tool, Tool::Shape) {
            let rect = egui::Rect::from_two_pos(start, pos).translate(-render_offset);
            let _bg = color32(&settings.background_color);
            match settings.shape_type {
                ShapeType::Rect => {
                    let bg_c = color32(&settings.background_color);
                    painter.rect_filled(rect, 0.0, egui::Color32::from_rgba_unmultiplied(bg_c.r(), bg_c.g(), bg_c.b(), bg_c.a()/2)); // Live preview with half opacity
                    painter.rect_stroke(rect, 0.0, egui::Stroke::new(settings.stroke_width, pen_c), egui::StrokeKind::Middle);
                }
                ShapeType::Circle => {
                    let radius = (start - pos).length();
                    let bg_c = color32(&settings.background_color);
                    painter.circle_filled(start - render_offset, radius, egui::Color32::from_rgba_unmultiplied(bg_c.r(), bg_c.g(), bg_c.b(), bg_c.a()/2));
                    painter.circle_stroke(start - render_offset, radius, egui::Stroke::new(settings.stroke_width, pen_c));
                }
                ShapeType::Star => {
                    let radius = (start - pos).length();
                    let bg_c = color32(&settings.background_color);
                    draw_star(&painter, start - render_offset, radius, pen_c, Some(egui::Color32::from_rgba_unmultiplied(bg_c.r(), bg_c.g(), bg_c.b(), bg_c.a()/2)), settings.stroke_width);
                }
                ShapeType::Heart => {
                    let radius = (start - pos).length();
                    let bg_c = color32(&settings.background_color);
                    draw_heart(&painter, start - render_offset, radius, pen_c, Some(egui::Color32::from_rgba_unmultiplied(bg_c.r(), bg_c.g(), bg_c.b(), bg_c.a()/2)), settings.stroke_width);
                }
            }
        } else if matches!(active_tool, Tool::Snip) || matches!(active_tool, Tool::Blur) {
            let rect = egui::Rect::from_two_pos(start, pos).translate(-render_offset);
            painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, egui::Color32::WHITE), egui::StrokeKind::Middle);
            painter.rect_filled(rect, 0.0, egui::Color32::from_black_alpha(40));
        }
    }

    // ── Pending text cursor ──
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
            let active_idx = project.active_layer;
            if active_idx < project.layers.len() {
                let mut ann = TextAnnotation::new(p.position, p.buffer, settings.pen_color, settings.font_size);
                ann.monospace = settings.text_monospace;
                ann.shadow = settings.text_shadow;
                ann.outline = settings.text_outline;
                ann.stroke_width = settings.text_stroke_width;
                ann.font = settings.text_font;
                project.layers[active_idx].text_annotations.push(ann);
            }
        }
        *pending_text = None;
    }



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

    // snip_created is declared later
    
    // Layer selection for Move tool
    if can_draw && edit_mode && matches!(active_tool, Tool::Move) {
        let is_double_click = ui.input(|i| i.pointer.button_double_clicked(egui::PointerButton::Primary));
        
        if left_just_pressed || is_double_click {
            if is_double_click {
                let mut hit_idx = None;
                for (i, l) in project.layers.iter().enumerate().rev() {
                    if let Some(b) = layer_bounds(l) {
                        if b.expand(20.0).contains(pos) {
                            hit_idx = Some(i);
                            break;
                        }
                    }
                }
                
                if let Some(idx) = hit_idx {
                    if is_double_click {
                        project.active_layer = idx;
                    }
                }
            }
        }
    }

    let mut remove_active_layer = false;
    let mut snip_created = false;
    let mut new_selection = None;
    let mut switch_to_move = false;
    let active_layer_idx = project.active_layer;

    if edit_mode && can_draw && !ui.ctx().is_pointer_over_area() && active_layer_idx < project.layers.len() {
        match active_tool {
            Tool::Brush => {
                let layer = &mut project.layers[active_layer_idx];
            if left_down {
                // Reject glitchy points (0,0) or huge jumps
                if pos.x < 1.0 && pos.y < 1.0 { return; }

                if let Some(last) = current_stroke.last().cloned() {
                    let dist = last.distance(pos);
                    if dist > 2000.0 { 
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
                    let s = Stroke::new(current_stroke.clone(), settings.pen_color, settings.pen_width, StrokeKind::Freehand, settings.brush_mode, Some(settings.background_color), settings.brush_shadow, settings.brush_shape, settings.brush_outline);
                    layer.strokes.push(s);
                    current_stroke.clear();
                }
            }
            Tool::Shape => {
                let layer = &mut project.layers[active_layer_idx];
            if left_just_pressed { *line_start = Some(pos); }
            if left_just_released {
                if let Some(start) = line_start.take() {
                    let kind = match settings.shape_type {
                        ShapeType::Rect => StrokeKind::Rect,
                        ShapeType::Circle => StrokeKind::Circle,
                        ShapeType::Star => StrokeKind::Star,
                        ShapeType::Heart => StrokeKind::Heart,
                    };
                    let s = Stroke::new(vec![start, pos], settings.pen_color, settings.stroke_width, kind, settings.brush_mode, Some(settings.background_color), settings.brush_shadow, settings.brush_shape, settings.brush_outline);
                    layer.strokes.push(s);
                }
                }
            }
            Tool::Snip => {
                let layer = &mut project.layers[active_layer_idx];
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
                        let w = rect.width();
                        let h = rect.height();
                        if w > 5.0 && h > 5.0 {
                            let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
                            if settings.snip_live {
                                snip_created = true;
                                let mut img = PlacedImage::new(id, rect.min, [w.round() as usize, h.round() as usize], Vec::new());
                                img.is_live = true;
                                img.source_rect = Some([rect.min.x, rect.min.y, w, h]);
                                img.blur = settings.blur_strength;
                                img.blur_effect = settings.blur_effect;
                                img.show_source_rect = true;
                                layer.placed_images.push(img);
                            } else {
                                let ppp = ui.ctx().pixels_per_point();
                                let (wx, wy) = crate::winapi_utils::get_window_screen_pos();
                                let sx = (rect.min.x * ppp) as i32 + if settings.use_absolute_screen_coords { 0 } else { wx };
                                let sy = (rect.min.y * ppp) as i32 + if settings.use_absolute_screen_coords { 0 } else { wy };
                                if let Some(pixels) = crate::winapi_utils::capture_screen_rect(sx, sy, (w * ppp) as i32, (h * ppp) as i32) {
                                    snip_created = true;
                                    let mut img = PlacedImage::new(id, rect.min, [w.round() as usize, h.round() as usize], pixels);
                                    img.shadow = settings.snip_shadow;
                                    layer.placed_images.push(img);
                                }
                            }
                        }
                    }
                }
            } else if mode == SnipMode::Circle {
                if left_just_pressed { *line_start = Some(pos); }
                if left_just_released {
                    if let Some(start) = line_start.take() {
                        let rect = egui::Rect::from_two_pos(start, pos);
                        let w = rect.width();
                        let h = rect.height();
                        if w > 5.0 && h > 5.0 {
                            let ppp = ui.ctx().pixels_per_point();
                            let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
                            
                            let mut mask = vec![255u8; (w * ppp) as usize * (h * ppp) as usize];
                            let center = egui::pos2(w * 0.5, h * 0.5);
                            let radius = w.min(h) * 0.5;
                            for py in 0..(h * ppp) as usize {
                                for px in 0..(w * ppp) as usize {
                                    let lp = egui::pos2(px as f32 / ppp, py as f32 / ppp);
                                    if lp.distance(center) > radius {
                                        mask[py * (w * ppp) as usize + px] = 0;
                                    }
                                }
                            }

                            if settings.snip_live {
                                snip_created = true;
                                let mut img = PlacedImage::new(id, rect.min, [w.round() as usize, h.round() as usize], Vec::new());
                                img.is_live = true;
                                img.source_rect = Some([rect.min.x, rect.min.y, w, h]);
                                img.mask = Some(mask);
                                img.blur = settings.blur_strength;
                                img.blur_effect = settings.blur_effect;
                                img.show_source_rect = true;
                                layer.placed_images.push(img);
                            } else {
                                let (wx, wy) = crate::winapi_utils::get_window_screen_pos();
                                let sx = (rect.min.x * ppp) as i32 + if settings.use_absolute_screen_coords { 0 } else { wx };
                                let sy = (rect.min.y * ppp) as i32 + if settings.use_absolute_screen_coords { 0 } else { wy };
                                if let Some(mut pixels) = crate::winapi_utils::capture_screen_rect(sx, sy, (w * ppp) as i32, (h * ppp) as i32) {
                                    snip_created = true;
                                    for (i, &m) in mask.iter().enumerate() { if m == 0 { pixels[i*4+3] = 0; } }
                                    let mut img = PlacedImage::new(id, rect.min, [w.round() as usize, h.round() as usize], pixels);
                                    img.shadow = settings.snip_shadow;
                                    layer.placed_images.push(img);
                                }
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
                    let sw = (bounds.width() * ppp) as i32;
                    let sh = (bounds.height() * ppp) as i32;
                    
                    if sw > 5 && sh > 5 {
                        let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
                        let poly: Vec<egui::Pos2> = current_stroke.iter().map(|p| egui::pos2(p.x - bounds.min.x, p.y - bounds.min.y)).collect();
                        let mut mask = vec![255u8; sw as usize * sh as usize];
                        for py in 0..sh as usize {
                            for px in 0..sw as usize {
                                let lp = egui::pos2(px as f32 / ppp, py as f32 / ppp);
                                if !is_inside_poly(&poly, lp) {
                                    mask[py * sw as usize + px] = 0;
                                }
                            }
                        }

                        if settings.snip_live {
                            snip_created = true;
                            let mut img = PlacedImage::new(id, bounds.min, [bounds.width().round() as usize, bounds.height().round() as usize], Vec::new());
                            img.is_live = true;
                            img.source_rect = Some([bounds.min.x, bounds.min.y, bounds.width(), bounds.height()]);
                            img.mask = Some(mask);
                            img.blur = settings.blur_strength;
                            img.blur_effect = settings.blur_effect;
                            layer.placed_images.push(img);
                        } else {
                            let (wx, wy) = crate::winapi_utils::get_window_screen_pos();
                            let sx = (bounds.min.x * ppp) as i32 + if settings.use_absolute_screen_coords { 0 } else { wx };
                            let sy = (bounds.min.y * ppp) as i32 + if settings.use_absolute_screen_coords { 0 } else { wy };
                            if let Some(mut pixels) = crate::winapi_utils::capture_screen_rect(sx, sy, sw, sh) {
                                snip_created = true;
                                for (i, &m) in mask.iter().enumerate() { if m == 0 { pixels[i*4+3] = 0; } }
                                let mut img = PlacedImage::new(id, bounds.min, [bounds.width().round() as usize, bounds.height().round() as usize], pixels);
                                img.shadow = settings.snip_shadow;
                                layer.placed_images.push(img);
                            }
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
                    let sw = (bounds.width() * ppp) as i32;
                    let sh = (bounds.height() * ppp) as i32;
                    
                    if sw > 5 && sh > 5 {
                        let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
                        let poly: Vec<egui::Pos2> = current_stroke.iter().map(|p| egui::pos2(p.x - bounds.min.x, p.y - bounds.min.y)).collect();
                        let mut mask = vec![255u8; sw as usize * sh as usize];
                        for py in 0..sh as usize {
                            for px in 0..sw as usize {
                                let lp = egui::pos2(px as f32 / ppp, py as f32 / ppp);
                                if !is_inside_poly(&poly, lp) {
                                    mask[py * sw as usize + px] = 0;
                                }
                            }
                        }

                        if settings.snip_live {
                            snip_created = true;
                            let mut img = PlacedImage::new(id, bounds.min, [bounds.width().round() as usize, bounds.height().round() as usize], Vec::new());
                            img.is_live = true;
                            img.source_rect = Some([bounds.min.x, bounds.min.y, bounds.width(), bounds.height()]);
                            img.mask = Some(mask);
                            img.blur = settings.blur_strength;
                            img.blur_effect = settings.blur_effect;
                            img.show_source_rect = true;
                            layer.placed_images.push(img);
                        } else {
                            let (wx, wy) = crate::winapi_utils::get_window_screen_pos();
                            let sx = (bounds.min.x * ppp) as i32 + if settings.use_absolute_screen_coords { 0 } else { wx };
                            let sy = (bounds.min.y * ppp) as i32 + if settings.use_absolute_screen_coords { 0 } else { wy };
                            if let Some(mut pixels) = crate::winapi_utils::capture_screen_rect(sx, sy, sw, sh) {
                                snip_created = true;
                                for (i, &m) in mask.iter().enumerate() { if m == 0 { pixels[i*4+3] = 0; } }
                                let mut img = PlacedImage::new(id, bounds.min, [bounds.width().round() as usize, bounds.height().round() as usize], pixels);
                                img.shadow = settings.snip_shadow;
                                layer.placed_images.push(img);
                            }
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
                let layer = &mut project.layers[active_layer_idx];
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
                let should_erase_point = |p: egui::Pos2| -> bool {
                    let inside = rect.contains(p);
                    if settings.inverted_cut { !inside } else { inside }
                };
                layer.strokes.retain(|s| !s.points.iter().any(|&p| should_erase_point(p)));
                layer.text_annotations.retain(|t| !should_erase_point(t.position));
            } else if let Some(poly) = cut_poly {
                painter.add(egui::Shape::convex_polygon(poly.clone(), egui::Color32::from_white_alpha(180), egui::Stroke::NONE));
                for img in &mut layer.placed_images {
                    let disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
                    let disp_h = img.display_size.unwrap_or([img.size[1] as f32, img.size[1] as f32])[1];
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
                let should_erase_point = |p: egui::Pos2| -> bool {
                    let inside = is_inside_poly(&poly, p);
                    if settings.inverted_cut { !inside } else { inside }
                };
                layer.strokes.retain(|s| !s.points.iter().any(|&p| should_erase_point(p)));
                layer.text_annotations.retain(|t| !should_erase_point(t.position));
            }
            }
            Tool::Mirror => {
                let layer = &mut project.layers[active_layer_idx];
                match settings.mirror_mode {
                MirrorMode::Rect => {
                    if left_just_pressed { *line_start = Some(pos); }
                    if let Some(start) = *line_start {
                        let rect = egui::Rect::from_two_pos(start, pos);
                        painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, egui::Color32::WHITE), egui::StrokeKind::Middle);
                    }
                    if left_just_released {
                        if let Some(start) = line_start.take() {
                            let rect = egui::Rect::from_two_pos(start, pos);
                            if rect.width() > 4.0 && rect.height() > 4.0 {
                                let mut mask = None;
                                if settings.mirror_mode == MirrorMode::Circle {
                                    let w = rect.width();
                                    let h = rect.height();
                                    let ppp = ui.ctx().pixels_per_point();
                                    let mut m = vec![255u8; (w * ppp) as usize * (h * ppp) as usize];
                                    let center = egui::pos2(w * 0.5, h * 0.5);
                                    let radius = w.min(h) * 0.5;
                                    for py in 0..(h * ppp) as usize {
                                        for px in 0..(w * ppp) as usize {
                                            let lp = egui::pos2(px as f32 / ppp, py as f32 / ppp);
                                            if lp.distance(center) > radius {
                                                m[py * (w * ppp) as usize + px] = 0;
                                            }
                                        }
                                    }
                                    mask = Some(m);
                                }
                                let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
                                let mut img = PlacedImage::new(id, rect.min, [rect.width() as usize, rect.height() as usize], Vec::new());
                                img.is_live = true;
                                img.source_rect = Some([rect.min.x, rect.min.y, rect.width(), rect.height()]);
                                img.mask = mask;
                                layer.placed_images.push(img);
                            }
                        }
                    }
                }
                MirrorMode::Circle => {
                    if left_just_pressed { *line_start = Some(pos); }
                    if let Some(start) = *line_start {
                        let rect = egui::Rect::from_two_pos(start, pos);
                        painter.circle_stroke(rect.center(), rect.width().min(rect.height()) * 0.5, egui::Stroke::new(1.0, egui::Color32::WHITE));
                    }
                    if left_just_released {
                        if let Some(start) = line_start.take() {
                            let rect = egui::Rect::from_two_pos(start, pos);
                            if rect.width() > 4.0 && rect.height() > 4.0 {
                                let ppp = ui.ctx().pixels_per_point();
                                let w = rect.width();
                                let h = rect.height();
                                let mut mask = vec![255u8; (w * ppp) as usize * (h * ppp) as usize];
                                let center = egui::pos2(w * 0.5, h * 0.5);
                                let radius = w.min(h) * 0.5;
                                for py in 0..(h * ppp) as usize {
                                    for px in 0..(w * ppp) as usize {
                                        let lp = egui::pos2(px as f32 / ppp, py as f32 / ppp);
                                        if lp.distance(center) > radius {
                                            mask[py * (w * ppp) as usize + px] = 0;
                                        }
                                    }
                                }
                                let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
                                let mut img = PlacedImage::new(id, rect.min, [w as usize, h as usize], Vec::new());
                                img.is_live = true;
                                img.source_rect = Some([rect.min.x, rect.min.y, w, h]);
                                img.mask = Some(mask);
                                layer.placed_images.push(img);
                            }
                        }
                    }
                }
                MirrorMode::Lasso | MirrorMode::Poly => {
                    if left_just_pressed {
                        if settings.mirror_mode == MirrorMode::Poly {
                            if current_stroke.is_empty() { current_stroke.push(pos); }
                            else {
                                if pos.distance(current_stroke[0]) < 10.0 {
                                    if current_stroke.len() >= 3 {
                                        let mut min = egui::pos2(f32::MAX, f32::MAX);
                                        let mut max = egui::pos2(f32::MIN, f32::MIN);
                                        for &p in current_stroke.iter() {
                                            min.x = min.x.min(p.x); min.y = min.y.min(p.y);
                                            max.x = max.x.max(p.x); max.y = max.y.max(p.y);
                                        }
                                        let rect = egui::Rect::from_min_max(min, max);
                                        let ppp = ui.ctx().pixels_per_point();
                                        let mut mask = vec![255u8; (rect.width() * ppp) as usize * (rect.height() * ppp) as usize];
                                        for py in 0..(rect.height() * ppp) as usize {
                                            for px in 0..(rect.width() * ppp) as usize {
                                                let p = rect.min + egui::vec2(px as f32 / ppp, py as f32 / ppp);
                                                if !is_inside_poly(&current_stroke, p) {
                                                    mask[py * (rect.width() * ppp) as usize + px] = 0;
                                                }
                                            }
                                        }
                                        let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
                                        let mut img = PlacedImage::new(id, rect.min, [rect.width() as usize, rect.height() as usize], Vec::new());
                                        img.is_live = true;
                                        img.source_rect = Some([rect.min.x, rect.min.y, rect.width(), rect.height()]);
                                        img.mask = Some(mask);
                                        layer.placed_images.push(img);
                                    }
                                    current_stroke.clear();
                                } else {
                                    current_stroke.push(pos);
                                }
                            }
                        } else {
                            current_stroke.clear();
                            current_stroke.push(pos);
                        }
                    }
                    if left_down && settings.mirror_mode == MirrorMode::Lasso {
                        if let Some(last) = current_stroke.last() {
                            if pos.distance(*last) > 4.0 { current_stroke.push(pos); }
                        }
                    }
                    if left_just_released && settings.mirror_mode == MirrorMode::Lasso {
                        if current_stroke.len() >= 3 {
                            let mut min = egui::pos2(f32::MAX, f32::MAX);
                            let mut max = egui::pos2(f32::MIN, f32::MIN);
                            for &p in current_stroke.iter() {
                                min.x = min.x.min(p.x); min.y = min.y.min(p.y);
                                max.x = max.x.max(p.x); max.y = max.y.max(p.y);
                            }
                            let rect = egui::Rect::from_min_max(min, max);
                            let ppp = ui.ctx().pixels_per_point();
                            let mut mask = vec![255u8; (rect.width() * ppp) as usize * (rect.height() * ppp) as usize];
                            for py in 0..(rect.height() * ppp) as usize {
                                for px in 0..(rect.width() * ppp) as usize {
                                    let p = rect.min + egui::vec2(px as f32 / ppp, py as f32 / ppp);
                                    if !is_inside_poly(&current_stroke, p) {
                                        mask[py * (rect.width() * ppp) as usize + px] = 0;
                                    }
                                }
                            }
                            let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
                            let mut img = PlacedImage::new(id, rect.min, [rect.width() as usize, rect.height() as usize], Vec::new());
                            img.is_live = true;
                            img.source_rect = Some([rect.min.x, rect.min.y, rect.width(), rect.height()]);
                            img.mask = Some(mask);
                            layer.placed_images.push(img);
                        }
                        current_stroke.clear();
                    }
                    if !current_stroke.is_empty() {
                        painter.add(egui::Shape::line(current_stroke.clone(), egui::Stroke::new(1.0, egui::Color32::WHITE)));
                        for &p in current_stroke.iter() { painter.circle_filled(p, 2.0, egui::Color32::WHITE); }
                    }
                }
                MirrorMode::Window => {
                    // Window mode now behaves like Rect mode initially
                    if left_just_pressed { *line_start = Some(pos); }
                    if let Some(start) = *line_start {
                        let rect = egui::Rect::from_two_pos(start, pos);
                        painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, egui::Color32::WHITE), egui::StrokeKind::Middle);
                    }
                    if left_just_released {
                        if let Some(start) = line_start.take() {
                            let rect = egui::Rect::from_two_pos(start, pos);
                            let w = rect.width();
                            let h = rect.height();
                            if w > 4.0 && h > 4.0 {
                                let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
                                let mut img = PlacedImage::new(id, rect.min, [w as usize, h as usize], Vec::new());
                                img.is_live = true;
                                img.source_rect = Some([rect.min.x, rect.min.y, w, h]);
                                layer.placed_images.push(img);
                                // Request auto-selection and tool switch after borrow ends
                                new_selection = Some(crate::project::SelectedObject {
                                    layer_idx: active_layer_idx,
                                    object_type: crate::project::ObjectType::Image,
                                    object_idx: layer.placed_images.len() - 1,
                                });
                                switch_to_move = true;
                            }
                        }
                    }
                }
            }
        }
        Tool::Blur => {
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
        Tool::Text => {
            let layer = &mut project.layers[active_layer_idx];
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
                        let mut ann = TextAnnotation::new(p.position, p.buffer, settings.pen_color, settings.font_size);
                        ann.monospace = settings.text_monospace;
                        ann.shadow = settings.text_shadow;
                        ann.outline = settings.text_outline;
                        ann.stroke_width = settings.text_stroke_width;
                        ann.font = settings.text_font;
                        layer.text_annotations.push(ann);
                    }
                } else {
                    *pending_text = Some(PendingText { position: pos, buffer: String::new() });
                }
            }
            }
            Tool::Embed => {
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
            Tool::Move => {
                if left_just_pressed {
                let mut found = false;
                for (l_idx, l) in project.layers.iter_mut().enumerate().rev() {
                    if !l.visible { continue; }
                    
                    // Hit test images
                    for (img_idx, img) in l.placed_images.iter_mut().enumerate().rev() {
                        let disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
                        let disp_h = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[1];
                        let img_rect = egui::Rect::from_min_size(img.position, egui::vec2(disp_w, disp_h));
                        

                        if img_rect.contains(pos) {
                            let is_already_selected = project.selected_object == Some(crate::project::SelectedObject {
                                layer_idx: l_idx, object_type: crate::project::ObjectType::Image, object_idx: img_idx
                            });
                            
                            if !is_already_selected || ui.input(|i| i.pointer.button_double_clicked(egui::PointerButton::Primary)) {
                                project.active_layer = l_idx;
                                project.selected_object = Some(crate::project::SelectedObject {
                                    layer_idx: l_idx,
                                    object_type: crate::project::ObjectType::Image,
                                    object_idx: img_idx,
                                });
                                found = true;
                                break;
                            }
                        }
                    }
                    if found { break; }
                    
                    // Hit test text
                    for (txt_idx, txt) in l.text_annotations.iter().enumerate().rev() {
                        let txt_rect = egui::Rect::from_min_size(txt.position, egui::vec2(txt.text.len() as f32 * txt.font_size * 0.6, txt.font_size));
                        if txt_rect.contains(pos) {
                            project.active_layer = l_idx;
                            project.selected_object = Some(crate::project::SelectedObject {
                                layer_idx: l_idx,
                                object_type: crate::project::ObjectType::Text,
                                object_idx: txt_idx,
                            });
                            found = true;
                            break;
                        }
                    }
                    if found { break; }

                    // Hit test strokes/shapes
                    for (stroke_idx, s) in l.strokes.iter().enumerate().rev() {
                        let hit = match s.kind {
                            crate::overlay::StrokeKind::Rect => {
                                if s.points.len() >= 2 {
                                    let rect = egui::Rect::from_two_pos(s.points[0], s.points[1]);
                                    rect.expand(s.width.max(5.0)).contains(pos)
                                } else { false }
                            },
                            crate::overlay::StrokeKind::Circle | crate::overlay::StrokeKind::Star | crate::overlay::StrokeKind::Heart => {
                                if s.points.len() >= 2 {
                                    let center = s.points[0];
                                    let radius = center.distance(s.points[1]);
                                    pos.distance(center) <= radius + s.width.max(5.0)
                                } else { false }
                            },
                            _ => {
                                if s.points.len() < 2 {
                                    s.points.iter().any(|p| p.distance(pos) < s.width + 5.0)
                                } else {
                                    let mut hit_line = false;
                                    for w in s.points.windows(2) {
                                        let (p0, p1) = (w[0], w[1]);
                                        let len_sq = p0.distance_sq(p1);
                                        if len_sq > 0.0 {
                                            let t = ((pos.x - p0.x) * (p1.x - p0.x) + (pos.y - p0.y) * (p1.y - p0.y)) / len_sq;
                                            let t = t.clamp(0.0, 1.0);
                                            let proj = p0 + (p1 - p0) * t;
                                            if proj.distance(pos) < s.width + 5.0 { hit_line = true; break; }
                                        } else if p0.distance(pos) < s.width + 5.0 {
                                            hit_line = true; break;
                                        }
                                    }
                                    hit_line
                                }
                            }
                        };
                        
                        if hit {
                            project.active_layer = l_idx;
                            project.selected_object = Some(crate::project::SelectedObject {
                                layer_idx: l_idx,
                                object_type: crate::project::ObjectType::Stroke,
                                object_idx: stroke_idx,
                            });
                            found = true;
                            break;
                        }
                    }
                    if found { break; }
                }
                
                if !found {
                    // Clicked empty space: select layer under mouse or clear selection?
                    // For now, let's just clear selection if we clicked far away
                }
            }

                if let Some(sel) = project.selected_object {
                    let layer = &mut project.layers[sel.layer_idx];
                    if let Some(raw_bounds) = layer_bounds(layer) {
                        // bounds = translated for drawing; raw_bounds = for hit-testing with pos
                        let bounds = raw_bounds.translate(-render_offset);
                        let draw_corners = [
                            bounds.left_top(), bounds.right_top(),
                            bounds.left_bottom(), bounds.right_bottom(),
                        ];
                        let hit_corners = [
                            raw_bounds.left_top(), raw_bounds.right_top(),
                            raw_bounds.left_bottom(), raw_bounds.right_bottom(),
                        ];

                        #[cfg(feature = "webengine")]
                        if sel.object_type == crate::project::ObjectType::Image {
                            if sel.object_idx < layer.placed_images.len() {
                                let img = &mut layer.placed_images[sel.object_idx];
                                if let Some(widget) = &mut img.web_widget {
                                    let disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
                                    let disp_h = img.display_size.unwrap_or([img.size[1] as f32, img.size[1] as f32])[1];
                                    let img_rect = egui::Rect::from_min_size(img.position, egui::vec2(disp_w, disp_h));
                                    
                                    if img_rect.contains(pos) {
                                        let rel_x = (pos.x - img.position.x) / disp_w * widget.width as f32;
                                        let rel_y = (pos.y - img.position.y) / disp_h * widget.height as f32;
                                        let is_move = !left_just_pressed && !left_just_released;
                                        widget.inject_mouse_event(rel_x, rel_y, mouse.left_down, is_move);
                                    }

                                    // Key events
                                    ui.input(|i| {
                                        for event in &i.events {
                                            match event {
                                                egui::Event::Text(text) => { widget.inject_text_event(text); }
                                                egui::Event::Key { key, pressed, .. } => { widget.inject_raw_key_event(*key, *pressed); }
                                                _ => {}
                                            }
                                        }
                                    });
                                }
                            }
                        }
                
                // --- Highlight Mirror Source Rect ---
                if ui.input(|i| i.pointer.button_double_clicked(egui::PointerButton::Primary)) && raw_bounds.contains(pos) {
                    for img in &mut layer.placed_images {
                        if img.is_live && img.source_rect.is_some() {
                            let disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
                            let disp_h = img.display_size.unwrap_or([img.size[1] as f32, img.size[1] as f32])[1];
                            let img_rect = egui::Rect::from_min_size(img.position, egui::vec2(disp_w, disp_h));
                            if img_rect.contains(pos) {
                                img.show_source_rect = !img.show_source_rect;
                            }
                        }
                    }
                }

                for img in &layer.placed_images {
                    if img.is_live && img.source_rect.is_some() && img.show_source_rect {
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
                // Determine what kind of object is selected
                let sel_is_blur = sel.object_type == crate::project::ObjectType::Image
                    && sel.object_idx < layer.placed_images.len()
                    && layer.placed_images[sel.object_idx].blur > 0.0;
                let sel_is_embed = sel.object_type == crate::project::ObjectType::Image
                    && sel.object_idx < layer.placed_images.len()
                    && layer.placed_images[sel.object_idx].url.is_some();

                // Use two rows: row 1 = URL bar (for embeds), row 2 = buttons
                let bar_height = if sel_is_embed { 52.0 } else { 32.0 };
                let top_btns_rect = egui::Rect::from_min_size(bounds.left_top() - egui::vec2(0.0, bar_height), egui::vec2(bounds.width().max(320.0), bar_height));
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(top_btns_rect), |ui| {
                    ui.visuals_mut().widgets.inactive.bg_fill = egui::Color32::from_black_alpha(150);

                    // Row 1: URL input bar (embed widgets only)
                    if sel_is_embed {
                        if sel.object_idx < layer.placed_images.len() {
                            let img = &mut layer.placed_images[sel.object_idx];
                            if let Some(ref mut url) = img.url {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("🌐").size(12.0));
                                    let edit_resp = ui.add(egui::TextEdit::singleline(url).desired_width(ui.available_width() - 10.0).hint_text("Enter URL..."));
                                    if edit_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                        #[cfg(feature = "webengine")]
                                        if let Some(widget) = &mut img.web_widget {
                                            crate::web_engine::navigate_widget(widget, url);
                                        }
                                    }
                                });
                            }
                        }
                    }

                    // Row 2: Action buttons
                    ui.horizontal(|ui| {
                        // Reset Transform button
                        if ui.add(egui::Button::new(egui::RichText::new("↺").color(egui::Color32::from_rgb(100, 200, 255)))).on_hover_text("Reset Transforms").clicked() {
                            for img in &mut layer.placed_images {
                                img.rotation = 0.0;
                                img.skew = egui::vec2(0.0, 0.0);
                                img.perspective = [egui::Vec2::ZERO; 4];
                                img.flipped_h = false;
                                img.flipped_v = false;
                            }
                            for s in &mut layer.strokes {
                                s.rotation = 0.0;
                                s.skew = egui::vec2(0.0, 0.0);
                            }
                        }
                        // Delete button
                        if ui.add(egui::Button::new(egui::RichText::new("🗑").color(egui::Color32::RED))).on_hover_text("Delete Layer Contents").clicked() {
                            layer.strokes.clear();
                            layer.text_annotations.clear();
                            layer.placed_images.clear();
                        }

                        ui.separator();

                        // Flip buttons
                        if ui.button("⬌").on_hover_text("Flip Horizontal").clicked() {
                            let center = bounds.center().x;
                            for img in &mut layer.placed_images { img.position.x = center - (img.position.x - center) - (img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0]); img.flipped_h = !img.flipped_h; }
                            for ann in &mut layer.text_annotations { ann.position.x = center - (ann.position.x - center) - (ann.text.len() as f32 * ann.font_size * 0.6); }
                            for s in &mut layer.strokes { for p in &mut s.points { p.x = center - (p.x - center); } }
                        }
                        if ui.button("⬍").on_hover_text("Flip Vertical").clicked() {
                            let center = bounds.center().y;
                            for img in &mut layer.placed_images { img.position.y = center - (img.position.y - center) - (img.display_size.unwrap_or([img.size[1] as f32, img.size[1] as f32])[1]); img.flipped_v = !img.flipped_v; }
                            for ann in &mut layer.text_annotations { ann.position.y = center - (ann.position.y - center) - ann.font_size; }
                            for s in &mut layer.strokes { for p in &mut s.points { p.y = center - (p.y - center); } }
                        }

                        ui.separator();

                        // Opacity
                        let mut op_val = layer.opacity * 100.0;
                        if ui.add(egui::DragValue::new(&mut op_val).range(0.0..=100.0).prefix("Op: ").suffix("%")).changed() {
                            layer.opacity = op_val / 100.0;
                        }

                        // Blur controls — ONLY for blur objects
                        if sel_is_blur {
                            ui.separator();
                            let img = &mut layer.placed_images[sel.object_idx];
                            let mut blur_val = img.blur;
                            if ui.add(egui::DragValue::new(&mut blur_val).range(0.0..=100.0).prefix("B: ")).changed() {
                                img.blur = blur_val;
                            }
                            ui.selectable_value(&mut img.blur_effect, BlurEffect::Gaussian, "Blur");
                            ui.selectable_value(&mut img.blur_effect, BlurEffect::Pixelate, "Pixel");
                            ui.selectable_value(&mut img.blur_effect, BlurEffect::Glitch, "VHS");
                        }

                        // Embed widget buttons — ONLY for embed objects with URLs
                        if sel_is_embed {
                            ui.separator();
                            let img = &mut layer.placed_images[sel.object_idx];
                            if ui.button("📋 Paste").on_hover_text("Paste URL from clipboard").clicked() {
                                if let Some(text) = crate::winapi_utils::get_clipboard_text() {
                                    img.url = Some(text.clone());
                                    #[cfg(feature = "webengine")]
                                    if let Some(widget) = &mut img.web_widget {
                                        crate::web_engine::navigate_widget(widget, &text);
                                    }
                                }
                            }
                            if ui.button("🌐 Web").on_hover_text("Open browser search").clicked() {
                                let text = "https://www.google.com".to_string();
                                img.url = Some(text.clone());
                                #[cfg(feature = "webengine")]
                                if let Some(widget) = &mut img.web_widget {
                                    crate::web_engine::navigate_widget(widget, &text);
                                }
                            }
                            if ui.button("📺 YT").on_hover_text("Open YouTube").clicked() {
                                let text = "https://www.youtube.com".to_string();
                                img.url = Some(text.clone());
                                #[cfg(feature = "webengine")]
                                if let Some(widget) = &mut img.web_widget {
                                    crate::web_engine::navigate_widget(widget, &text);
                                }
                            }
                        }
                    });
                });

                // Skew handles (mid-points of sides) — draw at bounds, hit-test at raw_bounds
                let draw_mids = [bounds.left_center(), bounds.right_center(), bounds.center_top(), bounds.center_bottom()];
                let hit_mids = [raw_bounds.left_center(), raw_bounds.right_center(), raw_bounds.center_top(), raw_bounds.center_bottom()];
                for m in draw_mids { painter.circle_filled(m, 3.0, egui::Color32::from_rgb(200, 200, 200)); }

                // Perspective handles (dots outside corners)
                let p_dist = 25.0;
                let draw_p_corners = [
                    bounds.left_top() + egui::vec2(-p_dist, -p_dist),
                    bounds.right_top() + egui::vec2(p_dist, -p_dist),
                    bounds.left_bottom() + egui::vec2(-p_dist, p_dist),
                    bounds.right_bottom() + egui::vec2(p_dist, p_dist),
                ];
                let hit_p_corners = [
                    raw_bounds.left_top() + egui::vec2(-p_dist, -p_dist),
                    raw_bounds.right_top() + egui::vec2(p_dist, -p_dist),
                    raw_bounds.left_bottom() + egui::vec2(-p_dist, p_dist),
                    raw_bounds.right_bottom() + egui::vec2(p_dist, p_dist),
                ];
                for pc in draw_p_corners {
                    painter.circle_filled(pc, 4.0, egui::Color32::from_rgb(100, 200, 255));
                    painter.circle_stroke(pc, 4.0, egui::Stroke::new(1.0, egui::Color32::BLACK));
                }

                // Transformation Handles (Resize)
                for corner in draw_corners {
                    painter.rect_filled(egui::Rect::from_center_size(corner, egui::vec2(8.0, 8.0)), 0.0, egui::Color32::WHITE);
                    painter.rect_stroke(egui::Rect::from_center_size(corner, egui::vec2(8.0, 8.0)), 0.0, egui::Stroke::new(1.0, egui::Color32::BLACK), egui::StrokeKind::Middle);
                }
                
                // Rotation handle: draw at bounds, hit-test at raw_bounds
                let draw_rot = bounds.center_top() - egui::vec2(0.0, 20.0);
                let hit_rot = raw_bounds.center_top() - egui::vec2(0.0, 20.0);
                painter.line_segment([bounds.center_top(), draw_rot], egui::Stroke::new(1.0, egui::Color32::WHITE));
                painter.circle_filled(draw_rot, 4.0, egui::Color32::WHITE);
                painter.circle_stroke(draw_rot, 4.0, egui::Stroke::new(1.0, egui::Color32::BLACK));

                if left_just_pressed && !*dragging_source_rect {
                    let mut hit = false;
                    if hit_rot.distance(pos) < 10.0 {
                        *line_start = Some(pos);
                        *drag_state = 1;
                        *initial_center = Some(raw_bounds.center());
                        *initial_layer = Some(layer.clone());
                        hit = true;
                    }
                    if !hit {
                        for (idx, pc) in hit_p_corners.iter().enumerate() {
                            if pc.distance(pos) < 10.0 {
                                *line_start = Some(pos);
                                *drag_state = 20 + idx;
                                *initial_center = Some(raw_bounds.center());
                                *initial_layer = Some(layer.clone());
                                hit = true; break;
                            }
                        }
                    }
                    if !hit {
                        for (idx, corner) in hit_corners.iter().enumerate() {
                            if egui::Rect::from_center_size(*corner, egui::vec2(12.0, 12.0)).contains(pos) {
                                *line_start = Some(pos);
                                *drag_state = 10 + idx;
                                *initial_bounds = Some(raw_bounds);
                                *initial_center = Some(raw_bounds.center());
                                *initial_layer = Some(layer.clone());
                                hit = true; break;
                            }
                        }
                    }
                    if !hit {
                        for (idx, &mp) in hit_mids.iter().enumerate() {
                            if egui::Rect::from_center_size(mp, egui::vec2(12.0, 12.0)).contains(pos) {
                                *line_start = Some(pos);
                                *drag_state = 2 + idx;
                                *initial_center = Some(raw_bounds.center());
                                *initial_layer = Some(layer.clone());
                                hit = true; break;
                            }
                        }
                    }
                    if !hit && raw_bounds.contains(pos) {
                        *line_start = Some(pos);
                        *drag_state = 0;
                        *initial_bounds = Some(raw_bounds);
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
                            if *drag_state == 1 {
                                // Rotation
                                let center = initial_center.unwrap();
                                let angle = (pos - center).angle() - (start - center).angle();
                                rotate_layer(layer, center, angle);
                            } else if *drag_state >= 20 {
                                // Perspective
                                let p_idx = *drag_state - 20;
                                let delta = pos - start;
                                for img in &mut layer.placed_images { img.perspective[p_idx] += delta; }
                                for s in &mut layer.strokes { s.perspective[p_idx] += delta; }
                                for ann in &mut layer.text_annotations { ann.perspective[p_idx] += delta; }
                            } else if *drag_state >= 10 {
                                // Resize
                                let handle_idx = *drag_state - 10;
                                let ib = initial_bounds.unwrap();
                                let ic = [ib.left_top(), ib.right_top(), ib.left_bottom(), ib.right_bottom()];
                                let anchor = ic[3 - handle_idx];
                                let old_vec = ic[handle_idx] - anchor;
                                let new_vec = pos - anchor;
                                if old_vec.x.abs() > 1.0 && old_vec.y.abs() > 1.0 {
                                    scale_layer(layer, anchor, egui::vec2(new_vec.x / old_vec.x, new_vec.y / old_vec.y));
                                }
                            } else if *drag_state >= 2 && *drag_state <= 5 {
                                // Skew
                                let handle_idx = *drag_state - 2;
                                let delta = pos - start;
                                let mut skew_delta = egui::vec2(0.0, 0.0);
                                if handle_idx == 0 || handle_idx == 1 { // Left or Right center -> Skew Y
                                    skew_delta.y = -delta.y * 0.01;
                                } else { // Top or Bottom center -> Skew X
                                    skew_delta.x = -delta.x * 0.01;
                                }
                                skew_layer(layer, initial_center.unwrap(), skew_delta);
                            } else {
                                // Translate
                                translate_layer(layer, pos - start);
                            }
                        }
                    }
                }
            }
        }
        if left_just_released {
            *line_start = None;
            *initial_layer = None;
            *dragging_source_rect = false;
            if active_layer_idx < project.layers.len() {
                let layer = &project.layers[active_layer_idx];
                if layer.strokes.is_empty() && layer.text_annotations.is_empty() && layer.placed_images.is_empty() {
                    if project.layers.len() > 1 { remove_active_layer = true; }
                }
            }
        }
    }
            Tool::Eraser => {
                let layer = &mut project.layers[active_layer_idx];
                if left_down {
                    let r = settings.pen_width.max(1.0);
                    let hit_test = |s: &crate::overlay::Stroke| -> bool {
                        if s.points.len() < 2 { return s.points.iter().any(|p| p.distance(pos) < r); }
                        match s.kind {
                            crate::overlay::StrokeKind::Freehand => s.points.iter().any(|p| p.distance(pos) < r),
                            crate::overlay::StrokeKind::Line => {
                                let (p0, p1) = (s.points[0], s.points[1]);
                                let len_sq = p0.distance_sq(p1);
                                if len_sq == 0.0 { return p0.distance(pos) < r; }
                                let t = ((pos.x - p0.x) * (p1.x - p0.x) + (pos.y - p0.y) * (p1.y - p0.y)) / len_sq;
                                let t = t.clamp(0.0, 1.0);
                                let proj = p0 + (p1 - p0) * t;
                                proj.distance(pos) < r
                            },
                            crate::overlay::StrokeKind::Rect => {
                                let rect = egui::Rect::from_two_pos(s.points[0], s.points[1]);
                                if let Some(bg) = s.background_color {
                                    if bg[3] > 0 && rect.contains(pos) { return true; }
                                }
                                rect.expand(r).contains(pos) && !rect.shrink(r).contains(pos)
                            },
                            crate::overlay::StrokeKind::Circle => {
                                let center = s.points[0];
                                let radius = s.points[0].distance(s.points[1]);
                                if let Some(bg) = s.background_color {
                                    if bg[3] > 0 && pos.distance(center) < radius { return true; }
                                }
                                (pos.distance(center) - radius).abs() < r
                            },
                            crate::overlay::StrokeKind::Star => {
                                let center = s.points[0];
                                let radius = s.points[0].distance(s.points[1]);
                                if let Some(bg) = s.background_color {
                                    if bg[3] > 0 && pos.distance(center) < radius { return true; }
                                }
                                // Rough distance check to star points
                                pos.distance(center) < radius + r && pos.distance(center) > (radius * 0.4) - r
                            },
                            crate::overlay::StrokeKind::Heart => {
                                let center = s.points[0];
                                let radius = s.points[0].distance(s.points[1]);
                                if let Some(bg) = s.background_color {
                                    if bg[3] > 0 && pos.distance(center) < radius { return true; }
                                }
                                // Bounding box check for heart
                                let rect = egui::Rect::from_center_size(center, egui::vec2(radius * 2.0, radius * 2.0));
                                rect.expand(r).contains(pos)
                            },
                            _ => s.points.iter().any(|p| p.distance(pos) < r),
                        }
                    };

                    if settings.eraser_mode == EraserMode::Stroke {
                        layer.strokes.retain(|s| !hit_test(s));
                    } else {
                        // Split mode
                        let mut new_strokes = Vec::new();
                        let mut keep_strokes = Vec::new();
                        
                        for s in layer.strokes.drain(..) {
                            if s.kind != crate::overlay::StrokeKind::Freehand {
                                if hit_test(&s) { continue; }
                                keep_strokes.push(s);
                                continue;
                            }
                        
                        let mut current_segment = Vec::new();
                        for p in s.points {
                            if p.distance(pos) < r {
                                if current_segment.len() >= 2 {
                                    let mut s2 = Stroke::new(current_segment.clone(), s.color, s.width, s.kind, s.brush_mode, s.background_color, s.shadow, s.brush_shape, s.outline);
                                    s2.opacity = s.opacity;
                                    new_strokes.push(s2);
                                }
                                current_segment.clear();
                            } else {
                                current_segment.push(p);
                            }
                        }
                        if current_segment.len() >= 2 {
                            let mut s2 = Stroke::new(current_segment, s.color, s.width, s.kind, s.brush_mode, s.background_color, s.shadow, s.brush_shape, s.outline);
                            s2.opacity = s.opacity;
                            new_strokes.push(s2);
                        }
                    }
                        layer.strokes.extend(keep_strokes);
                        layer.strokes.extend(new_strokes);
                    }
                layer.text_annotations.retain(|ann| ann.position.distance(pos) > r);

                if settings.eraser_mode == EraserMode::Stroke {
                    layer.placed_images.retain(|img| {
                        let disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
                        let disp_h = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[1];
                        let img_rect = egui::Rect::from_min_size(img.position, egui::vec2(disp_w, disp_h));
                        if settings.brush_shape == BrushShape::Square {
                            !img_rect.intersects(egui::Rect::from_center_size(pos, egui::vec2(r*2.0, r*2.0)))
                        } else {
                            img_rect.distance_to_pos(pos) > r
                        }
                    });
                } else {
                for img in &mut layer.placed_images {
                    let disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
                    let disp_h = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[1];
                    let img_rect = egui::Rect::from_min_size(img.position, egui::vec2(disp_w, disp_h));
                    
                    let overlaps = if settings.brush_shape == BrushShape::Square {
                        img_rect.intersects(egui::Rect::from_center_size(pos, egui::vec2(r*2.0, r*2.0)))
                    } else {
                        img_rect.distance_to_pos(pos) <= r
                    };

                    if overlaps {
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
                                let erase_hit = if settings.brush_shape == BrushShape::Square {
                                    (local_pos.x - pos.x).abs() <= r && (local_pos.y - pos.y).abs() <= r
                                } else {
                                    local_pos.distance(pos) < r
                                };
                                if erase_hit {
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
                            img.texture = None;
                        }
                    }
                }
            }
        }

        // ── Visual cursor: circle/square showing erase/brush radius ──
        if matches!(active_tool, Tool::Eraser | Tool::Brush) {
                let cursor_r = settings.pen_width.max(1.0);
                let color = if matches!(active_tool, Tool::Eraser) {
                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, 180)
                } else {
                    color32(&settings.pen_color)
                };
                let outline_color = if matches!(active_tool, Tool::Eraser) {
                    egui::Color32::from_rgba_unmultiplied(0, 0, 0, 120)
                } else {
                    let c = color32(&settings.pen_color);
                    if c.r() as u32 + c.g() as u32 + c.b() as u32 > 382 { egui::Color32::from_rgba_unmultiplied(0, 0, 0, 120) } else { egui::Color32::from_rgba_unmultiplied(255, 255, 255, 120) }
                };

                if settings.brush_shape == BrushShape::Square {
                    let rect = egui::Rect::from_center_size(pos, egui::vec2(cursor_r * 2.0, cursor_r * 2.0));
                    if matches!(active_tool, Tool::Brush) {
                        painter.rect_filled(rect, 0.0, color.gamma_multiply(0.2));
                    }
                    painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.5, color), egui::StrokeKind::Middle);
                    painter.rect_stroke(rect.expand(1.5), 0.0, egui::Stroke::new(0.8, outline_color), egui::StrokeKind::Middle);
                } else {
                    if matches!(active_tool, Tool::Brush) {
                        painter.circle_filled(pos, cursor_r, color.gamma_multiply(0.2));
                    }
                    painter.circle_stroke(pos, cursor_r, egui::Stroke::new(1.5, color));
                    painter.circle_stroke(pos, cursor_r + 1.5, egui::Stroke::new(0.8, outline_color));
                }
            }
        }
    }
}

    if remove_active_layer {
        project.layers.remove(project.active_layer);
        project.active_layer = project.active_layer.saturating_sub(1);
        project.selected_object = None; // Prevent crash by clearing selection to removed layer
    }

    if snip_created {
        project.layers.push(crate::project::Layer::new(&format!("Snip {}", project.layers.len() + 1)));
        project.active_layer = project.layers.len() - 1;
        let prev_idx = project.active_layer - 1;
        if let Some(snip) = project.layers[prev_idx].placed_images.pop() {
        
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
    
    if let Some(sel) = new_selection { project.selected_object = Some(sel); }
    if switch_to_move { *active_tool = Tool::Move; }
}

fn draw_quad(painter: &egui::Painter, p1: egui::Pos2, p2: egui::Pos2, p3: egui::Pos2, p4: egui::Pos2, color: egui::Color32) {
    let mut mesh = egui::Mesh::default();
    mesh.vertices.push(egui::epaint::Vertex { pos: p1, uv: egui::Pos2::ZERO, color });
    mesh.vertices.push(egui::epaint::Vertex { pos: p2, uv: egui::Pos2::ZERO, color });
    mesh.vertices.push(egui::epaint::Vertex { pos: p3, uv: egui::Pos2::ZERO, color });
    mesh.vertices.push(egui::epaint::Vertex { pos: p4, uv: egui::Pos2::ZERO, color });
    mesh.indices.extend([0, 1, 2, 0, 2, 3]);
    painter.add(egui::Shape::mesh(mesh));
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
