use cgt::{
    grid::{vec_grid::VecGrid, FiniteGrid, Grid},
    numeric::v2f::V2f,
    short::partizan::{
        games::konane::{Konane, Tile},
        partizan_game::PartizanGame,
    },
};
use imgui::{ComboBoxFlags, Condition};
use std::str::FromStr;

use crate::{
    imgui_enum, impl_game_window, impl_titled_window,
    widgets::{
        self, canonical_form::CanonicalFormWindow, interactive_color, GridEditorAction, COLOR_BLUE,
        COLOR_RED, TILE_COLOR_EMPTY, TILE_COLOR_FILLED, TILE_SIZE,
    },
    Details, EvalTask, GuiContext, IsCgtWindow, RawOf, Task, TitledWindow,
};

imgui_enum! {
    GridEditingMode {
        AddBlocked, "Add Blocked Tile",
        AddBlue, "Add Blue Piece",
        AddRed, "Add Red Piece",
        ClearTile, "Clear Tile",
        MoveLeft, "Left move",
        MoveRight, "Right move",
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
enum PendingMove {
    None,
    PieceSelected { piece: (u8, u8) },
}

#[derive(Debug, Clone)]
pub struct KonaneWindow {
    game: Konane,
    editing_mode: RawOf<GridEditingMode>,
    alternating_moves: bool,
    pending_move: PendingMove,
    pub details: Option<Details>,
}

impl KonaneWindow {
    pub fn new() -> KonaneWindow {
        KonaneWindow {
            game: Konane::from_str("....|....|....").unwrap(),
            editing_mode: RawOf::new(GridEditingMode::AddBlocked),
            alternating_moves: true,
            pending_move: PendingMove::None,
            details: None,
        }
    }
}

impl IsCgtWindow for TitledWindow<KonaneWindow> {
    impl_titled_window!("Konane");
    impl_game_window!("Konane", EvalKonane, KonaneDetails);

    fn draw(&mut self, ui: &imgui::Ui, ctx: &mut GuiContext) {
        let width = self.content.game.grid().width();
        let height = self.content.game.grid().height();

        let mut new_width = width;
        let mut new_height = height;

        let mut is_dirty = false;

        ui.window(&self.title)
            .position(ui.io().mouse_pos, Condition::Appearing)
            .size([800.0, 450.0], Condition::Appearing)
            .bring_to_front_on_focus(true)
            .menu_bar(true)
            .opened(&mut self.is_open)
            .build(|| {
                let draw_list = ui.get_window_draw_list();

                if let Some(_menu_bar) = ui.begin_menu_bar() {
                    if let Some(_new_menu) = ui.begin_menu("New") {
                        if ui.menu_item("Duplicate") {
                            let w = self.content.clone();
                            ctx.new_windows
                                .push(Box::new(TitledWindow::without_title(w)));
                        };
                        if ui.menu_item("Canonical Form") {
                            if let Some(details) = self.content.details.clone() {
                                let w = CanonicalFormWindow::with_details(details);
                                ctx.new_windows
                                    .push(Box::new(TitledWindow::without_title(w)));
                            }
                        }
                    }
                }

                ui.columns(2, "Columns", true);

                let short_inputs = ui.push_item_width(200.0);
                let edit_mode_changed =
                    self.content
                        .editing_mode
                        .combo(ui, "Edit Mode", ComboBoxFlags::HEIGHT_LARGE);
                if edit_mode_changed {
                    self.content.pending_move = PendingMove::None;
                }
                short_inputs.end();

                if matches!(
                    self.content.editing_mode.get(),
                    GridEditingMode::MoveLeft | GridEditingMode::MoveRight
                ) {
                    ui.same_line();
                    ui.checkbox("Alternating", &mut self.content.alternating_moves);
                }

                widgets::grid_size_selector(ui, &mut new_width, &mut new_height);
                ui.spacing();
                let action = widgets::grid(
                    ui,
                    &draw_list,
                    self.content.game.grid(),
                    |screen_pos, (x, y), tile, draw_list| {
                        draw_list
                            .add_rect(
                                screen_pos,
                                screen_pos
                                    + V2f {
                                        x: TILE_SIZE,
                                        y: TILE_SIZE,
                                    },
                                interactive_color(TILE_COLOR_EMPTY, ui),
                            )
                            .filled(true)
                            .build();

                        if matches!(self.content.pending_move,
                                    PendingMove::PieceSelected { piece } if
                                    piece == (x, y))
                        {
                            match self.content.editing_mode.get() {
                                GridEditingMode::MoveLeft => {
                                    draw_list
                                        .add_rect(
                                            screen_pos,
                                            screen_pos
                                                + V2f {
                                                    x: TILE_SIZE,
                                                    y: TILE_SIZE,
                                                },
                                            COLOR_BLUE,
                                        )
                                        .thickness(4.0)
                                        .filled(false)
                                        .build();
                                }
                                GridEditingMode::MoveRight => {
                                    draw_list
                                        .add_rect(
                                            screen_pos,
                                            screen_pos
                                                + V2f {
                                                    x: TILE_SIZE,
                                                    y: TILE_SIZE,
                                                },
                                            COLOR_RED,
                                        )
                                        .thickness(4.0)
                                        .filled(false)
                                        .build();
                                }
                                _ => {}
                            }
                        }

                        if let Some(stone_color) = match tile {
                            Tile::Empty => None,
                            Tile::Left => Some(COLOR_BLUE),
                            Tile::Right => Some(COLOR_RED),
                            Tile::Blocked => Some(TILE_COLOR_FILLED),
                        } {
                            draw_list
                                .add_circle(
                                    screen_pos
                                        + V2f {
                                            x: TILE_SIZE * 0.5,
                                            y: TILE_SIZE * 0.5,
                                        },
                                    TILE_SIZE * 0.4,
                                    interactive_color(stone_color, ui),
                                )
                                .filled(true)
                                .build();
                        }
                    },
                );

                match action {
                    GridEditorAction::None => {}
                    GridEditorAction::Clicked { x, y } => {
                        macro_rules! place_tile {
                            ($tile:ident) => {
                                if self.content.game.grid().get(x, y) != Tile::$tile {
                                    self.content.game.grid_mut().set(x, y, Tile::$tile);
                                    is_dirty = true;
                                }
                            };
                        }

                        macro_rules! make_move {
                            ($own_tile:ident, $other_mode:ident) => {
                                match self.content.pending_move {
                                    PendingMove::None => {
                                        if matches!(
                                            self.content.game.grid().get(x, y),
                                            Tile::$own_tile
                                        ) {
                                            self.content.pending_move =
                                                PendingMove::PieceSelected { piece: (x, y) };
                                        }
                                    }
                                    PendingMove::PieceSelected { piece } => {
                                        let target = (x, y);
                                        let mut new_game = self.content.game.clone();

                                        if target.0 == piece.0 {
                                            if target.1 < piece.1 {
                                                for y in target.1..piece.1 {
                                                    new_game.grid_mut().set(
                                                        piece.0,
                                                        y,
                                                        Tile::Empty,
                                                    );
                                                }
                                            } else {
                                                for y in piece.1..target.1 {
                                                    new_game.grid_mut().set(
                                                        piece.0,
                                                        y,
                                                        Tile::Empty,
                                                    );
                                                }
                                            }
                                        } else if target.1 == piece.1 {
                                            if target.0 < piece.0 {
                                                for x in target.0..piece.0 {
                                                    new_game.grid_mut().set(
                                                        x,
                                                        piece.1,
                                                        Tile::Empty,
                                                    );
                                                }
                                            } else {
                                                for x in piece.0..target.0 {
                                                    new_game.grid_mut().set(
                                                        x,
                                                        piece.1,
                                                        Tile::Empty,
                                                    );
                                                }
                                            }
                                        }

                                        new_game.grid_mut().set(piece.0, piece.1, Tile::Empty);
                                        new_game.grid_mut().set(
                                            target.0,
                                            target.1,
                                            Tile::$own_tile,
                                        );
                                        self.content.pending_move = PendingMove::None;

                                        match Tile::$own_tile {
                                            Tile::Left => {
                                                let moves = self.content.game.left_moves();
                                                if moves.contains(&new_game) {
                                                    self.content.game = new_game;
                                                    if self.content.alternating_moves {
                                                        self.content.editing_mode = RawOf::new(
                                                            GridEditingMode::$other_mode,
                                                        );
                                                    }
                                                    is_dirty = true;
                                                }
                                            }
                                            Tile::Right => {
                                                let moves = self.content.game.right_moves();
                                                if moves.contains(&new_game) {
                                                    self.content.game = new_game;
                                                    if self.content.alternating_moves {
                                                        self.content.editing_mode = RawOf::new(
                                                            GridEditingMode::$other_mode,
                                                        );
                                                    }
                                                    is_dirty = true;
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            };
                        }

                        match self.content.editing_mode.get() {
                            GridEditingMode::AddBlocked => place_tile!(Blocked),
                            GridEditingMode::AddBlue => place_tile!(Left),
                            GridEditingMode::AddRed => place_tile!(Right),
                            GridEditingMode::ClearTile => place_tile!(Empty),
                            GridEditingMode::MoveLeft => make_move!(Left, MoveRight),
                            GridEditingMode::MoveRight => make_move!(Right, MoveLeft),
                        }
                    }
                }

                if new_width != width || new_height != height {
                    is_dirty = true;
                    if let Some(mut new_grid) = VecGrid::filled(new_width, new_height, Tile::Empty)
                    {
                        for y in 0..height.min(new_height) {
                            for x in 0..width.min(new_width) {
                                new_grid.set(x, y, self.content.game.grid().get(x, y));
                            }
                        }
                        *self.content.game.grid_mut() = new_grid;
                    }
                }

                // Section: Right of grid
                ui.next_column();

                widgets::game_details!(self, ui, draw_list);

                // SAFETY: We're fine because we're not pushing any style changes
                let pad_x = unsafe { ui.style().window_padding[0] };
                if is_dirty {
                    self.content.details = None;
                    ui.set_column_width(
                        0,
                        f32::max(
                            (widgets::TILE_SIZE + widgets::TILE_SPACING)
                                .mul_add(new_width as f32, pad_x),
                            ui.column_width(0),
                        ),
                    );
                    ctx.schedule_task(
                        "Konane",
                        Task::EvalKonane(EvalTask {
                            window: self.window_id,
                            game: self.content.game.clone(),
                        }),
                    );
                }
            });
    }
}
