//! Canvas that can draw to SVG

use crate::{
    drawing::{Color, Rgba, Shade, TextAlignment, Theme},
    numeric::v2f::V2f,
};
use core::fmt::Write;
use std::fmt::Display;

struct Rgb(Rgba);

impl Display for Rgb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{rgb,255:red,{};green,{};blue,{}}}",
            self.0.r, self.0.g, self.0.b,
        )
    }
}

struct Point(V2f);

impl Display for Point {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.0.x, self.0.y)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Canvas {
    buffer: String,
    theme: Theme,
}

impl Canvas {
    pub const fn new() -> Self {
        Self {
            buffer: String::new(),
            theme: Theme::Light,
        }
    }

    #[must_use]
    pub const fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    pub fn to_tikz(&self) -> &str {
        &self.buffer
    }
}

impl crate::drawing::Canvas for Canvas {
    fn rect(&mut self, position: V2f, size: V2f, color: Color, shade: Shade) {
        write!(
            self.buffer,
            "\\draw [fill = {color}] {start} rectangle {end};",
            color = Rgb(self.theme.shaded(color, shade)),
            start = Point(position),
            end = Point(position + size),
        )
        .unwrap();
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
        write!(
            self.buffer,
            "\\draw [fill = {fill_color}, line width = {stroke_width}cm, draw = {draw_color}] {position} circle ({radius}cm);",
            fill_color = Rgb(self.theme.shaded(fill_color, fill_shade)),
            position = Point(position),
            draw_color = Rgb(self.theme.color(stroke_color)),
        )
        .unwrap();
    }

    fn line(&mut self, start: V2f, end: V2f, weight: f32, color: Color) {
        write!(
            self.buffer,
            "\\draw [line width = {weight}cm, draw={color}] {start} -- {end};",
            color = Rgb(self.theme.color(color)),
            start = Point(start),
            end = Point(end),
        )
        .unwrap();
    }

    fn text(
        &mut self,
        position: V2f,
        content: std::fmt::Arguments<'_>,
        alignment: TextAlignment,
        color: Color,
    ) {
        let align = match alignment {
            TextAlignment::Left => "left",
            TextAlignment::Center => "center",
            TextAlignment::Right => "right",
        };
        write!(
            self.buffer,
            "\\node [draw={color}, align={align}, text={color}] at {position} {{{content}}};",
            color = Rgb(self.theme.color(color)),
            position = Point(position),
        )
        .unwrap();
    }

    fn large_char(&mut self, letter: char, position: V2f, color: Color) {
        self.text(
            position,
            format_args!("{letter}"),
            TextAlignment::Left,
            color,
        );
    }

    fn vertex_radius() -> f32 {
        0.25
    }

    fn tile_size() -> V2f {
        V2f { x: 1.0, y: 1.0 }
    }

    /// LaTeX picks the font, so this is only the 10pt of a standard document expressed in
    /// the centimetres that every other tikz length is in
    fn text_size() -> f32 {
        0.35
    }

    /// A tikz picture is sized by whatever LaTeX lays it out next to, not here
    fn max_canvas_size(&self) -> Option<V2f> {
        None
    }

    fn thick_line_weight() -> f32 {
        0.05
    }
}
