//! Canvas that can draw to SVG

use crate::{drawing::Color, numeric::v2f::V2f};
use core::fmt::Write;
use std::fmt::Display;

struct Rgba(Color);

impl Display for Rgba {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "rgba({},{},{},{})",
            self.0.r,
            self.0.g,
            self.0.b,
            self.0.a as f32 / 255.0
        )
    }
}

struct SelfClosingTag<'buf> {
    buffer: &'buf mut String,
}

impl Drop for SelfClosingTag<'_> {
    fn drop(&mut self) {
        self.buffer.push_str(" />");
    }
}

impl<'buf> SelfClosingTag<'buf> {
    fn new(buffer: &'buf mut String, tag: &str) -> SelfClosingTag<'buf> {
        buffer.push('<');
        buffer.push_str(tag);
        SelfClosingTag { buffer }
    }

    fn attribute<V>(&mut self, name: &str, value: V)
    where
        V: Display,
    {
        self.buffer.push(' ');
        self.buffer.push_str(name);
        self.buffer.push_str("=\"");
        write!(self.buffer, "{}", value).unwrap();
        self.buffer.push('"');
    }
}

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

    fn self_closing_tag(&mut self, tag: &str) -> SelfClosingTag<'_> {
        SelfClosingTag::new(&mut self.buffer, tag)
    }
}

impl crate::drawing::Canvas for Canvas {
    fn rect(&mut self, position: V2f, size: V2f, color: Color) {
        let mut rect = self.self_closing_tag("rect");
        rect.attribute("x", position.x);
        rect.attribute("y", position.y);
        rect.attribute("width", size.x);
        rect.attribute("height", size.y);
        rect.attribute("fill", Rgba(color));
    }

    fn circle(&mut self, position: V2f, radius: f32, color: Color) {
        let mut circle = self.self_closing_tag("circle");
        circle.attribute("cx", position.x);
        circle.attribute("cy", position.y);
        circle.attribute("r", radius);
        circle.attribute("stroke", 2.0);
        circle.attribute("fill", Rgba(color));
    }

    fn line(&mut self, start: V2f, end: V2f) {
        let mut line = self.self_closing_tag("line");
        line.attribute("x1", start.x);
        line.attribute("y1", start.y);
        line.attribute("x2", end.x);
        line.attribute("y2", end.y);
        line.attribute("stroke-width", Self::default_line_weight());
        line.attribute("stroke", Rgba(Color::BLACK));
    }

    fn tile_size() -> V2f {
        V2f { x: 64.0, y: 64.0 }
    }

    fn default_line_weight() -> f32 {
        2.0
    }
}
