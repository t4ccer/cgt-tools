#![allow(missing_docs)]

//! Drawing module

use std::fmt::Arguments;

use crate::{graph::VertexIndex, numeric::v2f::V2f, short::partizan::Player};

pub mod svg;
pub mod tikz;

#[cfg(feature = "tiny_skia")]
pub mod tiny_skia;

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

    /// Scale each color channel, leaving the alpha alone. Used to darken things that the
    /// pointer is doing something to
    #[must_use]
    pub fn scaled(self, factor: f32) -> Color {
        let scale = |channel: u8| (channel as f32 * factor).round() as u8;
        Color {
            r: scale(self.r),
            g: scale(self.g),
            b: scale(self.b),
            a: self.a,
        }
    }

    /// Left is bLue, Right is Red
    pub const fn of_player(player: Player) -> Color {
        match player {
            Player::Left => Color::BLUE,
            Player::Right => Color::RED,
        }
    }
}

#[cfg(feature = "tiny_skia")]
impl From<Color> for ::tiny_skia::Color {
    fn from(color: Color) -> ::tiny_skia::Color {
        ::tiny_skia::Color::from_rgba8(color.r, color.g, color.b, color.a)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TextAlignment {
    Left,
    Center,
    Right,
}

/// Part of the canvas that the pointer can be over
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum Area {
    Rect { position: V2f, size: V2f },
    Circle { position: V2f, radius: f32 },
}

impl Area {
    #[must_use]
    pub fn contains(self, point: V2f) -> bool {
        match self {
            Area::Rect { position, size } => point.inside_rect(position, size),
            Area::Circle { position, radius } => point.inside_circle(position, radius),
        }
    }
}

/// Primary pointer button
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum Button {
    Up,

    /// Held down since it went down at that position
    Down(V2f),

    /// Went up during the current frame, having gone down at that position
    Released(V2f),
}

impl Button {
    /// Where the button went down, if it is down or has just been released
    #[must_use]
    pub const fn origin(self) -> Option<V2f> {
        match self {
            Button::Up => None,
            Button::Down(origin) | Button::Released(origin) => Some(origin),
        }
    }
}

/// Where the pointer is and what its button is doing. Image backends never have one
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Pointer {
    /// Where the pointer is, or [`None`] when it is away from the canvas
    pub position: Option<V2f>,
    pub button: Button,
}

impl Pointer {
    /// How far the pointer may travel while held down and still count as a click rather
    /// than as a drag
    pub const CLICK_SLOP: f32 = 4.0;
}

/// Pointer dragging an area of the canvas around
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Drag {
    /// Where the pointer went down, in canvas coordinates
    pub origin: V2f,

    /// Where the pointer is now, in canvas coordinates
    pub cursor: V2f,

    /// Where the dragged area was when the pointer grabbed it
    pub grabbed_at: V2f,

    /// Whether the pointer was released during this frame, ending the drag
    pub dropped: bool,
}

impl Drag {
    /// How far the pointer has travelled since it went down
    #[must_use]
    pub fn delta(self) -> V2f {
        self.cursor - self.origin
    }

    /// Where the dragged area should be now to keep the point that the pointer grabbed
    /// under it
    #[must_use]
    pub fn position(self) -> V2f {
        self.grabbed_at + self.delta()
    }
}

/// What the pointer is doing to an area of the canvas. Image backends report
/// [`Interaction::NONE`] for everything drawn on them
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Interaction {
    /// Pointer is over the area and nothing drawn on top of it
    pub hovered: bool,

    /// Button went down on the area and has not been released since
    pub pressed: bool,

    /// Where the pointer clicked the area during this frame, if it did. A click is a press
    /// and a release without travelling far enough in between to drag the area
    pub clicked: Option<V2f>,

    /// Set once the pointer holding the area travels far enough to drag it
    pub drag: Option<Drag>,
}

impl Interaction {
    /// What every area of a canvas that cannot be interacted with is doing
    pub const NONE: Interaction = Interaction {
        hovered: false,
        pressed: false,
        clicked: None,
        drag: None,
    };

    /// Shade a color to show what the pointer is doing to the thing painted with it
    #[must_use]
    pub fn shade(self, color: Color) -> Color {
        if self.pressed {
            color.scaled(0.7)
        } else if self.hovered {
            color.scaled(0.9)
        } else {
            color
        }
    }
}

/// What the pointer is doing to the elements drawn in a single pass, e.g. the tiles of a
/// grid or the vertices of a graph. At most one element can be doing each of these because
/// only the topmost element under the pointer interacts with it
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Hits<T> {
    /// Element under the pointer
    pub hovered: Option<T>,

    /// Element that was clicked during this frame
    pub clicked: Option<T>,

    /// Element that the pointer is dragging
    pub dragged: Option<(T, Drag)>,
}

impl<T> Hits<T> {
    /// Nothing has been interacted with yet
    #[must_use]
    pub const fn new() -> Hits<T> {
        Hits {
            hovered: None,
            clicked: None,
            dragged: None,
        }
    }

    /// Remember what `element` was found doing
    pub const fn record(&mut self, element: T, interaction: Interaction)
    where
        T: Copy,
    {
        if interaction.hovered {
            self.hovered = Some(element);
        }
        if interaction.clicked.is_some() {
            self.clicked = Some(element);
        }
        if let Some(drag) = interaction.drag {
            self.dragged = Some((element, drag));
        }
    }
}

/// Pointer tracking and hit testing for canvases that can be interacted with.
///
/// Which area of the canvas the pointer is over is only known once everything has been
/// drawn, because areas drawn later cover the ones drawn before them. Interactive canvases
/// therefore draw every frame twice: [`Interactions::measure`] collects the areas without
/// painting anything, and [`Interactions::paint`] paints them, reporting what the pointer
/// is doing to each one along the way.
#[derive(Debug, Clone)]
pub struct Interactions {
    pointer: Pointer,

    /// Areas registered so far during the current pass, in the order they are drawn
    areas: Vec<Area>,

    /// Index of the area under the pointer, resolved from the measuring pass
    hovered: Option<usize>,

    /// What the pointer went down on, [`None`] until the press is resolved
    press: Option<Press>,

    /// Once the pointer travels far enough it keeps dragging until it is released, even
    /// if it comes back to where it went down
    dragging: bool,

    /// The pointer is leaving the canvas and the frame being drawn is its last one
    leaving: bool,

    painting: bool,
}

/// What the pointer went down on. Resolved on the frame that the press lands and kept
/// until it is released, because a dragged area moves away from where it was pressed and
/// would slip out from under the press if it were resolved again on every frame
#[derive(Debug, Clone, Copy, PartialEq)]
struct Press {
    /// Index of the area, if the pointer went down on one at all
    area: Option<usize>,

    /// Where that area was when the pointer went down on it
    grabbed_at: V2f,
}

/// Where an area is anchored, which is what moves as the area is dragged around
const fn anchor(area: Area) -> V2f {
    match area {
        Area::Rect { position, .. } | Area::Circle { position, .. } => position,
    }
}

impl Interactions {
    #[must_use]
    pub const fn new() -> Interactions {
        Interactions {
            pointer: Pointer {
                position: None,
                button: Button::Up,
            },
            areas: Vec::new(),
            hovered: None,
            press: None,
            dragging: false,
            leaving: false,
            painting: false,
        }
    }

    #[must_use]
    pub const fn pointer(&self) -> Pointer {
        self.pointer
    }

    /// Whether the current pass is only measuring where things are. [`Canvas`]
    /// implementations must not paint anything while it is
    #[must_use]
    pub const fn is_measuring(&self) -> bool {
        !self.painting
    }

    pub fn pointer_moved(&mut self, position: V2f) {
        self.pointer.position = Some(position);
        if let Some(origin) = self.pointer.button.origin()
            && V2f::distance(origin, position) > Pointer::CLICK_SLOP
        {
            self.dragging = true;
        }
    }

    /// The pointer went away from the canvas, dropping whatever it was holding where it
    /// was last seen. The frame drawn next reports that drop and is its last one
    pub fn pointer_left(&mut self) {
        match self.pointer.button.origin() {
            Some(origin) => {
                self.pointer.button = Button::Released(origin);
                self.leaving = true;
            }
            None => *self = Interactions::new(),
        }
    }

    pub const fn pointer_pressed(&mut self, position: V2f) {
        self.pointer.position = Some(position);
        self.pointer.button = Button::Down(position);
        self.press = None;
        self.dragging = false;
    }

    /// The pointer button went up. The frame drawn next reports the click or the drop that
    /// it ends, and [`Interactions::finish`] consumes it
    pub fn pointer_released(&mut self, position: V2f) {
        self.pointer_moved(position);
        if let Some(origin) = self.pointer.button.origin() {
            self.pointer.button = Button::Released(origin);
        }
    }

    /// Start collecting the areas that the frame draws, painting nothing
    pub fn measure(&mut self) {
        self.areas.clear();
        self.painting = false;
    }

    /// Resolve what the pointer is over from the areas that [`Interactions::measure`]
    /// collected, and start painting them
    pub fn paint(&mut self) {
        let topmost = |point: V2f| self.areas.iter().rposition(|area| area.contains(point));
        self.hovered = self.pointer.position.and_then(topmost);

        // The press is resolved once, on the frame that it lands, and grabs the area it
        // landed on where that area was at the time
        if self.press.is_none()
            && let Some(origin) = self.pointer.button.origin()
        {
            let area = topmost(origin);
            self.press = Some(Press {
                area,
                grabbed_at: area.map_or(origin, |area| anchor(self.areas[area])),
            });
        }

        self.areas.clear();
        self.painting = true;
    }

    /// Finish the frame, consuming the events that it has reported
    pub fn finish(&mut self) {
        if let Button::Released(_) = self.pointer.button {
            self.pointer.button = Button::Up;
            self.press = None;
            self.dragging = false;
        }

        if self.leaving {
            *self = Interactions::new();
        }
    }

    /// Register the area about to be drawn and report what the pointer is doing to it
    pub fn interact(&mut self, area: Area) -> Interaction {
        let index = self.areas.len();
        self.areas.push(area);

        if !self.painting {
            return Interaction::NONE;
        }

        let hovered = self.hovered == Some(index);
        let Some((press, origin)) = self
            .press
            .filter(|press| press.area == Some(index))
            .zip(self.pointer.button.origin())
        else {
            return Interaction {
                hovered,
                ..Interaction::NONE
            };
        };

        let cursor = self.pointer.position.unwrap_or(origin);
        let released = matches!(self.pointer.button, Button::Released(_));
        Interaction {
            hovered,
            pressed: true,
            clicked: (!self.dragging && released).then_some(cursor),
            drag: self.dragging.then_some(Drag {
                origin,
                cursor,
                grabbed_at: press.grabbed_at,
                dropped: released,
            }),
        }
    }
}

/// Anything that can be used for drawing
pub trait Canvas {
    fn rect(&mut self, position: V2f, size: V2f, color: Color);

    fn circle(
        &mut self,
        position: V2f,
        radius: f32,
        fill_color: Color,
        stroke_width: f32,
        stroke_color: Color,
    );

    fn line(&mut self, start: V2f, end: V2f, weight: f32, color: Color);

    /// Draw a line with an arrow head at its `end`, pointing away from `start`
    fn arrow(&mut self, start: V2f, end: V2f, weight: f32, color: Color) {
        self.line(start, end, weight, color);

        let size = Self::vertex_radius() * 0.3;
        let direction = V2f::direction(start, end);

        // Both barbs sit behind the tip, offset to either side of the line
        for side in [size, -size] {
            self.line(
                end,
                V2f {
                    x: direction.y.mul_add(side, direction.x.mul_add(-size, end.x)),
                    y: direction
                        .x
                        .mul_add(-side, direction.y.mul_add(-size, end.y)),
                },
                weight,
                color,
            );
        }
    }

    fn text(&mut self, position: V2f, text: Arguments<'_>, alignment: TextAlignment, color: Color);

    fn large_char(&mut self, letter: char, position: V2f, color: Color);

    /// Register the area about to be drawn and report what the pointer is doing to it.
    /// Canvases that draw to an image have no pointer and report [`Interaction::NONE`]
    fn interact(&mut self, _area: Area) -> Interaction {
        Interaction::NONE
    }

    fn tile(&mut self, position: V2f, tile: Tile) -> Interaction {
        let tile_size = Self::tile_size();
        let interaction = self.interact(Area::Rect {
            position,
            size: tile_size,
        });

        match tile {
            Tile::Square { color } => {
                self.rect(position, tile_size, interaction.shade(color));
            }
            Tile::Circle {
                tile_color,
                circle_color,
            } => {
                self.rect(position, tile_size, interaction.shade(tile_color));
                self.circle(
                    position + tile_size * 0.5,
                    tile_size.x * 0.4,
                    interaction.shade(circle_color),
                    Self::thin_line_weight(),
                    Color::BLACK,
                );
            }
            Tile::Char {
                tile_color,
                text_color,
                letter,
            } => {
                self.rect(position, tile_size, interaction.shade(tile_color));
                self.large_char(letter, position, text_color);
            }
        }

        interaction
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

    fn vertex(&mut self, position: V2f, color: Color, _idx: VertexIndex) -> Interaction {
        let radius = Self::vertex_radius();
        let interaction = self.interact(Area::Circle { position, radius });
        self.circle(
            position,
            radius,
            interaction.shade(color),
            Self::thin_line_weight(),
            Color::BLACK,
        );
        interaction
    }

    fn tile_size() -> V2f;

    fn vertex_radius() -> f32;

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
        V2f {
            x: f32::abs(self.bottom_right.x - self.top_left.x),
            y: f32::abs(self.bottom_right.y - self.top_left.y),
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    const BACKGROUND: Area = Area::Rect {
        position: V2f::ZERO,
        size: V2f { x: 100.0, y: 100.0 },
    };

    const LOWER: Area = Area::Circle {
        position: V2f { x: 30.0, y: 50.0 },
        radius: 16.0,
    };

    /// Overlaps [`LOWER`] and is drawn on top of it
    const UPPER: Area = Area::Circle {
        position: V2f { x: 50.0, y: 50.0 },
        radius: 16.0,
    };

    /// Inside [`LOWER`] only
    const ON_LOWER: V2f = V2f { x: 20.0, y: 50.0 };

    /// Inside both circles
    const ON_BOTH: V2f = V2f { x: 40.0, y: 50.0 };

    /// Outside both circles
    const ON_BACKGROUND: V2f = V2f { x: 90.0, y: 90.0 };

    /// Draw a frame of the three areas above, bottom first, and report what the pointer did
    /// to each of them
    fn frame(interactions: &mut Interactions) -> [Interaction; 3] {
        interactions.measure();
        for area in [BACKGROUND, LOWER, UPPER] {
            interactions.interact(area);
        }

        interactions.paint();
        let painted = [BACKGROUND, LOWER, UPPER].map(|area| interactions.interact(area));

        interactions.finish();
        painted
    }

    /// Draw a frame of a single circle that has been dragged to `position`
    fn frame_of_dragged_circle(interactions: &mut Interactions, position: V2f) -> Interaction {
        let area = Area::Circle {
            position,
            radius: 16.0,
        };

        interactions.measure();
        interactions.interact(area);

        interactions.paint();
        let painted = interactions.interact(area);

        interactions.finish();
        painted
    }

    #[test]
    fn no_interactions() {
        let mut interactions = Interactions::new();
        assert_eq!(frame(&mut interactions), [Interaction::NONE; 3]);
    }

    #[test]
    fn dragging_keeps_press() {
        let mut interactions = Interactions::new();
        let mut position = V2f { x: 30.0, y: 50.0 };

        interactions.pointer_pressed(position);
        frame_of_dragged_circle(&mut interactions, position);

        // Follow the pointer for a while, far enough that the circle no longer covers the
        // point that it was pressed at
        for step in 1..=4u8 {
            let cursor = V2f {
                x: f32::from(step).mul_add(25.0, 30.0),
                y: 50.0,
            };
            interactions.pointer_moved(cursor);

            let interaction = frame_of_dragged_circle(&mut interactions, position);
            position = interaction
                .drag
                .unwrap_or_else(|| panic!("still dragging after {} steps", step))
                .position();
        }

        assert_eq!(position, V2f { x: 130.0, y: 50.0 });
    }

    #[test]
    fn only_topmost_hovered() {
        let mut interactions = Interactions::new();

        interactions.pointer_moved(ON_BOTH);
        let [background, lower, upper] = frame(&mut interactions);

        assert!(!background.hovered);
        assert!(!lower.hovered);
        assert!(upper.hovered);
    }

    #[test]
    fn hovered_background() {
        let mut interactions = Interactions::new();

        interactions.pointer_moved(ON_BACKGROUND);
        let [background, _, _] = frame(&mut interactions);

        assert!(background.hovered);
    }

    #[test]
    fn press_and_release_no_move_click() {
        let mut interactions = Interactions::new();

        interactions.pointer_pressed(ON_LOWER);
        let [_, lower, _] = frame(&mut interactions);
        assert!(lower.pressed);
        assert_eq!(lower.clicked, None);
        assert_eq!(lower.drag, None);

        interactions.pointer_released(ON_LOWER);
        let [_, lower, _] = frame(&mut interactions);
        assert_eq!(lower.clicked, Some(ON_LOWER));
        assert_eq!(lower.drag, None);

        // The click is reported once and only once
        let [_, lower, _] = frame(&mut interactions);
        assert_eq!(lower.clicked, None);
        assert!(!lower.pressed);
    }

    #[test]
    fn must_move_to_drag() {
        let mut interactions = Interactions::new();

        interactions.pointer_pressed(ON_LOWER);
        interactions.pointer_moved(V2f {
            x: ON_LOWER.x,
            y: ON_LOWER.y + Pointer::CLICK_SLOP,
        });
        let [_, lower, _] = frame(&mut interactions);
        assert_eq!(lower.drag, None, "still within the click slop");

        let dragged_to = V2f {
            x: ON_LOWER.x,
            y: ON_LOWER.y + 30.0,
        };
        interactions.pointer_moved(dragged_to);
        let [_, lower, _] = frame(&mut interactions);
        let drag = lower.drag.expect("dragging");
        assert_eq!(drag.origin, ON_LOWER);
        assert_eq!(drag.cursor, dragged_to);
        assert_eq!(drag.delta(), V2f { x: 0.0, y: 30.0 });
        assert!(!drag.dropped);
        assert_eq!(lower.clicked, None, "a drag is not a click");
    }

    #[test]
    fn drag_after_return() {
        let mut interactions = Interactions::new();

        interactions.pointer_pressed(ON_LOWER);
        interactions.pointer_moved(V2f {
            x: ON_LOWER.x,
            y: ON_LOWER.y + 30.0,
        });
        frame(&mut interactions);

        interactions.pointer_released(ON_LOWER);
        let [_, lower, _] = frame(&mut interactions);
        assert_eq!(lower.clicked, None);
        assert!(lower.drag.expect("dropping").dropped);
    }

    #[test]
    fn drag_grab_at_pointer() {
        let mut interactions = Interactions::new();

        // Grab the lower circle 8px below its center and drag it 30px to the right
        interactions.pointer_pressed(V2f { x: 30.0, y: 58.0 });
        interactions.pointer_moved(V2f { x: 60.0, y: 58.0 });
        let [_, lower, _] = frame(&mut interactions);

        let drag = lower.drag.expect("dragging");
        assert_eq!(drag.grabbed_at, V2f { x: 30.0, y: 50.0 });
        assert_eq!(drag.position(), V2f { x: 60.0, y: 50.0 });
    }

    #[test]
    fn still_pressed_after_cursor_move() {
        let mut interactions = Interactions::new();

        interactions.pointer_pressed(ON_LOWER);
        interactions.pointer_moved(ON_BACKGROUND);
        let [background, lower, _] = frame(&mut interactions);

        assert!(lower.pressed, "the press stays with what it went down on");
        assert!(
            background.hovered,
            "while the pointer is over something else"
        );
        assert!(!background.pressed);
    }

    #[test]
    fn cursor_leave_drop_hold() {
        let mut interactions = Interactions::new();

        interactions.pointer_pressed(ON_LOWER);
        interactions.pointer_moved(V2f {
            x: ON_LOWER.x,
            y: ON_LOWER.y + 30.0,
        });
        frame(&mut interactions);

        interactions.pointer_left();
        let [_, lower, _] = frame(&mut interactions);
        assert!(
            lower.drag.expect("dropping").dropped,
            "the drag ends where the pointer was last seen"
        );

        let [_, lower, _] = frame(&mut interactions);
        assert_eq!(lower, Interaction::NONE);
        assert_eq!(
            interactions.pointer(),
            Pointer {
                position: None,
                button: Button::Up
            }
        );
    }
}
