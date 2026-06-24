//! Ultralight-based offscreen web renderer for Owerlayer.
//!
//! This module is only compiled when the `webengine` Cargo feature is enabled.
//! It renders webpages to pixel buffers that can be displayed as PlacedImages.

#![allow(dead_code)]
#![allow(static_mut_refs)]

use std::sync::Arc;
use ul_next::config::Config;
use ul_next::view::ViewConfig;
use ul_next::{Library, Renderer, View};
use ul_next::event::ScrollEvent;

static mut UL_LIB: Option<Arc<Library>> = None;
static mut UL_RENDERER: Option<Renderer> = None;

/// A single offscreen web "widget" that renders a URL to pixels.
pub struct WebWidget {
    view: View,
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,   // RGBA pixel buffer
    pub dirty: bool,
    pub url: String,
    pub loading: bool,
    pub lib: Arc<Library>,
}

/// Initialize the Ultralight renderer. Call once at app startup.
/// Returns true if successful.
pub fn init() -> bool {
    unsafe {
        if UL_RENDERER.is_some() {
            return true; // Already initialized
        }

        // Run diagnostics
        if let Err(errors) = check_diagnostics() {
            eprintln!("[WebEngine] Diagnostics failed. Missing files:");
            for err in &errors {
                eprintln!("  - {:?}", err);
            }
            return false;
        }

        // Load the Ultralight library dynamically
        let lib = match Library::load() {
            Ok(lib) => lib,
            Err(e) => {
                eprintln!("[WebEngine] Failed to load Ultralight library: {:?}", e);
                eprintln!("[WebEngine] Make sure Ultralight DLLs are in the same folder as owerlayer.exe");
                return false;
            }
        };

        // Set up platform handlers (logger, filesystem, fontloader)
        let _ = ul_next::platform::enable_default_logger(lib.clone(), "./ultralight.log");
        let _ = ul_next::platform::enable_platform_filesystem(lib.clone(), ".");
        ul_next::platform::enable_platform_fontloader(lib.clone());

        // Create config using the builder pattern
        let config = match Config::start().build(lib.clone()) {
            Some(config) => config,
            None => {
                eprintln!("[WebEngine] Failed to create Ultralight config");
                return false;
            }
        };

        match Renderer::create(config) {
            Ok(renderer) => {
                UL_LIB = Some(lib);
                UL_RENDERER = Some(renderer);
                println!("[WebEngine] Ultralight renderer initialized successfully");
                true
            }
            Err(e) => {
                eprintln!("[WebEngine] Failed to create Ultralight renderer: {:?}", e);
                false
            }
        }
    }
}

/// Create a new web widget that renders the given URL.
pub fn create_widget(url: &str, width: u32, height: u32) -> Option<WebWidget> {
    unsafe {
        let renderer = UL_RENDERER.as_ref()?;
        let lib = UL_LIB.as_ref()?;

        let view_config = ViewConfig::start().build(lib.clone())?;
        let view = renderer.create_view(width, height, &view_config, None)?;
        let _ = view.load_url(url);

        let pixel_count = (width * height * 4) as usize;
        let pixels = vec![0u8; pixel_count];

        println!("[WebEngine] Created web widget for: {} ({}x{})", url, width, height);

        Some(WebWidget {
            view,
            width,
            height,
            pixels,
            dirty: true,
            url: url.to_string(),
            loading: true,
            lib: lib.clone(),
        })
    }
}

impl WebWidget {
    pub fn inject_mouse_event(&mut self, x: f32, y: f32, is_down: bool, is_move: bool) {
        use ul_next::event::{MouseEvent, MouseEventType, MouseButton};
        let evt_type = if is_move { MouseEventType::MouseMoved }
                       else if is_down { MouseEventType::MouseDown }
                       else { MouseEventType::MouseUp };
        
        if let Ok(event) = MouseEvent::new(self.lib.clone(), evt_type, x as i32, y as i32, MouseButton::Left) {
            let _ = self.view.fire_mouse_event(event);
        }
    }

    pub fn inject_text_event(&mut self, text: &str) {
        use ul_next::event::{KeyEventCreationInfo, KeyEventType, KeyEvent};
        use ul_next::key_code::VirtualKeyCode;
        for c in text.chars() {
            let s = c.to_string();
            let info = KeyEventCreationInfo {
                ty: KeyEventType::Char,
                modifiers: unsafe { std::mem::transmute(0u32) },
                virtual_key_code: VirtualKeyCode::Unknown,
                native_key_code: 0,
                text: &s,
                unmodified_text: &s,
                is_keypad: false,
                is_auto_repeat: false,
                is_system_key: false,
            };
            if let Ok(evt) = KeyEvent::new(self.lib.clone(), info) {
                let _ = self.view.fire_key_event(evt);
            }
        }
    }

    pub fn inject_raw_key_event(&mut self, key: egui::Key, pressed: bool) {
        use ul_next::event::{KeyEventCreationInfo, KeyEventType, KeyEvent};
        use ul_next::key_code::VirtualKeyCode;
        
        let vk = match key {
            egui::Key::Enter => VirtualKeyCode::Return,
            egui::Key::Backspace => VirtualKeyCode::Back,
            egui::Key::Delete => VirtualKeyCode::Delete,
            egui::Key::Escape => VirtualKeyCode::Escape,
            egui::Key::Tab => VirtualKeyCode::Tab,
            egui::Key::ArrowLeft => VirtualKeyCode::Left,
            egui::Key::ArrowRight => VirtualKeyCode::Right,
            egui::Key::ArrowUp => VirtualKeyCode::Up,
            egui::Key::ArrowDown => VirtualKeyCode::Down,
            egui::Key::Space => VirtualKeyCode::Space,
            _ => VirtualKeyCode::Unknown,
        };

        if matches!(vk, VirtualKeyCode::Unknown) { return; }

        let info = KeyEventCreationInfo {
            ty: if pressed { KeyEventType::RawKeyDown } else { KeyEventType::KeyUp },
            modifiers: unsafe { std::mem::transmute(0u32) },
            virtual_key_code: vk,
            native_key_code: 0,
            text: "",
            unmodified_text: "",
            is_keypad: false,
            is_auto_repeat: false,
            is_system_key: false,
        };
        if let Ok(evt) = KeyEvent::new(self.lib.clone(), info) {
            let _ = self.view.fire_key_event(evt);
        }
    }

    pub fn inject_scroll_event(&mut self, event: ScrollEvent) {
        let _ = self.view.fire_scroll_event(event);
    }

    pub fn update_view(&mut self) {
        if let Some(mut surface) = self.view.surface() {
            if let Some(pixels_guard) = surface.lock_pixels() {
                let src: &[u8] = pixels_guard.as_ref();
                let expected_len = (self.width * self.height * 4) as usize;

                if src.len() >= expected_len {
                    // Ultralight outputs BGRA, we need RGBA
                    self.pixels.resize(expected_len, 0);
                    for i in 0..(self.width * self.height) as usize {
                        let si = i * 4;
                        self.pixels[si]     = src[si + 2]; // R <- B
                        self.pixels[si + 1] = src[si + 1]; // G
                        self.pixels[si + 2] = src[si];     // B <- R
                        self.pixels[si + 3] = src[si + 3]; // A
                    }
                    self.dirty = true;
                    self.loading = false;
                }
            }
        }
    }
}

/// Update the renderer. Call this once per frame.
pub fn update_renderer() {
    let renderer = match unsafe { UL_RENDERER.as_ref() } {
        Some(r) => r,
        None => return,
    };

    renderer.update();
    renderer.render();
}

/// Update the renderer and extract pixels from all provided widgets.
/// Call this once per frame.
pub fn update_widgets(widgets: &mut Vec<WebWidget>) {
    update_renderer();
    for widget in widgets.iter_mut() {
        widget.update_view();
    }
}

/// Resize a widget's viewport.
pub fn resize_widget(widget: &mut WebWidget, new_width: u32, new_height: u32) {
    if new_width == widget.width && new_height == widget.height {
        return;
    }
    widget.width = new_width;
    widget.height = new_height;
    widget.view.resize(new_width, new_height);
    widget.pixels.resize((new_width * new_height * 4) as usize, 0);
    widget.dirty = true;
    println!("[WebEngine] Resized widget to {}x{} for {}", new_width, new_height, widget.url);
}

/// Navigate a widget to a new URL.
pub fn navigate_widget(widget: &mut WebWidget, url: &str) {
    widget.url = url.to_string();
    widget.loading = true;
    let _ = widget.view.load_url(url);
}

/// Check if the renderer is available.
pub fn is_available() -> bool {
    unsafe { UL_RENDERER.is_some() }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebEngineError {
    MissingDll(String),
    MissingResourcesDir,
    MissingResourceFile(String),
    Other(String),
}

pub fn check_diagnostics() -> Result<(), Vec<WebEngineError>> {
    let mut errors = Vec::new();
    
    // Check DLLs
    let exe_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf()));
    let dlls = vec!["Ultralight.dll", "WebCore.dll", "AppCore.dll"];
    let mut missing_dlls = Vec::new();
    
    for dll in &dlls {
        let in_cwd = std::path::Path::new(dll).exists();
        let in_exe_dir = exe_dir.as_ref().map(|d| d.join(dll).exists()).unwrap_or(false);
        if !in_cwd && !in_exe_dir {
            missing_dlls.push(dll.to_string());
        }
    }
    
    if !missing_dlls.is_empty() {
        for dll in missing_dlls {
            errors.push(WebEngineError::MissingDll(dll));
        }
    }
    
    // Check resources directory
    let res_dir_cwd = std::path::Path::new("resources");
    let res_dir_exe = exe_dir.as_ref().map(|d| d.join("resources"));
    
    let res_dir = if res_dir_cwd.exists() && res_dir_cwd.is_dir() {
        Some(res_dir_cwd.to_path_buf())
    } else if res_dir_exe.as_ref().map(|d| d.exists() && d.is_dir()).unwrap_or(false) {
        res_dir_exe
    } else {
        None
    };
    
    if let Some(dir) = res_dir {
        // Check files in resources folder
        let mut has_icu = false;
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with("icudt") && name.ends_with(".dat") {
                        has_icu = true;
                        break;
                    }
                }
            }
        }
        if !has_icu {
            errors.push(WebEngineError::MissingResourceFile("icudt67l.dat (Unicode data file)".to_string()));
        }
        
        let cacert_path = dir.join("cacert.pem");
        if !cacert_path.exists() {
            errors.push(WebEngineError::MissingResourceFile("cacert.pem (certificate bundle)".to_string()));
        }
    } else {
        errors.push(WebEngineError::MissingResourcesDir);
    }
    
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn get_status_string() -> String {
    if is_available() {
        return "Web Engine: Initialized & Ready".to_string();
    }
    match check_diagnostics() {
        Ok(_) => "Web Engine: Ready to initialize. Click 'Try Initialize'.".to_string(),
        Err(errors) => {
            let mut msg = String::new();
            for err in errors {
                match err {
                    WebEngineError::MissingDll(dll) => {
                        msg.push_str(&format!("• Missing DLL: {} (must be in root folder next to owerlayer.exe)\n", dll));
                    }
                    WebEngineError::MissingResourcesDir => {
                        msg.push_str("• Missing 'resources' directory in root folder.\n");
                    }
                    WebEngineError::MissingResourceFile(file) => {
                        msg.push_str(&format!("• Missing file inside 'resources/': {}\n", file));
                    }
                    WebEngineError::Other(e) => {
                        msg.push_str(&format!("• Error: {}\n", e));
                    }
                }
            }
            msg
        }
    }
}

pub fn reinit_web_widgets(project: &mut crate::project::Project) {
    if !is_available() {
        init();
    }
    if is_available() {
        for layer in &mut project.layers {
            for img in &mut layer.placed_images {
                if img.is_live && img.web_widget.is_none() {
                    if let Some(ref url) = img.url {
                        if url.starts_with("http") || url.starts_with("about:") {
                            if let Some(widget) = create_widget(url, img.size[0] as u32, img.size[1] as u32) {
                                img.web_widget = Some(std::sync::Arc::new(std::sync::Mutex::new(widget)));
                                println!("[WebEngine] Re-initialized web widget for URL: {}", url);
                            }
                        }
                    }
                }
            }
        }
    }
}
