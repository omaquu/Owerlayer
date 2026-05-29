use eframe::egui;
use serde::{Deserialize, Serialize};
use crate::hotkey::HotkeyBinding;

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum ObjectType { Image, Stroke, Text }

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct SelectedObject {
    pub layer_idx: usize,
    pub object_type: ObjectType,
    pub object_idx: usize,
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct RasterizeRequest {
    pub layer_idx: usize,
    pub object_idx: Option<(ObjectType, usize)>,
}

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
pub enum ShapeType { Rect, Circle, Star, Heart, Arrow, Poly }

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
    PaintBucket,
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
            Tool::PaintBucket => "Paint Bucket Tool",
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
            Tool::Embed => "I",
            Tool::PaintBucket => "G",
        }
    }
}



#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BrushMode { Solid, Highlighter, Calligraphy, Spray, Real }

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum MirrorMode { Rect, Circle, Lasso, Poly, Window }
impl Default for MirrorMode { fn default() -> Self { Self::Rect } }

#[derive(Clone, Serialize, Deserialize)]
pub struct Stroke {
    #[serde(default = "default_stroke_id")]
    pub id: usize,
    #[serde(default = "default_stroke_name")]
    pub name: String,
    pub points: Vec<egui::Pos2>,
    pub color: [u8; 4],
    pub width: f32,
    pub kind: StrokeKind,
    pub brush_mode: BrushMode,
    #[serde(default)]
    pub background_color: Option<[u8; 4]>,
    #[serde(default)]
    pub grayscale: bool,
    #[serde(default)]
    pub invert: bool,
    #[serde(default)]
    pub sepia: bool,
    #[serde(default)]
    pub glow: bool,
    #[serde(default)]
    pub glow_strength: f32,
    #[serde(default = "default_glow_color")]
    pub glow_color: [u8; 4],
    #[serde(default)]
    pub glow_spread: f32,
    #[serde(default)]
    pub blur: f32,
    #[serde(default)]
    pub blur_effect: crate::types::BlurEffect,
    #[serde(default)]
    pub shadow: bool,
    #[serde(default)]
    pub shadow_spread: f32,
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
    #[serde(default = "default_scale")]
    pub scale: egui::Vec2,
    #[serde(default)]
    pub skew: egui::Vec2,
    #[serde(default)]
    pub outline: bool,
    #[serde(default)]
    pub arrow: bool,
    #[serde(default = "default_visible")]
    pub visible: bool,
    #[serde(default = "default_opacity")]
    pub opacity: f32,
    pub outline_color: [u8; 4],
    pub outline_width: f32,
    pub shadow_color: [u8; 4],
    pub shadow_offset: [f32; 2],
    pub shadow_blur: f32,
    #[serde(default)]
    pub locked: bool,
    #[serde(default = "default_stroke_spray_density")]
    pub spray_density: u32,
    #[serde(default = "default_stroke_highlight_opacity")]
    pub highlight_opacity: f32,
    #[serde(skip)]
    pub cached_texture: Option<egui::TextureHandle>,
    #[serde(skip)]
    pub cached_rect: Option<egui::Rect>,
}
fn default_stroke_spray_density() -> u32 { 40 }
fn default_stroke_highlight_opacity() -> f32 { 0.4 }

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
        arrow: bool,
        spray_density: u32,
        highlight_opacity: f32,
    ) -> Self {
        let name = match kind {
            StrokeKind::Rect => "Rectangle".to_string(),
            StrokeKind::Circle => "Circle".to_string(),
            StrokeKind::Star => "Star".to_string(),
            StrokeKind::Heart => "Heart".to_string(),
            StrokeKind::Line => "Line".to_string(),
            StrokeKind::Arrow => "Arrow".to_string(),
            StrokeKind::Blur => "Blur".to_string(),
            StrokeKind::Poly => "Polygon".to_string(),
            _ => "Stroke".to_string(),
        };
        Self {
            id: default_stroke_id(),
            name,
            points,
            color,
            width,
            kind,
            brush_mode,
            background_color,
            shadow, shadow_spread: 0.0,
            brush_shape,
            outline,
            arrow,
            visible: true,
            opacity: 1.0,
            rotation: 0.0,
            flipped_h: false,
            flipped_v: false,
            perspective: [egui::Vec2::ZERO; 4],
            scale: egui::vec2(1.0, 1.0),
            skew: egui::Vec2::ZERO,
            outline_color: [0, 0, 0, 0],
            outline_width: 0.0,
            shadow_color: [0, 0, 0, 0],
            shadow_offset: [0.0, 0.0],
            shadow_blur: 0.0,
            grayscale: false,
            invert: false,
            sepia: false,
            glow: false,
            glow_strength: 0.0, glow_color: [255, 255, 255, 255], glow_spread: 0.0,
            blur: 0.0,
            blur_effect: BlurEffect::Gaussian,
            locked: false,
            spray_density,
            highlight_opacity,
            cached_texture: None,
            cached_rect: None,
        }
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum StrokeKind { Freehand, Line, Arrow, Rect, Circle, Star, Heart, Blur, Poly }

#[derive(Clone, Serialize, Deserialize)]
pub struct TextAnnotation {
    #[serde(default = "default_text_name")]
    pub name: String,
    pub position: egui::Pos2,
    pub text: String,
    pub color: [u8; 4],
    pub font_size: f32,
    pub monospace: bool,
    pub shadow: bool,
    #[serde(default)]
    pub shadow_spread: f32,
    pub outline: bool,
    #[serde(default = "default_outline_color")]
    pub outline_color: [u8; 4],
    #[serde(default = "default_outline_width")]
    pub outline_width: f32,
    #[serde(default = "default_shadow_color")]
    pub shadow_color: [u8; 4],
    #[serde(default = "default_shadow_offset")]
    pub shadow_offset: [f32; 2],
    #[serde(default)]
    pub shadow_blur: f32,
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
    #[serde(default = "default_scale")]
    pub scale: egui::Vec2,
    #[serde(default)]
    pub skew: egui::Vec2,
    #[serde(default = "default_visible")]
    pub visible: bool,
    #[serde(default = "default_opacity")]
    pub opacity: f32,
    #[serde(default)]
    pub wave_warp: bool,
    #[serde(default)]
    pub exact_size: [f32; 2],
    #[serde(default)]
    pub grayscale: bool,
    #[serde(default)]
    pub invert: bool,
    #[serde(default)]
    pub sepia: bool,
    #[serde(default)]
    pub glow: bool,
    #[serde(default)]
    pub glow_strength: f32,
    #[serde(default = "default_glow_color")]
    pub glow_color: [u8; 4],
    #[serde(default)]
    pub glow_spread: f32,
    #[serde(default)]
    pub blur: f32,
    #[serde(default)]
    pub blur_effect: crate::overlay::BlurEffect,
    #[serde(default)]
    pub locked: bool,
    #[serde(skip)]
    pub cached_texture: Option<egui::TextureHandle>,
    #[serde(skip)]
    pub cached_rect: Option<egui::Rect>,
}

impl TextAnnotation {
    pub fn new(position: egui::Pos2, text: String, color: [u8; 4], font_size: f32) -> Self {
        Self {
            name: "Text".to_string(),
            position, text, color, font_size,
            monospace: false, shadow: false, outline: false,
            outline_color: [0, 0, 0, 0], outline_width: 1.0,
            shadow_color: [0, 0, 0, 0], shadow_offset: [0.0, 0.0], shadow_blur: 0.0, shadow_spread: 0.0,
            stroke_width: 1.0,
            rotation: 0.0, flipped_h: false, flipped_v: false,
            perspective: [egui::Vec2::ZERO; 4],
            font: TextFont::Sans,
            scale: egui::vec2(1.0, 1.0),
            skew: egui::Vec2::ZERO,
            visible: true,
            opacity: 1.0,
            wave_warp: false,
            exact_size: [0.0, 0.0],
            grayscale: false,
            invert: false,
            sepia: false,
            glow: false,
            glow_strength: 0.0, glow_color: [255, 255, 255, 255], glow_spread: 0.0,
            blur: 0.0,
            blur_effect: crate::overlay::BlurEffect::Gaussian,
            locked: false,
            cached_texture: None,
            cached_rect: None,
        }
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum TextFont { Sans, Serif, Mono, Handwriting, Heading, Custom }

impl Default for TextFont { fn default() -> Self { Self::Sans } }

#[derive(Serialize, Deserialize)]
pub struct PlacedImage {
    #[serde(default = "default_image_name")]
    pub name: String,
    pub id: usize,
    pub position: egui::Pos2,
    pub size: [usize; 2],
    pub display_size: Option<[f32; 2]>,
    #[serde(skip)]
    pub pixels: Vec<u8>,
    #[serde(skip)]
    pub texture: Option<egui::TextureHandle>,
    pub shadow: bool,
    #[serde(default)]
    pub shadow_spread: f32,
    pub rotation: f32,
    pub flipped_h: bool,
    pub flipped_v: bool,
    #[serde(default)]
    pub perspective: [egui::Vec2; 4],
    #[serde(default = "default_scale")]
    pub scale: egui::Vec2,
    #[serde(default)]
    pub skew: egui::Vec2,
    #[serde(default = "default_visible")]
    pub visible: bool,
    #[serde(default = "default_opacity")]
    pub opacity: f32,
    pub is_live: bool,
    #[serde(default)]
    pub outline: bool,
    #[serde(default = "default_outline_color")]
    pub outline_color: [u8; 4],
    #[serde(default = "default_outline_width")]
    pub outline_width: f32,
    #[serde(default = "default_shadow_color")]
    pub shadow_color: [u8; 4],
    #[serde(default = "default_shadow_offset")]
    pub shadow_offset: [f32; 2],
    #[serde(default)]
    pub shadow_blur: f32,
    pub source_rect: Option<[f32; 4]>,
    pub url: Option<String>,
    #[serde(default)]
    pub blur: f32,
    #[serde(default)]
    pub blur_effect: BlurEffect,
    #[serde(default)]
    pub grayscale: bool,
    #[serde(default)]
    pub invert: bool,
    #[serde(default)]
    pub sepia: bool,
    #[serde(default)]
    pub glow: bool,
    #[serde(default)]
    pub glow_strength: f32,
    #[serde(default = "default_glow_color")]
    pub glow_color: [u8; 4],
    #[serde(default)]
    pub glow_spread: f32,
    #[serde(skip)]
    pub mask: Option<Vec<u8>>,
    #[serde(skip)]
    pub mask_texture: Option<egui::TextureHandle>,
    #[serde(skip)]
    pub mask_dirty: bool,
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
    pub web_widget: Option<std::sync::Arc<std::sync::Mutex<crate::web_engine::WebWidget>>>,
    #[serde(skip)]
    pub tight_bounds: Option<egui::Rect>,
    #[serde(skip)]
    pub tight_bounds_dirty: bool,
    #[serde(skip)]
    pub thumbnail_texture: Option<egui::TextureHandle>,
    #[serde(skip)]
    pub thumbnail_dirty: bool,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub snip_points: Option<Vec<egui::Pos2>>,
    #[serde(skip)]
    pub cached_texture: Option<egui::TextureHandle>,
    #[serde(skip)]
    pub cached_rect: Option<egui::Rect>,
}

impl Clone for PlacedImage {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            id: self.id,
            position: self.position,
            size: self.size,
            display_size: self.display_size,
            pixels: self.pixels.clone(),
            texture: None,
            shadow: self.shadow,
            shadow_spread: self.shadow_spread,
            shadow_offset: self.shadow_offset,
            shadow_color: self.shadow_color,
            shadow_blur: self.shadow_blur,
            glow: self.glow,
            glow_strength: self.glow_strength,
            glow_color: self.glow_color,
            glow_spread: self.glow_spread,
            rotation: self.rotation,
            flipped_h: self.flipped_h,
            flipped_v: self.flipped_v,
            perspective: self.perspective,
            scale: self.scale,
            skew: self.skew,
            visible: self.visible,
            opacity: self.opacity,
            is_live: self.is_live,
            outline: self.outline,
            outline_color: self.outline_color,
            outline_width: self.outline_width,
            
            
            
            
            source_rect: self.source_rect,
            url: self.url.clone(),
            blur: self.blur,
            blur_effect: self.blur_effect,
            mask: self.mask.clone(),
            mask_texture: None,
            show_source_rect: self.show_source_rect,
            frames: self.frames.clone(),
            frame_durations: self.frame_durations.clone(),
            current_frame: self.current_frame,
            last_frame_time: self.last_frame_time,
            hwnd: self.hwnd,
            #[cfg(feature = "webengine")]
            web_widget: self.web_widget.clone(),
            tight_bounds: self.tight_bounds,
            tight_bounds_dirty: self.tight_bounds_dirty,
            thumbnail_texture: None,
            thumbnail_dirty: self.thumbnail_dirty,
            mask_dirty: self.mask_dirty,
            grayscale: self.grayscale,
            invert: self.invert,
            sepia: self.sepia,
            
            
            locked: self.locked,
            snip_points: self.snip_points.clone(),
            cached_texture: None,
            cached_rect: None,
        }
    }
}

impl PlacedImage {
    pub fn new(id: usize, position: egui::Pos2, size: [usize; 2], pixels: Vec<u8>) -> Self {
        Self {
            name: "Image".to_string(),
            id, position, size, pixels,
            display_size: Some([size[0] as f32, size[1] as f32]),
            texture: None, shadow: false, rotation: 0.0,
            flipped_h: false, flipped_v: false,
            perspective: [egui::Vec2::ZERO; 4],
            scale: egui::vec2(1.0, 1.0),
            skew: egui::Vec2::ZERO,
            visible: true,
            opacity: 1.0,
            is_live: false,
            outline: false,
            outline_color: [0, 0, 0, 0],
            outline_width: 1.0,
            shadow_color: [0, 0, 0, 0],
            shadow_offset: [0.0, 0.0],
            shadow_blur: 0.0,
            shadow_spread: 0.0,
            source_rect: None,
            url: None,
            blur: 0.0,
            blur_effect: BlurEffect::Gaussian,
            mask: None,
            mask_texture: None,
            show_source_rect: false,
            frames: Vec::new(),
            frame_durations: Vec::new(),
            current_frame: 0,
            last_frame_time: 0.0,
            hwnd: 0,
            #[cfg(feature = "webengine")]
            web_widget: None,
            tight_bounds: None,
            tight_bounds_dirty: true,
            thumbnail_texture: None,
            thumbnail_dirty: true,
            mask_dirty: false,
            grayscale: false,
            invert: false,
            sepia: false,
            glow: false,
            glow_strength: 0.0,
            glow_color: [255, 255, 255, 255],
            glow_spread: 0.0,
            locked: false,
            snip_points: None,
            cached_texture: None,
            cached_rect: None,
        }
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SnipMode { Rect, Circle, Lasso, Polygon, Star, Heart, Window }

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
fn default_visible() -> bool { true }
fn default_scale() -> egui::Vec2 { egui::vec2(1.0, 1.0) }

static STROKE_ID_GEN: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
fn default_stroke_id() -> usize {
    use std::sync::atomic::Ordering;
    use std::time::SystemTime;
    let base = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
    base + STROKE_ID_GEN.fetch_add(1, Ordering::Relaxed)
}

fn default_stroke_name() -> String { "Stroke".to_string() }
fn default_text_name() -> String { "Text".to_string() }
fn default_image_name() -> String { "Image".to_string() }

fn default_outline_color() -> [u8; 4] { [0, 0, 0, 255] }
fn default_outline_width() -> f32 { 1.0 }
fn default_shadow_color() -> [u8; 4] { [0, 0, 0, 255] }
fn default_shadow_offset() -> [f32; 2] { [0.0, 0.0] }
fn default_glow_color() -> [u8; 4] { [255, 255, 255, 255] }

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum EraserMode { Stroke, Pixel }

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CutMode { Rect, Circle, Lasso, Polygon, MagicWand }

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
    #[serde(default = "default_polygon_sides")]
    pub polygon_sides: u32,
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
    pub brush_arrow: bool,
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
    pub text_wave_warp: bool,
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
    #[serde(default)]
    pub virtual_matrix: bool,
    #[serde(default)]
    pub monitor_lock: Option<usize>,
    #[serde(default = "default_capture_fps")]
    pub capture_fps: f32,
    #[serde(default)]
    pub preferred_gpu: Option<String>,
    #[serde(default = "default_fso_fix")]
    pub fso_fix: bool,
    #[serde(default)]
    pub is_vertical: bool,
    #[serde(skip)]
    pub fx_open: Option<SelectedObject>,
    #[serde(default = "default_toolbar_pos")]
    pub toolbar_pos: egui::Pos2,
    #[serde(default = "default_layer_menu_pos")]
    pub layer_menu_pos: egui::Pos2,
    #[serde(default)]
    pub auto_new_layer: Option<bool>,
    #[serde(default = "default_prompt_delete")]
    pub prompt_delete_layer: bool,
    #[serde(default = "default_settings_menu_pos")]
    pub settings_menu_pos: egui::Pos2,
    #[serde(default = "default_filter_menu_pos")]
    pub filter_menu_pos: egui::Pos2,
    #[serde(default = "default_history_menu_pos")]
    pub history_menu_pos: egui::Pos2,
    #[serde(default = "default_object_fx_menu_pos")]
    pub object_fx_menu_pos: egui::Pos2,
    #[serde(default = "default_creation_prompt_pos")]
    pub creation_prompt_pos: egui::Pos2,
    #[serde(default)]
    pub saved_embed_urls: Vec<(String, String)>,
    #[serde(default = "default_spray_density")]
    pub spray_density: u32,
    #[serde(default = "default_highlight_opacity")]
    pub highlight_opacity: f32,
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum BlurEffect { Gaussian, Pixelate, Glitch }

impl Default for BlurEffect { fn default() -> Self { Self::Gaussian } }

fn default_toolbar_bg() -> [u8; 4] { [30, 30, 30, 220] }

fn default_blur_strength() -> f32 { 10.0 }
fn default_capture_fps() -> f32 { 15.0 }
fn default_fso_fix() -> bool { true }
fn default_polygon_sides() -> u32 { 5 }
fn default_toolbar_pos() -> egui::Pos2 { egui::pos2(40.0, 60.0) }
fn default_layer_menu_pos() -> egui::Pos2 { egui::pos2(200.0, 60.0) }
fn default_prompt_delete() -> bool { true }
fn default_settings_menu_pos() -> egui::Pos2 { egui::pos2(360.0, 60.0) }
fn default_filter_menu_pos() -> egui::Pos2 { egui::pos2(520.0, 60.0) }
fn default_history_menu_pos() -> egui::Pos2 { egui::pos2(680.0, 60.0) }
fn default_object_fx_menu_pos() -> egui::Pos2 { egui::pos2(840.0, 60.0) }
fn default_creation_prompt_pos() -> egui::Pos2 { egui::pos2(500.0, 300.0) }
fn default_spray_density() -> u32 { 40 }
fn default_highlight_opacity() -> f32 { 0.4 }

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
            toggle_mode: true,
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
            polygon_sides: 5,
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
            text_wave_warp: false,
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
            virtual_matrix: false,
            monitor_lock: None,
            capture_fps: default_capture_fps(),
            preferred_gpu: None,
            fso_fix: true,
            is_vertical: false,
            brush_arrow: false,
            fx_open: None,
            toolbar_pos: default_toolbar_pos(),
            layer_menu_pos: default_layer_menu_pos(),
            auto_new_layer: None,
            prompt_delete_layer: true,
            settings_menu_pos: default_settings_menu_pos(),
            filter_menu_pos: default_filter_menu_pos(),
            history_menu_pos: default_history_menu_pos(),
            object_fx_menu_pos: default_object_fx_menu_pos(),
            creation_prompt_pos: default_creation_prompt_pos(),
            saved_embed_urls: Vec::new(),
            spray_density: default_spray_density(),
            highlight_opacity: default_highlight_opacity(),
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
