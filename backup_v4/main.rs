#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod hotkey;
mod overlay;
mod winapi_utils;
mod project;

use eframe::egui;
use overlay::{
    MouseState, PendingText, Settings, Stroke, TextAnnotation, Tool,
    render_mode_indicator, render_settings_window, render_toolbar, render_layers_window,
};

struct OwerlayerApp {
    edit_mode: bool,

    active_tool: Tool,
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
    last_tool_used: Option<Tool>,
    last_action_time: std::time::Instant,
}

impl OwerlayerApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut v = egui::Visuals::dark();
        v.window_shadow = egui::Shadow::NONE;
        v.popup_shadow = egui::Shadow::NONE;
        cc.egui_ctx.set_visuals(v);

        Self {
            edit_mode: false,
            active_tool: Tool::Brush,
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
            show_settings_panel: false,
            show_layers_panel: false,
            show_exit_dialog: false,
            listening_for_hotkey: false,
            prev_mouse_down: false,
            prev_mouse_pos: egui::pos2(0.0, 0.0),
            prev_hotkey_held: false,
            initialized: false,
            frame_count: 0,
            show_debug_window: false,
            prev_passthrough: true, // Initially transparent
            last_tool_used: None,
            last_action_time: std::time::Instant::now(),
        }
    }
}

impl eframe::App for OwerlayerApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_pixels_per_point(self.settings.ui_scale);
        // ---- 0. First-frame init ----
        if !self.initialized {
            self.initialized = true;
            winapi_utils::setup_overlay_window();
            winapi_utils::set_capture_exclusion(self.settings.exclude_from_capture);
        }

        // ---- 1. Poll mouse ----
        let mouse = MouseState::poll(self.prev_mouse_down, self.prev_mouse_pos);
        self.prev_mouse_down = mouse.left_down;
        self.prev_mouse_pos = mouse.pos;

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
            // Check if mouse is over any active egui area (windows, panels)
            // We use the area_rects from the previous frame
            mem.layer_ids().any(|layer| {
                if let Some(rect) = mem.area_rect(layer.id) {
                    rect.contains(mouse.pos)
                } else {
                    false
                }
            })
        });

        let should_be_interactive = self.edit_mode || (self.settings.keep_ui_visible && is_over_ui);
        let passthrough = !should_be_interactive;

        if passthrough != self.prev_passthrough {
            ctx.send_viewport_cmd(egui::ViewportCommand::MousePassthrough(passthrough));
            self.prev_passthrough = passthrough;
        }

        // ---- 4. On mode change ----
        if self.edit_mode != was_edit {
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));

            if self.edit_mode {
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            } else {
                // Leaving edit → finalize pending work
                if let Some(pending) = self.pending_text.take() {
                    if !pending.buffer.is_empty() {
                        if let Some(layer) = self.project.get_active_layer_mut() {
                            layer.text_annotations.push(TextAnnotation {
                                position: pending.position, text: pending.buffer,
                                color: self.settings.pen_color, font_size: self.settings.font_size,
                                monospace: self.settings.text_monospace, shadow: self.settings.text_shadow, outline: self.settings.text_outline,
                                stroke_width: self.settings.text_stroke_width,
                                rotation: 0.0, flipped_h: false, flipped_v: false,
                                perspective: [egui::Vec2::ZERO; 4],
                                font: self.settings.text_font,
                                skew: egui::Vec2::ZERO,
                            });
                        }
                    }
                }
                if !self.current_stroke.is_empty() {
                    if let Some(layer) = self.project.get_active_layer_mut() {
                        layer.strokes.push(Stroke {
                            points: self.current_stroke.clone(),
                            color: self.settings.pen_color,
                            width: self.settings.pen_width,
                            kind: overlay::StrokeKind::Freehand,
                            brush_mode: self.settings.brush_mode,
                            background_color: Some(self.settings.background_color),
                            shadow: self.settings.brush_shadow,
                            rotation: 0.0, flipped_h: false, flipped_v: false,
                            perspective: [egui::Vec2::ZERO; 4],
                            brush_shape: self.settings.brush_shape,
                            skew: egui::Vec2::ZERO,
                        });
                    }
                    self.current_stroke.clear();
                }
                self.line_start = None;
                self.initial_bounds = None;
                self.initial_center = None;
                if !self.settings.keep_ui_visible { self.show_settings_panel = false; self.show_layers_panel = false; }
                
                // Save project automatically when exiting edit mode in background thread
                let proj_clone = self.project.clone();
                std::thread::spawn(move || proj_clone.save());
            }
        }

        // ---- 4. Text keyboard input ----
        if self.edit_mode {
            if let Some(pending) = self.pending_text.as_mut() {
                ctx.input(|i| {
                    for event in &i.events {
                        if let egui::Event::Text(t) = event { pending.buffer.push_str(t); }
                        if let egui::Event::Key { key: egui::Key::Backspace, pressed: true, .. } = event {
                            pending.buffer.pop();
                        }
                    }
                });
                let finalize = ctx.input(|i| i.key_pressed(egui::Key::Enter));
                let cancel   = ctx.input(|i| i.key_pressed(egui::Key::Escape));
                if finalize {
                    if let Some(p) = self.pending_text.take() {
                        if !p.buffer.is_empty() {
                            if let Some(layer) = self.project.get_active_layer_mut() {
                                layer.text_annotations.push(TextAnnotation {
                                    position: p.position, text: p.buffer,
                                    color: self.settings.pen_color, font_size: self.settings.font_size,
                                    monospace: self.settings.text_monospace, shadow: self.settings.text_shadow, outline: self.settings.text_outline,
                                    stroke_width: self.settings.text_stroke_width,
                                    rotation: 0.0, flipped_h: false, flipped_v: false,
                                    perspective: [egui::Vec2::ZERO; 4],
                                    font: self.settings.text_font,
                                    skew: egui::Vec2::ZERO,
                                });
                            }
                        }
                    }
                } else if cancel { self.pending_text = None; }
            }
            if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Z)) {
                if let Some(layer) = self.project.get_active_layer_mut() {
                    // Prevent out of bounds panics by strictly checking before popping
                    if !layer.strokes.is_empty() {
                        layer.strokes.pop();
                    } else if !layer.text_annotations.is_empty() {
                        layer.text_annotations.pop();
                    } else if !layer.placed_images.is_empty() {
                        layer.placed_images.pop();
                    }
                }
            }
            if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::S)) {
                self.project.save();
            }
            if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::D)) {
                self.show_debug_window = !self.show_debug_window;
            }
        }

        // ---- 5. Render UI ----
        let show_ui = self.edit_mode || self.settings.keep_ui_visible;
        if show_ui {
            render_mode_indicator(ctx, self.edit_mode, self.settings.hotkey.display_name(), self.settings.toggle_mode, &self.settings);
            render_toolbar(ctx, &mut self.active_tool, &mut self.settings, &mut self.show_settings_panel, &mut self.show_layers_panel, &mut self.show_exit_dialog, &mut self.project);
            
            if self.show_exit_dialog {
                let mut close = false;
                egui::Window::new("Exit Owerlayer?")
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
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
                    &mut clear_all, &mut self.listening_for_hotkey);
                if clear_all {
                    if let Some(layer) = self.project.get_active_layer_mut() {
                        layer.strokes.clear(); layer.text_annotations.clear();
                        layer.placed_images.clear();
                    }
                    self.current_stroke.clear(); self.pending_text = None;
                }
            }
            if self.show_layers_panel && self.edit_mode {
                render_layers_window(ctx, &mut self.project, &self.settings, &mut self.show_layers_panel);
            }
        }

        if self.show_debug_window {
            egui::Window::new("Debug Overlay").open(&mut self.show_debug_window).show(ctx, |ui| {
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
        let is_over_ui_window = ctx.memory(|mem| {
            [
                egui::Id::new("photoshop_panel"),
                egui::Id::new("Settings"),
                egui::Id::new("Layers & Projects"),
                egui::Id::new("subtools_popup"),
                egui::Id::new("Exit Owerlayer?"),
                egui::Id::new("Debug Overlay"),
                egui::Id::new("mode_indicator"),
            ]
            .iter()
            .any(|id| mem.area_rect(*id).map_or(false, |r| r.contains(mouse.pos)))
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
                overlay::render_canvas(
                    ui,
                    &self.active_tool,
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
                    &mut self.last_tool_used,
                    self.edit_mode,
                    &mouse,
                    can_draw,
                );
            });

        // ---- 7. Repaint strategy ----
        let has_live = self.project.layers.iter().any(|l| l.placed_images.iter().any(|img| img.is_live));
        if self.edit_mode || has_live {
            ctx.request_repaint(); // Native framerate for smooth brush or live mirror
        } else if self.settings.keep_ui_visible {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }
}

fn main() -> eframe::Result<()> {
    let (sw, sh) = winapi_utils::get_screen_size();

    let icon_data = if let Ok(img) = image::load_from_memory(include_bytes!("../icon.png")) {
            let rgba = img.to_rgba8();
            let (width, height) = rgba.dimensions();
            Some(egui::IconData { rgba: rgba.into_raw(), width, height })
        } else {
            None
        };

    let settings = Settings::load();
    let hw_accel = if settings.software_rendering { eframe::HardwareAcceleration::Off } else { eframe::HardwareAcceleration::Required };

    let viewport = egui::ViewportBuilder::default()
        .with_decorations(false)
        .with_transparent(true)
        .with_always_on_top()
        .with_inner_size([sw + 2.0, sh + 2.0])
        .with_position([-1.0, -1.0])
        .with_title("Owerlayer");
        
    let options = if let Some(icon) = icon_data {
        eframe::NativeOptions {
            viewport: viewport.with_icon(icon),
            hardware_acceleration: hw_accel,
            ..Default::default()
        }
    } else {
        eframe::NativeOptions {
            viewport: viewport,
            hardware_acceleration: hw_accel,
            ..Default::default()
        }
    };
    eframe::run_native("Owerlayer", options,
        Box::new(|cc| Ok(Box::new(OwerlayerApp::new(cc)))))
}
