//! Canvas that can draw to SVG

use crate::{drawing::Color, numeric::v2f::V2f};
use core::fmt::Write;

pub struct Canvas {
    buffer: String,
}

impl Canvas {
    pub fn new(size: V2f) -> Self {
        Self {
            buffer: format!("<svg width=\"{}\" height=\"{}\">", size.x, size.y),
        }
    }

    pub fn to_svg(mut self) -> String {
        self.buffer.push_str("</svg>");
        self.buffer
    }
}

impl crate::drawing::Canvas for Canvas {
    fn tile(&mut self, position: V2f, color: Color) {
        let tile_size = Self::tile_size();
        write!(
            &mut self.buffer,
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"rgba({},{},{},{})\"/>",
            position.x,
            position.y,
            position.x + tile_size.x,
            position.y + tile_size.y,
            color.r,
            color.g,
            color.b,
            color.a as f32 / 255.0,
        )
        .unwrap();
    }

    fn circle(&mut self, position: V2f, radius: f32, color: Color) {
        write!(
            &mut self.buffer,
            "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" stroke=\"{}\" fill=\"rgba({},{},{},{})\" />",
            position.x,
            position.y,
            radius,
            2.0,
            color.r,
            color.g,
            color.b,
            color.a as f32 / 255.0,
        )
        .unwrap();
    }

    fn line(&mut self, start: V2f, end: V2f) {
        write!(
            &mut self.buffer,
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke-width=\"{}\" stroke=\"black\"/>",
            start.x,
            start.y,
            end.x,
            end.y,
            Self::default_line_weight()
        )
        .unwrap();
    }

    fn tile_size() -> V2f {
        V2f { x: 50.0, y: 50.0 }
    }

    fn default_line_weight() -> f32 {
        2.0
    }
}
