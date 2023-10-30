//! Simple SVG immediate drawing utilities

use std::fmt::{self, Write};

/// SVG renderer
pub struct Svg;

impl Svg {
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
    pub fn text<W>(w: &mut W, x: i32, y: i32, text: &str) -> fmt::Result
    where
        W: Write,
    {
        write!(w, "<text x=\"{}\" y=\"{}\">{}</text>", x, y, text,)
    }
}
