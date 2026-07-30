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
    pub fn poll(prev_down: bool, prev_pos: egui::Pos2, ppp: f32, multi_monitor: bool) -> Self {
        let (x_px, y_px, down) = crate::winapi_utils::poll_mouse();
        let (wx_f, wy_f) = if let Some((wx, wy)) = crate::winapi_utils::get_window_screen_pos_opt() {
            (wx as f32, wy as f32)
        } else {
            if multi_monitor {
                let (vx, vy) = crate::winapi_utils::get_virtual_origin();
                (vx - 1.0, vy - 1.0)
            } else {
                (-1.0, -1.0)
            }
        };
        // egui_point = (screen_pixel - window_origin_pixel) / ppp
        let pos = egui::pos2((x_px - wx_f) / ppp, (y_px - wy_f) / ppp);
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

#[derive(Clone, Debug, PartialEq)]
pub enum SelectionShape {
    Rect(egui::Rect),
    Circle { center: egui::Pos2, radius: f32 },
    Poly(Vec<egui::Pos2>),
}

impl SelectionShape {
    pub fn bounds(&self) -> egui::Rect {
        match self {
            SelectionShape::Rect(r) => *r,
            SelectionShape::Circle { center, radius } => {
                egui::Rect::from_center_size(*center, egui::vec2(radius * 2.0, radius * 2.0))
            }
            SelectionShape::Poly(pts) => {
                if pts.is_empty() {
                    egui::Rect::ZERO
                } else {
                    egui::Rect::from_points(pts)
                }
            }
        }
    }

    pub fn contains(&self, p: egui::Pos2) -> bool {
        match self {
            SelectionShape::Rect(r) => r.contains(p),
            SelectionShape::Circle { center, radius } => p.distance(*center) <= *radius,
            SelectionShape::Poly(pts) => crate::utils::is_inside_poly(pts, p),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum SelectionOp {
    Add(SelectionShape),
    Subtract(SelectionShape),
}

#[derive(Clone, Debug, PartialEq)]
pub struct MarqueeSelection {
    pub shape: SelectionShape,
    pub ops: Vec<SelectionOp>,
}

impl MarqueeSelection {
    pub fn contains(&self, p: egui::Pos2) -> bool {
        let mut inside = self.shape.contains(p);
        for op in &self.ops {
            match op {
                SelectionOp::Add(sh) => {
                    if sh.contains(p) {
                        inside = true;
                    }
                }
                SelectionOp::Subtract(sh) => {
                    if sh.contains(p) {
                        inside = false;
                    }
                }
            }
        }
        inside
    }

    pub fn bounds(&self) -> egui::Rect {
        let mut r = self.shape.bounds();
        for op in &self.ops {
            match op {
                SelectionOp::Add(sh) => {
                    r = r.union(sh.bounds());
                }
                SelectionOp::Subtract(_) => {}
            }
        }
        r
    }

    pub fn translate(&mut self, delta: egui::Vec2) {
        match &mut self.shape {
            SelectionShape::Rect(r) => { *r = r.translate(delta); }
            SelectionShape::Circle { center, .. } => { *center += delta; }
            SelectionShape::Poly(pts) => { for p in pts { *p += delta; } }
        }
        for op in &mut self.ops {
            match op {
                SelectionOp::Add(sh) | SelectionOp::Subtract(sh) => {
                    match sh {
                        SelectionShape::Rect(r) => { *r = r.translate(delta); }
                        SelectionShape::Circle { center, .. } => { *center += delta; }
                        SelectionShape::Poly(pts) => { for p in pts { *p += delta; } }
                    }
                }
            }
        }
    }
}

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
            Tool::Cut => "Marquee Tool",
            Tool::Mirror => "Mirror Tool",
            Tool::Blur => "Blur Tool",
            Tool::Embed => "Widgets Tool",
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
    #[serde(default)]
    pub arrow_size: ArrowSize,
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
        arrow_size: ArrowSize,
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
            arrow_size,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum WidgetType {
    Calculator,
    VolumeMixer,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum WidgetPlaceType {
    Browser,
    Calculator,
    VolumeMixer,
}

impl Default for WidgetPlaceType {
    fn default() -> Self {
        WidgetPlaceType::Browser
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalcState {
    pub display: String,
    pub prev_val: Option<f64>,
    pub op: Option<char>,
    pub clear_on_next: bool,
}

impl Default for CalcState {
    fn default() -> Self {
        Self {
            display: "0".to_string(),
            prev_val: None,
            op: None,
            clear_on_next: false,
        }
    }
}

impl CalcState {
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    
    pub fn push_char(&mut self, c: char) {
        if self.clear_on_next {
            self.display.clear();
            self.clear_on_next = false;
        }
        if self.display == "0" && c != '.' {
            self.display.clear();
        }
        if c == '.' && self.display.contains('.') {
            return;
        }
        self.display.push(c);
    }
    
    pub fn negate(&mut self) {
        if self.display.starts_with('-') {
            self.display.remove(0);
        } else if self.display != "0" && !self.display.is_empty() {
            self.display.insert(0, '-');
        }
    }
    
    pub fn percent(&mut self) {
        if let Ok(val) = self.display.parse::<f64>() {
            let res = val / 100.0;
            self.display = res.to_string();
        }
    }
    
    pub fn backspace(&mut self) {
        if self.clear_on_next {
            self.clear_on_next = false;
            self.display = "0".to_string();
            return;
        }
        self.display.pop();
        if self.display.is_empty() || self.display == "-" {
            self.display = "0".to_string();
        }
    }
    
    pub fn set_op(&mut self, op: char) {
        if let Ok(val) = self.display.parse::<f64>() {
            if let Some(prev) = self.prev_val {
                if let Some(prev_op) = self.op {
                    let res = match prev_op {
                        '+' => prev + val,
                        '-' => prev - val,
                        '*' => prev * val,
                        '/' => if val != 0.0 { prev / val } else { f64::NAN },
                        _ => val,
                    };
                    self.prev_val = Some(res);
                    self.display = res.to_string();
                }
            } else {
                self.prev_val = Some(val);
            }
            self.op = Some(op);
            self.clear_on_next = true;
        }
    }
    
    pub fn calculate(&mut self) {
        if let Some(prev) = self.prev_val {
            if let Some(op) = self.op {
                if let Ok(val) = self.display.parse::<f64>() {
                    let res = match op {
                        '+' => prev + val,
                        '-' => prev - val,
                        '*' => prev * val,
                        '/' => if val != 0.0 { prev / val } else { f64::NAN },
                        _ => val,
                    };
                    self.display = res.to_string();
                    self.prev_val = None;
                    self.op = None;
                    self.clear_on_next = true;
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MixerMode {
    Full,
    Single,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SliderOrientation {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SliderStyle {
    SleekPill,
    ThinMetal,
    ThickTicks,
    GradientBar,
    Knob,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VolumeMixerState {
    pub mode: MixerMode,
    pub target_session: String,
    pub orientation: SliderOrientation,
    pub style: SliderStyle,
}

impl Default for VolumeMixerState {
    fn default() -> Self {
        Self {
            mode: MixerMode::Full,
            target_session: "Master".to_string(),
            orientation: SliderOrientation::Horizontal,
            style: SliderStyle::SleekPill,
        }
    }
}

#[derive(Clone)]
pub struct GifRecorderState {
    pub is_recording: bool,
    pub is_encoding: bool,
    pub target_duration_secs: u32,
    pub start_time: Option<std::time::Instant>,
    pub last_sample_time: Option<std::time::Instant>,
    pub frames: Vec<(Vec<u8>, [usize; 2])>,
    pub status: String,
}

impl Default for GifRecorderState {
    fn default() -> Self {
        Self {
            is_recording: false,
            is_encoding: false,
            target_duration_secs: 5,
            start_time: None,
            last_sample_time: None,
            frames: Vec::new(),
            status: String::new(),
        }
    }
}

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
    pub mask_size: Option<[usize; 2]>,
    #[serde(skip)]
    pub mask_texture: Option<egui::TextureHandle>,
    #[serde(skip)]
    pub mask_dirty: bool,
    #[serde(skip)]
    pub cached_mask_outline: Option<Vec<Vec<egui::Pos2>>>,
    pub show_source_rect: bool,
    #[serde(default)]
    pub snip_source_overlay: bool,
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
    #[serde(skip)]
    pub texture_version: usize,
    #[serde(default)]
    pub widget_type: Option<WidgetType>,
    #[serde(default)]
    pub calculator_state: Option<CalcState>,
    #[serde(default)]
    pub volume_mixer_state: Option<VolumeMixerState>,
    #[serde(skip)]
    pub gif_recorder: GifRecorderState,
    #[serde(default)]
    pub chromatic_aberration: f32,
    #[serde(default)]
    pub antialias: bool,
    #[serde(default)]
    pub antialias_outline: bool,
    #[serde(default = "default_true")]
    pub antialias_shadow: bool,
    #[serde(default)]
    pub eraser_apply_to_source: Option<bool>,
    #[serde(default)]
    pub source_rotation: f32,
    #[serde(default)]
    pub source_skew: egui::Vec2,
    #[serde(default)]
    pub source_perspective: [egui::Vec2; 4],
    #[serde(default = "default_source_scale")]
    pub source_scale: egui::Vec2,
    #[serde(default = "default_capture_source")]
    pub capture_source: CaptureSource,
    #[serde(default = "default_target_hwnd")]
    pub target_hwnd: isize,
    #[serde(default)]
    pub transparent_bg: bool,
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
            mask_size: self.mask_size,
            mask_texture: None,
            show_source_rect: self.show_source_rect,
            snip_source_overlay: self.snip_source_overlay,
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
            cached_mask_outline: None,
            grayscale: self.grayscale,
            invert: self.invert,
            sepia: self.sepia,
            locked: self.locked,
            snip_points: self.snip_points.clone(),
            cached_texture: None,
            cached_rect: None,
            texture_version: self.texture_version,
            chromatic_aberration: self.chromatic_aberration,
            antialias: self.antialias,
            antialias_outline: self.antialias_outline,
            antialias_shadow: self.antialias_shadow,
            eraser_apply_to_source: self.eraser_apply_to_source,
            source_rotation: self.source_rotation,
            source_skew: self.source_skew,
            source_perspective: self.source_perspective,
            source_scale: self.source_scale,
            capture_source: self.capture_source,
            target_hwnd: self.target_hwnd,
            widget_type: self.widget_type,
            calculator_state: self.calculator_state.clone(),
            volume_mixer_state: self.volume_mixer_state.clone(),
            gif_recorder: self.gif_recorder.clone(),
            transparent_bg: self.transparent_bg,
        }
    }
}

impl PlacedImage {
    pub fn new(id: usize, position: egui::Pos2, size: [usize; 2], pixels: Vec<u8>) -> Self {
        Self {
            gif_recorder: GifRecorderState::default(),
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
            mask_size: None,
            mask_texture: None,
            show_source_rect: true,
            snip_source_overlay: false,
            frames: Vec::new(),
            frame_durations: Vec::new(),
            current_frame: 0,
            last_frame_time: 0.0,
            hwnd: 0,
            #[cfg(feature = "webengine")]
            web_widget: None,
            widget_type: None,
            calculator_state: None,
            volume_mixer_state: None,
            tight_bounds: None,
            tight_bounds_dirty: true,
            thumbnail_texture: None,
            thumbnail_dirty: true,
            mask_dirty: false,
            cached_mask_outline: None,
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
            texture_version: 0,
            chromatic_aberration: 0.0,
            antialias: false,
            antialias_outline: false,
            antialias_shadow: true,
            eraser_apply_to_source: None,
            source_rotation: 0.0,
            source_skew: egui::Vec2::ZERO,
            source_perspective: [egui::Vec2::ZERO; 4],
            source_scale: egui::vec2(1.0, 1.0),
            capture_source: CaptureSource::Desktop,
            target_hwnd: 0,
            transparent_bg: false,
        }
    }

    pub fn clear_texture(&mut self) {
        self.texture = None;
        self.thumbnail_texture = None;
        self.texture_version += 1;
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum CaptureSource {
    Desktop,
    Overlay,
    Origin,
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SnipMode { Rect, Circle, Lasso, Polygon, Star, Heart, Window }

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BrushShape { Round, Square }

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum ArrowSize { Small, Medium, Large }

impl Default for ArrowSize { fn default() -> Self { Self::Medium } }

pub struct PendingText {
    pub position: egui::Pos2,
    pub buffer: String,
    pub original: Option<TextAnnotation>,
    pub layer_idx: Option<usize>,
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

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum CutMode { Rect, Circle, Lasso, Polygon, MagicWand, Star, Heart }

impl Default for CutMode { fn default() -> Self { Self::Rect } }

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum SelectionMode { New, Add, Subtract }

impl Default for SelectionMode { fn default() -> Self { Self::New } }

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct Settings {
    pub hotkey: HotkeyBinding,
    #[serde(default = "default_snip_source")]
    pub snip_source: CaptureSource,
    #[serde(default = "default_target_hwnd")]
    pub origin_target_hwnd: isize,
    #[serde(default = "default_keybind_move")]
    pub keybind_move: HotkeyBinding,
    #[serde(default = "default_keybind_brush")]
    pub keybind_brush: HotkeyBinding,
    #[serde(default = "default_keybind_eraser")]
    pub keybind_eraser: HotkeyBinding,
    #[serde(default = "default_keybind_text")]
    pub keybind_text: HotkeyBinding,
    #[serde(default = "default_keybind_shape")]
    pub keybind_shape: HotkeyBinding,
    #[serde(default = "default_keybind_snip")]
    pub keybind_snip: HotkeyBinding,
    #[serde(default = "default_keybind_cut")]
    pub keybind_cut: HotkeyBinding,
    #[serde(default = "default_keybind_mirror")]
    pub keybind_mirror: HotkeyBinding,
    #[serde(default = "default_keybind_blur")]
    pub keybind_blur: HotkeyBinding,
    #[serde(default = "default_keybind_paint_bucket")]
    pub keybind_paint_bucket: HotkeyBinding,
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
    #[serde(default)]
    pub selection_mode: SelectionMode,
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
    #[serde(default = "default_brush_hardness")]
    pub brush_hardness: f32,
    #[serde(default = "default_brush_spacing")]
    pub brush_spacing: f32,
    #[serde(default)]
    pub brush_arrow: bool,
    #[serde(default)]
    pub arrow_size: ArrowSize,
    #[serde(default)]
    pub hide_all: bool,
    #[serde(default)]
    pub auto_hide_seconds: f32,
    #[serde(default)]
    pub exclude_from_capture: bool,
    /// Auto-hide overlay from capture while live desktop snip is active (prevents mirror loop).
    #[serde(default = "default_auto_exclude_live")]
    pub auto_exclude_live_capture: bool,
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
    #[serde(default = "default_show_screen_controls")]
    pub show_screen_controls: bool,
    #[serde(default)]
    pub snip_source_overlay: bool,
    #[serde(default)]
    pub live_performance_mode: bool,
    #[serde(default)]
    pub show_source_rect: bool,
    #[serde(default)]
    pub show_profiler: bool,
    #[serde(skip)]
    pub eraser_source_prompt_open: bool,
    #[serde(skip)]
    pub eraser_source_prompt_target: Option<(usize, usize)>,
    #[serde(skip)]
    pub eraser_source_prompt_remember: bool,
    #[serde(default)]
    pub use_original_capture: bool,
    #[serde(skip)]
    pub ui_reset_frames: u8,
    #[serde(default)]
    pub show_grid: bool,
    #[serde(default = "default_grid_size")]
    pub grid_size: f32,
    #[serde(default = "default_snap_to_grid")]
    pub snap_to_grid: bool,
    #[serde(default)]
    pub show_calculator: bool,
    #[serde(default)]
    pub show_volume_mixer: bool,
    #[serde(default = "default_keybind_calculator")]
    pub keybind_calculator: HotkeyBinding,
    #[serde(default = "default_keybind_volume_mixer")]
    pub keybind_volume_mixer: HotkeyBinding,
    #[serde(default)]
    pub selected_mixer_apps: Vec<String>,
    #[serde(default = "default_calculator_pos")]
    pub calculator_pos: egui::Pos2,
    #[serde(default = "default_volume_mixer_pos")]
    pub volume_mixer_pos: egui::Pos2,
    #[serde(default)]
    pub widget_place_type: WidgetPlaceType,
    #[serde(default = "default_layer_menu_size")]
    pub layer_menu_size: egui::Vec2,
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum BlurEffect { Gaussian, Pixelate, Glitch }

impl Default for BlurEffect { fn default() -> Self { Self::Gaussian } }

fn default_toolbar_bg() -> [u8; 4] { [30, 30, 30, 220] }

fn default_blur_strength() -> f32 { 0.0 }
fn default_capture_fps() -> f32 { 60.0 }
fn default_fso_fix() -> bool { true }
fn default_polygon_sides() -> u32 { 5 }
fn default_toolbar_pos() -> egui::Pos2 { egui::pos2(40.0, 60.0) }
fn default_layer_menu_pos() -> egui::Pos2 { egui::pos2(200.0, 60.0) }
fn default_layer_menu_size() -> egui::Vec2 { egui::vec2(340.0, 500.0) }
fn default_prompt_delete() -> bool { true }
fn default_settings_menu_pos() -> egui::Pos2 { egui::pos2(360.0, 60.0) }
fn default_filter_menu_pos() -> egui::Pos2 { egui::pos2(520.0, 60.0) }
fn default_history_menu_pos() -> egui::Pos2 { egui::pos2(680.0, 60.0) }
fn default_object_fx_menu_pos() -> egui::Pos2 { egui::pos2(840.0, 60.0) }
fn default_creation_prompt_pos() -> egui::Pos2 { egui::pos2(500.0, 300.0) }
fn default_spray_density() -> u32 { 40 }
fn default_highlight_opacity() -> f32 { 0.4 }
fn default_show_screen_controls() -> bool { true }
fn default_auto_exclude_live() -> bool { true }

fn default_capture_source() -> CaptureSource { CaptureSource::Desktop }
fn default_target_hwnd() -> isize { 0 }
fn default_source_scale() -> egui::Vec2 { egui::vec2(1.0, 1.0) }
fn default_snip_source() -> CaptureSource { CaptureSource::Desktop }

fn default_keybind_move() -> HotkeyBinding { HotkeyBinding { vk_code: 0x56, name: "Ctrl + V".to_string(), ctrl: true, alt: false, shift: false } }
fn default_keybind_brush() -> HotkeyBinding { HotkeyBinding { vk_code: 0x42, name: "Ctrl + B".to_string(), ctrl: true, alt: false, shift: false } }
fn default_keybind_eraser() -> HotkeyBinding { HotkeyBinding { vk_code: 0x45, name: "Ctrl + E".to_string(), ctrl: true, alt: false, shift: false } }
fn default_keybind_text() -> HotkeyBinding { HotkeyBinding { vk_code: 0x54, name: "Ctrl + T".to_string(), ctrl: true, alt: false, shift: false } }
fn default_keybind_shape() -> HotkeyBinding { HotkeyBinding { vk_code: 0x55, name: "Ctrl + U".to_string(), ctrl: true, alt: false, shift: false } }
fn default_keybind_snip() -> HotkeyBinding { HotkeyBinding { vk_code: 0x53, name: "Ctrl + S".to_string(), ctrl: true, alt: false, shift: false } }
fn default_keybind_cut() -> HotkeyBinding { HotkeyBinding { vk_code: 0x43, name: "Ctrl + C".to_string(), ctrl: true, alt: false, shift: false } }
fn default_keybind_mirror() -> HotkeyBinding { HotkeyBinding { vk_code: 0x4D, name: "Ctrl + M".to_string(), ctrl: true, alt: false, shift: false } }
fn default_keybind_blur() -> HotkeyBinding { HotkeyBinding { vk_code: 0x4B, name: "Ctrl + K".to_string(), ctrl: true, alt: false, shift: false } }
fn default_keybind_paint_bucket() -> HotkeyBinding { HotkeyBinding { vk_code: 0x47, name: "Ctrl + G".to_string(), ctrl: true, alt: false, shift: false } }
fn default_grid_size() -> f32 { 50.0 }
fn default_snap_to_grid() -> bool { true }
fn default_brush_hardness() -> f32 { 100.0 }
fn default_brush_spacing() -> f32 { 10.0 }
fn default_keybind_calculator() -> HotkeyBinding { HotkeyBinding { vk_code: 0x43, name: "Ctrl + Alt + C".to_string(), ctrl: true, alt: true, shift: false } }
fn default_keybind_volume_mixer() -> HotkeyBinding { HotkeyBinding { vk_code: 0x56, name: "Ctrl + Alt + V".to_string(), ctrl: true, alt: true, shift: false } }
fn default_calculator_pos() -> egui::Pos2 { egui::pos2(150.0, 150.0) }
fn default_volume_mixer_pos() -> egui::Pos2 { egui::pos2(400.0, 150.0) }

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
            snip_source: CaptureSource::Desktop,
            origin_target_hwnd: 0,
            keybind_move: default_keybind_move(),
            keybind_brush: default_keybind_brush(),
            keybind_eraser: default_keybind_eraser(),
            keybind_text: default_keybind_text(),
            keybind_shape: default_keybind_shape(),
            keybind_snip: default_keybind_snip(),
            keybind_cut: default_keybind_cut(),
            keybind_mirror: default_keybind_mirror(),
            keybind_blur: default_keybind_blur(),
            keybind_paint_bucket: default_keybind_paint_bucket(),
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
            selection_mode: SelectionMode::New,
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
            brush_hardness: 100.0,
            brush_spacing: 10.0,
            hide_all: false,
            auto_hide_seconds: 0.0,
            exclude_from_capture: false,
            auto_exclude_live_capture: false,
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
            arrow_size: ArrowSize::Medium,
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
            show_screen_controls: default_show_screen_controls(),
            snip_source_overlay: false,
            live_performance_mode: false,
            show_source_rect: true,
            show_profiler: false,
            eraser_source_prompt_open: false,
            eraser_source_prompt_target: None,
            eraser_source_prompt_remember: false,
            use_original_capture: false,
            ui_reset_frames: 0,
            show_grid: false,
            grid_size: default_grid_size(),
            snap_to_grid: default_snap_to_grid(),
            show_calculator: false,
            show_volume_mixer: false,
            keybind_calculator: default_keybind_calculator(),
            keybind_volume_mixer: default_keybind_volume_mixer(),
            selected_mixer_apps: Vec::new(),
            calculator_pos: default_calculator_pos(),
            volume_mixer_pos: default_volume_mixer_pos(),
            widget_place_type: WidgetPlaceType::Browser,
            layer_menu_size: default_layer_menu_size(),
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        let mut settings = if let Some(path) = Self::config_path() {
            if let Ok(data) = std::fs::read_to_string(&path) {
                if let Ok(s) = serde_json::from_str::<Self>(&data) { s } else { Self::default() }
            } else {
                Self::default()
            }
        } else {
            Self::default()
        };
        settings.blur_strength = 0.0; // Blur should start 0 always
        let _ = crate::utils::ensure_ui_on_primary_monitor(&mut settings);
        settings
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

#[derive(Default, Debug, Clone)]
pub struct AppPerfStats {
    pub frame_count: u32,
    pub wgc_gpu_copy_us_sum: u128,
    pub wgc_map_wait_us_sum: u128,
    pub wgc_pixel_swap_us_sum: u128,
    pub gdi_capture_us_sum: u128,
    pub thread_mask_effects_us_sum: u128,
    pub thread_color32_conv_us_sum: u128,
    pub thread_total_us_sum: u128,
    pub upload_us_sum: u128,
}

#[derive(Default, Debug, Clone)]
pub struct PerfDisplayAverages {
    pub frame_count: u32,
    pub wgc_gpu_copy_us: f32,
    pub wgc_map_wait_us: f32,
    pub wgc_pixel_swap_us: f32,
    pub gdi_capture_us: f32,
    pub thread_mask_effects_us: f32,
    pub thread_color32_conv_us: f32,
    pub thread_total_us: f32,
    pub upload_us: f32,
}

pub fn default_true() -> bool { true }

