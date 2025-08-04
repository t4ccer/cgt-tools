//! Canvas that can draw to PNG

// TODO: Remove unwraps

use crate::{drawing::Color, numeric::v2f::V2f};
use tiny_skia;

pub struct Canvas {
    pixmap: tiny_skia::Pixmap,
}

impl Canvas {
    pub fn new(size: V2f) -> Canvas {
        Canvas {
            pixmap: tiny_skia::Pixmap::new(size.x as u32, size.y as u32).unwrap(),
        }
    }

    pub fn to_png(&self) -> Vec<u8> {
        self.pixmap.encode_png().unwrap()
    }
}

impl super::Canvas for Canvas {
    fn tile(&mut self, position: V2f, color: Color) {
        let size = Self::tile_size();
        self.pixmap.fill_rect(
            tiny_skia::Rect::from_xywh(position.x, position.y, size.x, size.y).unwrap(),
            &paint_solid_color(color),
            tiny_skia::Transform::identity(),
            None,
        );
    }

    fn circle(&mut self, position: V2f, radius: f32, color: Color) {
        let path = tiny_skia::PathBuilder::from_circle(position.x, position.y, radius).unwrap();
        self.pixmap.fill_path(
            &path,
            &paint_solid_color(color),
            tiny_skia::FillRule::Winding,
            tiny_skia::Transform::identity(),
            None,
        );
    }

    fn line(&mut self, start: V2f, end: V2f) {
        // TODO: with_capacity
        let mut path = tiny_skia::PathBuilder::new();
        path.move_to(start.x, start.y);
        path.line_to(end.x, end.y);
        let path = path.finish().unwrap();
        self.pixmap.stroke_path(
            &path,
            &paint_solid_color(Color::BLACK),
            &tiny_skia::Stroke {
                width: Self::default_line_weight(),
                miter_limit: 4.0,
                line_cap: tiny_skia::LineCap::Butt,
                line_join: tiny_skia::LineJoin::Miter,
                dash: None,
            },
            tiny_skia::Transform::identity(),
            None,
        );
    }

    fn tile_size() -> V2f {
        V2f { x: 50.0, y: 50.0 }
    }

    fn default_line_weight() -> f32 {
        2.0
    }
}

fn paint_solid_color(color: Color) -> tiny_skia::Paint<'static> {
    tiny_skia::Paint {
        shader: tiny_skia::Shader::SolidColor(tiny_skia::Color::from(color)),
        blend_mode: tiny_skia::BlendMode::SourceOver,
        anti_alias: false,
        force_hq_pipeline: false,
    }
}
