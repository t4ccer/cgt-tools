use ::imgui::{ComboBoxFlags, Condition, Ui};
use cgt::{
    drawing::{imgui, Canvas, Color},
    grid::{vec_grid::VecGrid, FiniteGrid, Grid},
    short::partizan::games::amazons::{Amazons, Tile},
};
use std::str::FromStr;

use crate::{
    imgui_enum, impl_game_window, impl_titled_window,
    widgets::{self, canonical_form::CanonicalFormWindow},
    Details, EvalTask, GuiContext, IsCgtWindow, RawOf, Task, TitledWindow,
};

imgui_enum! {
    GridEditingMode {
        AddStone, "Add Stone",
        AddBlueQueen, "Add Blue Queen",
        AddRedQueen, "Add Red Queen",
        ClearTile, "Clear Tile",
        MoveLeft, "Left move",
        MoveRight, "Right move",
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
enum PendingMove {
    None,
    AmazonSelected { amazon: (u8, u8) },
    AmazonTargetSelected { amazon: (u8, u8), target: (u8, u8) },
}

#[derive(Debug, Clone)]
pub struct AmazonsWindow {
    game: Amazons,
    editing_mode: RawOf<GridEditingMode>,
    alternating_moves: bool,
    pending_move: PendingMove,
    pub details: Option<Details>,
}

impl AmazonsWindow {
    pub fn new() -> AmazonsWindow {
        AmazonsWindow {
            game: Amazons::from_str("x..#|....|.#.o").unwrap(),
            editing_mode: RawOf::new(GridEditingMode::AddStone),
            alternating_moves: true,
            pending_move: PendingMove::None,
            details: None,
        }
    }
}

impl IsCgtWindow for TitledWindow<AmazonsWindow> {
    impl_titled_window!("Amazons");
    impl_game_window!("Amazons", EvalAmazons, AmazonsDetails);

    fn draw(&mut self, ui: &Ui, ctx: &mut GuiContext) {
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

                let mut canvas = imgui::Canvas::new(ui, &draw_list);
                self.content.game.draw(&mut canvas);
                if let Some((x, y)) = canvas.clicked_tile(self.content.game.grid()) {
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
                                    if matches!(self.content.game.grid().get(x, y), Tile::$own_tile)
                                    {
                                        self.content.pending_move =
                                            PendingMove::AmazonSelected { amazon: (x, y) };
                                    }
                                }
                                PendingMove::AmazonSelected { amazon } => {
                                    if matches!(self.content.game.grid().get(x, y), Tile::Empty) {
                                        self.content.pending_move =
                                            PendingMove::AmazonTargetSelected {
                                                amazon,
                                                target: (x, y),
                                            };
                                    }
                                }
                                PendingMove::AmazonTargetSelected { amazon, target } => {
                                    let stone_target = (x, y);
                                    let mut new_game = self.content.game.clone();
                                    new_game.grid_mut().set(amazon.0, amazon.1, Tile::Empty);
                                    new_game.grid_mut().set(target.0, target.1, Tile::$own_tile);
                                    new_game.grid_mut().set(
                                        stone_target.0,
                                        stone_target.1,
                                        Tile::Stone,
                                    );
                                    self.content.pending_move = PendingMove::None;
                                    let moves = self.content.game.moves_for(Tile::$own_tile, false);
                                    if moves.contains(&new_game) {
                                        self.content.game = new_game;
                                        if self.content.alternating_moves {
                                            self.content.editing_mode =
                                                RawOf::new(GridEditingMode::$other_mode);
                                        }
                                        is_dirty = true;
                                    }
                                }
                            }
                        };
                    }

                    match self.content.editing_mode.get() {
                        GridEditingMode::AddStone => place_tile!(Stone),
                        GridEditingMode::AddBlueQueen => place_tile!(Left),
                        GridEditingMode::AddRedQueen => place_tile!(Right),
                        GridEditingMode::ClearTile => place_tile!(Empty),
                        GridEditingMode::MoveLeft => make_move!(Left, MoveRight),
                        GridEditingMode::MoveRight => make_move!(Right, MoveLeft),
                    }
                };

                let highlight_color =
                    if matches!(self.content.editing_mode.get(), GridEditingMode::MoveLeft) {
                        Color::BLUE
                    } else {
                        Color::RED
                    };
                match self.content.pending_move {
                    PendingMove::None => {}
                    PendingMove::AmazonSelected { amazon } => {
                        canvas.highlight_tile(
                            imgui::Canvas::tile_position(amazon.0, amazon.1),
                            highlight_color,
                        );
                    }
                    PendingMove::AmazonTargetSelected { amazon, target } => {
                        canvas.highlight_tile(
                            imgui::Canvas::tile_position(amazon.0, amazon.1),
                            highlight_color,
                        );
                        canvas.highlight_tile(
                            imgui::Canvas::tile_position(target.0, target.1),
                            highlight_color,
                        );
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
                        "Amazons",
                        Task::EvalAmazons(EvalTask {
                            window: self.window_id,
                            game: self.content.game.clone(),
                        }),
                    );
                }
            });
    }
}
