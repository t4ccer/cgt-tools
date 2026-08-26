//! Canvas that can draw to PNG

// TODO: Remove unwraps

use crate::{
    drawing::{BoundingBox, Color, Rgba, Shade, Theme},
    numeric::v2f::V2f,
};
use tiny_skia;

#[derive(Debug, Clone, PartialEq)]
pub struct TinySkiaCanvas {
    offset: V2f,
    pixmap: tiny_skia::Pixmap,
    theme: Theme,
    max_canvas_size: Option<V2f>,
}

impl TinySkiaCanvas {
    pub fn new(viewport: BoundingBox) -> TinySkiaCanvas {
        let size = viewport.size();
        let offset = -viewport.top_left;
        TinySkiaCanvas {
            offset,
            pixmap: tiny_skia::Pixmap::new(size.x as u32, size.y as u32).unwrap(),
            theme: Theme::Light,
            max_canvas_size: None,
        }
    }

    #[must_use]
    pub const fn with_theme(mut self, theme: Theme) -> TinySkiaCanvas {
        self.theme = theme;
        self
    }

    /// Set the canvas size
    #[must_use]
    pub const fn with_max_canvas_size(mut self, max_canvas_size: V2f) -> TinySkiaCanvas {
        self.max_canvas_size = Some(max_canvas_size);
        self
    }

    pub fn to_png(&self) -> Vec<u8> {
        self.pixmap.encode_png().unwrap()
    }
}

impl super::Canvas for TinySkiaCanvas {
    fn rect(&mut self, position: V2f, size: V2f, color: Color, shade: Shade) {
        let position = self.offset + position;
        self.pixmap.fill_rect(
            tiny_skia::Rect::from_xywh(position.x, position.y, size.x, size.y).unwrap(),
            &paint_solid_color(self.theme.shaded(color, shade)),
            tiny_skia::Transform::identity(),
            None,
        );
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
        let position = self.offset + position;

        {
            let path =
                tiny_skia::PathBuilder::from_circle(position.x, position.y, radius - stroke_width)
                    .unwrap();
            self.pixmap.fill_path(
                &path,
                &paint_solid_color(self.theme.color(stroke_color)),
                tiny_skia::FillRule::Winding,
                tiny_skia::Transform::identity(),
                None,
            );
        }

        {
            let path = tiny_skia::PathBuilder::from_circle(position.x, position.y, radius).unwrap();
            self.pixmap.fill_path(
                &path,
                &paint_solid_color(self.theme.shaded(fill_color, fill_shade)),
                tiny_skia::FillRule::Winding,
                tiny_skia::Transform::identity(),
                None,
            );
        }
    }

    fn line(&mut self, start: V2f, end: V2f, weight: f32, color: Color) {
        let start = self.offset + start;
        let end = self.offset + end;

        // TODO: with_capacity
        let mut path = tiny_skia::PathBuilder::new();
        path.move_to(start.x, start.y);
        path.line_to(end.x, end.y);
        let path = path.finish().unwrap();
        self.pixmap.stroke_path(
            &path,
            &paint_solid_color(self.theme.color(color)),
            &tiny_skia::Stroke {
                width: weight,
                miter_limit: 4.0,
                line_cap: tiny_skia::LineCap::Butt,
                line_join: tiny_skia::LineJoin::Miter,
                dash: None,
            },
            tiny_skia::Transform::identity(),
            None,
        );
    }

    fn text(
        &mut self,
        _position: V2f,
        _text: std::fmt::Arguments<'_>,
        _alignment: super::TextAlignment,
        _color: Color,
    ) {
        // TODO: Not implemented to not crash the whole renderer
    }

    fn large_char(&mut self, _letter: char, _position: V2f, _color: Color) {
        // TODO: Not implemented to not crash the whole renderer
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

fn paint_solid_color(color: Rgba) -> tiny_skia::Paint<'static> {
    tiny_skia::Paint {
        shader: tiny_skia::Shader::SolidColor(tiny_skia::Color::from(color)),
        blend_mode: tiny_skia::BlendMode::SourceOver,
        anti_alias: false,
        force_hq_pipeline: false,
        colorspace: tiny_skia::ColorSpace::Linear,
    }
}
