//! Canvas that can draw to SVG

use crate::{
    drawing::{BoundingBox, Color, TextAlignment},
    numeric::v2f::V2f,
};
use core::fmt::Write;
use std::{fmt::Display, marker::PhantomData, mem::ManuallyDrop};

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

struct SelfClosing;
struct Open;
struct Content;

trait TagType {
    fn close(buffer: &mut String, tag: &str);
}

impl TagType for SelfClosing {
    fn close(buffer: &mut String, _tag: &str) {
        buffer.push_str(" />");
    }
}

impl TagType for Open {
    fn close(buffer: &mut String, tag: &str) {
        buffer.push_str("></");
        buffer.push_str(tag);
        buffer.push('>');
    }
}

impl TagType for Content {
    fn close(buffer: &mut String, tag: &str) {
        buffer.push_str("</");
        buffer.push_str(tag);
        buffer.push('>');
    }
}

trait HasAttributes: TagType {}

impl HasAttributes for SelfClosing {}
impl HasAttributes for Open {}

struct Tag<'buf, 'tag, Type>
where
    Type: TagType,
{
    buffer: &'buf mut String,
    #[allow(clippy::struct_field_names)]
    tag_name: &'tag str,
    _type: PhantomData<Type>,
}

impl<Type> Drop for Tag<'_, '_, Type>
where
    Type: TagType,
{
    fn drop(&mut self) {
        Type::close(self.buffer, self.tag_name);
    }
}

impl<Type> Tag<'_, '_, Type>
where
    Type: HasAttributes,
{
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

impl<'buf, 'tag> Tag<'buf, 'tag, Open> {
    fn finish_attributes(self) -> Tag<'buf, 'tag, Content> {
        self.buffer.push('>');
        let tag: &'tag str = self.tag_name;
        let buffer: &'buf mut String = unsafe { &mut *{ std::ptr::from_mut(self.buffer) } };
        let _ = ManuallyDrop::new(self);
        Tag {
            buffer,
            tag_name: tag,
            _type: PhantomData,
        }
    }
}

impl Tag<'_, '_, Content> {
    fn content<T>(&mut self, value: &T)
    where
        T: Display,
    {
        write!(self.buffer, "{}", value).unwrap();
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Canvas {
    buffer: String,
}

impl Canvas {
    pub fn new(viewport: BoundingBox) -> Self {
        let size = viewport.size();
        Self {
            buffer: format!(
                "<svg viewBox=\"{} {} {} {}\">",
                viewport.top_left.x, viewport.top_left.y, size.x, size.y
            ),
        }
    }

    pub fn to_svg(mut self) -> String {
        self.buffer.push_str("</svg>");
        self.buffer
    }

    fn self_closing_tag<'buf, 'tag>(
        &'buf mut self,
        tag: &'tag str,
    ) -> Tag<'buf, 'tag, SelfClosing> {
        self.buffer.push('<');
        self.buffer.push_str(tag);
        Tag {
            buffer: &mut self.buffer,
            tag_name: tag,
            _type: PhantomData,
        }
    }

    fn tag<'buf, 'tag>(&'buf mut self, tag: &'tag str) -> Tag<'buf, 'tag, Open> {
        self.buffer.push('<');
        self.buffer.push_str(tag);
        Tag {
            buffer: &mut self.buffer,
            tag_name: tag,
            _type: PhantomData,
        }
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

    fn line(&mut self, start: V2f, end: V2f, weight: f32, color: Color) {
        let mut line = self.self_closing_tag("line");
        line.attribute("x1", start.x);
        line.attribute("y1", start.y);
        line.attribute("x2", end.x);
        line.attribute("y2", end.y);
        line.attribute("stroke-width", weight);
        line.attribute("stroke", Rgba(color));
    }

    fn text(
        &mut self,
        position: V2f,
        content: std::fmt::Arguments<'_>,
        alignment: super::TextAlignment,
        color: Color,
    ) {
        let mut text = self.tag("text");
        text.attribute("x", position.x);
        text.attribute("y", position.y);
        text.attribute(
            "text-anchor",
            match alignment {
                TextAlignment::Left => "start",
                TextAlignment::Center => "middle",
                TextAlignment::Right => "end",
            },
        );
        text.attribute("dominant-baseline", "central");
        text.attribute("font-size", "13px");
        text.attribute("fill", Rgba(color));

        let mut text = text.finish_attributes();
        text.content(&content);
    }

    fn large_char(&mut self, letter: char, position: V2f, color: Color) {
        let tile_size = Self::tile_size();

        let mut text = self.tag("text");
        text.attribute("x", tile_size.x.mul_add(0.5, position.x));
        text.attribute("y", tile_size.y.mul_add(0.5, position.y));
        text.attribute("text-anchor", "middle");
        text.attribute("dominant-baseline", "central");
        text.attribute("font-size", "52px");
        text.attribute("fill", Rgba(color));

        let mut text = text.finish_attributes();
        let mut buf = [0u8; 4];
        let content = letter.encode_utf8(&mut buf);
        text.content(&content);
    }

    fn tile_size() -> V2f {
        V2f { x: 64.0, y: 64.0 }
    }

    fn thick_line_weight() -> f32 {
        2.0
    }
}
