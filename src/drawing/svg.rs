//! Simple SVG immediate drawing utilities

use std::fmt::{self, Write};

// TODO: trait SVG
// TODO: ImmSvg
/// SVG renderer
pub struct Svg;

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
    pub fn as_str(&self) -> &'static str {
        match self {
            TextAnchor::Start => "start",
            TextAnchor::Middle => "middle",
            TextAnchor::End => "end",
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
        Text {
            x: 0,
            y: 0,
            text: String::new(),
            text_anchor: TextAnchor::Start,
        }
    }
}

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
}
