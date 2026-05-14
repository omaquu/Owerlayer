use eframe::egui;
use crate::overlay::*;

use crate::tools::ToolContext;

pub fn update(ctx: &mut ToolContext) {
    let project = &mut *ctx.project;
    let settings = &mut *ctx.settings;
    let mouse = ctx.mouse;
    let ui = &mut *ctx.ui;
    let canvas_response = ctx.canvas_response;
    let painter = ui.painter_at(canvas_response.rect);
    let pos = mouse.pos;
    let left_down = mouse.left_down;
    let active_layer_idx = project.active_layer;

                let layer = &mut project.layers[active_layer_idx];
                if left_down {
                    let r = settings.pen_width.max(1.0);
                    let hit_test = |s: &crate::overlay::Stroke| -> bool {
                        if s.points.len() < 2 { return s.points.iter().any(|p| p.distance(pos) < r); }
                        match s.kind {
                            crate::overlay::StrokeKind::Line => {
                                let p1 = s.points[0];
                                let p2 = s.points[1];
                                let d = (p2.x - p1.x).powi(2) + (p2.y - p1.y).powi(2);
                                if d == 0.0 { return p1.distance(pos) < r; }
                                let t = ((pos.x - p1.x) * (p2.x - p1.x) + (pos.y - p1.y) * (p2.y - p1.y)) / d;
                                let t = t.clamp(0.0, 1.0);
                                let p = egui::pos2(p1.x + t * (p2.x - p1.x), p1.y + t * (p2.y - p1.y));
                                p.distance(pos) < r
                            },
                            _ => s.points.iter().any(|p| p.distance(pos) < r),
                        }
                    };

                    if settings.eraser_mode == EraserMode::Stroke {
                        layer.strokes.retain(|s| !hit_test(s));
                        layer.placed_images.retain(|img| {
                            let disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
                            let disp_h = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[1];
                            let img_rect = egui::Rect::from_min_size(img.position, egui::vec2(disp_w, disp_h));
                            if settings.brush_shape == BrushShape::Square {
                                !img_rect.intersects(egui::Rect::from_center_size(pos, egui::vec2(r*2.0, r*2.0)))
                            } else {
                                img_rect.distance_to_pos(pos) > r
                            }
                        });
                        // In Stroke mode: touching the anchor point of a text annotation deletes it
                        layer.text_annotations.retain(|ann| ann.position.distance(pos) > r);
                    } else {
                        // Split mode
                        let mut new_strokes = Vec::new();
                        let mut keep_strokes = Vec::new();
                        
                        let old_strokes = std::mem::take(&mut layer.strokes);
                        for s in old_strokes {
                            if s.kind != crate::overlay::StrokeKind::Freehand {
                                // In pixel mode, preserve non-freehand strokes (shapes/lines)
                                keep_strokes.push(s);
                                continue;
                            }
                            
                            let mut current_segment = Vec::new();
                            for p in s.points {
                                if p.distance(pos) < r {
                                    if current_segment.len() >= 2 {
                                        let mut s2 = Stroke::new(current_segment.clone(), s.color, s.width, s.kind, s.brush_mode, s.background_color, s.shadow, s.brush_shape, s.outline, s.arrow);
                                        s2.opacity = s.opacity;
                                        new_strokes.push(s2);
                                    }
                                    current_segment.clear();
                                } else {
                                    current_segment.push(p);
                                }
                            }
                            if current_segment.len() >= 2 {
                                let mut s2 = Stroke::new(current_segment, s.color, s.width, s.kind, s.brush_mode, s.background_color, s.shadow, s.brush_shape, s.outline, s.arrow);
                                s2.opacity = s.opacity;
                                new_strokes.push(s2);
                            }
                        }
                        layer.strokes.extend(keep_strokes);
                        layer.strokes.extend(new_strokes);

                        for img in &mut layer.placed_images {
                            let disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
                            let disp_h = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[1];
                            let img_rect = egui::Rect::from_min_size(img.position, egui::vec2(disp_w, disp_h));
                            
                            let overlaps = if settings.brush_shape == BrushShape::Square {
                                img_rect.intersects(egui::Rect::from_center_size(pos, egui::vec2(r*2.0, r*2.0)))
                            } else {
                                img_rect.distance_to_pos(pos) <= r
                            };

                            if overlaps {
                                let mut modified = false;
                                if img.is_live && img.mask.is_none() {
                                    img.mask = Some(vec![255; img.size[0] * img.size[1]]);
                                }
                                
                                for py in 0..img.size[1] {
                                    for px in 0..img.size[0] {
                                        let local_pos = img.position + egui::vec2(
                                            px as f32 * (disp_w / img.size[0] as f32),
                                            py as f32 * (disp_h / img.size[1] as f32)
                                        );
                                        let erase_hit = if settings.brush_shape == BrushShape::Square {
                                            (local_pos.x - pos.x).abs() <= r && (local_pos.y - pos.y).abs() <= r
                                        } else {
                                            local_pos.distance(pos) < r
                                        };
                                        if erase_hit {
                                            let idx = py * img.size[0] + px;
                                            if img.is_live {
                                                let mask = img.mask.as_mut().unwrap();
                                                if mask[idx] != 0 { 
                                                    mask[idx] = 0; 
                                                    modified = true; 
                                                    img.mask_dirty = true;
                                                }
                                            } else {
                                                let b_idx = idx * 4;
                                                if img.pixels[b_idx + 3] != 0 {
                                                    img.pixels[b_idx + 3] = 0;
                                                    modified = true;
                                                }
                                            }
                                        }
                                    }
                                }
                                if modified {
                                    img.texture = None;
                                }
                            }
                        }
                    }
                }

                // Visual cursor
                let cursor_r = settings.pen_width.max(1.0);
                let color = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 180);
                let outline_color = egui::Color32::from_rgba_unmultiplied(0, 0, 0, 120);

                if settings.brush_shape == BrushShape::Square {
                    let rect = egui::Rect::from_center_size(pos, egui::vec2(cursor_r * 2.0, cursor_r * 2.0));
                    painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.5, color), egui::StrokeKind::Middle);
                    painter.rect_stroke(rect.expand(1.5), 0.0, egui::Stroke::new(0.8, outline_color), egui::StrokeKind::Middle);
                } else {
                    painter.circle_stroke(pos, cursor_r, egui::Stroke::new(1.5, color));
                    painter.circle_stroke(pos, cursor_r + 1.5, egui::Stroke::new(0.8, outline_color));
                }

}
