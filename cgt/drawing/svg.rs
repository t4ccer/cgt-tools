//! Canvas that can draw to SVG

use crate::{
    drawing::{BoundingBox, Color, Rgba, Shade, TextAlignment, Theme},
    numeric::v2f::V2f,
};
use core::fmt::Write;
use std::{fmt::Display, marker::PhantomData, mem::ManuallyDrop};

struct Css(Rgba);

impl Display for Css {
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

/// Custom property that [`Canvas::palette`] defines for `color` in both themes
const fn variable(color: Color) -> &'static str {
    match color {
        Color::Primary => "--cgt-primary",
        Color::Secondary => "--cgt-secondary",
        Color::Surface => "--cgt-surface",
        Color::Background => "--cgt-background",
        Color::Blue => "--cgt-blue",
        Color::Red => "--cgt-red",
        Color::Green => "--cgt-green",
    }
}

/// What a [`Color`] is painted with: the concrete color of the canvas theme, or the custom
/// property that follows whichever theme the viewer is using
#[derive(Clone, Copy)]
enum Paint {
    Fixed(Rgba),
    Themed { color: Color, shade: Shade },
}

impl Display for Paint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Paint::Fixed(color) => write!(f, "{}", Css(color)),
            Paint::Themed {
                color,
                shade: Shade::Plain,
            } => write!(f, "var({})", variable(color)),
            Paint::Themed { color, shade } => write!(
                f,
                "color-mix(in srgb, var({}) {}%, var({}))",
                variable(color),
                shade.amount().mul_add(-100.0, 100.0),
                variable(Color::Primary),
            ),
        }
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
        // HACK: We need to re-tag without running drop code
        // SAFETY: self.buffer is a valid `&'buf mut String`
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

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Canvas {
    buffer: String,

    /// Theme to paint with, or [`None`] to leave the choice to whoever looks at the image
    theme: Option<Theme>,

    max_canvas_size: Option<V2f>,
}

impl Canvas {
    /// Both palettes are written into every image, and a `<style>` element inside an inline
    /// SVG applies to the whole page it is embedded in, so the rules have to be scoped to
    /// the images this canvas produces. Every one of them defines the same properties, so
    /// they can all share the one class
    const CLASS: &'static str = "cgt-canvas";

    pub fn new(viewport: BoundingBox) -> Self {
        let size = viewport.size();
        Self {
            buffer: format!(
                "<svg xmlns=\"http://www.w3.org/2000/svg\" class=\"{}\" \
                 viewBox=\"{} {} {} {}\" width=\"{}\" height=\"{}\">",
                Canvas::CLASS,
                viewport.top_left.x,
                viewport.top_left.y,
                size.x,
                size.y,
                viewport.size().x,
                viewport.size().y,
            ),
            theme: None,
            max_canvas_size: None,
        }
    }

    /// Paint with `theme` instead of following the color scheme of whoever looks at the
    /// image
    #[must_use]
    pub const fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Set the canvas size
    #[must_use]
    pub const fn with_max_canvas_size(mut self, max_canvas_size: V2f) -> Self {
        self.max_canvas_size = Some(max_canvas_size);
        self
    }

    pub fn to_svg(mut self) -> String {
        if self.theme.is_none() {
            self.palette();
        }
        self.buffer.push_str("</svg>");
        self.buffer
    }

    fn paint(&self, color: Color, shade: Shade) -> Paint {
        self.theme.map_or(Paint::Themed { color, shade }, |theme| {
            Paint::Fixed(theme.shaded(color, shade))
        })
    }

    /// Define every color of both themes, so that the image follows the color scheme of
    /// whoever looks at it
    fn palette(&mut self) {
        write!(self.buffer, "<style>svg.{}{{", Canvas::CLASS).unwrap();
        for color in Color::ALL {
            write!(
                self.buffer,
                "{}:{};",
                variable(color),
                Css(Theme::Light.color(color))
            )
            .unwrap();
        }

        write!(
            self.buffer,
            "}}@media (prefers-color-scheme: dark){{svg.{}{{",
            Canvas::CLASS
        )
        .unwrap();
        for color in Color::ALL {
            let dark = Theme::Dark.color(color);
            if dark != Theme::Light.color(color) {
                write!(self.buffer, "{}:{};", variable(color), Css(dark)).unwrap();
            }
        }
        self.buffer.push_str("}}</style>");
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
    fn rect(&mut self, position: V2f, size: V2f, color: Color, shade: Shade) {
        let color = self.paint(color, shade);
        let mut rect = self.self_closing_tag("rect");
        rect.attribute("x", position.x);
        rect.attribute("y", position.y);
        rect.attribute("width", size.x);
        rect.attribute("height", size.y);
        rect.attribute("style", format_args!("fill:{}", color));
    }

    fn circle(
        &mut self,
        position: V2f,
        radius: f32,
        fill_color: Color,
        fill_shade: Shade,
        stroke_width: f32,
        stroke_color: Color,
    ) {
        let fill_color = self.paint(fill_color, fill_shade);
        let stroke_color = self.paint(stroke_color, Shade::Plain);
        let mut circle = self.self_closing_tag("circle");
        circle.attribute("cx", position.x);
        circle.attribute("cy", position.y);
        // A stroke straddles the path it is drawn on, so the path has to come in by half of
        // it for the circle to stay within `radius` of its center
        circle.attribute("r", f32::max(stroke_width.mul_add(-0.5, radius), 0.0));
        circle.attribute("stroke-width", stroke_width);
        circle.attribute(
            "style",
            format_args!("fill:{};stroke:{}", fill_color, stroke_color),
        );
    }

    fn line(&mut self, start: V2f, end: V2f, weight: f32, color: Color) {
        let color = self.paint(color, Shade::Plain);
        let mut line = self.self_closing_tag("line");
        line.attribute("x1", start.x);
        line.attribute("y1", start.y);
        line.attribute("x2", end.x);
        line.attribute("y2", end.y);
        line.attribute("stroke-width", weight);
        line.attribute("style", format_args!("stroke:{}", color));
    }

    fn text(
        &mut self,
        position: V2f,
        content: std::fmt::Arguments<'_>,
        alignment: super::TextAlignment,
        color: Color,
    ) {
        let color = self.paint(color, Shade::Plain);
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
        text.attribute("font-size", format_args!("{}px", Self::text_size()));
        text.attribute("style", format_args!("fill:{}", color));

        let mut text = text.finish_attributes();
        text.content(&content);
    }

    fn large_char(&mut self, letter: char, position: V2f, color: Color) {
        let tile_size = Self::tile_size();

        let color = self.paint(color, Shade::Plain);
        let mut text = self.tag("text");
        text.attribute("x", tile_size.x.mul_add(0.5, position.x));
        text.attribute("y", tile_size.y.mul_add(0.5, position.y));
        text.attribute("text-anchor", "middle");
        text.attribute("dominant-baseline", "central");
        text.attribute("font-size", "52px");
        text.attribute("style", format_args!("fill:{}", color));

        let mut text = text.finish_attributes();
        let mut buf = [0u8; 4];
        let content = letter.encode_utf8(&mut buf);
        text.content(&content);
    }

    fn tile_size() -> V2f {
        V2f { x: 64.0, y: 64.0 }
    }

    fn text_size() -> f32 {
        13.0
    }

    fn max_canvas_size(&self) -> Option<V2f> {
        self.max_canvas_size
    }

    fn thick_line_weight() -> f32 {
        2.0
    }

    fn vertex_radius() -> f32 {
        16.0
    }
}
