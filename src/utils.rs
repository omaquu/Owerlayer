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
    let mut extend = |pos: egui::Pos2, padding: f32| {
        let p_rect = egui::Rect::from_center_size(pos, egui::vec2(padding, padding));
        if let Some(r) = &mut rect { *r = r.union(p_rect); }
        else { rect = Some(p_rect); }
    };

    for img in &layer.placed_images {
        let disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
        let disp_h = img.display_size.unwrap_or([img.size[1] as f32, img.size[1] as f32])[1];
        extend(img.position, 0.0);
        extend(img.position + egui::vec2(disp_w, disp_h), 0.0);
    }
    for s in &layer.strokes {
        let padding = s.width;
        match s.kind {
            StrokeKind::Circle | StrokeKind::Star | StrokeKind::Heart => {
                if s.points.len() >= 2 {
                    let center = s.points[0];
                    let radius = center.distance(s.points[1]) + padding / 2.0;
                    extend(center - egui::vec2(radius, radius), 0.0);
                    extend(center + egui::vec2(radius, radius), 0.0);
                }
            }
            _ => {
                for p in &s.points { extend(*p, padding); }
            }
        }
    }
    for ann in &layer.text_annotations {
        extend(ann.position, 0.0);
        extend(ann.position + egui::vec2(ann.text.len() as f32 * ann.font_size * 0.6, ann.font_size * 1.2), 0.0);
    }
    rect
}

pub fn object_bounds(layer: &crate::project::Layer, obj_type: crate::project::ObjectType, obj_idx: usize) -> Option<egui::Rect> {
    let mut rect: Option<egui::Rect> = None;
    let mut extend = |pos: egui::Pos2, padding: f32| {
        let p_rect = egui::Rect::from_center_size(pos, egui::vec2(padding, padding));
        if let Some(r) = &mut rect { *r = r.union(p_rect); }
        else { rect = Some(p_rect); }
    };

    match obj_type {
        crate::project::ObjectType::Image => {
            if let Some(img) = layer.placed_images.get(obj_idx) {
                let disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
                let disp_h = img.display_size.unwrap_or([img.size[1] as f32, img.size[1] as f32])[1];
                extend(img.position, 0.0);
                extend(img.position + egui::vec2(disp_w, disp_h), 0.0);
            }
        }
        crate::project::ObjectType::Stroke => {
            if let Some(s) = layer.strokes.get(obj_idx) {
                let padding = s.width;
                match s.kind {
                    StrokeKind::Circle | StrokeKind::Star | StrokeKind::Heart => {
                        if s.points.len() >= 2 {
                            let center = s.points[0];
                            let radius = center.distance(s.points[1]) + padding / 2.0;
                            extend(center - egui::vec2(radius, radius), 0.0);
                            extend(center + egui::vec2(radius, radius), 0.0);
                        }
                    }
                    _ => {
                        for p in &s.points { extend(*p, padding); }
                    }
                }
            }
        }
        crate::project::ObjectType::Text => {
            if let Some(ann) = layer.text_annotations.get(obj_idx) {
                extend(ann.position, 0.0);
                extend(ann.position + egui::vec2(ann.text.len() as f32 * ann.font_size * 0.6, ann.font_size * 1.2), 0.0);
            }
        }
    }
    rect
}

pub fn translate_layer(layer: &mut crate::project::Layer, delta: egui::Vec2) {
    for img in &mut layer.placed_images { img.position += delta; }
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
    }
    for s in &mut layer.strokes {
        for p in &mut s.points {
            let rel = *p - center;
            *p = center + egui::vec2(rel.x * scale.x, rel.y * scale.y);
        }
        s.width *= (scale.x.abs() + scale.y.abs()) * 0.5;
    }
    for ann in &mut layer.text_annotations {
        let rel = ann.position - center;
        ann.position = center + egui::vec2(rel.x * scale.x, rel.y * scale.y);
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
    for img in &mut layer.placed_images { img.position = rot(img.position); img.rotation += angle; }
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
    for img in &mut layer.placed_images { img.position = skew_p(img.position); img.skew += skew_delta; }
    for s in &mut layer.strokes {
        for p in &mut s.points { *p = skew_p(*p); }
        s.skew += skew_delta;
    }
    for ann in &mut layer.text_annotations { ann.position = skew_p(ann.position); ann.skew += skew_delta; }
}

pub fn transform_mesh(mesh: &mut egui::Mesh, center: egui::Pos2, rotation: f32, skew: egui::Vec2, perspective: [egui::Vec2; 4]) {
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
            
        let rel_p = p - egui::Pos2::ZERO; 
        let px = rel_p.x + p_offset.x + rel_p.y * skew.x;
        let py = rel_p.y + p_offset.y + rel_p.x * skew.y;
        
        v.pos.x = center.x + px * cos - py * sin;
        v.pos.y = center.y + py * cos + px * sin;
    }
}

pub fn transform_point_complex(p: egui::Pos2, center: egui::Pos2, rotation: f32, skew: egui::Vec2, perspective: [egui::Vec2; 4]) -> egui::Pos2 {
    let rel = p - center;
    let cos = rotation.cos();
    let sin = rotation.sin();
    let rx = rel.x * cos - rel.y * sin;
    let ry = rel.y * cos + rel.x * sin;
    let sx = rx + ry * skew.x;
    let sy = ry + rx * skew.y;
    let px = sx + (sy * perspective[0].x + sx * perspective[0].y);
    let py = sy + (sx * perspective[1].x + sy * perspective[1].y);
    center + egui::vec2(px, py)
}
