use cgt::{
    graph::adjacency_matrix::undirected::UndirectedGraph,
    grid::vec_grid::VecGrid,
    impl_has,
    numeric::v2f::V2f,
    short::partizan::games::{amazons, domineering, fission},
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

#[derive(Clone, Copy)]
pub enum GridPreset {
    Domineering = 1,
    Fission = 2,
    Amazons = 3,
}

impl GridPreset {
    pub const fn into_flag_bits(self) -> u32 {
        1 << self as u32
    }

    pub const fn from_flag_bits(bits: u32) -> Option<GridPreset> {
        match bits {
            b if b == GridPreset::Domineering.into_flag_bits() => Some(GridPreset::Domineering),
            b if b == GridPreset::Fission.into_flag_bits() => Some(GridPreset::Fission),
            b if b == GridPreset::Amazons.into_flag_bits() => Some(GridPreset::Amazons),
            _ => None,
        }
    }

    pub const fn into_flag(self) -> GridPresetFlag {
        GridPresetFlag::from_bits_truncate(self.into_flag_bits())
    }

    pub const fn intersects(self, flags: GridPresetFlag) -> bool {
        self.into_flag().intersects(flags)
    }
}

bitflags::bitflags! {
    #[derive(Clone, Copy)]
    pub struct GridPresetFlag: u32 {
        const DOMINEERING = GridPreset::Domineering.into_flag_bits();
        const FISSION = GridPreset::Fission.into_flag_bits();
        const AMAZONS = GridPreset::Amazons.into_flag_bits();
    }
}

/// Color of a graph vertex. Not tied to any game yet
#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum VertexColor {
    White,
    Blue,
    Red,
    Green,
}

impl VertexColor {
    /// Color to paint the vertex with, as `0xRRGGBB`
    pub const fn rgb(self) -> u32 {
        // TODO: Use our Color module
        match self {
            VertexColor::White => 0xf5f5f5,
            VertexColor::Blue => 0x4e4afb,
            VertexColor::Red => 0xf92672,
            VertexColor::Green => 0xa6e22e,
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

/// Graph vertex as drawn by the widget. Games will add their own data here
#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    pub position: V2f,
    pub color: VertexColor,
}

impl_has!(Vertex -> position -> V2f);
impl_has!(Vertex -> color -> VertexColor);

// TODO: Make it all generic over graph
pub type WidgetGraph = UndirectedGraph<Vertex>;

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
