use eframe::egui;
use crate::types::*;

pub fn color32(c: &[u8; 4]) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])
}

pub fn is_inside_poly(poly: &[egui::Pos2], p: egui::Pos2) -> bool {
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        if ((poly[i].y > p.y) != (poly[j].y > p.y)) &&
           (p.x < (poly[j].x - poly[i].x) * (p.y - poly[i].y) / (poly[j].y - poly[i].y) + poly[i].x) {
            inside = !inside;
        }
        j = i;
    }
    inside
}

pub fn get_star_points(center: egui::Pos2, radius: f32) -> Vec<egui::Pos2> {
    let mut points = Vec::new();
    for i in 0..10 {
        let r = if i % 2 == 0 { radius } else { radius * 0.4 };
        let angle = i as f32 * std::f32::consts::PI / 5.0 - std::f32::consts::PI / 2.0;
        points.push(center + egui::vec2(angle.cos() * r, angle.sin() * r));
    }
    points
}

pub fn get_heart_points(center: egui::Pos2, radius: f32) -> Vec<egui::Pos2> {
    let mut points = Vec::new();
    let steps = 40;
    for i in 0..steps {
        let t = i as f32 * std::f32::consts::PI * 2.0 / steps as f32;
        let x = 16.0 * t.sin().powi(3);
        let y = -(13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos());
        points.push(center + egui::vec2(x * radius / 16.0, y * radius / 16.0));
    }
    points
}

pub fn magic_wand_flood_fill(img: &mut PlacedImage, start_x: i32, start_y: i32, target_color: [u8; 4], threshold: f32) {
    let w = img.size[0] as i32;
    let h = img.size[1] as i32;
    let mut stack = vec![(start_x, start_y)];
    let mut visited = vec![false; (w * h) as usize];
    
    let color_diff = |c1: [u8; 4], c2: [u8; 4]| -> f32 {
        let dr = (c1[0] as f32 - c2[0] as f32).abs();
        let dg = (c1[1] as f32 - c2[1] as f32).abs();
        let db = (c1[2] as f32 - c2[2] as f32).abs();
        (dr + dg + db) / 3.0
    };

    while let Some((x, y)) = stack.pop() {
        if x < 0 || x >= w || y < 0 || y >= h { continue; }
        let idx = (y * w + x) as usize;
        if visited[idx] { continue; }
        visited[idx] = true;

        let pixel_idx = idx * 4;
        let current_color = [img.pixels[pixel_idx], img.pixels[pixel_idx+1], img.pixels[pixel_idx+2], img.pixels[pixel_idx+3]];
        
        if current_color[3] > 0 && color_diff(current_color, target_color) <= threshold {
            img.pixels[pixel_idx + 3] = 0; // Erase
            stack.push((x + 1, y));
            stack.push((x - 1, y));
            stack.push((x, y + 1));
            stack.push((x, y - 1));
        }
    }
}

pub fn draw_quad(painter: &egui::Painter, p1: egui::Pos2, p2: egui::Pos2, p3: egui::Pos2, p4: egui::Pos2, color: egui::Color32) {
    let mut mesh = egui::Mesh::default();
    mesh.vertices.push(egui::epaint::Vertex { pos: p1, uv: egui::Pos2::ZERO, color });
    mesh.vertices.push(egui::epaint::Vertex { pos: p2, uv: egui::Pos2::ZERO, color });
    mesh.vertices.push(egui::epaint::Vertex { pos: p3, uv: egui::Pos2::ZERO, color });
    mesh.vertices.push(egui::epaint::Vertex { pos: p4, uv: egui::Pos2::ZERO, color });
    mesh.indices.extend([0, 1, 2, 0, 2, 3]);
    painter.add(egui::Shape::mesh(mesh));
}

pub fn draw_arrow(painter: &egui::Painter, from: egui::Pos2, to: egui::Pos2, width: f32, color: egui::Color32) {
    painter.line_segment([from, to], egui::Stroke::new(width, color));
    let dir = (to - from).normalized();
    let perp = egui::vec2(-dir.y, dir.x);
    let head_len = (width * 5.0).max(12.0);
    let p1 = to - dir * head_len + perp * head_len * 0.5;
    let p2 = to - dir * head_len - perp * head_len * 0.5;
    painter.add(egui::Shape::line(vec![p1, to, p2], egui::Stroke::new(width, color)));
}

pub fn draw_star_shape(painter: &egui::Painter, center: egui::Pos2, radius: f32, color: egui::Color32, bg_color: Option<egui::Color32>, width: f32) {
    let pts = get_star_points(center, radius);
    if let Some(bg) = bg_color {
        let mut mesh = egui::Mesh::default();
        let c_idx = mesh.vertices.len() as u32;
        mesh.vertices.push(egui::epaint::Vertex { pos: center, uv: egui::Pos2::ZERO, color: bg });
        for i in 0..pts.len() {
            mesh.vertices.push(egui::epaint::Vertex { pos: pts[i], uv: egui::Pos2::ZERO, color: bg });
            let next = (i + 1) % pts.len();
            mesh.indices.push(c_idx);
            mesh.indices.push(c_idx + 1 + i as u32);
            mesh.indices.push(c_idx + 1 + next as u32);
        }
        painter.add(egui::Shape::mesh(mesh));
    }
    let mut loop_pts = pts;
    loop_pts.push(loop_pts[0]);
    painter.add(egui::Shape::line(loop_pts, egui::Stroke::new(width, color)));
}

pub fn draw_heart_shape(painter: &egui::Painter, center: egui::Pos2, radius: f32, color: egui::Color32, bg_color: Option<egui::Color32>, width: f32) {
    let pts = get_heart_points(center, radius);
    if let Some(bg) = bg_color {
        let mut mesh = egui::Mesh::default();
        let c_idx = mesh.vertices.len() as u32;
        mesh.vertices.push(egui::epaint::Vertex { pos: center, uv: egui::Pos2::ZERO, color: bg });
        for i in 0..pts.len() {
            mesh.vertices.push(egui::epaint::Vertex { pos: pts[i], uv: egui::Pos2::ZERO, color: bg });
            let next = (i + 1) % pts.len();
            mesh.indices.push(c_idx);
            mesh.indices.push(c_idx + 1 + i as u32);
            mesh.indices.push(c_idx + 1 + next as u32);
        }
        painter.add(egui::Shape::mesh(mesh));
    }
    let mut loop_pts = pts;
    loop_pts.push(loop_pts[0]);
    painter.add(egui::Shape::line(loop_pts, egui::Stroke::new(width, color)));
}

pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let h = ((h % 360.0) + 360.0) % 360.0;
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    (((r + m) * 255.0) as u8, ((g + m) * 255.0) as u8, ((b + m) * 255.0) as u8)
}

pub fn layer_bounds(layer: &crate::project::Layer) -> Option<egui::Rect> {
    let mut rect: Option<egui::Rect> = None;
    let mut extend = |r: egui::Rect| {
        if let Some(curr) = rect { rect = Some(curr.union(r)); }
        else { rect = Some(r); }
    };

    for i in 0..layer.placed_images.len() {
        if let Some(b) = object_bounds(layer, ObjectType::Image, i) { extend(b); }
    }
    for i in 0..layer.strokes.len() {
        if let Some(b) = object_bounds(layer, ObjectType::Stroke, i) { extend(b); }
    }
    for i in 0..layer.text_annotations.len() {
        if let Some(b) = object_bounds(layer, ObjectType::Text, i) { extend(b); }
    }
    
    // If empty, return a default reasonable rect to allow selection
    rect.or_else(|| Some(egui::Rect::from_center_size(egui::pos2(500.0, 500.0), egui::vec2(200.0, 150.0))))
}

pub fn object_bounds(layer: &crate::project::Layer, obj_type: ObjectType, obj_idx: usize) -> Option<egui::Rect> {
    let mut rect: Option<egui::Rect> = None;
    let mut extend = |pos: egui::Pos2, padding: f32| {
        let p_rect = egui::Rect::from_center_size(pos, egui::vec2(padding, padding));
        if let Some(r) = &mut rect { *r = r.union(p_rect); }
        else { rect = Some(p_rect); }
    };

    match obj_type {
        ObjectType::Image => {
            if let Some(img) = layer.placed_images.get(obj_idx) {
                let disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
                let disp_h = img.display_size.unwrap_or([img.size[1] as f32, img.size[1] as f32])[1];
                let initial_rect = egui::Rect::from_min_size(img.position, egui::vec2(disp_w, disp_h));
                let center = initial_rect.center();
                for c in [initial_rect.left_top(), initial_rect.right_top(), initial_rect.left_bottom(), initial_rect.right_bottom()] {
                    extend(transform_point_complex(c, center, img.rotation, img.skew, img.perspective, initial_rect, egui::vec2(1.0, 1.0)), 0.0);
                }
            }
        }
        ObjectType::Stroke => {
            if let Some(s) = layer.strokes.get(obj_idx) {
                let padding = s.width;
                if s.points.is_empty() { return None; }
                let mut min = egui::pos2(f32::MAX, f32::MAX);
                let mut max = egui::pos2(f32::MIN, f32::MIN);
                for &pt in &s.points {
                    min.x = min.x.min(pt.x); min.y = min.y.min(pt.y);
                    max.x = max.x.max(pt.x); max.y = max.y.max(pt.y);
                }
                let initial_rect = egui::Rect::from_min_max(min, max);
                let center = initial_rect.center();

                match s.kind {
                    StrokeKind::Circle | StrokeKind::Star | StrokeKind::Heart => {
                        if s.points.len() >= 2 {
                            let r_center = s.points[0];
                            let radius = r_center.distance(s.points[1]) + padding / 2.0;
                            extend(r_center - egui::vec2(radius, radius), 0.0);
                            extend(r_center + egui::vec2(radius, radius), 0.0);
                        }
                    }
                    _ => {
                        for p in &s.points {
                            extend(transform_point_complex(*p, center, s.rotation, s.skew, s.perspective, initial_rect, s.scale), padding);
                        }
                    }
                }
            }
        }
        ObjectType::Text => {
            if let Some(ann) = layer.text_annotations.get(obj_idx) {
                let size = egui::vec2(ann.exact_size[0], ann.exact_size[1]);
                let initial_rect = egui::Rect::from_min_size(ann.position, size);
                let center = initial_rect.center();
                for c in [initial_rect.left_top(), initial_rect.right_top(), initial_rect.left_bottom(), initial_rect.right_bottom()] {
                    extend(transform_point_complex(c, center, ann.rotation, ann.skew, ann.perspective, initial_rect, ann.scale), 0.0);
                }
            }
        }
    }
    rect
}

pub fn translate_layer(layer: &mut crate::project::Layer, delta: egui::Vec2) {
    for img in &mut layer.placed_images { img.position += delta; img.thumbnail_dirty = true; }
    for s in &mut layer.strokes {
        for p in &mut s.points { *p += delta; }
    }
    for ann in &mut layer.text_annotations { ann.position += delta; }
}

pub fn scale_layer(layer: &mut crate::project::Layer, center: egui::Pos2, scale: egui::Vec2) {
    if scale.x.abs() < 0.01 || scale.y.abs() < 0.01 { return; }

    for img in &mut layer.placed_images {
        let rel = img.position - center;
        img.position = center + egui::vec2(rel.x * scale.x, rel.y * scale.y);
        
        let mut disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
        let mut disp_h = img.display_size.unwrap_or([img.size[1] as f32, img.size[1] as f32])[1];
        disp_w *= scale.x;
        disp_h *= scale.y;
        img.display_size = Some([disp_w, disp_h]);
        img.thumbnail_dirty = true;
    }
    for s in &mut layer.strokes {
        for p in &mut s.points {
            let rel = *p - center;
            *p = center + egui::vec2(rel.x * scale.x, rel.y * scale.y);
        }
        s.scale.x *= scale.x;
        s.scale.y *= scale.y;
        s.width *= (scale.x.abs() + scale.y.abs()) * 0.5;
    }
    for ann in &mut layer.text_annotations {
        let rel = ann.position - center;
        ann.position = center + egui::vec2(rel.x * scale.x, rel.y * scale.y);
        ann.scale.x *= scale.x;
        ann.scale.y *= scale.y;
        ann.font_size *= (scale.x.abs() + scale.y.abs()) * 0.5;
    }
}

pub fn rotate_layer(layer: &mut crate::project::Layer, center: egui::Pos2, angle: f32) {
    let sin = angle.sin();
    let cos = angle.cos();
    let rot = |p: egui::Pos2| {
        let rel = p - center;
        center + egui::vec2(rel.x * cos - rel.y * sin, rel.y * cos + rel.x * sin)
    };
    for img in &mut layer.placed_images { img.position = rot(img.position); img.rotation += angle; img.thumbnail_dirty = true; }
    for s in &mut layer.strokes {
        for p in &mut s.points { *p = rot(*p); }
        s.rotation += angle;
    }
    for ann in &mut layer.text_annotations { ann.position = rot(ann.position); ann.rotation += angle; }
}

pub fn skew_layer(layer: &mut crate::project::Layer, center: egui::Pos2, skew_delta: egui::Vec2) {
    let skew_p = |p: egui::Pos2| {
        let rel = p - center;
        center + egui::vec2(rel.x + rel.y * skew_delta.x, rel.y + rel.x * skew_delta.y)
    };
    for img in &mut layer.placed_images { img.position = skew_p(img.position); img.skew += skew_delta; img.thumbnail_dirty = true; }
    for s in &mut layer.strokes {
        for p in &mut s.points { *p = skew_p(*p); }
        s.skew += skew_delta;
    }
    for ann in &mut layer.text_annotations { ann.position = skew_p(ann.position); ann.skew += skew_delta; }
}

pub fn perspective_layer(layer: &mut crate::project::Layer, p_idx: usize, delta: egui::Vec2) {
    for img in &mut layer.placed_images { img.perspective[p_idx] += delta; }
    for s in &mut layer.strokes { s.perspective[p_idx] += delta; }
    for ann in &mut layer.text_annotations { ann.perspective[p_idx] += delta; }
}

pub fn transform_mesh(mesh: &mut egui::Mesh, center: egui::Pos2, rotation: f32, skew: egui::Vec2, perspective: [egui::Vec2; 4], scale: egui::Vec2) {
    if mesh.vertices.is_empty() { return; }
    let mut min = egui::pos2(f32::MAX, f32::MAX);
    let mut max = egui::pos2(f32::MIN, f32::MIN);
    for v in &mesh.vertices {
        min.x = min.x.min(v.pos.x); min.y = min.y.min(v.pos.y);
        max.x = max.x.max(v.pos.x); max.y = max.y.max(v.pos.y);
    }
    let size = max - min;
    let sin = rotation.sin();
    let cos = rotation.cos();

    for v in &mut mesh.vertices {
        let p = v.pos;
        let tx = if size.x > 0.0 { (p.x - min.x) / size.x } else { 0.5 };
        let ty = if size.y > 0.0 { (p.y - min.y) / size.y } else { 0.5 };
        
        let p_offset = 
            perspective[0] * (1.0 - tx) * (1.0 - ty) + 
            perspective[1] * tx * (1.0 - ty) +        
            perspective[2] * (1.0 - tx) * ty +        
            perspective[3] * tx * ty;                 
            
        let rel_p = p - center; 
        let px = rel_p.x * scale.x + p_offset.x + rel_p.y * scale.y * skew.x;
        let py = rel_p.y * scale.y + p_offset.y + rel_p.x * scale.x * skew.y;
        
        v.pos.x = center.x + px * cos - py * sin;
        v.pos.y = center.y + py * cos + px * sin;
    }
}

pub fn transform_point_complex(p: egui::Pos2, center: egui::Pos2, rotation: f32, skew: egui::Vec2, perspective: [egui::Vec2; 4], initial_rect: egui::Rect, scale: egui::Vec2) -> egui::Pos2 {
    let size = initial_rect.size();
    let tx = if size.x > 0.0 { (p.x - initial_rect.min.x) / size.x } else { 0.5 };
    let ty = if size.y > 0.0 { (p.y - initial_rect.min.y) / size.y } else { 0.5 };
    
    let p_offset = 
        perspective[0] * (1.0 - tx) * (1.0 - ty) + 
        perspective[1] * tx * (1.0 - ty) +        
        perspective[2] * (1.0 - tx) * ty +        
        perspective[3] * tx * ty;                 
        
    let rel_p = p - center; 
    let cos = rotation.cos();
    let sin = rotation.sin();
    
    let px = rel_p.x * scale.x + p_offset.x + rel_p.y * scale.y * skew.x;
    let py = rel_p.y * scale.y + p_offset.y + rel_p.x * scale.x * skew.y;
    
    center + egui::vec2(px * cos - py * sin, py * cos + px * sin)
}

pub fn apply_mesh_filters(mesh: &mut egui::Mesh, grayscale: bool, invert: bool, sepia: bool, glow: bool, glow_strength: f32) {
    if !grayscale && !invert && !sepia && !glow { return; }
    for v in &mut mesh.vertices {
        v.color = apply_color_effects(v.color, grayscale, invert, sepia, glow, glow_strength);
    }
}

pub fn apply_color_effects(mut color: egui::Color32, grayscale: bool, invert: bool, sepia: bool, glow: bool, glow_strength: f32) -> egui::Color32 {
    let mut r = color.r() as f32 / 255.0;
    let mut g = color.g() as f32 / 255.0;
    let mut b = color.b() as f32 / 255.0;
    let a = color.a();

    if grayscale {
        let gray = 0.299 * r + 0.587 * g + 0.114 * b;
        r = gray; g = gray; b = gray;
    }
    if invert {
        r = 1.0 - r; g = 1.0 - g; b = 1.0 - b;
    }
    if sepia {
        let tr = (r * 0.393) + (g * 0.769) + (b * 0.189);
        let tg = (r * 0.349) + (g * 0.686) + (b * 0.168);
        let tb = (r * 0.272) + (g * 0.534) + (b * 0.131);
        r = tr.min(1.0); g = tg.min(1.0); b = tb.min(1.0);
    }
    if glow {
        let glow_mod = 1.0 + (glow_strength * 0.05);
        r = (r * glow_mod).min(1.0);
        g = (g * glow_mod).min(1.0);
        b = (b * glow_mod).min(1.0);
    }
    egui::Color32::from_rgba_unmultiplied((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8, a)
}
