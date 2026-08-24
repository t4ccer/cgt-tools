pub mod layout;

use cgt::{
    drawing::{self, Color},
    graph::adjacency_matrix::directed::DirectedGraph,
    grid::vec_grid::VecGrid,
    impl_has,
    numeric::v2f::V2f,
    short::partizan::games::{amazons, col, digraph_placement, domineering, fission, snort},
};

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum Tile {
    Empty,
    Taken,
    BlueStone,
    RedStone,
    BlackStone,
}

impl Tile {
    pub const fn drawing(self) -> drawing::Tile {
        match self {
            Tile::Empty => drawing::Tile::Square {
                color: Color::LIGHT_GRAY,
            },
            Tile::Taken => drawing::Tile::Square {
                color: Color::DARK_GRAY,
            },
            Tile::BlueStone => drawing::Tile::Circle {
                tile_color: Color::LIGHT_GRAY,
                circle_color: Color::BLUE,
            },
            Tile::RedStone => drawing::Tile::Circle {
                tile_color: Color::LIGHT_GRAY,
                circle_color: Color::RED,
            },
            Tile::BlackStone => drawing::Tile::Circle {
                tile_color: Color::LIGHT_GRAY,
                circle_color: Color::DARK_GRAY,
            },
        }
    }
}

impl std::fmt::Display for Tile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Tile::Empty => write!(f, "Empty"),
            Tile::Taken => write!(f, "Taken"),
            Tile::BlueStone => write!(f, "Blue Stone"),
            Tile::RedStone => write!(f, "Red Stone"),
            Tile::BlackStone => write!(f, "Black Stone"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct UnsupportedTileError(Tile);

impl std::fmt::Display for UnsupportedTileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unsupported tile: {}", self.0)
    }
}

impl std::error::Error for UnsupportedTileError {}

/// Macro helper to define bidirectional mapping between widget tile and game tiles
macro_rules! impl_tile_map {
    (match $ty:ty {
        $($t:ident => $u:ident,)*
    }) => {
        impl From<$ty> for Tile {
            fn from(tile: $ty) -> Self {
                match tile {
                    $(<$ty>::$t => Tile::$u,)*
                }
            }
        }

        impl From<&$ty> for Tile {
            fn from(tile: &$ty) -> Self {
                match tile {
                    $(<$ty>::$t => Tile::$u,)*
                }
            }
        }

        impl TryFrom<Tile> for $ty {
            type Error = UnsupportedTileError;

            fn try_from(tile: Tile) -> Result<Self, Self::Error> {
                match tile {
                    $(Tile::$u => Ok(<$ty>::$t),)*
                    _ => Err(UnsupportedTileError(tile)),
                }
            }
        }

        impl TryFrom<&Tile> for $ty {
            type Error = UnsupportedTileError;

            fn try_from(tile: &Tile) -> Result<Self, Self::Error> {
                match tile {
                    $(Tile::$u => Ok(<$ty>::$t),)*
                    _ => Err(UnsupportedTileError(*tile)),
                }
            }
        }
    };
}

impl_tile_map! {
    match domineering::Tile {
        Empty => Empty,
        Taken => Taken,
    }
}

impl_tile_map! {
    match fission::Tile {
        Empty => Empty,
        Blocked => Taken,
        Stone => BlackStone,
    }
}

impl_tile_map! {
    match amazons::Tile {
        Empty => Empty,
        Stone => Taken, // FIXME: Stone is rendered as circle to we'll render it differently in the widget but "lessons in play" renders it as a "taken" tile
        Left => BlueStone,
        Right => RedStone,
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(tag = "type")]
pub enum GridBackendMessage {
    Initialized,
    SetGrid { grid: VecGrid<Tile> },
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(tag = "type")]
pub enum GridFrontendMessage {
    SetGrid(VecGrid<Tile>),
}

macro_rules! preset {
    (
        $(#[$flag_attr:meta])*
        $flag_vis:vis struct $flag:ident;

        $(#[$preset_attr:meta])*
        $preset_vis:vis enum $preset:ident {$(
            $(#[$variant_attr:meta])*
            $variant:ident $(= $value:literal)?,
        )*}) => {
        $(#[$preset_attr])* $preset_vis enum $preset {
            $($(#[$variant_attr])* $variant $(= $value)?,)*
        }

        impl $preset {
            pub const fn into_flag_bits(self) -> u32 {
                1 << self as u32
            }

            pub const fn from_flag_bits(bits: u32) -> Option<$preset> {
                match bits {
                    $(b if b == $preset::$variant.into_flag_bits() => Some($preset::$variant),)*
                    _ => None,
                }
            }

            pub const fn into_flag(self) -> $flag {
                $flag::from_bits_truncate(self.into_flag_bits())
            }

            pub const fn intersects(self, flags: $flag) -> bool {
                self.into_flag().intersects(flags)
            }
        }

        // TODO: Do it ourselves since we are in the macro and only use const fn union()
        bitflags::bitflags! {
            $(#[$flag_attr])* $flag_vis struct $flag: u32 {
                $(const $variant = $preset::$variant.into_flag_bits();)*
            }
        }

        impl $flag {
            pub const fn from_slice(flags: &[$flag]) -> $flag {
                // const-hack
                let mut res = $flag::empty();
                let mut i = 0;
                while i < flags.len() {
                    res = res.union(flags[i]);
                    i+= 1;
                }
                res
            }
        }
    };
}

preset! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct GridPresetFlag;

    #[derive(Debug, Clone, Copy)]
    pub enum GridPreset {
        Domineering = 1,
        Fission = 2,
        Amazons = 3,
        // TODO: Konane, SkiJumps, ToadsAndFrogs
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum VertexColor {
    White,
    Blue,
    Red,
    Green,
}

#[derive(Debug, Clone, Copy)]
pub struct UnsupportedVertexError(VertexColor);

impl std::fmt::Display for UnsupportedVertexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unsupported vertex: {}", self.0)
    }
}

impl std::error::Error for UnsupportedVertexError {}

/// Macro helper to define bidirectional mapping between widget vertex and game vertices
macro_rules! impl_vertex_map {
    (match $ty:ty {
        $($t:ident => $u:ident,)*
    }) => {
        impl From<$ty> for VertexColor {
            fn from(vertex: $ty) -> Self {
                match vertex {
                    $(<$ty>::$t => VertexColor::$u,)*
                }
            }
        }

        impl From<&$ty> for VertexColor {
            fn from(vertex: &$ty) -> Self {
                match vertex {
                    $(<$ty>::$t => VertexColor::$u,)*
                }
            }
        }

        impl TryFrom<VertexColor> for $ty {
            type Error = UnsupportedVertexError;

            fn try_from(vertex: VertexColor) -> Result<Self, Self::Error> {
                match vertex {
                    $(VertexColor::$u => Ok(<$ty>::$t),)*
                    _ => Err(UnsupportedVertexError(vertex)),
                }
            }
        }

        impl TryFrom<&VertexColor> for $ty {
            type Error = UnsupportedVertexError;

            fn try_from(vertex: &VertexColor) -> Result<Self, Self::Error> {
                match vertex {
                    $(VertexColor::$u => Ok(<$ty>::$t),)*
                    _ => Err(UnsupportedVertexError(*vertex)),
                }
            }
        }
    };
}

impl_vertex_map! {
    match snort::VertexColor {
        Empty => White,
        TintLeft => Blue,
        TintRight => Red,
    }
}

impl_vertex_map! {
    match col::VertexColor {
        Empty => White,
        TintLeft => Blue,
        TintRight => Red,
    }
}

impl_vertex_map! {
    match digraph_placement::VertexColor {
        Left => Blue,
        Right => Red,
    }
}

impl VertexColor {
    pub const fn color(self) -> Color {
        match self {
            VertexColor::White => Color::from_hex(0xf5f5f5ff),
            VertexColor::Blue => Color::BLUE,
            VertexColor::Red => Color::RED,
            VertexColor::Green => Color::from_hex(0xa6e22eff),
        }
    }
}

impl std::fmt::Display for VertexColor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VertexColor::White => write!(f, "White"),
            VertexColor::Blue => write!(f, "Blue"),
            VertexColor::Red => write!(f, "Red"),
            VertexColor::Green => write!(f, "Green"),
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    pub position: V2f,
    pub color: VertexColor,
}

impl_has!(Vertex -> position -> V2f);
impl_has!(Vertex -> color -> VertexColor);

/// Graph behind every graph widget. It is always directed - games with undirected edges
/// store each of them as a pair of opposite arcs, see [`GraphPreset::directed_edges`]
pub type WidgetGraph = DirectedGraph<Vertex>;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(tag = "type")]
pub enum GraphBackendMessage {
    Initialized,
    SetGraph { graph: WidgetGraph },
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(tag = "type")]
pub enum GraphFrontendMessage {
    SetGraph(WidgetGraph),
}

preset! {
    #[derive(Clone, Copy, Debug)]
    pub struct GraphPresetFlag;

    #[derive(Clone, Copy, Debug)]
    pub enum GraphPreset {
        Snort = 1,
        Col = 2,
        DigraphPlacement = 3,
        // TODO: BipartiteSnort
    }
}

impl GraphPreset {
    /// Whether edges of the preset's game point one way. Edges of games played on undirected
    /// graphs are stored as a pair of opposite arcs, so connecting two vertices adds both
    pub const fn directed_edges(self) -> bool {
        match self {
            GraphPreset::Snort | GraphPreset::Col => false,
            GraphPreset::DigraphPlacement => true,
        }
    }
}
