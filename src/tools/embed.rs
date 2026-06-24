use eframe::egui;
use crate::overlay::*;
use crate::tools::ToolContext;
use crate::types::{PlacedImage, WidgetType, WidgetPlaceType, CalcState, ObjectType, SelectedObject};

pub fn update(ctx: &mut ToolContext) {
    if *ctx.layer_prompt_open { return; }
    let left_just_pressed = ctx.mouse.left_just_pressed;
    let pos = ctx.mouse.pos;
    let ui = &mut *ctx.ui;
    let canvas_response = ctx.canvas_response;
    let painter = ui.painter_at(canvas_response.rect);

    let help_text = match ctx.settings.widget_place_type {
        WidgetPlaceType::Browser => "Click to place Web Browser",
        WidgetPlaceType::Calculator => "Click to place Calculator",
        WidgetPlaceType::VolumeMixer => "Click to place Volume Mixer",
    };
    painter.text(pos + egui::vec2(10.0, 10.0), egui::Align2::LEFT_TOP, help_text, egui::FontId::proportional(12.0), egui::Color32::WHITE);

    if left_just_pressed {
        ctx.auto_create_layer();
        let project = &mut *ctx.project;
        let active_tool = &mut *ctx.active_tool;
        let last_tool_used = &mut *ctx.last_tool_used;
        
        if project.active_layer < project.layers.len() {
            let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() as usize;
            
            match ctx.settings.widget_place_type {
                WidgetPlaceType::Browser => {
                    #[cfg(feature = "webengine")]
                    {
                        if !crate::web_engine::is_available() {
                            crate::web_engine::init();
                        }
                        if crate::web_engine::is_available() {
                            let mut new_img = PlacedImage::new(id, pos, [400, 300], vec![200; 400 * 300 * 4]);
                            new_img.display_size = Some([400.0, 300.0]);
                            new_img.url = Some("about:blank".to_string());
                            new_img.is_live = true;
                            if let Some(widget) = crate::web_engine::create_widget("about:blank", 400, 300) {
                                new_img.pixels = widget.pixels.clone();
                                new_img.web_widget = Some(std::sync::Arc::new(std::sync::Mutex::new(widget)));
                            }
                            let layer = &mut project.layers[project.active_layer];
                            layer.placed_images.push(new_img);
                            layer.expanded = true;
                            project.selected_object = Some(SelectedObject {
                                layer_idx: project.active_layer,
                                object_type: ObjectType::Image,
                                object_idx: layer.placed_images.len() - 1,
                            });
                            *last_tool_used = Some(*active_tool);
                            *active_tool = Tool::Move;
                            return;
                        }
                    }
                    
                    // Fallback placeholder
                    let w = 400; let h = 80;
                    let mut pixels = vec![0u8; w * h * 4];
                    for chunk in pixels.chunks_exact_mut(4) {
                        chunk[0] = 40; chunk[1] = 30; chunk[2] = 30; chunk[3] = 220;
                    }
                    let mut new_img = PlacedImage::new(id, pos, [w, h], pixels);
                    new_img.display_size = Some([w as f32, h as f32]);
                    new_img.name = "Web Browser (Unavailable)".to_string();
                    new_img.url = Some("about:blank".to_string());
                    let layer = &mut project.layers[project.active_layer];
                    layer.placed_images.push(new_img);
                    layer.expanded = true;
                    layer.text_annotations.push(crate::overlay::TextAnnotation::new(pos + egui::vec2(10.0, 10.0), "Web Engine Not Available".to_string(), [255, 180, 180, 255], 14.0));
                    project.selected_object = Some(SelectedObject {
                        layer_idx: project.active_layer,
                        object_type: ObjectType::Image,
                        object_idx: layer.placed_images.len() - 1,
                    });
                    *last_tool_used = Some(*active_tool);
                    *active_tool = Tool::Move;
                }
                WidgetPlaceType::Calculator => {
                    let mut new_img = PlacedImage::new(id, pos, [140, 180], Vec::new());
                    new_img.display_size = Some([140.0, 180.0]);
                    new_img.name = "Calculator".to_string();
                    new_img.widget_type = Some(WidgetType::Calculator);
                    new_img.calculator_state = Some(CalcState::default());
                    new_img.is_live = false;
                    
                    let layer = &mut project.layers[project.active_layer];
                    layer.placed_images.push(new_img);
                    layer.expanded = true;
                    project.selected_object = Some(SelectedObject {
                        layer_idx: project.active_layer,
                        object_type: ObjectType::Image,
                        object_idx: layer.placed_images.len() - 1,
                    });
                    *last_tool_used = Some(*active_tool);
                    *active_tool = Tool::Move;
                }
                WidgetPlaceType::VolumeMixer => {
                    let mut new_img = PlacedImage::new(id, pos, [220, 240], Vec::new());
                    new_img.display_size = Some([220.0, 240.0]);
                    new_img.name = "Volume Mixer".to_string();
                    new_img.widget_type = Some(WidgetType::VolumeMixer);
                    new_img.volume_mixer_state = Some(crate::types::VolumeMixerState::default());
                    new_img.is_live = false;
                    
                    let layer = &mut project.layers[project.active_layer];
                    layer.placed_images.push(new_img);
                    layer.expanded = true;
                    project.selected_object = Some(SelectedObject {
                        layer_idx: project.active_layer,
                        object_type: ObjectType::Image,
                        object_idx: layer.placed_images.len() - 1,
                    });
                    *last_tool_used = Some(*active_tool);
                    *active_tool = Tool::Move;
                }
            }
        }
    }
}
