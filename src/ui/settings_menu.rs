use eframe::egui;
use crate::types::*;
use crate::utils::color32;
use crate::ui::toolbar::photoshop_frame;
use crate::hotkey::detect_pressed_key;


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
        .default_pos(egui::pos2(ctx.screen_rect().max.x - 380.0, 60.0))
        .pivot(egui::Align2::RIGHT_TOP)
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
            ui.checkbox(&mut settings.prompt_delete_layer, "Prompt before deleting layer");
            
            ui.add_space(4.0);
            let mut auto_new = settings.auto_new_layer.unwrap_or(true);
            let mut prompt = settings.auto_new_layer.is_none();
            ui.horizontal(|ui| {
                ui.label("When switching tools:");
                if ui.selectable_value(&mut prompt, true, "Prompt").clicked() {
                    settings.auto_new_layer = None;
                }
                if ui.selectable_value(&mut prompt, false, "Remember").clicked() {
                    settings.auto_new_layer = Some(auto_new);
                }
            });
            if !prompt {
                if ui.checkbox(&mut auto_new, "Auto-create new layer").changed() {
                    settings.auto_new_layer = Some(auto_new);
                }
            }

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

            // ── GPU & Rendering ──
            section_heading(ui, "GPU & Rendering", accent);
            
            ui.label(egui::RichText::new("Preferred GPU (Disabled in Glow mode)").size(10.0).color(egui::Color32::GRAY));
            
            ui.add_space(4.0);
            ui.checkbox(&mut settings.fso_fix, "Fullscreen Optimization Fix");
            ui.label(egui::RichText::new("Bypasses Windows FSO by offsetting the window by 4px. Turn off if alignment is wrong.").size(10.0).color(egui::Color32::GRAY));

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
                    ui.label(egui::RichText::new("v0.6.5").size(11.0).color(egui::Color32::GRAY));
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

pub fn section_heading(ui: &mut egui::Ui, text: &str, accent: egui::Color32) {
    ui.add_space(4.0);
    ui.label(egui::RichText::new(text).size(14.0).strong().color(accent));
    ui.add_space(2.0);
}

