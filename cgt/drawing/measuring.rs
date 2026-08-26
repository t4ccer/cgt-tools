//! Measuring what a drawing needs without painting it

use crate::{
    drawing::{BoundingBox, Canvas, Color, Draw, Shade, TextAlignment},
    numeric::v2f::V2f,
};
use std::{
    fmt::{Arguments, Write},
    marker::PhantomData,
};

/// Sink that counts the characters written to it rather than keeping them
struct CharacterCount(usize);

impl Write for CharacterCount {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        self.0 += text.chars().count();
        Ok(())
    }
}

/// Canvas that paints nothing and instead measures where `C` would have painted it.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct MeasuringCanvas<C> {
    /// [`None`] until something is painted, because there is nowhere to put an empty box
    /// until then
    bounding_box: Option<BoundingBox>,

    max_canvas_size: Option<V2f>,

    /// `fn() -> C` rather than `C`, so that measuring for a canvas neither owns one nor
    /// inherits what it can be sent across
    canvas: PhantomData<fn() -> C>,
}

impl<C> MeasuringCanvas<C>
where
    C: Canvas,
{
    #[must_use]
    pub const fn new(max_canvas_size: Option<V2f>) -> MeasuringCanvas<C> {
        MeasuringCanvas {
            bounding_box: None,
            max_canvas_size,
            canvas: PhantomData,
        }
    }

    /// Room that everything painted so far needs
    #[must_use]
    pub fn bounding_box(&self) -> BoundingBox {
        self.bounding_box.unwrap_or(BoundingBox {
            top_left: V2f::ZERO,
            bottom_right: V2f::ZERO,
        })
    }

    /// Room that `drawing` needs on a `C` that has `max_canvas_size` to give it
    #[must_use]
    pub fn measure<D>(drawing: &D, max_canvas_size: Option<V2f>) -> BoundingBox
    where
        D: Draw,
    {
        let mut canvas = MeasuringCanvas::<C>::new(max_canvas_size);
        drawing.draw(&mut canvas);
        canvas.bounding_box()
    }

    /// Grow the box to hold the rectangle that `corner` and `opposite` span
    fn extend(&mut self, corner: V2f, opposite: V2f) {
        let grown = self.bounding_box.unwrap_or(BoundingBox {
            top_left: corner,
            bottom_right: corner,
        });
        self.bounding_box = Some(BoundingBox {
            top_left: V2f {
                x: f32::min(grown.top_left.x, f32::min(corner.x, opposite.x)),
                y: f32::min(grown.top_left.y, f32::min(corner.y, opposite.y)),
            },
            bottom_right: V2f {
                x: f32::max(grown.bottom_right.x, f32::max(corner.x, opposite.x)),
                y: f32::max(grown.bottom_right.y, f32::max(corner.y, opposite.y)),
            },
        });
    }
}

impl<C> Canvas for MeasuringCanvas<C>
where
    C: Canvas,
{
    fn rect(&mut self, position: V2f, size: V2f, _color: Color, _shade: Shade) {
        self.extend(position, position + size);
    }

    fn circle(
        &mut self,
        position: V2f,
        radius: f32,
        _fill_color: Color,
        _fill_shade: Shade,
        _stroke_width: f32,
        _stroke_color: Color,
    ) {
        // Canvases keep the outline inside the circle, so it stays within `radius` of its
        // center however thick the outline is
        let extent = V2f {
            x: radius,
            y: radius,
        };
        self.extend(position - extent, position + extent);
    }

    fn line(&mut self, start: V2f, end: V2f, weight: f32, _color: Color) {
        // A stroke straddles the line it is drawn on, reaching half of its weight to either
        // side but, the ends being square cut, no further than them
        let direction = V2f::direction(start, end);
        let extent = V2f {
            x: -direction.y * weight * 0.5,
            y: direction.x * weight * 0.5,
        };
        self.extend(start - extent, end + extent);
        self.extend(start + extent, end - extent);
    }

    fn text(
        &mut self,
        position: V2f,
        text: Arguments<'_>,
        alignment: TextAlignment,
        _color: Color,
    ) {
        // Real glyph widths need the font that only the backend has, so estimate from the
        // character count at the 0.6em that digits and punctuation average in a sans serif
        let mut characters = CharacterCount(0);
        let _ = write!(characters, "{}", text);
        let width = Self::text_size() * 0.6 * characters.0 as f32;

        let left = match alignment {
            TextAlignment::Left => position.x,
            TextAlignment::Center => width.mul_add(-0.5, position.x),
            TextAlignment::Right => position.x - width,
        };
        // Text sits on a central baseline, so it reaches as far above `position` as below
        let half_height = Self::text_size() * 0.5;
        self.extend(
            V2f {
                x: left,
                y: position.y - half_height,
            },
            V2f {
                x: left + width,
                y: position.y + half_height,
            },
        );
    }

    fn large_char(&mut self, _letter: char, position: V2f, _color: Color) {
        self.extend(position, position + Self::tile_size());
    }

    fn tile_size() -> V2f {
        C::tile_size()
    }

    fn text_size() -> f32 {
        C::text_size()
    }

    fn max_canvas_size(&self) -> Option<V2f> {
        self.max_canvas_size
    }

    fn vertex_radius() -> f32 {
        C::vertex_radius()
    }

    fn arrow_head_size() -> f32 {
        C::arrow_head_size()
    }

    fn thick_line_weight() -> f32 {
        C::thick_line_weight()
    }

    fn thin_line_weight() -> f32 {
        C::thin_line_weight()
    }

    fn tile_position(x: u8, y: u8) -> V2f {
        C::tile_position(x, y)
    }
}
