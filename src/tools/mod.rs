use eframe::egui;
use crate::types::*;
use crate::project::{Project, SelectedObject};

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
    pub initial_bounds: &'a mut Option<egui::Rect>,
    pub initial_center: &'a mut Option<egui::Pos2>,
    pub initial_layer: &'a mut Option<crate::project::Layer>,
    pub drag_state: &'a mut usize,
    pub dragging_source_rect: &'a mut bool,
}

impl<'a, 'b> ToolContext<'a, 'b> {
    pub fn auto_create_layer(&mut self) {
        let needs_new_layer = match self.last_tool_used {
            Some(t) => *t != *self.active_tool,
            None => true,
        };
        if needs_new_layer {
            // Check if current layer is completely empty and named "Layer 1"
            let is_empty_start = self.project.layers.len() == 1 
                && self.project.layers[0].strokes.is_empty() 
                && self.project.layers[0].text_annotations.is_empty() 
                && self.project.layers[0].placed_images.is_empty();
                
            if is_empty_start {
                self.project.layers[0].name = self.active_tool.name().to_string();
            } else {
                self.project.layers.push(crate::project::Layer::new(self.active_tool.name()));
                self.project.active_layer = self.project.layers.len() - 1;
            }
            *self.last_tool_used = Some(*self.active_tool);
        }
    }
}
