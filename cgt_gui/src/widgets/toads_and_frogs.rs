use crate::{
    imgui_enum, impl_game_window, impl_titled_window,
    widgets::{self, canonical_form::CanonicalFormWindow},
    Details, EvalTask, GuiContext, IsCgtWindow, RawOf, Task, TitledWindow,
};
use ::imgui::{ComboBoxFlags, Condition, Ui};
use cgt::{
    drawing::{imgui, Draw},
    short::partizan::games::toads_and_frogs::{Tile, ToadsAndFrogs},
};
use std::str::FromStr;

imgui_enum! {
    GridEditingMode {
        PlaceToad, "Place Toad",
        PlaceFrog, "Place Frog",
        ClearTile, "Clear Tile",
        MoveLeft, "Left move",
        MoveRight, "Right move",
    }
}

#[derive(Debug, Clone)]
pub struct ToadsAndFrogsWindow {
    game: ToadsAndFrogs,
    editing_mode: RawOf<GridEditingMode>,
    alternating_moves: bool,
    pub details: Option<Details>,
}

impl ToadsAndFrogsWindow {
    pub fn new() -> ToadsAndFrogsWindow {
        ToadsAndFrogsWindow {
            game: ToadsAndFrogs::from_str("T.TF.").unwrap(),
            editing_mode: RawOf::new(GridEditingMode::ClearTile),
            alternating_moves: true,
            details: None,
        }
    }
}

impl IsCgtWindow for TitledWindow<ToadsAndFrogsWindow> {
    impl_titled_window!("Toads and Frogs");
    impl_game_window!("Toads and Frogs", EvalToadsAndFrogs, ToadsAndFrogsDetails);

    fn draw(&mut self, ui: &Ui, ctx: &mut GuiContext) {
        let width = self.content.game.row().len();

        let mut new_width = width;

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
                    self.content.editing_mode.get(),
                    GridEditingMode::MoveLeft | GridEditingMode::MoveRight
                ) {
                    ui.same_line();
                    ui.checkbox("Alternating", &mut self.content.alternating_moves);
                }

                let short_inputs = ui.push_item_width(100.0);
                ui.input_scalar("Width", &mut new_width).step(1).build();
                short_inputs.end();

                ui.spacing();

                let mut canvas =
                    imgui::Canvas::new(ui, &draw_list, ctx.large_font_id, &mut self.scratch_buffer);
                self.content.game.draw(&mut canvas);
                if let Some((x, _)) = canvas.clicked_tile(&self.content.game.grid()) {
                    let grid_x = x as usize;

                    let mut place_tile = |tile| {
                        if self.content.game.row()[grid_x] != tile {
                            self.content.game.row_mut()[grid_x] = tile;
                            is_dirty = true;
                        }
                    };

                    match self.content.editing_mode.get() {
                        GridEditingMode::PlaceToad => place_tile(Tile::Toad),
                        GridEditingMode::PlaceFrog => place_tile(Tile::Frog),
                        GridEditingMode::ClearTile => place_tile(Tile::Empty),
                        GridEditingMode::MoveLeft => {
                            // TODO: De-duplicate game logic, introduce abstract move associated
                            // type to each game
                            if matches!(self.content.game.row()[grid_x], Tile::Toad) {
                                if grid_x + 1 < width
                                    && matches!(self.content.game.row()[grid_x + 1], Tile::Empty)
                                {
                                    self.content.game.row_mut()[grid_x] = Tile::Empty;
                                    self.content.game.row_mut()[grid_x + 1] = Tile::Toad;

                                    if self.content.alternating_moves {
                                        self.content.editing_mode =
                                            RawOf::new(GridEditingMode::MoveRight);
                                    }

                                    is_dirty = true;
                                } else if grid_x + 2 < width
                                    && matches!(self.content.game.row()[grid_x + 1], Tile::Frog)
                                    && matches!(self.content.game.row()[grid_x + 2], Tile::Empty)
                                {
                                    self.content.game.row_mut()[grid_x] = Tile::Empty;
                                    self.content.game.row_mut()[grid_x + 2] = Tile::Toad;

                                    if self.content.alternating_moves {
                                        self.content.editing_mode =
                                            RawOf::new(GridEditingMode::MoveRight);
                                    }

                                    is_dirty = true;
                                }
                            }
                        }
                        GridEditingMode::MoveRight => {
                            if matches!(self.content.game.row()[grid_x], Tile::Frog) {
                                if grid_x >= 1
                                    && matches!(self.content.game.row()[grid_x - 1], Tile::Empty)
                                {
                                    self.content.game.row_mut()[grid_x] = Tile::Empty;
                                    self.content.game.row_mut()[grid_x - 1] = Tile::Frog;

                                    if self.content.alternating_moves {
                                        self.content.editing_mode =
                                            RawOf::new(GridEditingMode::MoveLeft);
                                    }

                                    is_dirty = true;
                                } else if grid_x >= 2
                                    && matches!(self.content.game.row()[grid_x - 1], Tile::Toad)
                                    && matches!(self.content.game.row()[grid_x - 2], Tile::Empty)
                                {
                                    self.content.game.row_mut()[grid_x] = Tile::Empty;
                                    self.content.game.row_mut()[grid_x - 2] = Tile::Frog;

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

                // Section: Right of grid
                ui.next_column();

                widgets::game_details(
                    self.content.details.as_ref(),
                    &mut self.scratch_buffer,
                    ui,
                    &draw_list,
                    ctx.large_font_id,
                );

                if new_width > width {
                    self.content.game.row_mut().push(Tile::Empty);
                    is_dirty = true;
                }
                if new_width < width {
                    self.content.game.row_mut().pop();
                    is_dirty = true;
                }

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
                        "Toads And Frogs",
                        Task::EvalToadsAndFrogs(EvalTask {
                            window: self.window_id,
                            game: self.content.game.clone(),
                        }),
                    );
                }
            });
    }
}
