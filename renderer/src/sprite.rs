use celest_core::command::Position;

use crate::vertex::Vertex;

pub fn position_to_ndc(
    position: Position,
    img_w: u32,
    img_h: u32,
    surface_w: u32,
    surface_h: u32,
) -> (f32, f32, f32, f32) {
    let w = surface_w as f32;
    let h = surface_h as f32;
    let max_w_px = w * 0.5;
    let max_h_px = h * 0.6;
    let scale = (max_w_px / img_w as f32).min(max_h_px / img_h as f32);

    let target_h = img_h as f32 * scale;
    let target_w = img_w as f32 * scale;
    let (cx, cy) = match position {
        Position::Left => (w * 0.25, h * 0.7),
        Position::Center => (w * 0.5, h * 0.7),
        Position::Right => (w * 0.75, h * 0.7),
        Position::Absolute(px, py) => (px, py),
    };

    let cx_ndc = cx / w * 2.0 - 1.0;
    let cy_ndc = 1.0 - cy / h * 2.0;

    let hw = target_w / w;
    let hh = target_h / h;

    let x = cx_ndc - hw;
    let y = cy_ndc + hh;

    (x, y, hw * 2.0, hh * 2.0)
}
pub fn build_quad_vertices(x: f32, y: f32, w: f32, h: f32) -> [Vertex; 6] {
    [
        Vertex {
            position: [x, y],
            tex_coord: [0.0, 0.0],
        },
        Vertex {
            position: [x + w, y],
            tex_coord: [1.0, 0.0],
        },
        Vertex {
            position: [x + w, y - h],
            tex_coord: [1.0, 1.0],
        },
        Vertex {
            position: [x, y],
            tex_coord: [0.0, 0.0],
        },
        Vertex {
            position: [x + w, y - h],
            tex_coord: [1.0, 1.0],
        },
        Vertex {
            position: [x, y - h],
            tex_coord: [0.0, 1.0],
        },
    ]
}
