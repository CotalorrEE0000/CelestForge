use std::path::Path;

use egui::{Color32, Mesh, Pos2, Rect, TextureHandle, TextureOptions};
use serde::Deserialize;
const DEFAULT_CORNER: f32 = 8.0;

#[derive(Debug, Default, Deserialize)]
pub struct NineCfg {
    pub image: String,

    #[serde(default = "default_corner")]
    pub corner: f32,
}

#[derive(Debug, Default, Deserialize)]
pub struct SkinFile {
    pub button: Option<NineCfg>,
    pub bar: Option<NineCfg>,
}

#[derive(Default)]
pub struct Skin {
    button: Option<NineSlice>,
    bar: Option<NineSlice>,
    tried: bool,
}

fn default_corner() -> f32 {
    DEFAULT_CORNER
}

pub struct NineSlice {
    tex: TextureHandle,
    src: Rect,
    corner: f32,
}
pub fn nine_slice(mesh: &mut Mesh, dst: Rect, src: Rect, corner: f32, tint: Color32) {
    let size = src.size();
    let norm = |p: Pos2| {
        Pos2::new(
            (p.x - src.min.x) / size.x.max(1.0),
            (p.y - src.min.y) / size.y.max(1.0),
        )
    };

    let cx = corner.min(dst.width() * 0.5).min(src.width() * 0.5);
    let cy = corner.min(dst.height() * 0.5).min(src.height() * 0.5);
    let d = [dst.min.x, dst.min.x + cx, dst.max.x - cx, dst.max.x];
    let e = [dst.min.y, dst.min.y + cy, dst.max.y - cy, dst.max.y];
    let s = [src.min.x, src.min.x + cx, src.max.x - cx, src.max.x];
    let t = [src.min.y, src.min.y + cy, src.max.y - cy, src.max.y];

    for row in 0..3 {
        for col in 0..3 {
            let base = mesh.vertices.len() as u32;
            let (x0, x1) = (d[col], d[col + 1]);
            let (y0, y1) = (e[row], e[row + 1]);
            let (u0, u1) = (s[col], s[col + 1]);
            let (v0, v1) = (t[row], t[row + 1]);
            for (pos, uv) in [
                (Pos2::new(x0, y0), Pos2::new(u0, v0)),
                (Pos2::new(x1, y0), Pos2::new(u1, v0)),
                (Pos2::new(x1, y1), Pos2::new(u1, v1)),
                (Pos2::new(x0, y1), Pos2::new(u0, v1)),
            ] {
                mesh.vertices.push(egui::epaint::Vertex {
                    pos,
                    uv: norm(uv),
                    color: tint,
                });
            }
            mesh.indices
                .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }
}

impl NineSlice {
    pub fn load(ctx: &egui::Context, key: &str, cfg: &NineCfg) -> Option<Self> {
        let img = image::open(&cfg.image).ok()?.to_rgba8();
        let (w, h) = img.dimensions();
        let color =
            egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], img.as_raw());
        let tex = ctx.load_texture(key, color, TextureOptions::LINEAR);
        let corner = cfg.corner.max(1.0).min(w as f32 * 0.5).min(h as f32 * 0.5);
        Some(Self {
            tex,
            src: Rect::from_min_size(Pos2::ZERO, egui::vec2(w as f32, h as f32)),
            corner,
        })
    }

    pub fn paint(&self, painter: &egui::Painter, dst: Rect, tint: Color32) {
        let mut mesh = Mesh::default();
        mesh.texture_id = self.tex.id();
        nine_slice(&mut mesh, dst, self.src, self.corner, tint);
        painter.add(egui::Shape::mesh(mesh));
    }
}

impl Skin {
    pub fn load(ctx: &egui::Context, path: &Path) -> Self {
        let mut skin = Skin {
            tried: true,
            ..Default::default()
        };
        let Ok(text) = std::fs::read_to_string(path) else {
            return skin;
        };

        let Ok(file) = serde_json::from_str::<SkinFile>(&text) else {
            eprintln!("[SKIN] CAN'T LOAD {path:?}, RETURNED TO DEFAULT SKIN");
            return skin;
        };
        skin.button = file
            .button
            .as_ref()
            .and_then(|c| NineSlice::load(ctx, "cf_skin_button", c));
        skin.bar = file
            .bar
            .as_ref()
            .and_then(|c| NineSlice::load(ctx, "cf_skin_bar", c));
        skin
    }

    pub fn ensure_loaded(&mut self, ctx: &egui::Context, path: &Path) {
        if !self.tried {
            *self = Skin::load(ctx, path);
        }
    }

    pub fn button(&self) -> Option<&NineSlice> {
        self.button.as_ref()
    }

    pub fn bar(&self) -> Option<&NineSlice> {
        self.bar.as_ref()
    }

    pub fn is_empty(&self) -> bool {
        self.button.is_none() && self.bar.is_none()
    }
}
