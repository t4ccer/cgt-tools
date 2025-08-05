#![allow(missing_docs)]

//! Drawing module

use crate::{graph::VertexIndex, numeric::v2f::V2f};

pub mod svg;

#[cfg(feature = "tiny_skia")]
pub mod tiny_skia;

#[cfg(feature = "imgui")]
pub mod imgui;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

    #[must_use]
    pub const fn from_hex(hex: u32) -> Color {
        Color {
            r: ((hex >> 24) & 0xff) as u8,
            g: ((hex >> 16) & 0xff) as u8,
            b: ((hex >> 8) & 0xff) as u8,
            a: (hex & 0xff) as u8,
        }
    }

    #[must_use]
    pub const fn faded(self, alpha: u8) -> Color {
        Color {
            a: ((self.a as f32) * (alpha as f32 / 255.0)) as u8,
            ..self
        }
    }
}

#[cfg(feature = "tiny_skia")]
impl From<Color> for ::tiny_skia::Color {
    fn from(color: Color) -> ::tiny_skia::Color {
        ::tiny_skia::Color::from_rgba8(color.r, color.g, color.b, color.a)
    }
}

#[cfg(feature = "imgui")]
impl From<Color> for ::imgui::ImColor32 {
    fn from(color: Color) -> ::imgui::ImColor32 {
        ::imgui::ImColor32::from_rgba(color.r, color.g, color.b, color.a)
    }
}

#[cfg(feature = "mint")]
impl From<Color> for ::mint::Vector4<f32> {
    fn from(color: Color) -> ::mint::Vector4<f32> {
        ::mint::Vector4 {
            x: color.r as f32 / 255.0,
            y: color.g as f32 / 255.0,
            z: color.b as f32 / 255.0,
            w: color.a as f32 / 255.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tile {
    Square {
        color: Color,
    },
    Circle {
        tile_color: Color,
        circle_color: Color,
    },
    Char {
        tile_color: Color,
        text_color: Color,
        letter: char,
    },
}

/// Anything that can be used for drawing
pub trait Canvas {
    fn rect(&mut self, position: V2f, size: V2f, color: Color);

    fn circle(&mut self, position: V2f, radius: f32, color: Color);

    fn line(&mut self, start: V2f, end: V2f, weight: f32, color: Color);

    fn large_char(&mut self, letter: char, position: V2f, color: Color);

    fn tile(&mut self, position: V2f, tile: Tile) {
        let tile_size = Self::tile_size();
        match tile {
            Tile::Square { color } => {
                self.rect(position, tile_size, color);
            }
            Tile::Circle {
                tile_color,
                circle_color,
            } => {
                self.rect(position, tile_size, tile_color);
                self.circle(position + tile_size * 0.5, tile_size.x * 0.4, circle_color);
            }
            Tile::Char {
                tile_color,
                text_color,
                letter,
            } => {
                self.rect(position, tile_size, tile_color);
                self.large_char(letter, position, text_color);
            }
        }
    }

    fn highlight_tile(&mut self, position: V2f, color: Color) {
        let tile_size = Self::tile_size();
        let weight = Self::thick_line_weight() * 2.0;

        self.line(
            position,
            position
                + V2f {
                    x: tile_size.x,
                    y: 0.0,
                },
            weight,
            color,
        );
        self.line(
            position,
            position
                + V2f {
                    x: 0.0,
                    y: Self::tile_size().y,
                },
            weight,
            color,
        );
        self.line(
            position
                + V2f {
                    x: tile_size.x,
                    y: 0.0,
                },
            position + tile_size,
            weight,
            color,
        );
        self.line(
            position
                + V2f {
                    x: 0.0,
                    y: tile_size.y,
                },
            position + tile_size,
            weight,
            color,
        );
    }

    fn grid(&mut self, position: V2f, columns: u32, rows: u32) {
        let cell_size = Self::tile_size();
        let grid_weight = Self::thick_line_weight();

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
            self.line(
                line_start,
                line_end,
                Self::thick_line_weight(),
                Color::BLACK,
            );
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
            self.line(
                line_start,
                line_end,
                Self::thick_line_weight(),
                Color::BLACK,
            );
        }
    }

    fn vertex(&mut self, position: V2f, color: Color, _idx: VertexIndex) {
        let radius = Self::node_radius();
        self.circle(position, radius, Color::BLACK);
        self.circle(position, radius - 1.0, color);
    }

    fn tile_size() -> V2f;

    fn node_radius() -> f32 {
        // FIXME: Remove default
        Self::tile_size().x * 0.25
    }

    fn thick_line_weight() -> f32;

    fn thin_line_weight() -> f32 {
        Self::thick_line_weight() * 0.5
    }

    fn tile_position(x: u8, y: u8) -> V2f {
        let tile_size = Self::tile_size();
        let grid_weight = Self::thick_line_weight();
        V2f {
            x: (x as f32).mul_add(tile_size.x, (x + 1) as f32 * grid_weight),
            y: (y as f32).mul_add(tile_size.y, (y + 1) as f32 * grid_weight),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct BoundingBox {
    pub top_left: V2f,
    pub bottom_right: V2f,
}

impl BoundingBox {
    pub fn size(self) -> V2f {
        self.bottom_right - self.top_left
    }
}

pub trait Draw {
    /// Paint position on existing canvas
    fn draw<C>(&self, canvas: &mut C)
    where
        C: Canvas;

    /// Minimum required canvas size to paint the whole position
    fn required_canvas<C>(&self) -> BoundingBox
    where
        C: Canvas;
}
