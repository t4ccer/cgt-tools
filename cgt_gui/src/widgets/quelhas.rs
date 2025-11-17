use std::ops::Deref;

use crate::{
    AccessTracker, GuiContext, IsCgtWindow, TitledWindow, imgui_enum, impl_titled_window, widgets,
};
use ::imgui::{Condition, Ui};
use cgt::{
    drawing::{Draw, imgui},
    grid::{FiniteGrid, Grid, vec_grid::VecGrid},
    misere::{
        p_free::GameForm,
        quelhas::{Quelhas, Tile},
    },
};

imgui_enum! {
    #[derive(Debug, Clone, Copy)]
    GraphEditingMode {
        SetBlue, "Set tile to blue (left)",
        SetRed, "Set tile to red (right)",
        SetNone, "Set tile to empty",
        MoveLeft, "Blue move (left)",
        MoveRight, "Red move (right)",
    }
}

enum Edit {
    Set(Tile),
    Move(Tile),
}

impl From<GraphEditingMode> for Edit {
    fn from(value: GraphEditingMode) -> Self {
        match value {
            GraphEditingMode::SetBlue => Edit::Set(Tile::Blue),
            GraphEditingMode::SetRed => Edit::Set(Tile::Red),
            GraphEditingMode::SetNone => Edit::Set(Tile::Empty),
            GraphEditingMode::MoveLeft => Edit::Move(Tile::Blue),
            GraphEditingMode::MoveRight => Edit::Move(Tile::Red),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FirstMove {
    tile: (u8, u8),
    preview_game: Quelhas<VecGrid<Tile>>,
}

#[derive(Debug, Clone)]
pub struct QuelhasWindow {
    game: AccessTracker<Quelhas<VecGrid<Tile>>>,
    editing_mode: GraphEditingMode,
    alternating_moves: bool,
    first_move: Option<FirstMove>,
    game_form: Option<GameForm>,
}

impl QuelhasWindow {
    pub fn new() -> QuelhasWindow {
        QuelhasWindow {
            game: AccessTracker::new(Quelhas::new(VecGrid::filled(10, 10, Tile::Empty).unwrap())),
            editing_mode: GraphEditingMode::MoveLeft,
            alternating_moves: true,
            first_move: None,
            game_form: None,
        }
    }
}

impl IsCgtWindow for TitledWindow<QuelhasWindow> {
    impl_titled_window!("Quelhas");

    fn initialize(&mut self, _ctx: &GuiContext) {}

    fn draw(&mut self, ui: &Ui, ctx: &mut GuiContext) {
        let width = self.content.game.grid().width();
        let height = self.content.game.grid().height();

        let mut new_width = width;
        let mut new_height = height;

        ui.window(&self.title)
            .position(ui.io().mouse_pos, Condition::Appearing)
            .size([700.0, 850.0], Condition::Appearing)
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
                    }
                }

                ui.columns(2, "Columns", true);

                let short_inputs = ui.push_item_width(200.0);
                self.content.editing_mode.combo(ui, "Edit Mode");
                if matches!(
                    self.content.editing_mode,
                    GraphEditingMode::MoveLeft | GraphEditingMode::MoveRight
                ) {
                    ui.same_line();
                    ui.checkbox("Alternating", &mut self.content.alternating_moves);
                }
                widgets::grid_size_selector(ui, &mut new_width, &mut new_height);
                if ui.button("Reset") {
                    *self.content.game =
                        Quelhas::new(VecGrid::filled(10, 10, Tile::Empty).unwrap());
                }
                short_inputs.end();
                ui.spacing();

                let mut canvas =
                    imgui::Canvas::new(ui, &draw_list, ctx.large_font_id, &mut self.scratch_buffer);

                if let Some(first_move) = &self.content.first_move {
                    first_move.preview_game.draw(&mut canvas);
                } else {
                    self.content.game.draw(&mut canvas);
                }

                if let Some(clicked_tile) = canvas.clicked_tile(self.content.game.grid()) {
                    match Edit::from(self.content.editing_mode) {
                        Edit::Set(player_tile) => {
                            self.content.game.grid_mut().set(
                                clicked_tile.0,
                                clicked_tile.1,
                                player_tile,
                            );
                        }
                        Edit::Move(player_tile) => match self.content.first_move.clone() {
                            None => {
                                let mut game = self.content.game.deref().clone();
                                game.grid_mut()
                                    .set(clicked_tile.0, clicked_tile.1, player_tile);
                                self.content.first_move = Some(FirstMove {
                                    tile: clicked_tile,
                                    preview_game: game,
                                })
                            }
                            Some(first_move) => 'outer: {
                                self.content.first_move = None;

                                if first_move.tile == clicked_tile {
                                } else if clicked_tile.0 == first_move.tile.0
                                    && matches!(player_tile, Tile::Blue)
                                {
                                    let start_y = u8::min(clicked_tile.1, first_move.tile.1);
                                    let end_y = u8::max(clicked_tile.1, first_move.tile.1);
                                    let mut new_game = self.content.game.clone();
                                    for y in start_y..=end_y {
                                        new_game.grid_mut().set(clicked_tile.0, y, player_tile);
                                        if !matches!(
                                            self.content.game.grid().get(clicked_tile.0, y),
                                            Tile::Empty
                                        ) {
                                            break 'outer;
                                        }
                                    }
                                    self.content.game = new_game;
                                    if self.content.alternating_moves {
                                        self.content.editing_mode = GraphEditingMode::MoveRight;
                                    }
                                } else if clicked_tile.1 == first_move.tile.1
                                    && matches!(player_tile, Tile::Red)
                                {
                                    let start_x = u8::min(clicked_tile.0, first_move.tile.0);
                                    let end_x = u8::max(clicked_tile.0, first_move.tile.0);
                                    let mut new_game = self.content.game.clone();
                                    for x in start_x..=end_x {
                                        new_game.grid_mut().set(x, clicked_tile.1, player_tile);
                                        if !matches!(
                                            self.content.game.grid().get(x, clicked_tile.1),
                                            Tile::Empty
                                        ) {
                                            break 'outer;
                                        }
                                    }
                                    self.content.game = new_game;
                                    if self.content.alternating_moves {
                                        self.content.editing_mode = GraphEditingMode::MoveLeft;
                                    }
                                }
                            }
                        },
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

                // SAFETY: We're fine because we're not pushing any style changes
                let pad_x = unsafe { ui.style().window_padding[0] };
                if self.content.game.clear_flag() {
                    self.content.game_form = None;
                    ui.set_column_width(
                        0,
                        f32::max(
                            (widgets::TILE_SIZE + widgets::TILE_SPACING)
                                .mul_add(new_width as f32, pad_x),
                            ui.column_width(0),
                        ),
                    );
                    // ctx.schedule_task(
                    //     "Quelhas",
                    //     Task::EvalDomineering(EvalTask {
                    //         window: self.window_id,
                    //         game: *self.content.game,
                    //     }),
                    // );
                }
            });
    }

    fn update(&mut self, _update: crate::UpdateKind) {
        todo!()
    }
}
