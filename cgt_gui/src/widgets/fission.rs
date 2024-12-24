use cgt::{
    grid::{vec_grid::VecGrid, FiniteGrid, Grid},
    numeric::v2f::V2f,
    short::partizan::games::fission::{Fission, Tile},
};
use imgui::{ComboBoxFlags, Condition};
use std::str::FromStr;

use crate::{
    imgui_enum, impl_game_window, impl_titled_window,
    widgets::{
        self, canonical_form::CanonicalFormWindow, interactive_color, GridEditorAction,
        TILE_COLOR_EMPTY, TILE_COLOR_FILLED, TILE_SIZE,
    },
    DetailOptions, Details, EvalTask, GuiContext, IsCgtWindow, RawOf, Task, TitledWindow,
};

imgui_enum! {
    GridEditingMode {
        AddStone, "Add Stone",
        BlockTile, "Block Tile",
        ClearTile, "Clear Tile",
        MoveLeft, "Left move",
        MoveRight, "Right move",
    }
}

#[derive(Debug, Clone)]
pub struct FissionWindow {
    game: Fission,
    details_options: DetailOptions,
    editing_mode: RawOf<GridEditingMode>,
    alternating_moves: bool,
    pub details: Option<Details>,
}

impl FissionWindow {
    pub fn new() -> FissionWindow {
        FissionWindow {
            game: Fission::from_str("....|..x.|....|....").unwrap(),
            details_options: DetailOptions::new(),
            editing_mode: RawOf::new(GridEditingMode::AddStone),
            alternating_moves: true,
            details: None,
        }
    }
}

impl IsCgtWindow for TitledWindow<FissionWindow> {
    impl_titled_window!("Fission");
    impl_game_window!(EvalFission, FissionDetails);

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
                self.content
                    .editing_mode
                    .combo(ui, "Edit Mode", ComboBoxFlags::HEIGHT_LARGE);
                short_inputs.end();

                if matches!(
                    self.content.editing_mode.as_enum(),
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
                    |pos, _, tile, draw_list| match tile {
                        Tile::Empty => {
                            let color = interactive_color(TILE_COLOR_EMPTY, ui);
                            draw_list
                                .add_rect(
                                    pos,
                                    pos + V2f {
                                        x: TILE_SIZE,
                                        y: TILE_SIZE,
                                    },
                                    color,
                                )
                                .filled(true)
                                .build();
                        }
                        Tile::Stone => {
                            draw_list
                                .add_rect(
                                    pos,
                                    pos + V2f {
                                        x: TILE_SIZE,
                                        y: TILE_SIZE,
                                    },
                                    TILE_COLOR_EMPTY,
                                )
                                .filled(true)
                                .build();

                            let color = interactive_color(TILE_COLOR_FILLED, ui);
                            draw_list
                                .add_circle(
                                    pos + V2f {
                                        x: TILE_SIZE * 0.5,
                                        y: TILE_SIZE * 0.5,
                                    },
                                    TILE_SIZE * 0.4,
                                    color,
                                )
                                .filled(true)
                                .build();
                        }
                        Tile::Blocked => {
                            let color = interactive_color(TILE_COLOR_FILLED, ui);
                            draw_list
                                .add_rect(
                                    pos,
                                    pos + V2f {
                                        x: TILE_SIZE,
                                        y: TILE_SIZE,
                                    },
                                    color,
                                )
                                .filled(true)
                                .build();
                        }
                    },
                );

                match action {
                    GridEditorAction::None => {}
                    GridEditorAction::Clicked { x, y } => {
                        match self.content.editing_mode.as_enum() {
                            GridEditingMode::AddStone => {
                                if self.content.game.grid().get(x, y) != Tile::Stone {
                                    self.content.game.grid_mut().set(x, y, Tile::Stone);
                                    is_dirty = true;
                                }
                            }
                            GridEditingMode::BlockTile => {
                                if self.content.game.grid().get(x, y) != Tile::Blocked {
                                    self.content.game.grid_mut().set(x, y, Tile::Blocked);
                                    is_dirty = true;
                                }
                            }
                            GridEditingMode::ClearTile => {
                                if self.content.game.grid().get(x, y) != Tile::Empty {
                                    self.content.game.grid_mut().set(x, y, Tile::Empty);
                                    is_dirty = true;
                                }
                            }
                            GridEditingMode::MoveLeft => {
                                let moves = self.content.game.available_moves_left();
                                if moves.contains(&(x, y)) {
                                    self.content.game = self.content.game.move_in_left(x, y);
                                    if self.content.alternating_moves {
                                        self.content.editing_mode =
                                            RawOf::new(GridEditingMode::MoveRight);
                                    }
                                    is_dirty = true;
                                }
                            }
                            GridEditingMode::MoveRight => {
                                let moves = self.content.game.available_moves_right();
                                if moves.contains(&(x, y)) {
                                    self.content.game = self.content.game.move_in_right(x, y);
                                    if self.content.alternating_moves {
                                        self.content.editing_mode =
                                            RawOf::new(GridEditingMode::MoveLeft);
                                    }
                                    is_dirty = true;
                                }
                            }
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
                            pad_x + (widgets::TILE_SIZE + widgets::TILE_SPACING) * new_width as f32,
                            ui.column_width(0),
                        ),
                    );
                    ctx.schedule_task(Task::EvalFission(EvalTask {
                        window: self.window_id,
                        game: self.content.game.clone(),
                    }));
                }
            });
    }
}
