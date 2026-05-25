use eframe::egui;
use crate::history::History;
use crate::project::Project;
use crate::ui::toolbar::photoshop_frame;
use crate::types::Settings;

/// Renders the floating History panel.
/// Returns `Some(Project)` when the user clicks an entry to jump to it.
pub fn render_history_window(
    ctx: &egui::Context,
    history: &mut History,
    open: &mut bool,
    settings: &mut Settings,
) -> Option<Project> {
    if !*open { return None; }

    let frame = photoshop_frame(settings);
    let mut jump_target: Option<Project> = None;

    let mut close_window = false;

    let win_resp = egui::Window::new("History")
        .open(open)
        .title_bar(false)
        .resizable(true)
        .default_width(240.0)
        .default_pos(settings.history_menu_pos)
        .frame(frame)
        .show(ctx, |ui| {
            // ── Header ──
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("🕓  History")
                    .color(egui::Color32::from_rgb(180, 180, 200))
                    .size(15.0)
                    .strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(
                        egui::Button::new(egui::RichText::new("✖").size(11.0))
                            .frame(false)
                    ).clicked() {
                        close_window = true;
                    }
                });
            });
            ui.separator();

            // ── Undo / Redo buttons ──
            ui.horizontal(|ui| {
                let can_undo = history.can_undo();
                let can_redo = history.can_redo();

                if ui.add_enabled(
                    can_undo,
                    egui::Button::new(egui::RichText::new("↩ Undo").size(12.0))
                        .min_size(egui::vec2(80.0, 22.0))
                ).clicked() {
                    if let Some(snap) = history.undo() {
                        jump_target = Some(snap.clone());
                    }
                }

                if ui.add_enabled(
                    can_redo,
                    egui::Button::new(egui::RichText::new("↪ Redo").size(12.0))
                        .min_size(egui::vec2(80.0, 22.0))
                ).clicked() {
                    if let Some(snap) = history.redo() {
                        jump_target = Some(snap.clone());
                    }
                }
            });
            ui.separator();

            // ── Entry list ──
            let cursor = history.cursor;
            let entry_count = history.entries.len();

            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    for i in (0..entry_count).rev() {
                        let is_current = cursor == Some(i);
                        let is_future  = cursor.map(|c| i > c).unwrap_or(false);

                        let label_color = if is_current {
                            egui::Color32::from_rgb(100, 210, 255)
                        } else if is_future {
                            egui::Color32::from_gray(90)
                        } else {
                            egui::Color32::from_gray(200)
                        };

                        let step_num = i + 1;
                        let text = egui::RichText::new(
                            format!("{:>2}. {}", step_num, history.entries[i].label)
                        )
                        .size(12.0)
                        .color(label_color);

                        let resp = ui.selectable_label(is_current, text);

                        if resp.clicked() && !is_current {
                            // Clone here so we can return after the loop.
                            jump_target = history.jump_to(i).cloned();
                        }

                        if is_current {
                            resp.scroll_to_me(None);
                        }
                    }
                });

            // ── Footer: entry count ──
            ui.separator();
            ui.label(egui::RichText::new(
                format!("{} / {} steps  (Ctrl+Z / Ctrl+Y)",
                    cursor.map(|c| c + 1).unwrap_or(0),
                    entry_count)
            ).size(10.0).color(egui::Color32::from_gray(120)));
        });

    // Persist window position.
    if let Some(resp) = win_resp {
        if resp.response.dragged() {
            let id = resp.response.layer_id;
            if let Some(rect) = ctx.memory(|m| m.area_rect(id.id)) {
                settings.history_menu_pos = rect.min;
            }
        }
    }

    if close_window {
        *open = false;
    }

    jump_target
}
