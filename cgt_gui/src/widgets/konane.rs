use crate::{
    AccessTracker, Details, EvalTask, GuiContext, IsCgtWindow, Task, TitledWindow, imgui_enum,
    impl_game_window, impl_titled_window,
    widgets::{self, canonical_form::CanonicalFormWindow},
};
use ::imgui::{Condition, Ui};
use cgt::{
    drawing::{Canvas, Color, Draw, imgui},
    grid::{FiniteGrid, Grid, vec_grid::VecGrid},
    short::partizan::{
        Player,
        games::konane::{Konane, Tile},
        partizan_game::PartizanGame,
    },
};
use std::{ops::Deref, str::FromStr};

imgui_enum! {
    #[derive(Debug, Clone, Copy)]
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
enum Edit {
    Place(Tile),
    Move(Player),
}

impl From<GridEditingMode> for Edit {
    fn from(mode: GridEditingMode) -> Self {
        match mode {
            GridEditingMode::AddBlocked => Edit::Place(Tile::Blocked),
            GridEditingMode::AddBlue => Edit::Place(Tile::Left),
            GridEditingMode::AddRed => Edit::Place(Tile::Right),
            GridEditingMode::ClearTile => Edit::Place(Tile::Empty),
            GridEditingMode::MoveLeft => Edit::Move(Player::Left),
            GridEditingMode::MoveRight => Edit::Move(Player::Right),
        }
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
enum PendingMove {
    None,
    PieceSelected { piece: (u8, u8) },
}

#[derive(Debug, Clone)]
pub struct KonaneWindow {
    game: AccessTracker<Konane>,
    editing_mode: GridEditingMode,
    alternating_moves: bool,
    pending_move: PendingMove,
    pub details: Option<Details>,
}

impl KonaneWindow {
    pub fn new() -> KonaneWindow {
        KonaneWindow {
            game: AccessTracker::new(Konane::from_str("....|....|....").unwrap()),
            editing_mode: GridEditingMode::AddBlocked,
            alternating_moves: true,
            pending_move: PendingMove::None,
            details: None,
        }
    }
}

impl IsCgtWindow for TitledWindow<KonaneWindow> {
    impl_titled_window!("Konane");
    impl_game_window!("Konane", EvalKonane, KonaneDetails);

    fn draw(&mut self, ui: &Ui, ctx: &mut GuiContext) {
        let width = self.content.game.grid().width();
        let height = self.content.game.grid().height();

        let mut new_width = width;
        let mut new_height = height;

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
                        }
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
                let edit_mode_changed = self.content.editing_mode.combo(ui, "Edit Mode");
                if edit_mode_changed {
                    self.content.pending_move = PendingMove::None;
                }
                short_inputs.end();

                if matches!(
                    self.content.editing_mode,
                    GridEditingMode::MoveLeft | GridEditingMode::MoveRight
                ) {
                    ui.same_line();
                    ui.checkbox("Alternating", &mut self.content.alternating_moves);
                }

                widgets::grid_size_selector(ui, &mut new_width, &mut new_height);
                ui.spacing();

                let mut canvas =
                    imgui::Canvas::new(ui, &draw_list, ctx.large_font_id, &mut self.scratch_buffer);
                self.content.game.draw(&mut canvas);
                if let Some((x, y)) = canvas.clicked_tile(self.content.game.grid()) {
                    match Edit::from(self.content.editing_mode) {
                        Edit::Place(tile) => {
                            if self.content.game.grid().get(x, y) != tile {
                                self.content.game.grid_mut().set(x, y, tile);
                            }
                        }
                        Edit::Move(player) => match self.content.pending_move {
                            PendingMove::None => {
                                if self.content.game.grid().get(x, y) == Tile::from(player) {
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
                                            new_game.grid_mut().set(piece.0, y, Tile::Empty);
                                        }
                                    } else {
                                        for y in piece.1..target.1 {
                                            new_game.grid_mut().set(piece.0, y, Tile::Empty);
                                        }
                                    }
                                } else if target.1 == piece.1 {
                                    if target.0 < piece.0 {
                                        for x in target.0..piece.0 {
                                            new_game.grid_mut().set(x, piece.1, Tile::Empty);
                                        }
                                    } else {
                                        for x in piece.0..target.0 {
                                            new_game.grid_mut().set(x, piece.1, Tile::Empty);
                                        }
                                    }
                                }

                                new_game.grid_mut().set(piece.0, piece.1, Tile::Empty);
                                new_game
                                    .grid_mut()
                                    .set(target.0, target.1, Tile::from(player));
                                self.content.pending_move = PendingMove::None;

                                let other_mode = match player {
                                    Player::Left => GridEditingMode::AddRed,
                                    Player::Right => GridEditingMode::AddBlue,
                                };

                                match player {
                                    Player::Left => {
                                        let moves = self.content.game.left_moves();
                                        if moves.contains(&new_game) {
                                            self.content.game = new_game;
                                            if self.content.alternating_moves {
                                                self.content.editing_mode = other_mode;
                                            }
                                        }
                                    }
                                    Player::Right => {
                                        let moves = self.content.game.right_moves();
                                        if moves.contains(&new_game) {
                                            self.content.game = new_game;
                                            if self.content.alternating_moves {
                                                self.content.editing_mode = other_mode;
                                            }
                                        }
                                    }
                                }
                            }
                        },
                    }
                }

                match self.content.pending_move {
                    PendingMove::None => {}
                    PendingMove::PieceSelected { piece } => {
                        canvas.highlight_tile(
                            imgui::Canvas::tile_position(piece.0, piece.1),
                            if matches!(self.content.editing_mode, GridEditingMode::MoveLeft) {
                                Color::BLUE
                            } else {
                                Color::RED
                            },
                        );
                    }
                }

                if new_width != width || new_height != height {
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

                widgets::game_details(
                    self.content.details.as_ref(),
                    &mut self.scratch_buffer,
                    ui,
                    &draw_list,
                    ctx.large_font_id,
                );

                // SAFETY: We're fine because we're not pushing any style changes
                let pad_x = unsafe { ui.style().window_padding[0] };
                if self.content.game.clear_flag() {
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
                            game: self.content.game.deref().clone(),
                        }),
                    );
                }
            });
    }
}
