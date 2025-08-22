use ::imgui::{ComboBoxFlags, Condition, Ui};
use cgt::{
    drawing::{Draw, imgui},
    grid::{FiniteGrid, Grid, vec_grid::VecGrid},
    short::partizan::games::fission::{Fission, Tile},
};
use std::str::FromStr;

use crate::{
    Details, EvalTask, GuiContext, IsCgtWindow, RawOf, Task, TitledWindow, imgui_enum,
    impl_game_window, impl_titled_window,
    widgets::{self, canonical_form::CanonicalFormWindow},
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
    editing_mode: RawOf<GridEditingMode>,
    alternating_moves: bool,
    pub details: Option<Details>,
}

impl FissionWindow {
    pub fn new() -> FissionWindow {
        FissionWindow {
            game: Fission::from_str("....|..x.|....|....").unwrap(),
            editing_mode: RawOf::new(GridEditingMode::AddStone),
            alternating_moves: true,
            details: None,
        }
    }
}

impl IsCgtWindow for TitledWindow<FissionWindow> {
    impl_titled_window!("Fission");
    impl_game_window!("Fission", EvalFission, FissionDetails);

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
                self.content
                    .editing_mode
                    .combo(ui, "Edit Mode", ComboBoxFlags::HEIGHT_LARGE);
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
                let mut canvas =
                    imgui::Canvas::new(ui, &draw_list, ctx.large_font_id, &mut self.scratch_buffer);
                self.content.game.draw(&mut canvas);
                if let Some((x, y)) = canvas.clicked_tile(self.content.game.grid()) {
                    let mut place_tile = |tile| {
                        if self.content.game.grid().get(x, y) != tile {
                            self.content.game.grid_mut().set(x, y, tile);
                            is_dirty = true;
                        }
                    };

                    match self.content.editing_mode.get() {
                        GridEditingMode::AddStone => place_tile(Tile::Stone),
                        GridEditingMode::BlockTile => place_tile(Tile::Blocked),
                        GridEditingMode::ClearTile => place_tile(Tile::Empty),
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

                widgets::game_details(
                    self.content.details.as_ref(),
                    &mut self.scratch_buffer,
                    ui,
                    &draw_list,
                    ctx.large_font_id,
                );

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
                        "Fission",
                        Task::EvalFission(EvalTask {
                            window: self.window_id,
                            game: self.content.game.clone(),
                        }),
                    );
                }
            });
    }
}
