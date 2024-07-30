//! Simple SVG immediate drawing utilities
#![cfg_attr(
    feature = "cargo-clippy",
    allow(clippy::missing_errors_doc, clippy::new_ret_no_self)
)]

use std::fmt::{self, Write};

/// Object that can be rendered as SVG
pub trait Svg {
    /// Render object as SVG
    fn to_svg<W>(&self, w: &mut W) -> fmt::Result
    where
        W: Write;
}

/// SVG renderer
pub struct ImmSvg;

/// SVG text element anchor
pub enum TextAnchor {
    /// The rendered characters are aligned such that the start of the text string is at the
    /// initial current text position
    Start,

    /// The rendered characters are aligned such that the middle of the text string is at the
    /// current text position
    Middle,

    /// The rendered characters are shifted such that the end of the resulting rendered text
    End,
}

impl TextAnchor {
    /// Get text anchor as string
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Middle => "middle",
            Self::End => "end",
        }
    }
}

/// SVG text element
pub struct Text {
    /// X position
    pub x: i32,
    /// Y position
    pub y: i32,
    /// Text to display
    pub text: String,
    /// Text anchor
    pub text_anchor: TextAnchor,
}

impl Default for Text {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            text: String::new(),
            text_anchor: TextAnchor::Start,
        }
    }
}

/// SVG circle element
pub struct Circle {
    /// X position of circle center
    pub cx: i32,

    /// Y position of circle center
    pub cy: i32,

    /// Circle radius
    pub r: u32,

    /// Stroke color
    pub stroke: String,

    /// Stroke width
    pub stroke_width: u32,

    /// Stroke color
    pub fill: String,
}

/// Custom grid element
pub struct Grid {
    /// Top left corner X position
    pub x1: i32,

    /// Top left corner Y position
    pub y1: i32,

    /// Bottom right corner X position
    pub x2: i32,

    /// Bottom right corner Y position
    pub y2: i32,

    /// Width of grid lines
    pub grid_width: u32,

    /// Width and height of each tile
    pub tile_size: u32,
}

impl ImmSvg {
    /// Create new SVG
    pub fn new<W>(
        w: &mut W,
        width: u32,
        height: u32,
        cont: impl FnOnce(&mut W) -> fmt::Result,
    ) -> fmt::Result
    where
        W: Write,
    {
        write!(w, "<svg width=\"{}\" height=\"{}\">", width, height)?;
        cont(w)?;
        write!(w, "</svg>")
    }

    /// Create [group element](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/g)
    pub fn g<W>(w: &mut W, stroke: &str, cont: impl FnOnce(&mut W) -> fmt::Result) -> fmt::Result
    where
        W: Write,
    {
        write!(w, "<g stroke=\"{}\">", stroke)?;
        cont(w)?;
        write!(w, "</g>")
    }

    /// Create [line element](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/line)
    pub fn line<W>(w: &mut W, x1: i32, y1: i32, x2: i32, y2: i32, stroke_width: u32) -> fmt::Result
    where
        W: Write,
    {
        write!(
            w,
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" style=\"stroke-width:{};\"/>",
            x1, y1, x2, y2, stroke_width
        )
    }

    /// Create [rectangle element](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/rect)
    pub fn rect<W>(w: &mut W, x: i32, y: i32, width: u32, height: u32, fill: &str) -> fmt::Result
    where
        W: Write,
    {
        write!(
            w,
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" style=\"fill:{};\"/>",
            x, y, width, height, fill,
        )
    }

    /// Create [text element](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/text)
    pub fn text<W>(w: &mut W, text: &Text) -> fmt::Result
    where
        W: Write,
    {
        write!(
            w,
            "<text text-anchor=\"{}\" x=\"{}\" y=\"{}\">{}</text>",
            text.text_anchor.as_str(),
            text.x,
            text.y,
            text.text,
        )
    }

    /// Create circle element
    pub fn circle<W>(w: &mut W, circle: &Circle) -> fmt::Result
    where
        W: Write,
    {
        write!(
            w,
            "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" stroke=\"{}\" stroke-width=\"{}\" fill=\"{}\" />",
            circle.cx, circle.cy, circle.r, circle.stroke, circle.stroke_width, circle.fill,
        )
    }

    /// Draw grid
    pub fn grid<W>(w: &mut W, grid: &Grid) -> fmt::Result
    where
        W: Write,
    {
        let offset = grid.grid_width / 2;
        Self::g(w, "black", |w| {
            for y in 0..=((grid.y2 - grid.y1) as u32 / grid.tile_size) {
                Self::line(
                    w,
                    grid.x1,
                    (y * grid.tile_size + offset) as i32,
                    grid.x2,
                    (y * grid.tile_size + offset) as i32,
                    grid.grid_width,
                )?;
            }

            for x in 0..=((grid.x2 - grid.x1) as u32 / grid.tile_size) {
                Self::line(
                    w,
                    (x * grid.tile_size + offset) as i32,
                    grid.y1,
                    (x * grid.tile_size + offset) as i32,
                    grid.y2,
                    grid.grid_width,
                )?;
            }

            Ok(())
        })
    }
}
