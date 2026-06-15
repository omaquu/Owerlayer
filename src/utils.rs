use eframe::egui;
use crate::types::*;

pub fn color32(c: &[u8; 4]) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])
}

pub fn is_inside_poly(poly: &[egui::Pos2], p: egui::Pos2) -> bool {
    let mut inside = false;
    if poly.is_empty() { return false; }
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let dy = poly[j].y - poly[i].y;
        if dy.abs() > 0.000001 {
            if ((poly[i].y > p.y) != (poly[j].y > p.y)) &&
               (p.x < (poly[j].x - poly[i].x) * (p.y - poly[i].y) / dy + poly[i].x) {
                inside = !inside;
            }
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
    if w <= 0 || h <= 0 { return; }
    if start_x < 0 || start_x >= w || start_y < 0 || start_y >= h { return; }
    
    let expected_len = (w * h * 4) as usize;
    if img.pixels.len() < expected_len { return; }

    let color_diff = |c1: [u8; 4], c2: [u8; 4]| -> f32 {
        let dr = (c1[0] as f32 - c2[0] as f32).abs();
        let dg = (c1[1] as f32 - c2[1] as f32).abs();
        let db = (c1[2] as f32 - c2[2] as f32).abs();
        (dr + dg + db) / 3.0
    };

    let matches = |x: i32, y: i32, pixels: &[u8]| -> bool {
        if x < 0 || x >= w || y < 0 || y >= h { return false; }
        let idx = (y * w + x) as usize;
        let pixel_idx = idx * 4;
        if pixel_idx + 3 >= pixels.len() { return false; }
        let current_color = [
            pixels[pixel_idx],
            pixels[pixel_idx+1],
            pixels[pixel_idx+2],
            pixels[pixel_idx+3]
        ];
        current_color[3] > 0 && color_diff(current_color, target_color) <= threshold
    };

    if !matches(start_x, start_y, &img.pixels) { return; }

    let mut stack = vec![(start_x, start_y)];
    let mut visited = vec![false; (w * h) as usize];
    visited[(start_y * w + start_x) as usize] = true;

    while let Some((x, y)) = stack.pop() {
        let idx = (y * w + x) as usize;
        let pixel_idx = idx * 4;
        if pixel_idx + 3 < img.pixels.len() {
            img.pixels[pixel_idx + 3] = 0; // Erase
        }

        let neighbors = [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)];
        for &(nx, ny) in &neighbors {
            if nx >= 0 && nx < w && ny >= 0 && ny < h {
                let nidx = (ny * w + nx) as usize;
                if !visited[nidx] && matches(nx, ny, &img.pixels) {
                    visited[nidx] = true;
                    stack.push((nx, ny));
                }
            }
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
    let scale_p = |p: egui::Pos2| {
        let rel = p - center;
        center + egui::vec2(rel.x * scale.x, rel.y * scale.y)
    };

    for img in &mut layer.placed_images {
        let disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
        let disp_h = img.display_size.unwrap_or([img.size[1] as f32, img.size[1] as f32])[1];
        let c = img.position + egui::vec2(disp_w * 0.5, disp_h * 0.5);
        let c_new = scale_p(c);
        img.position += c_new - c;
        img.display_size = Some([disp_w * scale.x, disp_h * scale.y]);
        img.thumbnail_dirty = true;
    }
    for s in &mut layer.strokes {
        if s.points.is_empty() { continue; }
        let mut min = egui::pos2(f32::MAX, f32::MAX);
        let mut max = egui::pos2(f32::MIN, f32::MIN);
        for &pt in &s.points {
            min.x = min.x.min(pt.x); min.y = min.y.min(pt.y);
            max.x = max.x.max(pt.x); max.y = max.y.max(pt.y);
        }
        let c = egui::Rect::from_min_max(min, max).center();
        let c_new = scale_p(c);
        let delta = c_new - c;
        for p in &mut s.points { *p += delta; }
        s.scale.x *= scale.x;
        s.scale.y *= scale.y;
        s.width *= (scale.x.abs() + scale.y.abs()) * 0.5;
    }
    for ann in &mut layer.text_annotations {
        let size = egui::vec2(ann.exact_size[0], ann.exact_size[1]);
        let c = egui::Rect::from_min_size(ann.position, size).center();
        let c_new = scale_p(c);
        ann.position += c_new - c;
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
    for img in &mut layer.placed_images {
        let disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
        let disp_h = img.display_size.unwrap_or([img.size[1] as f32, img.size[1] as f32])[1];
        let c = img.position + egui::vec2(disp_w * 0.5, disp_h * 0.5);
        let c_new = rot(c);
        img.position += c_new - c;
        img.rotation += angle;
        img.thumbnail_dirty = true;
    }
    for s in &mut layer.strokes {
        if s.points.is_empty() { continue; }
        let mut min = egui::pos2(f32::MAX, f32::MAX);
        let mut max = egui::pos2(f32::MIN, f32::MIN);
        for &pt in &s.points {
            min.x = min.x.min(pt.x); min.y = min.y.min(pt.y);
            max.x = max.x.max(pt.x); max.y = max.y.max(pt.y);
        }
        let c = egui::Rect::from_min_max(min, max).center();
        let c_new = rot(c);
        let delta = c_new - c;
        for p in &mut s.points { *p += delta; }
        s.rotation += angle;
    }
    for ann in &mut layer.text_annotations {
        let size = egui::vec2(ann.exact_size[0], ann.exact_size[1]);
        let c = egui::Rect::from_min_size(ann.position, size).center();
        let c_new = rot(c);
        ann.position += c_new - c;
        ann.rotation += angle;
    }
}

pub fn skew_layer(layer: &mut crate::project::Layer, center: egui::Pos2, skew_delta: egui::Vec2) {
    let skew_p = |p: egui::Pos2| {
        let rel = p - center;
        center + egui::vec2(rel.x + rel.y * skew_delta.x, rel.y + rel.x * skew_delta.y)
    };
    for img in &mut layer.placed_images {
        let disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
        let disp_h = img.display_size.unwrap_or([img.size[1] as f32, img.size[1] as f32])[1];
        let c = img.position + egui::vec2(disp_w * 0.5, disp_h * 0.5);
        let c_new = skew_p(c);
        img.position += c_new - c;
        img.skew += skew_delta;
        img.thumbnail_dirty = true;
    }
    for s in &mut layer.strokes {
        if s.points.is_empty() { continue; }
        let mut min = egui::pos2(f32::MAX, f32::MAX);
        let mut max = egui::pos2(f32::MIN, f32::MIN);
        for &pt in &s.points {
            min.x = min.x.min(pt.x); min.y = min.y.min(pt.y);
            max.x = max.x.max(pt.x); max.y = max.y.max(pt.y);
        }
        let c = egui::Rect::from_min_max(min, max).center();
        let c_new = skew_p(c);
        let delta = c_new - c;
        for p in &mut s.points { *p += delta; }
        s.skew += skew_delta;
    }
    for ann in &mut layer.text_annotations {
        let size = egui::vec2(ann.exact_size[0], ann.exact_size[1]);
        let c = egui::Rect::from_min_size(ann.position, size).center();
        let c_new = skew_p(c);
        ann.position += c_new - c;
        ann.skew += skew_delta;
    }
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
    if color.a() == 0 { return color; }
    let a_f32 = color.a() as f32 / 255.0;

    let mut r = (color.r() as f32 / 255.0) / a_f32;
    let mut g = (color.g() as f32 / 255.0) / a_f32;
    let mut b = (color.b() as f32 / 255.0) / a_f32;

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
    
    r = (r * a_f32).clamp(0.0, 1.0);
    g = (g * a_f32).clamp(0.0, 1.0);
    b = (b * a_f32).clamp(0.0, 1.0);

    egui::Color32::from_rgba_premultiplied((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8, color.a())
}

pub fn trace_boundary(visited: &[bool], w: i32, h: i32, start_x: i32, start_y: i32) -> Vec<(i32, i32)> {
    let mut boundary = Vec::new();
    let dirs = [
        (-1, 0),  // W
        (-1, -1), // NW
        (0, -1),  // N
        (1, -1),  // NE
        (1, 0),   // E
        (1, 1),   // SE
        (0, 1),   // S
        (-1, 1),  // SW
    ];
    
    let is_visited = |x: i32, y: i32| -> bool {
        if x < 0 || x >= w || y < 0 || y >= h {
            false
        } else {
            visited[(y * w + x) as usize]
        }
    };

    let mut curr = (start_x, start_y);
    let mut backtrack_idx = 0;
    
    boundary.push(curr);
    
    let max_iterations = (w * h) as usize;
    let mut iterations = 0;
    
    loop {
        iterations += 1;
        if iterations > max_iterations {
            break;
        }
        
        let mut found_next = false;
        for i in 0..8 {
            let idx = (backtrack_idx + i) % 8;
            let neighbor = (curr.0 + dirs[idx].0, curr.1 + dirs[idx].1);
            if is_visited(neighbor.0, neighbor.1) {
                curr = neighbor;
                backtrack_idx = (idx + 5) % 8; 
                found_next = true;
                break;
            }
        }
        
        if !found_next {
            break;
        }
        
        if curr == (start_x, start_y) {
            break;
        }
        
        boundary.push(curr);
    }
    
    boundary
}

pub fn magic_wand_to_selection(img: &PlacedImage, start_x: i32, start_y: i32, target_color: [u8; 4], threshold: f32) -> Vec<egui::Pos2> {
    let w = img.size[0] as i32;
    let h = img.size[1] as i32;
    if w <= 0 || h <= 0 { return Vec::new(); }
    if start_x < 0 || start_x >= w || start_y < 0 || start_y >= h { return Vec::new(); }
    
    let expected_len = (w * h * 4) as usize;
    if img.pixels.len() < expected_len { return Vec::new(); }

    let color_diff = |c1: [u8; 4], c2: [u8; 4]| -> f32 {
        let dr = (c1[0] as f32 - c2[0] as f32).abs();
        let dg = (c1[1] as f32 - c2[1] as f32).abs();
        let db = (c1[2] as f32 - c2[2] as f32).abs();
        (dr + dg + db) / 3.0
    };

    let matches = |x: i32, y: i32, pixels: &[u8]| -> bool {
        if x < 0 || x >= w || y < 0 || y >= h { return false; }
        let idx = (y * w + x) as usize;
        let pixel_idx = idx * 4;
        if pixel_idx + 3 >= pixels.len() { return false; }
        let current_color = [
            pixels[pixel_idx],
            pixels[pixel_idx+1],
            pixels[pixel_idx+2],
            pixels[pixel_idx+3]
        ];
        current_color[3] > 0 && color_diff(current_color, target_color) <= threshold
    };

    if !matches(start_x, start_y, &img.pixels) { return Vec::new(); }

    let mut stack = vec![(start_x, start_y)];
    let mut visited = vec![false; (w * h) as usize];
    visited[(start_y * w + start_x) as usize] = true;

    while let Some((x, y)) = stack.pop() {
        let neighbors = [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)];
        for &(nx, ny) in &neighbors {
            if nx >= 0 && nx < w && ny >= 0 && ny < h {
                let nidx = (ny * w + nx) as usize;
                if !visited[nidx] && matches(nx, ny, &img.pixels) {
                    visited[nidx] = true;
                    stack.push((nx, ny));
                }
            }
        }
    }

    let mut start_opt = None;
    'outer: for y in 0..h {
        for x in 0..w {
            if visited[(y * w + x) as usize] {
                start_opt = Some((x, y));
                break 'outer;
            }
        }
    }

    let Some((sx, sy)) = start_opt else {
        return Vec::new();
    };

    let boundary_px = trace_boundary(&visited, w, h, sx, sy);
    
    let disp_w = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[0];
    let disp_h = img.display_size.unwrap_or([img.size[0] as f32, img.size[1] as f32])[1];
    let img_w = img.size[0] as f32;
    let img_h = img.size[1] as f32;

    boundary_px.into_iter().map(|(x, y)| {
        let rx = img.position.x + (x as f32) * (disp_w / img_w);
        let ry = img.position.y + (y as f32) * (disp_h / img_h);
        egui::pos2(rx, ry)
    }).collect()
}

pub fn draw_dashed_path(painter: &egui::Painter, points: &[egui::Pos2], time: f64) {
    if points.len() < 2 { return; }
    
    // Draw solid black line under the path to ensure perfect contrast and prevent flashing
    painter.add(egui::Shape::line(
        points.to_vec(),
        egui::Stroke::new(2.0, egui::Color32::BLACK)
    ));
    
    let dash_len = 6.0f32;
    let speed = 8.0f32; // Crawl slowly and gracefully
    
    let mut current_offset = (time as f32 * speed) % (dash_len * 2.0);
    let mut draw_white = current_offset < dash_len;
    if !draw_white {
        current_offset -= dash_len;
    }
    
    for i in 0..points.len() - 1 {
        let p1 = points[i];
        let p2 = points[i+1];
        let dir = p2 - p1;
        let dist = dir.length();
        if dist < 0.001 { continue; }
        
        let dir = dir / dist;
        let mut t = 0.0f32;
        
        while t < dist {
            let dash_left = dash_len - current_offset;
            let step = dash_left.min(dist - t);
            
            let start = p1 + dir * t;
            let end = p1 + dir * (t + step);
            
            if draw_white {
                painter.line_segment([start, end], egui::Stroke::new(1.2, egui::Color32::WHITE));
            }
            
            t += step;
            current_offset += step;
            if current_offset >= dash_len {
                current_offset = 0.0;
                draw_white = !draw_white;
            }
        }
    }
}

pub fn draw_dashed_path_color(painter: &egui::Painter, points: &[egui::Pos2], time: f64, stroke_color: egui::Color32, stroke_width: f32) {
    if points.len() < 2 { return; }
    
    // Draw solid black line under the path to ensure perfect contrast
    painter.add(egui::Shape::line(
        points.to_vec(),
        egui::Stroke::new(stroke_width + 1.0, egui::Color32::BLACK)
    ));
    
    let dash_len = 6.0f32;
    let speed = 8.0f32;
    
    let mut current_offset = (time as f32 * speed) % (dash_len * 2.0);
    let mut draw_color = current_offset < dash_len;
    if !draw_color {
        current_offset -= dash_len;
    }
    
    for i in 0..points.len() - 1 {
        let p1 = points[i];
        let p2 = points[i+1];
        let dir = p2 - p1;
        let dst = dir.length();
        if dst < 0.001 { continue; }
        
        let dir = dir / dst;
        let mut t = 0.0f32;
        
        while t < dst {
            let dash_left = dash_len - current_offset;
            let step = dash_left.min(dst - t);
            
            let start = p1 + dir * t;
            let end = p1 + dir * (t + step);
            
            if draw_color {
                painter.line_segment([start, end], egui::Stroke::new(stroke_width, stroke_color));
            }
            
            t += step;
            current_offset += step;
            if current_offset >= dash_len {
                current_offset = 0.0;
                draw_color = !draw_color;
            }
        }
    }
}

/// Clamp rect for UI panels — uses primary monitor region when spanning virtual desktop.
pub fn ui_clamp_rect(ctx: &egui::Context, multi_monitor: bool) -> egui::Rect {
    if multi_monitor {
        #[cfg(windows)]
        {
            let (ox, oy) = crate::winapi_utils::primary_monitor_viewport_origin();
            let (w, h, _, _) = crate::winapi_utils::get_primary_monitor_rect();
            return egui::Rect::from_min_size(egui::pos2(ox, oy), egui::vec2(w, h));
        }
    }
    ctx.screen_rect()
}

pub fn enforce_window_bounds(
    ctx: &egui::Context,
    _id: egui::Id,
    saved_pos: &mut egui::Pos2,
    min_visible_width: f32,
    min_visible_height: f32,
) {
    enforce_window_bounds_in_rect(
        saved_pos,
        min_visible_width,
        min_visible_height,
        ctx.screen_rect(),
    );
}

pub fn enforce_window_bounds_monitor(
    ctx: &egui::Context,
    _id: egui::Id,
    saved_pos: &mut egui::Pos2,
    min_visible_width: f32,
    min_visible_height: f32,
    multi_monitor: bool,
) {
    enforce_window_bounds_in_rect(
        saved_pos,
        min_visible_width,
        min_visible_height,
        ui_clamp_rect(ctx, multi_monitor),
    );
}

fn enforce_window_bounds_in_rect(
    saved_pos: &mut egui::Pos2,
    min_visible_width: f32,
    min_visible_height: f32,
    rect: egui::Rect,
) {
    let min_x = rect.min.x;
    let min_y = rect.min.y;
    let max_x = (rect.max.x - min_visible_width).max(min_x);
    let max_y = (rect.max.y - min_visible_height).max(min_y);
    saved_pos.x = saved_pos.x.clamp(min_x, max_x);
    saved_pos.y = saved_pos.y.clamp(min_y, max_y);
}

/// Reset all floating panel positions to the primary monitor (monitor 1).
pub fn reset_ui_positions_to_primary(settings: &mut Settings) {
    let (ox, oy) = crate::winapi_utils::primary_monitor_viewport_origin();
    let base = egui::pos2(ox, oy);
    settings.toolbar_pos = base + egui::vec2(40.0, 60.0);
    settings.layer_menu_pos = base + egui::vec2(200.0, 60.0);
    settings.settings_menu_pos = base + egui::vec2(360.0, 60.0);
    settings.filter_menu_pos = base + egui::vec2(520.0, 60.0);
    settings.history_menu_pos = base + egui::vec2(680.0, 60.0);
    settings.object_fx_menu_pos = base + egui::vec2(840.0, 60.0);
    settings.creation_prompt_pos = base + egui::vec2(500.0, 300.0);
    settings.ui_reset_frames = 8;
}

/// Move panels onto the primary monitor if saved positions are off-screen.
pub fn ensure_ui_on_primary_monitor(settings: &mut Settings) -> bool {
    #[cfg(not(windows))]
    {
        let _ = settings;
        return false;
    }
    #[cfg(windows)]
    {
        let (sw, sh) = crate::winapi_utils::get_screen_size(settings.multi_monitor);
        let rect = egui::Rect::from_min_size(egui::pos2(1.0, 1.0), egui::vec2(sw, sh));
        if rect.contains(settings.toolbar_pos) {
            return false;
        }
        reset_ui_positions_to_primary(settings);
        true
    }
}

pub fn panel_window_pos<'a>(
    win: egui::Window<'a>,
    pos: egui::Pos2,
    reset_frames: u8,
) -> egui::Window<'a> {
    if reset_frames > 0 {
        win.fixed_pos(pos)
    } else {
        win.default_pos(pos)
    }
}


