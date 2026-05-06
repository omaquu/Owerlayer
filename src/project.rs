use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::overlay::{Stroke, TextAnnotation, PlacedImage};

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ObjectType { Image, Stroke, Text }

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelectedObject {
    pub layer_idx: usize,
    pub object_type: ObjectType,
    pub object_idx: usize,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Layer {
    pub name: String,
    pub visible: bool,
    #[serde(default = "default_opacity")]
    pub opacity: f32,
    pub strokes: Vec<Stroke>,
    pub text_annotations: Vec<TextAnnotation>,
    pub placed_images: Vec<PlacedImage>,
    #[serde(default)]
    pub shadow: bool,
    #[serde(default = "default_shadow_offset")]
    pub shadow_offset: [f32; 2],
    #[serde(default = "default_shadow_color")]
    pub shadow_color: [u8; 4],
    #[serde(default)]
    pub outline: bool,
    #[serde(default = "default_outline_width")]
    pub outline_width: f32,
    #[serde(default = "default_outline_color")]
    pub outline_color: [u8; 4],
    #[serde(default)]
    pub expanded: bool,
}

fn default_shadow_offset() -> [f32; 2] { [2.0, 2.0] }
fn default_shadow_color() -> [u8; 4] { [0, 0, 0, 128] }
fn default_outline_width() -> f32 { 1.0 }
fn default_outline_color() -> [u8; 4] { [255, 255, 255, 255] }

fn default_opacity() -> f32 { 1.0 }

impl Layer {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            visible: true,
            opacity: 1.0,
            strokes: Vec::new(),
            text_annotations: Vec::new(),
            placed_images: Vec::new(),
            shadow: false,
            shadow_offset: [2.0, 2.0],
            shadow_color: [0, 0, 0, 128],
            outline: false,
            outline_width: 1.0,
            outline_color: [255, 255, 255, 255],
            expanded: false,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub layers: Vec<Layer>,
    pub active_layer: usize,
    #[serde(skip)]
    pub selected_object: Option<SelectedObject>,
    #[serde(skip)]
    pub last_left_down: bool,
}

impl Project {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            layers: vec![Layer::new("Layer 1")],
            active_layer: 0,
            selected_object: None,
            last_left_down: false,
        }
    }

    pub fn get_active_layer_mut(&mut self) -> Option<&mut Layer> {
        if self.layers.is_empty() { return None; }
        let idx = self.active_layer.min(self.layers.len() - 1);
        Some(&mut self.layers[idx])
    }

    pub fn get_active_layer(&self) -> Option<&Layer> {
        if self.layers.is_empty() { return None; }
        let idx = self.active_layer.min(self.layers.len() - 1);
        Some(&self.layers[idx])
    }

    pub fn project_dir(name: &str) -> PathBuf {
        if let Some(proj_dirs) = directories::ProjectDirs::from("com", "omaquu", "owerlayer") {
            proj_dirs.config_dir().join("projects").join(name)
        } else {
            PathBuf::from("projects").join(name)
        }
    }

    pub fn save(&self) {
        let dir = Self::project_dir(&self.name);
        if !dir.exists() {
            let _ = std::fs::create_dir_all(&dir);
        }

        // Save images
        for (l_idx, layer) in self.layers.iter().enumerate() {
            for img in &layer.placed_images {
                let img_path = dir.join(format!("img_{}_{}.png", l_idx, img.id));
                if !img.pixels.is_empty() {
                    let mut img_buf = image::RgbaImage::new(img.size[0] as u32, img.size[1] as u32);
                    img_buf.copy_from_slice(&img.pixels);
                    let _ = img_buf.save(&img_path);
                }
            }
        }

        let json_path = dir.join("project.json");
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(json_path, json);
        }
        
        // Save last loaded
        if let Some(proj_dirs) = directories::ProjectDirs::from("com", "omaquu", "owerlayer") {
            let last_file = proj_dirs.config_dir().join("last_project.txt");
            let _ = std::fs::write(last_file, &self.name);
        }
    }

    pub fn load_last() -> Option<Self> {
        if let Some(proj_dirs) = directories::ProjectDirs::from("com", "omaquu", "owerlayer") {
            let last_file = proj_dirs.config_dir().join("last_project.txt");
            if let Ok(name) = std::fs::read_to_string(last_file) {
                return Self::load(&name.trim());
            }
        }
        None
    }

    pub fn load(name: &str) -> Option<Self> {
        let dir = Self::project_dir(name);
        let json_path = dir.join("project.json");
        
        if let Ok(json) = std::fs::read_to_string(&json_path) {
            if let Ok(mut proj) = serde_json::from_str::<Project>(&json) {
                // Load images
                for (l_idx, layer) in proj.layers.iter_mut().enumerate() {
                    for img in &mut layer.placed_images {
                        let img_path = dir.join(format!("img_{}_{}.png", l_idx, img.id));
                        if let Ok(img_buf) = image::open(&img_path) {
                            let rgba = img_buf.to_rgba8();
                            img.pixels = rgba.into_raw();
                        }
                    }
                }
                return Some(proj);
            }
        }
        None
    }

    #[allow(dead_code)]
    pub fn list_projects() -> Vec<String> {
        let mut projects = Vec::new();
        if let Some(proj_dirs) = directories::ProjectDirs::from("com", "omaquu", "owerlayer") {
            let dir = proj_dirs.config_dir().join("projects");
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        if let Ok(name) = entry.file_name().into_string() {
                            projects.push(name);
                        }
                    }
                }
            }
        }
        projects
    }
}
