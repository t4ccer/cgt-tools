#![allow(missing_docs)]

//! Drawing module

use crate::numeric::v2f::V2f;

pub mod svg;
#[cfg(feature = "tiny_skia")]
pub mod tiny_skia;

pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    #[allow(clippy::unreadable_literal)]
    pub const BLUE: Color = Color::from_hex(0x4e4afbff);

    #[allow(clippy::unreadable_literal)]
    pub const RED: Color = Color::from_hex(0xf92672ff);

    #[allow(clippy::unreadable_literal)]
    pub const BLACK: Color = Color::from_hex(0x000000ff);

    #[allow(clippy::unreadable_literal)]
    pub const LIGHT_GRAY: Color = Color::from_hex(0xccccccff);

    #[allow(clippy::unreadable_literal)]
    pub const DARK_GRAY: Color = Color::from_hex(0x444444ff);

    pub const fn from_hex(hex: u32) -> Color {
        Color {
            r: ((hex >> 24) & 0xff) as u8,
            g: ((hex >> 16) & 0xff) as u8,
            b: ((hex >> 8) & 0xff) as u8,
            a: (hex & 0xff) as u8,
        }
    }
}

#[cfg(feature = "tiny_skia")]
impl From<Color> for ::tiny_skia::Color {
    fn from(color: Color) -> ::tiny_skia::Color {
        ::tiny_skia::Color::from_rgba8(color.r, color.g, color.b, color.a)
    }
}

/// Anything that can be used for drawing
pub trait Canvas {
    fn tile(&mut self, position: V2f, color: Color);
    fn circle(&mut self, position: V2f, radius: f32, color: Color);
    // TODO: Weight?
    fn line(&mut self, start: V2f, end: V2f);
    fn grid(&mut self, position: V2f, columns: u32, rows: u32) {
        let cell_size = Self::tile_size();
        let grid_weight = Self::default_line_weight();

        for row in 0..=rows {
            let line_start = V2f {
                x: position.x,
                y: grid_weight.mul_add(
                    row as f32 + 0.5,
                    cell_size.y.mul_add(row as f32, position.y),
                ),
            };
            let line_end = V2f {
                x: grid_weight.mul_add(
                    (columns + 1) as f32,
                    cell_size.x.mul_add(columns as f32, position.x),
                ),
                y: grid_weight.mul_add(
                    row as f32 + 0.5,
                    cell_size.y.mul_add(row as f32, position.y),
                ),
            };
            self.line(line_start, line_end);
        }

        for column in 0..=columns {
            let line_start = V2f {
                x: grid_weight.mul_add(
                    column as f32 + 0.5,
                    cell_size.x.mul_add(column as f32, position.x),
                ),
                y: position.y,
            };
            let line_end = V2f {
                x: grid_weight.mul_add(
                    column as f32 + 0.5,
                    cell_size.x.mul_add(column as f32, position.x),
                ),
                y: grid_weight.mul_add(
                    (rows + 1) as f32,
                    cell_size.y.mul_add(rows as f32, position.y),
                ),
            };
            self.line(line_start, line_end);
        }
    }
    fn tile_size() -> V2f;
    fn default_line_weight() -> f32;
}
