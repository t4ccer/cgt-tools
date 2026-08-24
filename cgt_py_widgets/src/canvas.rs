//! Canvas that paints on a HTML `<canvas>` element and reports what the mouse is doing to
//! whatever is painted on it

use cgt::{
    drawing::{Area, Canvas, Color, Interaction, Interactions, TextAlignment},
    numeric::v2f::V2f,
};
use core::f64;
use web_sys::CanvasRenderingContext2d;

/// The mouse state lives in the widget rather than here because a canvas is created for
/// every frame, while what the mouse is doing spans many of them
pub(crate) struct HtmlCanvas<'a> {
    context: CanvasRenderingContext2d,
    interactions: &'a mut Interactions,
}

fn css(color: Color) -> String {
    format!(
        "rgba({},{},{},{})",
        color.r,
        color.g,
        color.b,
        color.a as f32 / 255.0
    )
}

impl<'a> HtmlCanvas<'a> {
    pub(crate) const fn new(
        context: CanvasRenderingContext2d,
        interactions: &'a mut Interactions,
    ) -> HtmlCanvas<'a> {
        HtmlCanvas {
            context,
            interactions,
        }
    }

    /// Paint a single frame and report what the mouse did to it.
    ///
    /// `draw` runs twice: once to collect where everything is going to be painted, so that
    /// the mouse can be tested against it, and once to actually paint it. Only the second
    /// run reports interactions, so `draw` must build whatever it returns from scratch
    pub(crate) fn frame<T>(&mut self, mut draw: impl FnMut(&mut HtmlCanvas<'a>) -> T) -> T {
        self.interactions.measure();
        draw(self);

        self.interactions.paint();
        let result = draw(self);

        self.interactions.finish();
        result
    }

    /// Nothing is painted during the measuring run of [`HtmlCanvas::frame`]
    fn is_measuring(&self) -> bool {
        self.interactions.is_measuring()
    }

    fn write_text(&mut self, text: &str, position: V2f, font: &str, align: &str, color: Color) {
        self.context.set_font(font);
        self.context.set_text_align(align);
        self.context.set_text_baseline("middle");
        self.context.set_fill_style_str(&css(color));
        // Fails only if the text cannot be laid out, in which case there is nothing to draw
        let _ = self
            .context
            .fill_text(text, position.x as f64, position.y as f64);
    }
}

impl Canvas for HtmlCanvas<'_> {
    fn rect(&mut self, position: V2f, size: V2f, color: Color) {
        if self.is_measuring() {
            return;
        }

        self.context.set_fill_style_str(&css(color));
        self.context.fill_rect(
            position.x as f64,
            position.y as f64,
            size.x as f64,
            size.y as f64,
        );
    }

    fn circle(
        &mut self,
        position: V2f,
        radius: f32,
        fill_color: Color,
        stroke_width: f32,
        stroke_color: Color,
    ) {
        if self.is_measuring() {
            return;
        }

        self.context.begin_path();
        // Fails only on a negative radius
        let _ = self.context.arc(
            position.x as f64,
            position.y as f64,
            radius as f64,
            0.0,
            2.0 * f64::consts::PI,
        );
        self.context.set_fill_style_str(&css(fill_color));
        self.context.fill();
        self.context.set_line_width(stroke_width as f64);
        self.context.set_stroke_style_str(&css(stroke_color));
        self.context.stroke();
    }

    fn line(&mut self, start: V2f, end: V2f, weight: f32, color: Color) {
        if self.is_measuring() {
            return;
        }

        self.context.begin_path();
        self.context.move_to(start.x as f64, start.y as f64);
        self.context.line_to(end.x as f64, end.y as f64);
        self.context.set_line_width(weight as f64);
        self.context.set_stroke_style_str(&css(color));
        self.context.stroke();
    }

    fn text(
        &mut self,
        position: V2f,
        text: std::fmt::Arguments<'_>,
        alignment: TextAlignment,
        color: Color,
    ) {
        if self.is_measuring() {
            return;
        }

        let align = match alignment {
            TextAlignment::Left => "left",
            TextAlignment::Center => "center",
            TextAlignment::Right => "right",
        };
        self.write_text(&text.to_string(), position, "13px sans-serif", align, color);
    }

    fn large_char(&mut self, letter: char, position: V2f, color: Color) {
        if self.is_measuring() {
            return;
        }

        let mut buffer = [0u8; 4];
        let text = letter.encode_utf8(&mut buffer).to_string();
        let center = position + Self::tile_size() * 0.5;
        self.write_text(&text, center, "52px sans-serif", "center", color);
    }

    fn interact(&mut self, area: Area) -> Interaction {
        self.interactions.interact(area)
    }

    fn tile_size() -> V2f {
        V2f { x: 64.0, y: 64.0 }
    }

    fn thick_line_weight() -> f32 {
        2.0
    }

    fn vertex_radius() -> f32 {
        16.0
    }
}
