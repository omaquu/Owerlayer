use eframe::egui;
use crate::types::*;
use crate::project::Project;

pub mod brush;
pub mod shape;
pub mod snip;
pub mod cut;
pub mod mirror;
pub mod blur;
pub mod text;
pub mod embed;
pub mod move_tool;
pub mod eraser;
pub mod paint_bucket;

pub struct ToolContext<'a, 'b> {
    pub ui: &'a mut egui::Ui,
    pub project: &'a mut Project,
    pub settings: &'a mut Settings,
    pub mouse: &'a MouseState,
    pub active_tool: &'a mut Tool,
    pub last_tool_used: &'a mut Option<Tool>,
    pub current_stroke: &'a mut Vec<egui::Pos2>,
    pub line_start: &'a mut Option<egui::Pos2>,
    pub frame_count: u64,
    pub render_offset: egui::Vec2,
    pub canvas_response: &'b egui::Response,
    pub remove_active_layer: &'a mut bool,
    pub snip_created: &'a mut bool,
    pub new_selection: &'a mut Option<SelectedObject>,
    pub switch_to_move: &'a mut bool,
    pub embed_trigger: &'a mut bool,
    pub pending_text: &'a mut Option<PendingText>,
    pub pending_stroke: &'a mut Option<Stroke>,
    pub pending_text_to_add: &'a mut Option<TextAnnotation>,
    pub initial_bounds: &'a mut Option<egui::Rect>,
    pub initial_center: &'a mut Option<egui::Pos2>,
    pub initial_layer: &'a mut Option<crate::project::Layer>,
    pub drag_state: &'a mut usize,
    pub dragging_source_rect: &'a mut bool,
    pub edit_mode: bool,
    pub layer_prompt_open: &'a mut bool,
    pub request_history_push: &'a mut Option<String>,
}

impl<'a, 'b> ToolContext<'a, 'b> {
    pub fn auto_create_layer(&mut self) {
        if self.project.selected_object.is_some() {
            *self.last_tool_used = Some(*self.active_tool);
            return;
        }
        let active_layer_idx = self.project.active_layer;

        // If no layers exist yet, create the first one and return
        if self.project.layers.is_empty() {
            self.project.layers.push(crate::project::Layer::new(self.active_tool.name()));
            self.project.active_layer = 0;
            *self.last_tool_used = Some(*self.active_tool);
            return;
        }

        // If we have a valid active layer and it's visible, check if we can reuse it
        if active_layer_idx < self.project.layers.len() && self.project.layers[active_layer_idx].visible {
            let layer = &self.project.layers[active_layer_idx];
            let can_reuse = match self.last_tool_used {
                Some(t) => {
                    // Reuse if same tool, or if switching from Move (neutral tool)
                    *t == *self.active_tool || *t == Tool::Move || layer.locked
                }
                None => true, // App just started, use the active layer
            };

            if can_reuse {
                *self.last_tool_used = Some(*self.active_tool);
                return;
            }
        }

        let needs_new_layer = match self.last_tool_used {
            Some(t) => *t != *self.active_tool,
            None => true,
        };

        if needs_new_layer {
            match self.settings.auto_new_layer {
                Some(true) => {
                    self.project.layers.push(crate::project::Layer::new(self.active_tool.name()));
                    self.project.active_layer = self.project.layers.len() - 1;
                    *self.last_tool_used = Some(*self.active_tool);
                }
                Some(false) => {
                    *self.last_tool_used = Some(*self.active_tool);
                }
                None => {
                    *self.last_tool_used = Some(*self.active_tool);
                }
            }
        }
    }
}
