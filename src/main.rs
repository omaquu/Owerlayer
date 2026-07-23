#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod hotkey;
mod overlay;
mod types;
mod ui;
mod winapi_utils;
mod tools;
mod project;
mod utils;
mod capture_thread;
mod wgc_capture;
mod gl_renderer;
mod rasterize;
mod history;
mod volume_mixer;
#[cfg(feature = "webengine")]
mod web_engine;

use eframe::egui;
use std::sync::Arc;
use types::*;
use ui::toolbar::{render_toolbar, photoshop_frame};
use ui::settings_menu::render_settings_window;
use ui::layer_menu::render_layers_window;
use ui::filter_menu::render_filter_menu;

fn marquee_to_local_points(sel: &crate::types::MarqueeSelection, bounds_min: egui::Pos2) -> Vec<egui::Pos2> {
    let bounds = sel.bounds();
    if bounds.width() <= 0.1 || bounds.height() <= 0.1 {
        return Vec::new();
    }
    
    let step = 3.0f32;
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
            grid[(r * cols + c) as usize] = sel.contains(egui::pos2(x, y));
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
    
    let mut loops = Vec::new();
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
            let mapped_path: Vec<egui::Pos2> = path.into_iter().map(|pt| {
                let screen_x = min_x + (pt.0 as f32 / 2.0) * step;
                let screen_y = min_y + (pt.1 as f32 / 2.0) * step;
                egui::pos2(screen_x - bounds_min.x, screen_y - bounds_min.y)
            }).collect();
            loops.push(mapped_path);
        }
    }
    
    let mut result = Vec::new();
    for (i, path) in loops.into_iter().enumerate() {
        if i > 0 {
            result.push(egui::Pos2::new(f32::NAN, f32::NAN));
        }
        result.extend(path);
    }
    result
}

#[derive(Clone)]
struct MarqueeCaptureRequest {
    trigger_copy: bool,
    trigger_cut: bool,
    trigger_blur: bool,
    marquee_selection: crate::types::MarqueeSelection,
}


struct OwerlayerApp {
    pub edit_mode: bool,
    marquee_capture_request: Option<MarqueeCaptureRequest>,
    pub embed_url: String,
    pub embed_trigger: bool,
    pub active_tool: overlay::Tool,
    settings: Settings,

    project: project::Project,

    current_stroke: Vec<egui::Pos2>,
    line_start: Option<egui::Pos2>,
    initial_bounds: Option<egui::Rect>,
    initial_center: Option<egui::Pos2>,
    initial_layer: Option<project::Layer>,
    initial_active_layer_idx: usize,
    dragging_source_rect: bool,

    pending_text: Option<PendingText>,
    pending_stroke: Option<Stroke>,
    pending_text_to_add: Option<overlay::TextAnnotation>,

    show_settings_panel: bool,
    show_layers_panel: bool,
    show_exit_dialog: bool,
    listening_for_hotkey: bool,
    
    prev_mouse_down: bool,
    prev_mouse_pos: egui::Pos2,
    prev_hotkey_held: bool,

    initialized: bool,
    frame_count: u64,
    show_debug_window: bool,
    prev_passthrough: bool,
    prev_exclude_capture: Option<bool>,
    prev_f12_down: bool,
    prev_ctrl_shift_r_down: bool,
    last_tool_used: Option<Tool>,
    last_action_time: std::time::Instant,
    filters_open: Option<usize>,
    owl_icon: Option<egui::TextureHandle>,
    capture_thread: capture_thread::CaptureThread,
    #[cfg(feature = "webengine")]
    web_widgets: Vec<web_engine::WebWidget>,
    #[cfg(feature = "webengine")]
    web_engine_initialized: bool,
    gl_renderer: Option<Arc<gl_renderer::GLRenderer>>,
    history: history::History,
    show_history_panel: bool,
    request_history_push: Option<String>,
    layer_prompt_open: bool,
    load_picker_open: bool,
    rasterize_phase: u8,  // 0=idle, 1=isolate+render, 2=read pixels
    rasterize_bbox: Option<[f32; 4]>,
    rasterize_capture: rasterize::CaptureBuffer,
    pub copied_image: Option<crate::types::PlacedImage>,

    // Performance profiling fields
    last_perf_print_time: std::time::Instant,
    perf_stats: crate::types::AppPerfStats,
    perf_display: Option<crate::types::PerfDisplayAverages>,

    prev_calc_hotkey_held: bool,
    prev_vol_hotkey_held: bool,
    volume_sessions: Vec<crate::volume_mixer::AudioSessionInfo>,
    volume_sessions_shared: std::sync::Arc<std::sync::Mutex<Vec<crate::volume_mixer::AudioSessionInfo>>>,
    volume_mixer_active: std::sync::Arc<std::sync::atomic::AtomicBool>,
    volume_mixer_cmd_tx: std::sync::mpsc::Sender<crate::volume_mixer::MixerCommand>,
}

impl OwerlayerApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut v = egui::Visuals::dark();
        v.window_shadow = egui::Shadow::NONE;
        v.popup_shadow = egui::Shadow::NONE;
        v.panel_fill = egui::Color32::TRANSPARENT;
        // Use a semi-transparent dark fill for tooltips and popups
        v.window_fill = egui::Color32::from_rgba_unmultiplied(30, 30, 30, 220);
        cc.egui_ctx.set_visuals(v);
        cc.egui_ctx.set_pixels_per_point(1.0);

        let volume_sessions_shared = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let volume_mixer_active = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let (tx, rx) = std::sync::mpsc::channel::<crate::volume_mixer::MixerCommand>();
        
        {
            let vs_clone = std::sync::Arc::clone(&volume_sessions_shared);
            let active_clone = std::sync::Arc::clone(&volume_mixer_active);
            std::thread::spawn(move || {
                #[cfg(windows)]
                unsafe {
                    let _ = windows::Win32::System::Com::CoInitializeEx(
                        None,
                        windows::Win32::System::Com::COINIT_APARTMENTTHREADED,
                    );
                }
                
                let mut last_poll = std::time::Instant::now() - std::time::Duration::from_secs(2);
                loop {
                    while let Ok(cmd) = rx.try_recv() {
                        match cmd {
                            crate::volume_mixer::MixerCommand::SetVolume { pid, volume } => {
                                crate::volume_mixer::set_session_volume(pid, volume);
                                // Delay next poll so the API change propagates before we re-read
                                last_poll = std::time::Instant::now();
                            }
                            crate::volume_mixer::MixerCommand::SetMute { pid, mute } => {
                                crate::volume_mixer::set_session_mute(pid, mute);
                                last_poll = std::time::Instant::now();
                            }
                            crate::volume_mixer::MixerCommand::ForcePoll => {
                                last_poll = std::time::Instant::now() - std::time::Duration::from_secs(2);
                            }
                        }
                    }
                    
                    if active_clone.load(std::sync::atomic::Ordering::Relaxed) {
                        if last_poll.elapsed() >= std::time::Duration::from_millis(1500) {
                            let sessions = crate::volume_mixer::get_active_sessions();
                            if let Ok(mut lock) = vs_clone.lock() {
                                *lock = sessions;
                            }
                            last_poll = std::time::Instant::now();
                        }
                    }
                    
                    if let Ok(cmd) = rx.recv_timeout(std::time::Duration::from_millis(100)) {
                        match cmd {
                            crate::volume_mixer::MixerCommand::SetVolume { pid, volume } => {
                                crate::volume_mixer::set_session_volume(pid, volume);
                                last_poll = std::time::Instant::now();
                            }
                            crate::volume_mixer::MixerCommand::SetMute { pid, mute } => {
                                crate::volume_mixer::set_session_mute(pid, mute);
                                last_poll = std::time::Instant::now();
                            }
                            crate::volume_mixer::MixerCommand::ForcePoll => {
                                last_poll = std::time::Instant::now() - std::time::Duration::from_secs(2);
                            }
                        }
                    }
                }
            });
        }

        let mut app = Self {
            edit_mode: true,
            marquee_capture_request: None,
            embed_url: String::new(),
            embed_trigger: false,
            active_tool: overlay::Tool::Brush,
            settings: Settings::load(),
            project: project::Project::load_last().unwrap_or_else(|| project::Project::new("Default Project")),
            current_stroke: Vec::new(),
            line_start: None,
            initial_bounds: None,
            initial_center: None,
            initial_layer: None,
            initial_active_layer_idx: 0,
            dragging_source_rect: false,
            pending_text: None,
            pending_stroke: None,
            pending_text_to_add: None,
            show_settings_panel: false,
            show_layers_panel: true,
            show_exit_dialog: false,
            load_picker_open: false,
            layer_prompt_open: false,
            listening_for_hotkey: false,
            prev_mouse_down: false,
            prev_mouse_pos: egui::pos2(0.0, 0.0),
            prev_hotkey_held: false,
            initialized: false,
            frame_count: 0,
            show_debug_window: false,
            prev_passthrough: true, // Initially transparent
            prev_exclude_capture: None,
            prev_f12_down: false,
            prev_ctrl_shift_r_down: false,
            last_tool_used: None,
            last_action_time: std::time::Instant::now(),
            filters_open: None,
            owl_icon: {
                if let Ok(img) = image::load_from_memory(include_bytes!("../icon.png")) {
                    let rgba = img.to_rgba8();
                    let (width, height) = rgba.dimensions();
                    let color_image = egui::ColorImage::from_rgba_unmultiplied([width as usize, height as usize], &rgba.into_raw());
                    Some(cc.egui_ctx.load_texture("owl_icon", color_image, Default::default()))
                } else {
                    None
                }
            },
            capture_thread: capture_thread::CaptureThread::new(15.0, cc.egui_ctx.clone()),
            history: history::History::new(),
            show_history_panel: false,
            request_history_push: None,
            #[cfg(feature = "webengine")]
            web_widgets: Vec::new(),
            #[cfg(feature = "webengine")]
            web_engine_initialized: false,
            gl_renderer: cc.gl.as_ref().map(|gl| Arc::new(gl_renderer::GLRenderer::new(gl))),
            rasterize_phase: 0,
            rasterize_bbox: None,
            rasterize_capture: rasterize::new_capture_buffer(),
            copied_image: None,
            last_perf_print_time: std::time::Instant::now(),
            perf_stats: crate::types::AppPerfStats::default(),
            perf_display: None,
            prev_calc_hotkey_held: false,
            prev_vol_hotkey_held: false,
            volume_sessions: Vec::new(),
            volume_sessions_shared,
            volume_mixer_active,
            volume_mixer_cmd_tx: tx,
        };
        #[cfg(feature = "webengine")]
        {
            web_engine::reinit_web_widgets(&mut app.project);
        }
        app
    }

    fn finalize_marquee_capture(&mut self, frame: crate::rasterize::CapturedFrame, m_req: MarqueeCaptureRequest, ctx: &egui::Context) {
        let ppp = ctx.pixels_per_point();
        let sel = &m_req.marquee_selection;
        let bounds = sel.bounds();
        let (wx, wy) = crate::winapi_utils::get_window_screen_pos();
        let render_offset = if self.settings.use_absolute_screen_coords {
            egui::vec2(wx as f32 / ppp, wy as f32 / ppp)
        } else {
            egui::Vec2::ZERO
        };
        let screen_bounds = bounds.translate(-render_offset);
        let sw = frame.width as usize;
        let sh = frame.height as usize;
        let mut pixels = frame.pixels;
        
        if self.settings.snip_source == CaptureSource::Origin {
            let logical_center_x = screen_bounds.min.x + screen_bounds.width() / 2.0;
            let logical_center_y = screen_bounds.min.y + screen_bounds.height() / 2.0;
            let center_x = (logical_center_x * ppp).round() as i32 + if self.settings.use_absolute_screen_coords { 0 } else { wx };
            let center_y = (logical_center_y * ppp).round() as i32 + if self.settings.use_absolute_screen_coords { 0 } else { wy };
            if let Some((hwnd, _name, _rect)) = crate::winapi_utils::get_window_at_point(center_x, center_y) {
                self.settings.origin_target_hwnd = hwnd as isize;
                self.settings.save();
            }
        }
        
        let mut mask = vec![255u8; sw * sh];
        for py in 0..sh {
            for px in 0..sw {
                let lp = screen_bounds.min + egui::vec2(px as f32 / ppp, py as f32 / ppp);
                let wp = lp + render_offset;
                if !sel.contains(wp) {
                    mask[py * sw + px] = 0;
                }
            }
        }
        
        let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
        let mut img = if m_req.trigger_blur {
            let mut new_img = overlay::PlacedImage::new(id, bounds.min, [sw, sh], pixels);
            new_img.display_size = Some([bounds.width(), bounds.height()]);
            new_img.source_rect = Some([bounds.min.x, bounds.min.y, bounds.width(), bounds.height()]);
            new_img.show_source_rect = self.settings.show_source_rect;
            new_img.snip_points = Some(marquee_to_local_points(&sel, bounds.min));
            new_img.shadow = self.settings.snip_shadow;
            new_img.is_live = self.settings.snip_live;
            new_img.blur = self.settings.blur_strength;
            new_img.blur_effect = self.settings.blur_effect;
            new_img.capture_source = self.settings.snip_source;
            new_img.target_hwnd = self.settings.origin_target_hwnd;
            new_img.mask = Some(mask);
            new_img.mask_size = Some([sw, sh]);
            new_img.mask_dirty = true;
            new_img.name = "Blur".to_string();
            new_img
        } else {
            for py in 0..sh {
                for px in 0..sw {
                    if mask[py * sw + px] == 0 {
                        let idx = (py * sw + px) * 4;
                        if idx + 3 < pixels.len() {
                            pixels[idx + 3] = 0;
                        }
                    }
                }
            }
            
            let mut new_img = overlay::PlacedImage::new(id, bounds.min, [sw, sh], pixels);
            new_img.name = "Marquee".to_string();
            new_img.display_size = Some([bounds.width(), bounds.height()]);
            new_img.source_rect = Some([bounds.min.x, bounds.min.y, bounds.width(), bounds.height()]);
            new_img.show_source_rect = self.settings.show_source_rect;
            new_img.snip_points = Some(marquee_to_local_points(&sel, bounds.min));
            new_img.shadow = self.settings.snip_shadow;
            new_img.is_live = self.settings.snip_live;
            new_img.blur = self.settings.blur_strength;
            new_img.blur_effect = self.settings.blur_effect;
            new_img.capture_source = self.settings.snip_source;
            new_img.target_hwnd = self.settings.origin_target_hwnd;
            
            let is_rect = matches!(sel.shape, SelectionShape::Rect(_)) && sel.ops.is_empty();
            if !is_rect {
                new_img.mask = Some(mask);
                new_img.mask_size = Some([sw, sh]);
                new_img.mask_dirty = true;
            }
            
            crate::winapi_utils::copy_image_to_clipboard(&new_img.pixels, new_img.size[0], new_img.size[1]);
            self.copied_image = Some(new_img.clone());
            new_img
        };
        
        if m_req.trigger_cut {
            self.history.push(&self.project, "Cut Selection");
            crate::tools::cut::erase_marquee_selection(&mut self.project, &self.settings);
            self.project.save();
        } else {
            let active_layer_idx = self.project.active_layer;
            if active_layer_idx < self.project.layers.len() {
                let is_locked = self.project.layers[active_layer_idx].locked;
                let ask_mode = self.settings.auto_new_layer.is_none();
                
                if is_locked || ask_mode {
                    self.project.layers[active_layer_idx].placed_images.push(img);
                    self.project.layers[active_layer_idx].expanded = true;
                    self.layer_prompt_open = true;
                } else {
                    let to_layer = match self.settings.auto_new_layer {
                        Some(true) => {
                            self.project.layers.push(crate::project::Layer::new(&format!("Layer {}", self.project.layers.len() + 1)));
                            self.project.active_layer = self.project.layers.len() - 1;
                            self.project.layers.len() - 1
                        }
                        _ => active_layer_idx,
                    };
                    self.project.layers[to_layer].placed_images.push(img);
                    self.project.layers[to_layer].expanded = true;
                }
                
                let action_name = if m_req.trigger_blur {
                    "Blur Area"
                } else {
                    "Snip Selection"
                };
                self.history.push(&self.project, action_name);
                self.project.save();
            }
        }
    }

    fn is_window_open(&self, id: egui::Id, ctx: &egui::Context) -> bool {
        if id == egui::Id::new("photoshop_panel") {
            return self.edit_mode && !self.settings.hide_all;
        }
        if id == egui::Id::new("Settings") {
            return self.edit_mode && !self.settings.hide_all && self.show_settings_panel;
        }
        if id == egui::Id::new("Layers") {
            return self.edit_mode && !self.settings.hide_all && self.show_layers_panel;
        }
        if id == egui::Id::new("History") {
            return self.edit_mode && !self.settings.hide_all && self.show_history_panel;
        }
        if id == egui::Id::new("exit_dialog") {
            return self.show_exit_dialog;
        }
        if id == egui::Id::new("Performance Profiler") {
            return self.perf_display.is_some();
        }
        if id == egui::Id::new("Debug Overlay") {
            return self.show_debug_window;
        }
        if id == egui::Id::new("creation_prompt") {
            return self.layer_prompt_open && self.edit_mode;
        }
        if id == egui::Id::new("Confirm Delete Layer") {
            let layer_to_delete: Option<usize> = ctx.memory(|m| m.data.get_temp(egui::Id::new("layer_to_delete")));
            return self.edit_mode && !self.settings.hide_all && layer_to_delete.is_some();
        }
        if id == egui::Id::new("Load Project") {
            return self.edit_mode && !self.settings.hide_all && self.load_picker_open;
        }
        if id == egui::Id::new("Object Effects") {
            return self.edit_mode && !self.settings.hide_all && self.settings.fx_open.is_some();
        }
        if id == egui::Id::new("layer_filters") {
            return self.edit_mode && !self.settings.hide_all && self.filters_open.is_some();
        }
        false
    }

    fn handle_embed_trigger(&mut self) {
        if self.embed_url.is_empty() { return; }
        
        let mut url = self.embed_url.clone();
        self.embed_url.clear();
        
        if url.starts_with("window://") {
            if let Some(hwnd_str) = url.strip_prefix("window://") {
                if let Ok(hwnd) = hwnd_str.parse::<usize>() {
                    if let Some(layer) = self.project.get_active_layer_mut() {
                        let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
                        // Use a default size, it will be updated on first capture
                        let mut img = overlay::PlacedImage::new(id, egui::pos2(100.0, 100.0), [800, 600], Vec::new());
                        img.is_live = true;
                        img.hwnd = hwnd;
                        layer.placed_images.push(img);
                        return;
                    }
                }
            }
        }
        
        if url.contains("youtube.com") || url.contains("youtu.be") {
            let id = if url.contains("v=") {
                url.split("v=").nth(1).and_then(|s| s.split('&').next())
            } else if url.contains("youtu.be/") {
                url.split("youtu.be/").nth(1).and_then(|s| s.split('?').next())
            } else {
                url.split('/').last()
            }.map(|s| s.to_string());

            if let Some(id) = id {
                url = format!("https://www.youtube-nocookie.com/embed/{}?autoplay=1&mute=1&controls=0&modestbranding=1&rel=0&iv_load_policy=3&disablekb=1", id);
                println!("Using YouTube Option B (best performance) for ID: {}", id);
            }
        }
        
        #[cfg(feature = "webengine")]
        {
            if url.starts_with("http") && crate::web_engine::is_available() {
                if let Some(widget) = crate::web_engine::create_widget(&url, 1280, 720) {
                    if let Some(layer) = self.project.get_active_layer_mut() {
                        let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
                        let mut img = overlay::PlacedImage::new(id, egui::pos2(100.0, 100.0), [1280, 720], widget.pixels.clone());
                        img.display_size = Some([640.0, 360.0]);
                        img.is_live = true;
                        img.url = Some(url.clone());
                        img.web_widget = Some(std::sync::Arc::new(std::sync::Mutex::new(widget)));
                        layer.placed_images.push(img);
                        return;
                    }
                }
            }
        }

        // Try to load as local path first
        let bytes = if let Ok(b) = std::fs::read(&url) {
            Some(b)
        } else {
            // Try as URL (fallback if webengine is off or failed)
            if url.contains("youtube-nocookie.com/embed/") {
                let id = url.split("/embed/").nth(1).and_then(|s| s.split('?').next());
                if let Some(id) = id {
                    let thumb_url = format!("https://img.youtube.com/vi/{}/maxresdefault.jpg", id);
                    if let Ok(resp) = reqwest::blocking::get(&thumb_url) {
                        if let Ok(b) = resp.bytes() {
                            Some(b.to_vec())
                        } else { None }
                    } else { None }
                } else { None }
            } else if let Ok(resp) = reqwest::blocking::get(&url) {
                if let Ok(b) = resp.bytes() {
                    Some(b.to_vec())
                } else { None }
            } else { None }
        };

        let mut loaded_as_image = false;
        if let Some(bytes) = bytes {
            // Try as GIF first for animation
            if let Ok(decoder) = image::codecs::gif::GifDecoder::new(std::io::Cursor::new(&bytes)) {
                use image::AnimationDecoder;
                if let Ok(frames) = decoder.into_frames().collect_frames() {
                    if !frames.is_empty() {
                        let first = frames[0].buffer();
                        let (w, h) = first.dimensions();
                        
                        let mut frame_bytes = Vec::new();
                        let mut durations = Vec::new();
                        for frame in frames {
                            let (numer, denom) = frame.delay().numer_denom_ms();
                            durations.push(numer as f32 / denom as f32 / 1000.0);
                            frame_bytes.push(frame.into_buffer().into_raw());
                        }

                        if let Some(layer) = self.project.get_active_layer_mut() {
                            let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
                            let mut img = overlay::PlacedImage::new(id, egui::pos2(100.0, 100.0), [w as usize, h as usize], frame_bytes[0].clone());
                            img.url = Some(url.clone());
                            img.frames = frame_bytes;
                            img.frame_durations = durations;
                            layer.placed_images.push(img);
                            loaded_as_image = true;
                        }
                    }
                }
            }

            // Fallback to static image
            if !loaded_as_image {
                if let Ok(img) = image::load_from_memory(&bytes) {
                    let rgba = img.to_rgba8();
                    let (w, h) = rgba.dimensions();
                    if let Some(layer) = self.project.get_active_layer_mut() {
                        let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
                        let mut p_img = overlay::PlacedImage::new(id, egui::pos2(100.0, 100.0), [w as usize, h as usize], rgba.into_raw());
                        p_img.url = Some(url.clone());
                        layer.placed_images.push(p_img);
                        loaded_as_image = true;
                    }
                }
            }
        }

        if !loaded_as_image && url.starts_with("http") {
            #[cfg(feature = "webengine")]
            {
                if self.settings.experimental_features {
                    // Initialize web engine if needed
                    if !self.web_engine_initialized {
                        self.web_engine_initialized = web_engine::init();
                    }

                    if self.web_engine_initialized {
                        if let Some(widget) = web_engine::create_widget(&url, 800, 600) {
                            if let Some(layer) = self.project.get_active_layer_mut() {
                                let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
                                let mut p_img = overlay::PlacedImage::new(id, egui::pos2(100.0, 100.0), [800, 600], widget.pixels.clone());
                                p_img.display_size = Some([800.0, 600.0]);
                                p_img.is_live = true;
                                p_img.url = Some(url.clone());
                                p_img.web_widget = Some(std::sync::Arc::new(std::sync::Mutex::new(widget)));
                                layer.placed_images.push(p_img);
                            }
                            println!("[WebEngine] Embedded webpage: {}", url);
                            return;
                        }
                    } else {
                        eprintln!("[WebEngine] Failed to initialize. Make sure Ultralight DLLs are in the same folder as owerlayer.exe");
                    }
                }
            }

            // Fallback: placeholder for non-webengine builds or when experimental is off
            let label = if self.settings.experimental_features {
                #[cfg(feature = "webengine")]
                { format!("WebEngine failed for: {}", url) }
                #[cfg(not(feature = "webengine"))]
                { format!("Requires webengine build: {}", url) }
            } else {
                format!("Enable Experimental Features: {}", url)
            };

            if let Some(layer) = self.project.get_active_layer_mut() {
                let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
                let w = 400; let h = 80;
                let mut pixels = vec![0u8; w * h * 4];
                for chunk in pixels.chunks_exact_mut(4) {
                    chunk[0] = 40; chunk[1] = 30; chunk[2] = 30; chunk[3] = 220;
                }
                let mut p_img = overlay::PlacedImage::new(id, egui::pos2(100.0, 100.0), [w, h], pixels);
                p_img.display_size = Some([w as f32, h as f32]);
                p_img.url = Some(url.clone());
                layer.placed_images.push(p_img);
                
                layer.text_annotations.push(overlay::TextAnnotation::new(egui::pos2(110.0, 110.0), label, [255, 180, 180, 255], 14.0));
            }
        }
    }
}

impl eframe::App for OwerlayerApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.settings.multi_monitor || self.settings.virtual_matrix {
            ctx.set_pixels_per_point(1.0);
        }
        #[cfg(windows)]
        {
            if self.frame_count == 2 {
                crate::winapi_utils::setup_overlay_window(self.settings.fso_fix);
                if self.settings.multi_monitor || self.settings.virtual_matrix {
                    let (sw, sh, ox, oy) = if let Some(idx) = self.settings.monitor_lock {
                        crate::winapi_utils::get_monitor_size_pos(idx)
                    } else {
                        let (sw, sh) = crate::winapi_utils::get_screen_size(true);
                        let (ox, oy) = crate::winapi_utils::get_virtual_origin();
                        (sw, sh, ox, oy)
                    };
                    if self.settings.fso_fix {
                        crate::winapi_utils::reposition_overlay_window(ox as i32 - 2, oy as i32 - 2, sw as i32 + 4, sh as i32 + 4);
                    } else {
                        crate::winapi_utils::reposition_overlay_window(ox as i32, oy as i32, sw as i32, sh as i32);
                    }
                }
            }
        }
        if self.history.entries.is_empty() {
            self.history.push(&self.project, "Initial State");
        }
        self.frame_count += 1;
        
        // Poll active volume sessions if a Volume Mixer is placed
        let has_volume_mixer = self.project.layers.iter().any(|l| {
            l.placed_images.iter().any(|img| img.widget_type == Some(crate::types::WidgetType::VolumeMixer))
        });
        self.volume_mixer_active.store(has_volume_mixer, std::sync::atomic::Ordering::Relaxed);
        if has_volume_mixer {
            if let Ok(lock) = self.volume_sessions_shared.lock() {
                self.volume_sessions = lock.clone();
            }
        }
        
        // Sanitizing selected_object bounds to prevent panics and crashes from deleted layers or objects
        if let Some(sel) = self.project.selected_object {
            if sel.layer_idx >= self.project.layers.len() {
                self.project.selected_object = None;
            } else {
                let layer = &self.project.layers[sel.layer_idx];
                let is_valid = match sel.object_type {
                    ObjectType::Image => sel.object_idx < layer.placed_images.len(),
                    ObjectType::Stroke => sel.object_idx < layer.strokes.len(),
                    ObjectType::Text => sel.object_idx < layer.text_annotations.len(),
                };
                if !is_valid {
                    self.project.selected_object = None;
                }
            }
        }
        
        // ---- 0. First-frame init ----
        if !self.initialized {
            self.initialized = true;
            println!("DEBUG: First frame init. rasterize_phase={}, rasterize_request_is_some={}", self.rasterize_phase, self.project.rasterize_request.is_some());
            ctx.request_repaint();
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));
            winapi_utils::setup_overlay_window(self.settings.fso_fix);
            winapi_utils::set_capture_exclusion(self.settings.exclude_from_capture);
        }

        // ---- 1. Poll mouse ----
        let ppp = self.settings.ui_scale;
        let mouse = MouseState::poll(self.prev_mouse_down, self.prev_mouse_pos, ppp, self.settings.multi_monitor);
        self.prev_mouse_down = mouse.left_down;
        self.prev_mouse_pos = mouse.pos;

        // Evaluate customizable tool hotkeys
        if !self.listening_for_hotkey {
            let tools_to_check = [
                (&self.settings.keybind_move, overlay::Tool::Move),
                (&self.settings.keybind_brush, overlay::Tool::Brush),
                (&self.settings.keybind_eraser, overlay::Tool::Eraser),
                (&self.settings.keybind_text, overlay::Tool::Text),
                (&self.settings.keybind_shape, overlay::Tool::Shape),
                (&self.settings.keybind_snip, overlay::Tool::Snip),
                (&self.settings.keybind_cut, overlay::Tool::Cut),
                (&self.settings.keybind_mirror, overlay::Tool::Mirror),
                (&self.settings.keybind_blur, overlay::Tool::Blur),
                (&self.settings.keybind_paint_bucket, overlay::Tool::PaintBucket),
            ];

            for (keybind, tool) in tools_to_check {
                if hotkey::is_hotkey_held(keybind) {
                    if self.active_tool != tool || !self.edit_mode {
                        self.active_tool = tool;
                        self.edit_mode = true;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                        crate::winapi_utils::force_focus();
                    }
                }
            }

            // Calculator widget hotkey
            let calc_held = hotkey::is_hotkey_held(&self.settings.keybind_calculator);
            if calc_held && !self.prev_calc_hotkey_held {
                for layer in &mut self.project.layers {
                    for img in &mut layer.placed_images {
                        if img.widget_type == Some(crate::types::WidgetType::Calculator) {
                            img.visible = !img.visible;
                        }
                    }
                }
                self.project.save();
            }
            self.prev_calc_hotkey_held = calc_held;
            
            // Volume mixer widget hotkey
            let vol_held = hotkey::is_hotkey_held(&self.settings.keybind_volume_mixer);
            if vol_held && !self.prev_vol_hotkey_held {
                for layer in &mut self.project.layers {
                    for img in &mut layer.placed_images {
                        if img.widget_type == Some(crate::types::WidgetType::VolumeMixer) {
                            img.visible = !img.visible;
                        }
                    }
                }
                self.project.save();
            }
            self.prev_vol_hotkey_held = vol_held;
        }

        // ---- 2. Hotkey / toggle ----
        let key_held = if self.listening_for_hotkey { false }
                       else { hotkey::is_hotkey_held(&self.settings.hotkey) };

        let was_edit = self.edit_mode;

        if self.settings.toggle_mode {
            if key_held && !self.prev_hotkey_held { self.edit_mode = !self.edit_mode; }
        } else {
            if key_held {
                self.edit_mode = true;
            } else if !mouse.left_down {
                self.edit_mode = false;
            }
        }
        self.prev_hotkey_held = key_held;

        // F12 or Ctrl+Shift+R: reset UI panels + overlay to primary monitor (monitor 1)
        #[cfg(windows)]
        {
            let f12_down = unsafe {
                windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(0x7B) & 0x8000u16 as i16 != 0
            };
            let ctrl_shift_r_down = unsafe {
                let ctrl = windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(0x11) & 0x8000u16 as i16 != 0;
                let shift = windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(0x10) & 0x8000u16 as i16 != 0;
                let r_key = windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(0x52) & 0x8000u16 as i16 != 0;
                ctrl && shift && r_key
            };
            let trigger_reset = (f12_down && !self.prev_f12_down) || (ctrl_shift_r_down && !self.prev_ctrl_shift_r_down);
            if trigger_reset {
                crate::utils::reset_ui_positions_to_primary(&mut self.settings);
                winapi_utils::reposition_overlay_to_primary_monitor(self.settings.fso_fix);
                self.settings.save();
                #[cfg(windows)]
                winapi_utils::refresh_overlay_composition();
            }
            self.prev_f12_down = f12_down;
            self.prev_ctrl_shift_r_down = ctrl_shift_r_down;
        }
        
        // Auto-open layers panel if pinned and Ctrl/Hotkey held
        if self.settings.pin_layers_panel && self.edit_mode {
            let ctrl_held = ctx.input(|i| i.modifiers.ctrl);
            if ctrl_held || key_held {
                self.show_layers_panel = true;
            }
        }

        if self.edit_mode && was_edit {
            if mouse.left_just_pressed { self.last_action_time = std::time::Instant::now(); }
        }
        
        // ---- Auto-hide logic ----
        if !self.edit_mode && self.settings.auto_hide_seconds > 0.0 {
            let elapsed = self.last_action_time.elapsed().as_secs_f32();
            if elapsed > self.settings.auto_hide_seconds {
                self.settings.hide_all = true;
            }
        } else if self.edit_mode {
            // When entering edit mode, always show
            if !was_edit { self.settings.hide_all = false; }
            self.last_action_time = std::time::Instant::now();
        }

        // ---- 3. Interaction logic (Passthrough) ----
        let is_over_ui = ctx.memory(|mem| {
            mem.layer_ids().any(|layer| {
                if layer.order == egui::Order::Background { return false; }
                if !self.is_window_open(layer.id, ctx) { return false; }
                if let Some(rect) = mem.area_rect(layer.id) {
                    rect.contains(mouse.pos)
                } else {
                    false
                }
            })
        });

        // Force interactive for the first few frames to ensure focus and layout calculation
        let widgets_visible = self.settings.show_calculator || self.settings.show_volume_mixer;
        let should_be_interactive = self.frame_count < 10 || self.edit_mode || (self.settings.keep_ui_visible && is_over_ui) || (widgets_visible && is_over_ui);
        let passthrough = !should_be_interactive;

        if passthrough != self.prev_passthrough {
            ctx.send_viewport_cmd(egui::ViewportCommand::MousePassthrough(passthrough));
            self.prev_passthrough = passthrough;
        }

        // ---- 3b. Update Capture Exclusion (Fix live snip mirror loop) ----
        if self.initialized {
            let mut expected_exclude = self.settings.exclude_from_capture;
            let has_active_live_desktop_snip = self.project.layers.iter()
                .filter(|l| l.visible)
                .flat_map(|l| &l.placed_images)
                .any(|img| img.is_live && img.capture_source == CaptureSource::Desktop && !img.snip_source_overlay);
            if has_active_live_desktop_snip && self.settings.auto_exclude_live_capture {
                expected_exclude = true;
            }

            if self.prev_exclude_capture != Some(expected_exclude) {
                winapi_utils::set_capture_exclusion(expected_exclude);
                self.prev_exclude_capture = Some(expected_exclude);
            }
        }

        // ---- 4. On mode change & Initial Focus ----
        if self.edit_mode != was_edit || self.frame_count < 5 {
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));

            if self.edit_mode || self.frame_count < 5 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            } else {
                // Leaving edit → finalize pending work
                if let Some(pending) = self.pending_text.take() {
                    if !pending.buffer.is_empty() {
                        let text_str = pending.buffer.clone();
                        let target_layer_idx = pending.layer_idx.unwrap_or(self.project.active_layer);
                        if target_layer_idx < self.project.layers.len() {
                            let mut ann = if let Some(mut orig) = pending.original {
                                orig.text = text_str.clone();
                                orig.position = pending.position;
                                orig.color = self.settings.pen_color;
                                orig.font = self.settings.text_font;
                                orig.font_size = self.settings.font_size;
                                orig.shadow = self.settings.text_shadow;
                                orig.outline = self.settings.text_outline;
                                orig.wave_warp = self.settings.text_wave_warp;
                                orig
                            } else {
                                overlay::TextAnnotation::new(pending.position, text_str.clone(), self.settings.pen_color, self.settings.font_size)
                            };
                            let font = crate::tools::text::resolve_font(ann.font, ann.font_size);
                            let galley = ctx.fonts(|f| f.layout_no_wrap(text_str.clone(), font, egui::Color32::WHITE));
                            ann.exact_size = [galley.size().x, galley.size().y];
                            self.project.layers[target_layer_idx].text_annotations.push(ann);
                            self.project.layers[target_layer_idx].expanded = true;
                        }
                        self.history.push(&self.project, format!("Text: {}", text_str));
                    }
                }
                if !self.current_stroke.is_empty() {
                    if let Some(layer) = self.project.get_active_layer_mut() {
                        let s = Stroke::new(
                            self.current_stroke.clone(),
                            self.settings.pen_color,
                            self.settings.pen_width,
                            overlay::StrokeKind::Freehand,
                            self.settings.brush_mode,
                            None,
                            self.settings.brush_shadow,
                            self.settings.brush_shape,
                            self.settings.brush_outline,
                            self.settings.brush_arrow,
                            self.settings.spray_density,
                            self.settings.highlight_opacity,
                            self.settings.arrow_size,
                        );
                        layer.strokes.push(s);
                    }
                    self.history.push(&self.project, "Brush Stroke");
                    self.current_stroke.clear();
                    self.project.save();
                }
                self.line_start = None;
                self.initial_bounds = None;
                self.initial_center = None;
                
                // Save project automatically when exiting edit mode
                self.project.save();
            }
        }
        // ---- 4. Text keyboard input ----
        if self.edit_mode {
            if let Some(pending) = self.pending_text.as_mut() {
                let mut finalize = false;
                let mut cancel = false;

                egui::Area::new(egui::Id::new("inline_text_edit"))
                    .fixed_pos(pending.position)
                    .show(ctx, |ui| {
                        let font = crate::tools::text::resolve_font(self.settings.text_font, self.settings.font_size);
                        let edit_resp = ui.add(
                            egui::TextEdit::singleline(&mut pending.buffer)
                                .frame(false)
                                .text_color(crate::utils::color32(&self.settings.pen_color))
                                .font(font)
                                .desired_width(500.0)
                        );
                        edit_resp.request_focus();
                        
                        if ui.input(|i| i.key_pressed(egui::Key::Enter)) || (edit_resp.lost_focus() && !ui.input(|i| i.key_pressed(egui::Key::Escape))) {
                            finalize = true;
                        }
                    });

                if ctx.input(|i| i.key_pressed(egui::Key::Escape)) { cancel = true; }

                if finalize {
                    if let Some(p) = self.pending_text.take() {
                        if !p.buffer.is_empty() {
                            let text_str = p.buffer.clone();
                            let is_edit = p.original.is_some();
                            let mut ann = if let Some(mut orig) = p.original {
                                orig.text = text_str.clone();
                                orig.position = p.position;
                                orig.color = self.settings.pen_color;
                                orig.font = self.settings.text_font;
                                orig.font_size = self.settings.font_size;
                                orig.shadow = self.settings.text_shadow;
                                orig.outline = self.settings.text_outline;
                                orig.wave_warp = self.settings.text_wave_warp;
                                orig
                            } else {
                                let mut a = overlay::TextAnnotation::new(p.position, text_str.clone(), self.settings.pen_color, self.settings.font_size);
                                a.monospace = self.settings.text_monospace;
                                a.shadow = self.settings.text_shadow;
                                a.outline = self.settings.text_outline;
                                a.stroke_width = self.settings.text_stroke_width;
                                a.font = self.settings.text_font;
                                a.wave_warp = self.settings.text_wave_warp;
                                a
                            };
                            
                            let font = crate::tools::text::resolve_font(ann.font, ann.font_size);
                            let galley = ctx.fonts(|f| f.layout_no_wrap(text_str.clone(), font, egui::Color32::WHITE));
                            ann.exact_size = [galley.size().x, galley.size().y];
                            
                            let target_layer_idx = p.layer_idx.unwrap_or(self.project.active_layer);
                            let is_locked = target_layer_idx < self.project.layers.len() && self.project.layers[target_layer_idx].locked;
                            let ask_mode = self.settings.auto_new_layer.is_none();
                            
                            if is_edit {
                                if target_layer_idx < self.project.layers.len() {
                                    self.project.layers[target_layer_idx].text_annotations.push(ann);
                                    let new_idx = self.project.layers[target_layer_idx].text_annotations.len() - 1;
                                    self.project.selected_object = Some(SelectedObject {
                                        layer_idx: target_layer_idx,
                                        object_type: crate::overlay::ObjectType::Text,
                                        object_idx: new_idx,
                                    });
                                    self.project.layers[target_layer_idx].expanded = true;
                                }
                                self.history.push(&self.project, format!("Text: {}", text_str));
                                self.project.save();
                            } else if is_locked || ask_mode {
                                self.pending_text_to_add = Some(ann);
                                self.layer_prompt_open = true;
                            } else {
                                if target_layer_idx < self.project.layers.len() {
                                    self.project.layers[target_layer_idx].text_annotations.push(ann);
                                    let new_idx = self.project.layers[target_layer_idx].text_annotations.len() - 1;
                                    self.project.selected_object = Some(SelectedObject {
                                        layer_idx: target_layer_idx,
                                        object_type: crate::overlay::ObjectType::Text,
                                        object_idx: new_idx,
                                    });
                                    self.project.layers[target_layer_idx].expanded = true;
                                }
                                self.history.push(&self.project, format!("Text: {}", text_str));
                                self.project.save();
                            }
                        }
                    }
                } else if cancel {
                    if let Some(p) = self.pending_text.take() {
                        if let Some(orig) = p.original {
                            if let Some(layer_idx) = p.layer_idx {
                                if layer_idx < self.project.layers.len() {
                                    self.project.layers[layer_idx].text_annotations.push(orig);
                                }
                            }
                        }
                    }
                }
            }
        // ── Undo / Redo ──
            if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Z)) {
                if let Some(snap) = self.history.undo() {
                    let snap = snap.clone();
                    self.project = snap;
                    self.project.selected_object = None;
                }
            }
            if ctx.input(|i| i.modifiers.ctrl && (i.key_pressed(egui::Key::Y) || (i.modifiers.shift && i.key_pressed(egui::Key::Z)))) {
                if let Some(snap) = self.history.redo() {
                    let snap = snap.clone();
                    self.project = snap;
                    self.project.selected_object = None;
                }
            }
            if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::S)) {
                self.project.save();
            }
            let trigger_copy = ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::C)) || self.project.request_copy;
            let trigger_cut = ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::X)) || self.project.request_cut;
            let trigger_blur = self.project.request_blur;
            
            if trigger_copy || trigger_cut || trigger_blur {
                self.project.request_copy = false;
                self.project.request_cut = false;
                self.project.request_blur = false;
                
                if let Some(sel) = &self.project.marquee_selection {
                    let ppp = ctx.pixels_per_point();
                    let bounds = sel.bounds();
                    let (wx, wy) = crate::winapi_utils::get_window_screen_pos();
                    let render_offset = if self.settings.use_absolute_screen_coords {
                        egui::vec2(wx as f32 / ppp, wy as f32 / ppp)
                    } else {
                        egui::Vec2::ZERO
                    };
                    let screen_bounds = bounds.translate(-render_offset);
                    let sw = (screen_bounds.width() * ppp).round() as i32;
                    let sh = (screen_bounds.height() * ppp).round() as i32;
                    if sw > 5 && sh > 5 {
                        if self.settings.snip_source_overlay {
                            self.rasterize_bbox = Some([screen_bounds.min.x, screen_bounds.min.y, screen_bounds.max.x, screen_bounds.max.y]);
                            self.marquee_capture_request = Some(MarqueeCaptureRequest {
                                trigger_copy,
                                trigger_cut,
                                trigger_blur,
                                marquee_selection: sel.clone(),
                            });
                            self.rasterize_phase = 1;
                            ctx.request_repaint();
                        } else {
                            let sx = (screen_bounds.min.x * ppp) as i32 + if self.settings.use_absolute_screen_coords { 0 } else { wx };
                            let sy = (screen_bounds.min.y * ppp) as i32 + if self.settings.use_absolute_screen_coords { 0 } else { wy };
                            
                            if self.settings.snip_source == CaptureSource::Origin {
                                let logical_center_x = screen_bounds.min.x + screen_bounds.width() / 2.0;
                                let logical_center_y = screen_bounds.min.y + screen_bounds.height() / 2.0;
                                let center_x = (logical_center_x * ppp).round() as i32 + if self.settings.use_absolute_screen_coords { 0 } else { wx };
                                let center_y = (logical_center_y * ppp).round() as i32 + if self.settings.use_absolute_screen_coords { 0 } else { wy };
                                if let Some((hwnd, _name, _rect)) = crate::winapi_utils::get_window_at_point(center_x, center_y) {
                                    self.settings.origin_target_hwnd = hwnd as isize;
                                    self.settings.save();
                                }
                            }
                            
                            let captured_pixels = if trigger_copy || trigger_cut || !self.settings.snip_live {
                                crate::tools::snip::capture_screen_rect_safe(&self.settings, sx, sy, sw, sh)
                            } else {
                                None
                            };
                            
                            let mut mask = vec![255u8; sw as usize * sh as usize];
                            for py in 0..sh as usize {
                                for px in 0..sw as usize {
                                    let lp = screen_bounds.min + egui::vec2(px as f32 / ppp, py as f32 / ppp);
                                    let wp = lp + render_offset;
                                    if !sel.contains(wp) {
                                        mask[py * sw as usize + px] = 0;
                                    }
                                }
                            }
                            
                            let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
                            let img = if trigger_blur {
                                let pixels = captured_pixels.unwrap_or_default();
                                let mut new_img = overlay::PlacedImage::new(id, bounds.min, [sw as usize, sh as usize], pixels);
                                new_img.snip_source_overlay = self.settings.snip_source_overlay;
                                new_img.display_size = Some([bounds.width(), bounds.height()]);
                                new_img.source_rect = Some([bounds.min.x, bounds.min.y, bounds.width(), bounds.height()]);
                                new_img.show_source_rect = self.settings.show_source_rect;
                                new_img.snip_points = Some(marquee_to_local_points(&sel, bounds.min));
                                new_img.shadow = self.settings.snip_shadow;
                                new_img.is_live = self.settings.snip_live;
                                new_img.blur = self.settings.blur_strength;
                                new_img.blur_effect = self.settings.blur_effect;
                                new_img.capture_source = self.settings.snip_source;
                                new_img.target_hwnd = self.settings.origin_target_hwnd;
                                new_img.mask = Some(mask);
                                new_img.mask_size = Some([sw as usize, sh as usize]);
                                new_img.mask_dirty = true;
                                new_img.name = "Blur".to_string();
                                new_img
                            } else {
                                let mut pixels = captured_pixels.unwrap_or_default();
                                for py in 0..sh as usize {
                                    for px in 0..sw as usize {
                                        if mask[py * sw as usize + px] == 0 {
                                            let idx = (py * sw as usize + px) * 4;
                                            if idx + 3 < pixels.len() {
                                                pixels[idx + 3] = 0;
                                            }
                                        }
                                    }
                                }
                                
                                let mut new_img = overlay::PlacedImage::new(id, bounds.min, [sw as usize, sh as usize], pixels);
                                new_img.name = "Marquee".to_string();
                                new_img.snip_source_overlay = self.settings.snip_source_overlay;
                                new_img.display_size = Some([bounds.width(), bounds.height()]);
                                new_img.source_rect = Some([bounds.min.x, bounds.min.y, bounds.width(), bounds.height()]);
                                new_img.show_source_rect = self.settings.show_source_rect;
                                new_img.snip_points = Some(marquee_to_local_points(&sel, bounds.min));
                                new_img.shadow = self.settings.snip_shadow;
                                new_img.is_live = self.settings.snip_live;
                                new_img.blur = self.settings.blur_strength;
                                new_img.blur_effect = self.settings.blur_effect;
                                new_img.capture_source = self.settings.snip_source;
                                new_img.target_hwnd = self.settings.origin_target_hwnd;
                                
                                let is_rect = matches!(sel.shape, SelectionShape::Rect(_)) && sel.ops.is_empty();
                                if !is_rect {
                                    new_img.mask = Some(mask);
                                    new_img.mask_size = Some([sw as usize, sh as usize]);
                                    new_img.mask_dirty = true;
                                }
                                
                                crate::winapi_utils::copy_image_to_clipboard(&new_img.pixels, new_img.size[0], new_img.size[1]);
                                self.copied_image = Some(new_img.clone());
                                new_img
                            };
                            if trigger_cut {
                                self.history.push(&self.project, "Cut Selection");
                                crate::tools::cut::erase_marquee_selection(&mut self.project, &self.settings);
                                self.project.save();
                            } else {
                                let active_layer_idx = self.project.active_layer;
                                if active_layer_idx < self.project.layers.len() {
                                    let is_locked = self.project.layers[active_layer_idx].locked;
                                    let ask_mode = self.settings.auto_new_layer.is_none();
                                    
                                    if is_locked || ask_mode {
                                        self.project.layers[active_layer_idx].placed_images.push(img);
                                        self.project.layers[active_layer_idx].expanded = true;
                                        self.layer_prompt_open = true;
                                    } else {
                                        self.project.layers[active_layer_idx].placed_images.push(img);
                                        self.project.layers[active_layer_idx].expanded = true;
                                        
                                        let action_name = if trigger_blur {
                                            "Blur Area"
                                        } else {
                                            "Snip Selection"
                                        };
                                        self.history.push(&self.project, action_name);
                                        self.project.save();
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::V)) {
                let img_to_paste = if self.copied_image.is_some() {
                    self.copied_image.clone()
                } else if let Some((pixels, w, h)) = crate::winapi_utils::get_clipboard_image() {
                    let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
                    let mut img = overlay::PlacedImage::new(id, egui::pos2(100.0, 100.0), [w, h], pixels);
                    img.display_size = Some([w as f32, h as f32]);
                    img.source_rect = Some([100.0, 100.0, w as f32, h as f32]);
                    img.show_source_rect = self.settings.show_source_rect;
                    img.shadow = self.settings.snip_shadow;
                    Some(img)
                } else {
                    None
                };

                if let Some(img) = &img_to_paste {
                    let mut img_clone = img.clone();
                    let active_layer_idx = self.project.active_layer;
                    if active_layer_idx < self.project.layers.len() {
                        let is_locked = self.project.layers[active_layer_idx].locked;
                        
                        // Center it at mouse hover position if mouse is hovering in the canvas!
                        let mouse_pos = ctx.input(|i| i.pointer.hover_pos());
                        if let Some(mpos) = mouse_pos {
                            let disp_size = img_clone.display_size.unwrap_or([img_clone.size[0] as f32, img_clone.size[1] as f32]);
                            img_clone.position = mpos - egui::vec2(disp_size[0] * 0.5, disp_size[1] * 0.5);
                        } else {
                            // Just shift it slightly from the copied position
                            img_clone.position += egui::vec2(20.0, 20.0);
                        }
                        
                        let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
                        img_clone.id = id;
                        
                        if is_locked {
                            // If locked, create a new layer automatically for the paste
                            self.project.layers.push(crate::project::Layer::new(&format!("Pasted Layer {}", self.project.layers.len() + 1)));
                            self.project.active_layer = self.project.layers.len() - 1;
                            self.project.layers.last_mut().unwrap().placed_images.push(img_clone);
                            self.project.layers.last_mut().unwrap().expanded = true;
                        } else {
                            self.project.layers[active_layer_idx].placed_images.push(img_clone);
                            self.project.layers[active_layer_idx].expanded = true;
                        }
                        
                        // Select the newly pasted object!
                        let new_idx = self.project.layers[self.project.active_layer].placed_images.len() - 1;
                        self.project.selected_object = Some(SelectedObject {
                            layer_idx: self.project.active_layer,
                            object_type: ObjectType::Image,
                            object_idx: new_idx,
                        });
                        
                        self.history.push(&self.project, "Paste Image");
                        self.project.save();
                    }
                }
            }
            if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::D)) {
                if self.project.marquee_selection.is_some() {
                    self.project.marquee_selection = None;
                } else {
                    self.show_debug_window = !self.show_debug_window;
                }
            }
            if ctx.input(|i| i.key_pressed(egui::Key::Delete)) {
                if self.project.marquee_selection.is_some() {
                    crate::tools::cut::erase_marquee_selection(&mut self.project, &self.settings);
                    self.history.push(&self.project, "Erase Selection");
                    self.project.save();
                }
            }
        }

        // ---- Rasterize Pipeline ----
        if self.rasterize_phase == 0 {
            if let Some(req) = self.project.rasterize_request {
                let ppp = self.settings.ui_scale;
                let (wx, wy) = crate::winapi_utils::get_window_screen_pos();
                let render_offset = if self.settings.use_absolute_screen_coords {
                    egui::vec2(wx as f32 / ppp, wy as f32 / ppp)
                } else { egui::Vec2::ZERO };
                if let Some(bbox) = rasterize::compute_target_bbox(&self.project, &req, render_offset) {
                    self.rasterize_bbox = Some(bbox);
                    self.rasterize_phase = 1;
                    ctx.request_repaint();
                } else {
                    self.project.rasterize_request = None;
                }
            }
        } else if self.rasterize_phase == 2 {
            let captured = if let Ok(mut buf) = self.rasterize_capture.lock() { buf.take() } else { None };
            if let Some(frame) = captured {
                let req_opt = self.project.rasterize_request.take();
                if let Some(req) = req_opt {
                    let ppp = self.settings.ui_scale;
                    let (wx, wy) = crate::winapi_utils::get_window_screen_pos();
                    let render_offset = if self.settings.use_absolute_screen_coords {
                        egui::vec2(wx as f32 / ppp, wy as f32 / ppp)
                    } else { egui::Vec2::ZERO };
                    rasterize::finalize_rasterize(&mut self.project, &req, frame, render_offset);
                    self.history.push(&self.project, "Rasterize");
                } else if let Some(m_req) = self.marquee_capture_request.take() {
                    self.finalize_marquee_capture(frame, m_req, ctx);
                }
                self.rasterize_phase = 0;
                self.rasterize_bbox = None;
                self.settings.fx_open = None;
            } else {
                ctx.request_repaint();
            }
        }

        // ---- 5. Render UI ----
        let rasterizing = self.rasterize_phase == 1;
        let show_ui = (self.edit_mode || self.settings.keep_ui_visible) && !rasterizing;
        let fps = if self.settings.live_performance_mode {
            10.0
        } else {
            self.settings.capture_fps
        };
        self.capture_thread.set_fps(fps);
        if show_ui {
            // println!("DEBUG: Frame {} | show_ui=true | edit_mode={} | rasterize_phase={} | req={:?}", self.frame_count, self.edit_mode, self.rasterize_phase, self.project.rasterize_request.is_some());
            overlay::render_mode_indicator(ctx, self.edit_mode, self.settings.hotkey.display_name(), self.settings.toggle_mode, &self.settings, &self.owl_icon);
            let mut embed_trigger = false;
            render_toolbar(ctx, &mut self.active_tool, &mut self.settings, &mut self.show_settings_panel, &mut self.show_layers_panel, &mut self.show_exit_dialog, &mut self.project, &mut self.embed_url, &mut embed_trigger, &mut self.show_history_panel, &mut self.request_history_push, &mut self.filters_open, &self.volume_sessions);
            if embed_trigger { self.handle_embed_trigger(); }
            
            render_filter_menu(ctx, &mut self.project, &mut self.settings, &mut self.filters_open);
            
            if self.show_exit_dialog {
                let mut close = false;
                egui::Window::new(egui::RichText::new("Exit Owerlayer?").color(egui::Color32::from_rgb(255, 100, 100)))
                    .id(egui::Id::new("exit_dialog"))
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .frame(photoshop_frame(&self.settings))
                    .show(ctx, |ui| {
                        ui.label("Are you sure you want to close Owerlayer?");
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            if ui.button("Yes, Close").clicked() {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                            if ui.button("Cancel").clicked() {
                                close = true;
                            }
                        });
                    });
                if close { self.show_exit_dialog = false; }
            }

            if self.show_settings_panel && self.edit_mode {
                let mut clear_all = false;
                render_settings_window(ctx, &mut self.settings, &mut self.show_settings_panel,
                    &mut clear_all, &mut self.listening_for_hotkey, &self.owl_icon);
                if clear_all {
                    if let Some(layer) = self.project.get_active_layer_mut() {
                        layer.strokes.clear(); layer.text_annotations.clear();
                        layer.placed_images.clear();
                    }
                    self.current_stroke.clear(); self.pending_text = None;
                }
            }
            if self.show_layers_panel && self.edit_mode {
                // println!("DEBUG: Rendering Layers window");
                render_layers_window(ctx, &mut self.project, &mut self.settings, &mut self.active_tool, &mut self.show_layers_panel, &mut self.filters_open, &mut self.load_picker_open);
            }
            if self.show_history_panel && self.edit_mode {
                if let Some(snap) = ui::history_menu::render_history_window(ctx, &mut self.history, &mut self.show_history_panel, &mut self.settings) {
                    self.project = snap;
                    self.project.selected_object = None;
                }
            }

            if self.settings.show_profiler && self.edit_mode {
                if let Some(ref display) = self.perf_display {
                    let mut show = self.settings.show_profiler;
                    egui::Window::new("Performance Profiler")
                        .open(&mut show)
                        .resizable(true)
                        .collapsible(true)
                        .default_size([250.0, 300.0])
                        .default_pos(egui::pos2(800.0, 100.0))
                        .frame(photoshop_frame(&self.settings))
                        .show(ctx, |ui| {
                            ui.label(format!("Captured {} frames in last 1.0s", display.frame_count));
                            ui.separator();
                            egui::Grid::new("perf_grid").num_columns(2).spacing([40.0, 4.0]).show(ui, |ui| {
                                ui.label("WGC GPU Copy:");
                                ui.label(format!("{:.1} μs", display.wgc_gpu_copy_us));
                                ui.end_row();

                                ui.label("WGC CPU-GPU Map:");
                                ui.label(format!("{:.1} μs", display.wgc_map_wait_us));
                                ui.end_row();

                                ui.label("BGRA -> RGBA Swap:");
                                ui.label(format!("{:.1} μs", display.wgc_pixel_swap_us));
                                ui.end_row();

                                ui.label("GDI Capture:");
                                ui.label(format!("{:.1} μs", display.gdi_capture_us));
                                ui.end_row();

                                ui.label("Blur & Mask:");
                                ui.label(format!("{:.1} μs", display.thread_mask_effects_us));
                                ui.end_row();

                                ui.label("Color32 Conv:");
                                ui.label(format!("{:.1} μs", display.thread_color32_conv_us));
                                ui.end_row();

                                ui.label("Thread Total:");
                                ui.label(format!("{:.1} μs", display.thread_total_us));
                                ui.end_row();

                                ui.label("GPU Tex Upload:");
                                ui.label(format!("{:.1} μs", display.upload_us));
                                ui.end_row();
                            });
                        });
                    self.settings.show_profiler = show;
                }
            }

            if self.settings.ui_reset_frames > 0 {
                self.settings.ui_reset_frames -= 1;
            }

            if self.layer_prompt_open && self.edit_mode {
                let layer_idx = self.project.active_layer;
                if layer_idx < self.project.layers.len() {
                    if !self.project.layers[layer_idx].lock_prompt_dismissed {
                        self.project.layers[layer_idx].lock_prompt_choice = None;
                    }
                }

                // If the layer is empty and not locked, skip the prompt entirely and just create a new object
                let layer_is_empty_and_unlocked = if layer_idx < self.project.layers.len() {
                    let layer = &self.project.layers[layer_idx];
                    !layer.locked && layer.strokes.is_empty() && layer.placed_images.is_empty() && layer.text_annotations.is_empty()
                } else {
                    false
                };

                let already_dismissed = self.project.layers
                    .get(layer_idx)
                    .map_or(false, |l| l.lock_prompt_dismissed && l.lock_prompt_choice.is_some());

                let mut close_prompt = false;
                let mut action: Option<u8> = None; // 1=unlock+use, 2=new object, 3=new layer, 4=same object
                let mut save_preference = false;

                if layer_is_empty_and_unlocked {
                    // Layer is empty - no need to ask, just create a new object directly
                    action = Some(2);
                    close_prompt = true;
                } else if already_dismissed {
                    action = self.project.layers.get(layer_idx).and_then(|l| l.lock_prompt_choice);
                    close_prompt = true;
                } else {
                    let layer_locked = layer_idx < self.project.layers.len() && self.project.layers[layer_idx].locked;
                    let mut obj_locked = false;
                    if let Some(sel) = self.project.selected_object {
                        if sel.layer_idx == layer_idx && layer_idx < self.project.layers.len() {
                            obj_locked = match sel.object_type {
                                crate::overlay::ObjectType::Image => {
                                    self.project.layers[layer_idx].placed_images.get(sel.object_idx).map_or(false, |img| img.locked)
                                }
                                crate::overlay::ObjectType::Stroke => {
                                    self.project.layers[layer_idx].strokes.get(sel.object_idx).map_or(false, |st| st.locked)
                                }
                                crate::overlay::ObjectType::Text => {
                                    self.project.layers[layer_idx].text_annotations.get(sel.object_idx).map_or(false, |t| t.locked)
                                }
                            };
                        }
                    }
                    let is_locked = layer_locked || obj_locked;
                    let (title, label) = if is_locked {
                        ("Layer is Locked", "The current layer or object is locked. What would you like to do?")
                    } else {
                        ("New Content Options", "You are creating new content. What would you like to do?")
                    };

                    let is_vector = self.active_tool == crate::overlay::Tool::Shape || self.active_tool == crate::overlay::Tool::Brush;
                    let new_content_is_in_placed_images = self.pending_stroke.is_none();
                    let has_merge_target = if self.pending_stroke.is_some() {
                        if is_vector {
                            self.project.layers.get(layer_idx).map_or(false, |l| {
                                l.strokes.iter().any(|st| st.kind == crate::types::StrokeKind::Freehand)
                            })
                        } else {
                            self.project.layers.get(layer_idx).map_or(false, |l| {
                                l.placed_images.iter().any(|img| !img.locked)
                            })
                        }
                    } else if let Some(sel) = self.project.selected_object {
                        let limit = if new_content_is_in_placed_images {
                            self.project.layers.get(layer_idx).map_or(0, |l| l.placed_images.len()).saturating_sub(1)
                        } else {
                            self.project.layers.get(layer_idx).map_or(0, |l| l.placed_images.len())
                        };
                        sel.layer_idx == layer_idx && sel.object_type == crate::overlay::ObjectType::Image
                            && sel.object_idx < limit
                    } else {
                        if layer_idx < self.project.layers.len() {
                            let len = self.project.layers[layer_idx].placed_images.len();
                            if new_content_is_in_placed_images {
                                len > 1
                            } else {
                                len > 0
                            }
                        } else {
                            false
                        }
                    };

                    let prompt_resp = egui::Window::new(title)
                        .id(egui::Id::new("creation_prompt"))
                        .collapsible(false)
                        .resizable(false)
                        .default_pos(self.settings.creation_prompt_pos)
                        .frame(photoshop_frame(&self.settings))
                        .show(ctx, |ui| {
                            ui.label(label);
                            ui.add_space(8.0);

                            ui.horizontal(|ui| {
                                if is_locked {
                                    if ui.button("🔓 Use Current Object (Unlock)").clicked() {
                                        action = Some(1);
                                        close_prompt = true;
                                        save_preference = true;
                                    }
                                }
                                if has_merge_target {
                                    if ui.button("✏️ Add to Same Object").clicked() {
                                        action = Some(4);
                                        close_prompt = true;
                                        save_preference = true;
                                    }
                                }
                                if ui.button("📄 New Object (Same Layer)").clicked() {
                                    action = Some(2);
                                    close_prompt = true;
                                    save_preference = true;
                                }
                                if ui.button("➕ New Layer").clicked() {
                                    action = Some(3);
                                    close_prompt = true;
                                    save_preference = true;
                                }
                                if ui.button("Cancel").clicked() {
                                    close_prompt = true;
                                }
                            });

                            ui.add_space(6.0);
                            if layer_idx < self.project.layers.len() {
                                let mut dismissed = self.project.layers[layer_idx].lock_prompt_dismissed;
                                if ui.checkbox(&mut dismissed, "Don't ask again for this layer").changed() {
                                    self.project.layers[layer_idx].lock_prompt_dismissed = dismissed;
                                    if !dismissed {
                                        self.project.layers[layer_idx].lock_prompt_choice = None;
                                    }
                                }
                            }
                        });

                    if let Some(resp) = prompt_resp {
                        if resp.response.dragged() {
                            let layer_id = resp.response.layer_id;
                            if let Some(rect) = ctx.memory(|m| m.area_rect(layer_id.id)) {
                                self.settings.creation_prompt_pos = rect.min;
                            }
                        }
                    }

                    crate::utils::enforce_window_bounds(ctx, egui::Id::new("creation_prompt"), &mut self.settings.creation_prompt_pos, 100.0, 100.0);

                    if save_preference && layer_idx < self.project.layers.len() {
                        if self.project.layers[layer_idx].lock_prompt_dismissed {
                            self.project.layers[layer_idx].lock_prompt_choice = action;
                        } else {
                            self.project.layers[layer_idx].lock_prompt_choice = None;
                        }
                    }
                }

                if let Some(act) = action {
                    if let Some(s) = self.pending_stroke.take() {
                        let is_vector = self.active_tool == crate::overlay::Tool::Shape || self.active_tool == crate::overlay::Tool::Brush;
                        
                        match act {
                            1 => {
                                // Unlock
                                if let Some(sel) = self.project.selected_object {
                                    if sel.layer_idx == layer_idx && layer_idx < self.project.layers.len() {
                                        match sel.object_type {
                                            crate::overlay::ObjectType::Image => {
                                                if let Some(img) = self.project.layers[layer_idx].placed_images.get_mut(sel.object_idx) {
                                                    img.locked = false;
                                                }
                                            }
                                            crate::overlay::ObjectType::Stroke => {
                                                if let Some(st) = self.project.layers[layer_idx].strokes.get_mut(sel.object_idx) {
                                                    st.locked = false;
                                                }
                                            }
                                            crate::overlay::ObjectType::Text => {
                                                if let Some(t) = self.project.layers[layer_idx].text_annotations.get_mut(sel.object_idx) {
                                                    t.locked = false;
                                                }
                                            }
                                        }
                                    }
                                } else if layer_idx < self.project.layers.len() {
                                    self.project.layers[layer_idx].locked = false;
                                }
                                
                                // Apply
                                if layer_idx < self.project.layers.len() {
                                    if is_vector {
                                        self.project.layers[layer_idx].strokes.push(s);
                                    } else {
                                        let mut applied = false;
                                        if let Some(sel) = self.project.selected_object {
                                            if sel.layer_idx == layer_idx && sel.object_type == crate::overlay::ObjectType::Image {
                                                if let Some(img) = self.project.layers[layer_idx].placed_images.get_mut(sel.object_idx) {
                                                    crate::tools::brush::rasterize_stroke_to_image(img, &s, &self.settings);
                                                    applied = true;
                                                }
                                            }
                                        }
                                        if !applied {
                                            let reuse_idx = self.project.layers[layer_idx].placed_images.iter().rposition(|img| !img.locked);
                                            if let Some(idx) = reuse_idx {
                                                let img = &mut self.project.layers[layer_idx].placed_images[idx];
                                                crate::tools::brush::rasterize_stroke_to_image(img, &s, &self.settings);
                                            } else {
                                                let ppp = ctx.pixels_per_point();
                                                let logical_w = 800.0f32;
                                                let logical_h = 600.0f32;
                                                let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
                                                let count = self.project.layers[layer_idx].placed_images.len();
                                                let mut new_img = crate::tools::brush::create_new_canvas(id, egui::pos2(100.0, 100.0), logical_w, logical_h, ppp);
                                                new_img.name = format!("Canvas {}", count + 1);
                                                crate::tools::brush::rasterize_stroke_to_image(&mut new_img, &s, &self.settings);
                                                self.project.layers[layer_idx].placed_images.push(new_img);
                                            }
                                        }
                                    }
                                    if layer_idx < self.project.layers.len() { self.project.layers[layer_idx].expanded = true; }
                                    self.request_history_push = Some("Draw".into());
                                }
                            }
                            2 => {
                                if layer_idx < self.project.layers.len() {
                                    if is_vector {
                                        let mut s = s;
                                        let count = if let Some(layer) = self.project.layers.get(layer_idx) {
                                            let mut names = std::collections::HashSet::new();
                                            for stroke in &layer.strokes {
                                                if stroke.kind == crate::types::StrokeKind::Freehand {
                                                    names.insert(&stroke.name);
                                                }
                                            }
                                            names.len()
                                        } else {
                                            0
                                        };
                                        s.name = format!("Brush Stroke {}", count + 1);
                                        self.project.layers[layer_idx].strokes.push(s);
                                        
                                        let new_idx = self.project.layers[layer_idx].strokes.len() - 1;
                                        self.project.selected_object = Some(SelectedObject {
                                            layer_idx,
                                            object_type: crate::overlay::ObjectType::Stroke,
                                            object_idx: new_idx,
                                        });
                                    } else {
                                        let ppp = ctx.pixels_per_point();
                                        let logical_w = 800.0f32;
                                        let logical_h = 600.0f32;
                                        let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
                                        let count = self.project.layers[layer_idx].placed_images.len();
                                        let mut new_img = crate::tools::brush::create_new_canvas(id, egui::pos2(100.0, 100.0), logical_w, logical_h, ppp);
                                        new_img.name = format!("Canvas {}", count + 1);
                                        crate::tools::brush::rasterize_stroke_to_image(&mut new_img, &s, &self.settings);
                                        self.project.layers[layer_idx].placed_images.push(new_img);
                                        
                                        let new_idx = self.project.layers[layer_idx].placed_images.len() - 1;
                                        self.project.selected_object = Some(SelectedObject {
                                            layer_idx,
                                            object_type: crate::overlay::ObjectType::Image,
                                            object_idx: new_idx,
                                        });
                                    }
                                    if layer_idx < self.project.layers.len() { self.project.layers[layer_idx].expanded = true; }
                                    self.request_history_push = Some("Draw".into());
                                }
                            }
                            3 => {
                                self.project.layers.push(crate::project::Layer::new(self.active_tool.name()));
                                let new_layer_idx = self.project.layers.len() - 1;
                                self.project.active_layer = new_layer_idx;
                                self.project.selected_object = None;
                                
                                if is_vector {
                                    self.project.layers[new_layer_idx].strokes.push(s);
                                } else {
                                    let ppp = ctx.pixels_per_point();
                                    let logical_w = 800.0f32;
                                    let logical_h = 600.0f32;
                                    let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
                                    let mut new_img = crate::tools::brush::create_new_canvas(id, egui::pos2(100.0, 100.0), logical_w, logical_h, ppp);
                                    new_img.name = "Canvas 1".to_string();
                                    crate::tools::brush::rasterize_stroke_to_image(&mut new_img, &s, &self.settings);
                                    self.project.layers[new_layer_idx].placed_images.push(new_img);
                                    
                                    self.project.selected_object = Some(SelectedObject {
                                        layer_idx: new_layer_idx,
                                        object_type: crate::overlay::ObjectType::Image,
                                        object_idx: 0,
                                    });
                                }
                                if new_layer_idx < self.project.layers.len() { self.project.layers[new_layer_idx].expanded = true; }
                                self.request_history_push = Some("Draw".into());
                            }
                            4 => {
                                // Same Object: rasterize pending stroke into existing image
                                if layer_idx < self.project.layers.len() {
                                    if is_vector {
                                        let mut s = s;
                                        let mut target_name = None;
                                        if let Some(sel) = self.project.selected_object {
                                            if sel.layer_idx == layer_idx && sel.object_type == crate::overlay::ObjectType::Stroke {
                                                if let Some(stroke) = self.project.layers[layer_idx].strokes.get(sel.object_idx) {
                                                    if stroke.kind == crate::types::StrokeKind::Freehand {
                                                        target_name = Some(stroke.name.clone());
                                                    }
                                                }
                                            }
                                        }
                                        if target_name.is_none() {
                                            if let Some(stroke) = self.project.layers[layer_idx].strokes.iter().rfind(|st| st.kind == crate::types::StrokeKind::Freehand) {
                                                target_name = Some(stroke.name.clone());
                                            }
                                        }
                                        if let Some(name) = target_name {
                                            s.name = name;
                                        } else {
                                            s.name = "Brush Stroke 1".to_string();
                                        }
                                        self.project.layers[layer_idx].strokes.push(s);
                                    } else {
                                        let new_content_is_in_placed_images = false;
                                        let limit = if new_content_is_in_placed_images {
                                            self.project.layers[layer_idx].placed_images.len().saturating_sub(1)
                                        } else {
                                            self.project.layers[layer_idx].placed_images.len()
                                        };

                                        let mut target_idx = None;
                                        if let Some(sel) = self.project.selected_object {
                                            if sel.layer_idx == layer_idx && sel.object_type == crate::overlay::ObjectType::Image
                                                && sel.object_idx < limit {
                                                target_idx = Some(sel.object_idx);
                                            }
                                        }
                                        if target_idx.is_none() {
                                            if limit > 0 {
                                                target_idx = self.project.layers[layer_idx].placed_images[..limit]
                                                    .iter()
                                                    .rposition(|img| !img.locked);
                                            }
                                        }
                                        if let Some(idx) = target_idx {
                                            let img = &mut self.project.layers[layer_idx].placed_images[idx];
                                            img.locked = false;
                                            crate::tools::brush::rasterize_stroke_to_image(img, &s, &self.settings);
                                            self.project.selected_object = Some(SelectedObject {
                                                layer_idx,
                                                object_type: crate::overlay::ObjectType::Image,
                                                object_idx: idx,
                                            });
                                        } else {
                                            // Fallback: create new object
                                            let ppp = ctx.pixels_per_point();
                                            let logical_w = 800.0f32;
                                            let logical_h = 600.0f32;
                                            let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
                                            let count = self.project.layers[layer_idx].placed_images.len();
                                            let mut new_img = crate::tools::brush::create_new_canvas(id, egui::pos2(100.0, 100.0), logical_w, logical_h, ppp);
                                            new_img.name = format!("Canvas {}", count + 1);
                                            crate::tools::brush::rasterize_stroke_to_image(&mut new_img, &s, &self.settings);
                                            self.project.layers[layer_idx].placed_images.push(new_img);
                                        }
                                    }
                                    if layer_idx < self.project.layers.len() { self.project.layers[layer_idx].expanded = true; }
                                    self.request_history_push = Some("Draw".into());
                                }
                            }
                            _ => {}
                        }
                    } else {
                        match act {
                            1 => {
                                if let Some(sel) = self.project.selected_object {
                                    if sel.layer_idx == layer_idx && layer_idx < self.project.layers.len() {
                                        match sel.object_type {
                                            crate::overlay::ObjectType::Image => {
                                                if let Some(img) = self.project.layers[layer_idx].placed_images.get_mut(sel.object_idx) {
                                                    img.locked = false;
                                                }
                                            }
                                            crate::overlay::ObjectType::Stroke => {
                                                if let Some(st) = self.project.layers[layer_idx].strokes.get_mut(sel.object_idx) {
                                                    st.locked = false;
                                                }
                                            }
                                            crate::overlay::ObjectType::Text => {
                                                if let Some(t) = self.project.layers[layer_idx].text_annotations.get_mut(sel.object_idx) {
                                                    t.locked = false;
                                                }
                                            }
                                        }
                                    }
                                } else if layer_idx < self.project.layers.len() {
                                    self.project.layers[layer_idx].locked = false;
                                }
                            }
                            2 => {
                                if layer_idx < self.project.layers.len() { self.project.layers[layer_idx].expanded = true; }
                                self.project.selected_object = None;
                            }
                            3 => {
                                self.project.layers.push(crate::project::Layer::new(self.active_tool.name()));
                                let new_layer_idx = self.project.layers.len() - 1;
                                let prev_layer_idx = self.project.active_layer;
                                if prev_layer_idx < new_layer_idx {
                                                                    if self.active_tool == overlay::Tool::Snip || self.active_tool == overlay::Tool::Blur || self.active_tool == overlay::Tool::Embed || self.active_tool == overlay::Tool::Cut {
                                        if let Some(img) = self.project.layers[prev_layer_idx].placed_images.pop() {
                                            self.project.layers[new_layer_idx].placed_images.push(img);
                                        }
                                    }
                                }
                                self.project.active_layer = new_layer_idx;
                                self.project.selected_object = None;
                                if new_layer_idx < self.project.layers.len() { self.project.layers[new_layer_idx].expanded = true; }
                            }
                            4 => {
                                // Same Object: merge new snip/image into existing PlacedImage
                                if layer_idx < self.project.layers.len() {
                                    let new_content_is_in_placed_images = self.pending_stroke.is_none();
                                    let limit = if new_content_is_in_placed_images {
                                        self.project.layers[layer_idx].placed_images.len().saturating_sub(1)
                                    } else {
                                        self.project.layers[layer_idx].placed_images.len()
                                    };

                                    let mut target_idx = None;
                                    if let Some(sel) = self.project.selected_object {
                                        if sel.layer_idx == layer_idx && sel.object_type == crate::overlay::ObjectType::Image
                                            && sel.object_idx < limit {
                                            target_idx = Some(sel.object_idx);
                                        }
                                    }
                                    if target_idx.is_none() {
                                        if limit > 0 {
                                            target_idx = self.project.layers[layer_idx].placed_images[..limit]
                                                .iter()
                                                .rposition(|img| !img.locked);
                                        }
                                    }
                                    if let Some(tidx) = target_idx {
                                        // The new image was just pushed by the tool; pop it and merge into target
                                        let last_idx = self.project.layers[layer_idx].placed_images.len().saturating_sub(1);
                                        if last_idx != tidx && last_idx < self.project.layers[layer_idx].placed_images.len() {
                                            let new_img = self.project.layers[layer_idx].placed_images.remove(last_idx);
                                            let dest = &mut self.project.layers[layer_idx].placed_images[tidx];
                                            dest.locked = false;
                                            crate::tools::brush::merge_images(dest, &new_img);
                                        }
                                        // Recompute tidx in case remove shifted it
                                        let final_idx = tidx.min(self.project.layers[layer_idx].placed_images.len().saturating_sub(1));
                                        self.project.selected_object = Some(SelectedObject {
                                            layer_idx,
                                            object_type: crate::overlay::ObjectType::Image,
                                            object_idx: final_idx,
                                        });
                                    }
                                    if layer_idx < self.project.layers.len() { self.project.layers[layer_idx].expanded = true; }
                                }
                            }
                            _ => {}
                        }
                    }

                    if let Some(ann) = self.pending_text_to_add.take() {
                        let text_str = ann.text.clone();
                        match act {
                            1 => {
                                if layer_idx < self.project.layers.len() {
                                    self.project.layers[layer_idx].locked = false;
                                    self.project.layers[layer_idx].text_annotations.push(ann);
                                    let new_idx = self.project.layers[layer_idx].text_annotations.len() - 1;
                                    self.project.selected_object = Some(SelectedObject {
                                        layer_idx,
                                        object_type: crate::overlay::ObjectType::Text,
                                        object_idx: new_idx,
                                    });
                                    self.project.layers[layer_idx].expanded = true;
                                }
                            }
                            2 => {
                                if layer_idx < self.project.layers.len() {
                                    self.project.layers[layer_idx].text_annotations.push(ann);
                                    let new_idx = self.project.layers[layer_idx].text_annotations.len() - 1;
                                    self.project.selected_object = Some(SelectedObject {
                                        layer_idx,
                                        object_type: crate::overlay::ObjectType::Text,
                                        object_idx: new_idx,
                                    });
                                    self.project.layers[layer_idx].expanded = true;
                                }
                            }
                            3 => {
                                self.project.layers.push(crate::project::Layer::new("Text Layer"));
                                let new_layer_idx = self.project.layers.len() - 1;
                                self.project.active_layer = new_layer_idx;
                                self.project.layers[new_layer_idx].text_annotations.push(ann);
                                self.project.selected_object = Some(SelectedObject {
                                    layer_idx: new_layer_idx,
                                    object_type: crate::overlay::ObjectType::Text,
                                    object_idx: 0,
                                });
                                self.project.layers[new_layer_idx].expanded = true;
                            }
                            _ => {}
                        }
                        self.request_history_push = Some(format!("Text: {}", text_str));
                        self.project.save();
                    }
                } else if close_prompt {
                    self.pending_stroke = None;
                    self.pending_text_to_add = None;
                    self.pending_text = None;
                    if self.active_tool == overlay::Tool::Snip || self.active_tool == overlay::Tool::Blur || self.active_tool == overlay::Tool::Embed || self.active_tool == overlay::Tool::Cut {
                        let layer_idx = self.project.active_layer;
                        if layer_idx < self.project.layers.len() {
                            self.project.layers[layer_idx].placed_images.pop();
                        }
                    }
                }

                if close_prompt {
                    self.layer_prompt_open = false;
                    self.last_tool_used = Some(self.active_tool);
                    self.project.save();
                }
            }
        }

        if self.show_debug_window {
            egui::Window::new("Debug Overlay").id(egui::Id::new("Debug Overlay")).open(&mut self.show_debug_window).show(ctx, |ui| {
                ui.label(format!("Edit Mode: {}", self.edit_mode));
                ui.label(format!("Frame Count: {}", self.frame_count));
                ui.label(format!("Mouse Pos: {:.1}, {:.1}", mouse.pos.x, mouse.pos.y));
                ui.label(format!("Active Tool: {:?}", self.active_tool));
                ui.label(format!("Strokes: {}", self.project.get_active_layer().map(|l| l.strokes.len()).unwrap_or(0)));
                ui.label(format!("Text Anns: {}", self.project.get_active_layer().map(|l| l.text_annotations.len()).unwrap_or(0)));
                if ui.button("Test Software Rendering (Not implemented)").clicked() {
                    println!("Software rendering fallback test clicked.");
                }
            });
        }
        self.frame_count += 1;

        // ---- 6. Canvas ----
        // Check if the pointer is over any UI window (NOT the canvas CentralPanel).
        // ctx.is_pointer_over_area() is always true because the CentralPanel covers the screen.
        let mouse_pos_points = ctx.input(|i| i.pointer.hover_pos()).unwrap_or(mouse.pos);
        let is_over_ui_window = ctx.memory(|mem| {
            mem.layer_ids().any(|layer| {
                if layer.order == egui::Order::Background { return false; }
                if let Some(rect) = mem.area_rect(layer.id) {
                    rect.contains(mouse_pos_points)
                } else {
                    false
                }
            })
        });
        let can_draw = !is_over_ui_window;
        egui::CentralPanel::default()
            .frame(egui::Frame {
                fill: egui::Color32::TRANSPARENT,
                inner_margin: egui::Margin::ZERO,
                outer_margin: egui::Margin::ZERO,
                ..Default::default()
            })
            .show(ctx, |ui| {
                // 4. Update web widgets
            #[cfg(feature = "webengine")]
            {
                if crate::web_engine::is_available() {
                    crate::web_engine::update_renderer();
                    for layer in &mut self.project.layers {
                        for img in &mut layer.placed_images {
                            if let Some(widget_arc) = &img.web_widget {
                                if let Ok(mut widget) = widget_arc.lock() {
                                    widget.update_view();
                                    if widget.dirty {
                                        img.pixels = widget.pixels.clone();
                                        img.texture = None; img.thumbnail_dirty = true; // Force reload
                                        widget.dirty = false;
                                    }
                                }
                            }
                        }
                    }
                }
            }

                // ── Count placed images before canvas update (for Snip detection) ──
                let img_count_before: usize = self.project.layers.iter().map(|l| l.placed_images.len()).sum();

                overlay::render_canvas(
                    ui,
                    &mut self.active_tool,
                    &mut self.settings,
                    &mut self.project,
                    &mut self.current_stroke,
                    &mut self.line_start,
                    &mut self.initial_bounds,
                    &mut self.initial_center,
                    &mut self.initial_layer,
                    &mut self.initial_active_layer_idx,
                    &mut self.dragging_source_rect,
                    &mut self.pending_text,
                    &mut self.pending_stroke,
                    &mut self.pending_text_to_add,
                    &mut self.last_tool_used,
                    self.edit_mode,
                    &mut self.layer_prompt_open,
                    &mut self.request_history_push,
                    &mouse,
                    can_draw,
                    &mut self.embed_trigger,
                    self.frame_count,
                    &self.capture_thread,
                    self.gl_renderer.clone(),
                    self.rasterize_phase,
                    self.rasterize_bbox,
                    self.rasterize_capture.clone(),
                    &mut self.perf_stats,
                    &mut self.volume_sessions,
                    &self.volume_mixer_cmd_tx,
                );

                // Advance rasterize phase after render
                if self.rasterize_phase == 1 {
                    self.rasterize_phase = 2;
                    ctx.request_repaint();
                }

                // ── Push history if requested ──
                if let Some(action) = self.request_history_push.take() {
                    self.history.push(&self.project, action);
                }
                
                // Snip creates a new placed image — detect and push.
                let img_count_after: usize = self.project.layers.iter().map(|l| l.placed_images.len()).sum();
                if self.edit_mode && img_count_after > img_count_before {
                    match self.active_tool {
                        overlay::Tool::Snip   => self.history.push(&self.project, "Snip"),
                        overlay::Tool::Embed  => self.history.push(&self.project, "Embed"),
                        _ => self.history.push(&self.project, "Add Image"),
                    }
                }
            });
        // Web engine updates are now handled in the main render/canvas loop directly via PlacedImage::web_widget
        // Print performance stats every 1 second if profiler enabled and frame_count > 0
        if self.settings.show_profiler && self.last_perf_print_time.elapsed() >= std::time::Duration::from_millis(1000) {
            if self.perf_stats.frame_count > 0 {
                let count = self.perf_stats.frame_count as f32;
                println!(
                    "[PERF LOG] Captured {} frames in last 1.0s:\n\
                     - WGC GPU Copy:     {:7.1} us / frame\n\
                     - WGC CPU-GPU Map:  {:7.1} us / frame (stalls)\n\
                     - BGRA -> RGBA Swap:{:7.1} us / frame\n\
                     - GDI Capture:      {:7.1} us / frame (fallback)\n\
                     - Blur & Mask:      {:7.1} us / frame\n\
                     - Color32 Conv:     {:7.1} us / frame\n\
                     - Thread Total:     {:7.1} us / frame\n\
                     - GPU Tex Upload:   {:7.1} us / frame",
                    self.perf_stats.frame_count,
                    self.perf_stats.wgc_gpu_copy_us_sum as f32 / count,
                    self.perf_stats.wgc_map_wait_us_sum as f32 / count,
                    self.perf_stats.wgc_pixel_swap_us_sum as f32 / count,
                    self.perf_stats.gdi_capture_us_sum as f32 / count,
                    self.perf_stats.thread_mask_effects_us_sum as f32 / count,
                    self.perf_stats.thread_color32_conv_us_sum as f32 / count,
                    self.perf_stats.thread_total_us_sum as f32 / count,
                    self.perf_stats.upload_us_sum as f32 / count,
                );
                self.perf_display = Some(crate::types::PerfDisplayAverages {
                    frame_count: self.perf_stats.frame_count,
                    wgc_gpu_copy_us: self.perf_stats.wgc_gpu_copy_us_sum as f32 / count,
                    wgc_map_wait_us: self.perf_stats.wgc_map_wait_us_sum as f32 / count,
                    wgc_pixel_swap_us: self.perf_stats.wgc_pixel_swap_us_sum as f32 / count,
                    gdi_capture_us: self.perf_stats.gdi_capture_us_sum as f32 / count,
                    thread_mask_effects_us: self.perf_stats.thread_mask_effects_us_sum as f32 / count,
                    thread_color32_conv_us: self.perf_stats.thread_color32_conv_us_sum as f32 / count,
                    thread_total_us: self.perf_stats.thread_total_us_sum as f32 / count,
                    upload_us: self.perf_stats.upload_us_sum as f32 / count,
                });
                // Reset stats
                self.perf_stats = crate::types::AppPerfStats::default();
            }
            self.last_perf_print_time = std::time::Instant::now();
        }

        // ---- 7. Repaint strategy ----
        let has_live = self.project.layers.iter().any(|l| l.placed_images.iter().any(|img| img.is_live))
            || (self.project.marquee_selection.is_some() && self.settings.snip_live);
        if self.edit_mode || has_live {
            ctx.request_repaint(); // Native framerate for smooth brush or live mirror/capture
        } else if self.settings.keep_ui_visible {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        } else if !self.settings.hide_all {
            ctx.request_repaint_after(std::time::Duration::from_millis(33)); // 30 FPS fallback to keep OBS capture active and realtime!
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        self.project.save();
        self.settings.save();
    }
}

fn main() -> eframe::Result<()> {
    let settings = Settings::load();
    let (sw, sh) = winapi_utils::get_screen_size(settings.multi_monitor);
    let (vx, vy) = if settings.multi_monitor {
        winapi_utils::get_virtual_origin()
    } else {
        (0.0, 0.0)
    };
    
    let icon_data = if let Ok(img) = image::load_from_memory(include_bytes!("../icon.png")) {
            let rgba = img.to_rgba8();
            let (width, height) = rgba.dimensions();
            Some(egui::IconData { rgba: rgba.into_raw(), width, height })
        } else {
            None
        };

    let hw_accel = if settings.software_rendering { 
        eframe::HardwareAcceleration::Off 
    } else { 
        eframe::HardwareAcceleration::Required 
    };

    let viewport = egui::ViewportBuilder::default()
        .with_decorations(false)
        .with_transparent(true)
        .with_always_on_top()
        .with_inner_size(egui::vec2(sw + 2.0, sh + 2.0))
        .with_position(egui::pos2(vx - 1.0, vy - 1.0))
        .with_active(true);

    let viewport = if let Some(icon) = icon_data {
        viewport.with_icon(icon)
    } else {
        viewport
    };

    let options = eframe::NativeOptions {
        viewport,
        hardware_acceleration: hw_accel,
        ..Default::default()
    };
    eframe::run_native("Owerlayer", options,
        Box::new(|cc| Ok(Box::new(OwerlayerApp::new(cc)))))
}
