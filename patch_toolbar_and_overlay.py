# Patch src/ui/layer_menu.rs
with open('src/ui/layer_menu.rs', 'r', encoding='utf-8') as f:
    content = f.read()

old_response_block = """                    let row_response = ui.interact(row_inner.response.rect, ui.id().with(i), egui::Sense::click());
                    if row_response.clicked() || row_response.double_clicked() {
                        project.active_layer = i;
                    }"""
new_response_block = """                    let row_rect = row_inner.response.rect;
                    if ui.input(|i| i.pointer.any_click()) || ui.input(|i| i.pointer.button_double_clicked(egui::PointerButton::Primary)) {
                        if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                            if row_rect.contains(pos) {
                                project.active_layer = i;
                            }
                        }
                    }"""
assert old_response_block in content
content = content.replace(old_response_block, new_response_block)

with open('src/ui/layer_menu.rs', 'w', encoding='utf-8') as f:
    f.write(content)
print("layer_menu.rs patched successfully!")

# Patch src/ui/toolbar.rs
with open('src/ui/toolbar.rs', 'r', encoding='utf-8') as f:
    content = f.read()

old_blur_effect_block = """                    // Blur styles when image is selected and has blur:
                    if let ObjectType::Image = sel.object_type {
                        let img = &mut project.layers[sel.layer_idx].placed_images[sel.object_idx];
                        if img.blur > 0.1 {
                            ui.separator();
                            ui.selectable_value(&mut img.blur_effect, BlurEffect::Gaussian, "Gaus");
                            ui.selectable_value(&mut img.blur_effect, BlurEffect::Pixelate, "Pix");
                            ui.selectable_value(&mut img.blur_effect, BlurEffect::Glitch, "VHS");
                        }
                    }"""
new_blur_effect_block = """                    // Blur styles when selected object has blur:
                    let (has_bl, bl_effect) = match sel.object_type {
                        ObjectType::Image => {
                            let img = &mut project.layers[sel.layer_idx].placed_images[sel.object_idx];
                            (img.blur > 0.1, &mut img.blur_effect)
                        }
                        ObjectType::Stroke => {
                            let s = &mut project.layers[sel.layer_idx].strokes[sel.object_idx];
                            (s.blur > 0.1, &mut s.blur_effect)
                        }
                        ObjectType::Text => {
                            let t = &mut project.layers[sel.layer_idx].text_annotations[sel.object_idx];
                            (t.blur > 0.1, &mut t.blur_effect)
                        }
                    };
                    if has_bl {
                        ui.separator();
                        ui.selectable_value(bl_effect, BlurEffect::Gaussian, "Gaus");
                        ui.selectable_value(bl_effect, BlurEffect::Pixelate, "Pix");
                        ui.selectable_value(bl_effect, BlurEffect::Glitch, "VHS");
                    }"""
assert old_blur_effect_block in content
content = content.replace(old_blur_effect_block, new_blur_effect_block)

with open('src/ui/toolbar.rs', 'w', encoding='utf-8') as f:
    f.write(content)
print("toolbar.rs patched successfully!")

# Patch src/overlay.rs
with open('src/overlay.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# 1. Update effect_pad calculation
old_effect_pad = """                let has_gl_effect = img.blur > 0.1 || img.grayscale || img.invert || img.sepia || img.glow || layer.grayscale || layer.invert || layer.sepia || layer.glow;
                let effect_pad = 0.0;"""
new_effect_pad = """                let has_gl_effect = img.blur > 0.1 || img.grayscale || img.invert || img.sepia || img.glow || layer.grayscale || layer.invert || layer.sepia || layer.glow;
                let mut effect_pad = img.blur.max(layer.blur);
                if img.glow || layer.glow {
                    let g_spread = if img.glow { img.glow_spread } else { layer.glow_spread };
                    effect_pad = effect_pad.max(g_spread + 20.0);
                }
                if img.shadow || layer.shadow {
                    let (s_off, s_spread) = if img.shadow { (img.shadow_offset, img.shadow_spread) } else { (layer.shadow_offset, layer.shadow_spread) };
                    let off_len = s_off[0].abs().max(s_off[1].abs());
                    effect_pad = effect_pad.max(off_len + s_spread + 20.0);
                }
                if img.outline || layer.outline {
                    let o_width = if img.outline { img.outline_width } else { layer.outline_width };
                    effect_pad = effect_pad.max(o_width * 2.0 + 10.0);
                }"""
assert old_effect_pad in content
content = content.replace(old_effect_pad, new_effect_pad)

# 2. Skip box blur in CPU if GL renderer is available
old_blur_cpu = """                        if let Some(mut pixels) = capture_opt {
                            let blur_amt = (s.blur.max(8.0)) as usize;
                            crate::ui::toolbar::apply_box_blur(&mut pixels, sw as usize, sh as usize, blur_amt);
                            let color_img = egui::ColorImage::from_rgba_unmultiplied([sw as usize, sh as usize], &pixels);"""
new_blur_cpu = """                        if let Some(mut pixels) = capture_opt {
                            if gl_renderer.is_none() {
                                let blur_amt = (s.blur.max(8.0)) as usize;
                                crate::ui::toolbar::apply_box_blur(&mut pixels, sw as usize, sh as usize, blur_amt);
                            }
                            let color_img = egui::ColorImage::from_rgba_unmultiplied([sw as usize, sh as usize], &pixels);"""
assert old_blur_cpu in content
content = content.replace(old_blur_cpu, new_blur_cpu)

# 3. Render Blur stroke with GL renderer if available
old_blur_render = """                    if let Some(ref tex) = s.cached_texture {
                        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                        let mut mesh = egui::Mesh::with_texture(tex.id());
                        let opacity_col = egui::Color32::from_white_alpha((255.0 * l_op * s.opacity) as u8);
                        mesh.add_rect_with_uv(rect_screen, uv, opacity_col);
                        painter.add(egui::Shape::mesh(mesh));
                    }"""
new_blur_render = """                    if let Some(ref tex) = s.cached_texture {
                        if let Some(ref renderer) = gl_renderer {
                            let effect_type = match s.blur_effect {
                                crate::types::BlurEffect::Gaussian => 1,
                                crate::types::BlurEffect::Pixelate => 2,
                                crate::types::BlurEffect::Glitch => 3,
                            };
                            let strength = s.blur * 0.2;
                            let res = [sw as f32, sh as f32];
                            let time = ui.input(|i| i.time) as f32;
                            let tex_id = tex.id();
                            
                            let mut mesh = egui::Mesh::with_texture(tex_id);
                            let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                            mesh.add_rect_with_uv(rect_screen, uv, egui::Color32::WHITE);
                            
                            let mut vertices = Vec::new();
                            for &idx in &mesh.indices {
                                let v = &mesh.vertices[idx as usize];
                                vertices.push(v.pos.x);
                                vertices.push(v.pos.y);
                                vertices.push(v.uv.x);
                                vertices.push(v.uv.y);
                            }
                            let vertex_count = mesh.indices.len() as i32;
                            let paint_rect = rect_screen;
                            let renderer = renderer.clone();
                            let pass_opacity = l_op * s.opacity;

                            painter.add(egui::PaintCallback {
                                rect: paint_rect,
                                callback: std::sync::Arc::new(egui_glow::CallbackFn::new(move |_info, render_ctx: &egui_glow::Painter| {
                                    let gl = render_ctx.gl();
                                    let ppp = _info.pixels_per_point;
                                    let screen_h_px = _info.screen_size_px[1] as f32;
                                    
                                    let x = (paint_rect.min.x * ppp).round() as i32;
                                    let y = (screen_h_px - paint_rect.max.y * ppp).round() as i32;
                                    let w = (paint_rect.width() * ppp).round() as i32;
                                    let h = (paint_rect.height() * ppp).round() as i32;
                                    
                                    if w <= 0 || h <= 0 { return; }
                                    
                                    let mut mapped_vertices = Vec::with_capacity(vertices.len());
                                    for i in (0..vertices.len()).step_by(4) {
                                        let vx = vertices[i];
                                        let vy = vertices[i+1];
                                        let gl_x = ((vx - paint_rect.min.x) / paint_rect.width().max(1.0)) * 2.0 - 1.0;
                                        let gl_y = 1.0 - ((vy - paint_rect.min.y) / paint_rect.height().max(1.0)) * 2.0;
                                        mapped_vertices.push(gl_x);
                                        mapped_vertices.push(gl_y);
                                        mapped_vertices.push(vertices[i+2]);
                                        mapped_vertices.push(vertices[i+3]);
                                    }
                                    
                                    let gl_tex = match render_ctx.texture(tex_id) {
                                        Some(t) => t, None => return,
                                    };

                                    let mut old_viewport = [0i32; 4];
                                    unsafe {
                                        gl.get_parameter_i32_slice(glow::VIEWPORT, &mut old_viewport);
                                        gl.viewport(x, y, w, h);
                                        
                                        renderer.render_effect(gl, gl_tex, None, effect_type, strength, res, time, false, false, false, [1.0, 1.0, 1.0, 1.0], false, pass_opacity, vertex_count, &mapped_vertices);
                                        
                                        gl.viewport(old_viewport[0], old_viewport[1], old_viewport[2], old_viewport[3]);
                                    }
                                })),
                            });
                        } else {
                            let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                            let mut mesh = egui::Mesh::with_texture(tex.id());
                            let opacity_col = egui::Color32::from_white_alpha((255.0 * l_op * s.opacity) as u8);
                            mesh.add_rect_with_uv(rect_screen, uv, opacity_col);
                            painter.add(egui::Shape::mesh(mesh));
                        }
                    }"""
assert old_blur_render in content
content = content.replace(old_blur_render, new_blur_render)

with open('src/overlay.rs', 'w', encoding='utf-8') as f:
    f.write(content)
print("overlay.rs patched successfully!")
