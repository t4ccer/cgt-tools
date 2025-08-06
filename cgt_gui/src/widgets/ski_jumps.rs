use ::imgui::{ComboBoxFlags, Condition, Ui};
use cgt::{
    drawing::{Canvas, Color, Draw, imgui},
    grid::{FiniteGrid, Grid, vec_grid::VecGrid},
    numeric::v2f::V2f,
    short::partizan::{
        Player,
        games::ski_jumps::{Move, SkiJumps, Tile},
    },
};
use std::str::FromStr;

use crate::{
    Details, EvalTask, GuiContext, IsCgtWindow, RawOf, Task, TitledWindow, imgui_enum,
    impl_game_window, impl_titled_window,
    widgets::{
        self, TILE_COLOR_EMPTY, TILE_COLOR_FILLED, TILE_SPACING,
        canonical_form::CanonicalFormWindow, interactive_color,
    },
};

const OFF_BUTTON_SCALE: f32 = 0.3;

imgui_enum! {
    GridEditingMode {
        LeftJumper, "Place Left Jumper",
        LeftSlipper, "Place Left Slipper",
        RightJumper, "Place Right Jumper",
        RightSlipper, "Place Right Slipper",
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
            GridEditingMode::LeftJumper => Edit::Place(Tile::LeftJumper),
            GridEditingMode::LeftSlipper => Edit::Place(Tile::LeftSlipper),
            GridEditingMode::RightJumper => Edit::Place(Tile::RightJumper),
            GridEditingMode::RightSlipper => Edit::Place(Tile::RightSlipper),
            GridEditingMode::ClearTile => Edit::Place(Tile::Empty),
            GridEditingMode::MoveLeft => Edit::Move(Player::Left),
            GridEditingMode::MoveRight => Edit::Move(Player::Right),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SkiJumpsWindow {
    game: SkiJumps,
    editing_mode: RawOf<GridEditingMode>,
    alternating_moves: bool,

    /// When making move, this is the starting skier position
    initial_position: Option<(u8, u8)>,
    pub details: Option<Details>,
}

impl SkiJumpsWindow {
    pub fn new() -> SkiJumpsWindow {
        SkiJumpsWindow {
            game: SkiJumps::from_str("L....|....R|.....").unwrap(),
            editing_mode: RawOf::new(GridEditingMode::ClearTile),
            alternating_moves: true,
            initial_position: None,
            details: None,
        }
    }
}

impl IsCgtWindow for TitledWindow<SkiJumpsWindow> {
    impl_titled_window!("Ski Jumps");
    impl_game_window!("Ski Jumps", EvalSkiJumps, SkiJumpsDetails);

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
                let changed_edit_mode =
                    self.content
                        .editing_mode
                        .combo(ui, "Edit Mode", ComboBoxFlags::HEIGHT_LARGE);
                if changed_edit_mode {
                    self.content.initial_position = None;
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
                let grid_start_pos = V2f::from(ui.cursor_pos());

                let mut slide_off_y = None;

                let editing_mode = self.content.editing_mode.get();

                let mut off_buttons = |text| {
                    let off_buttons = ui.push_id("off_buttons");
                    let current_pos = V2f::from(ui.cursor_pos());
                    for grid_y in 0..height {
                        let _y_id = ui.push_id_usize(grid_y as usize);
                        ui.set_cursor_pos([
                            current_pos.x,
                            (imgui::Canvas::tile_size().y + TILE_SPACING)
                                .mul_add(grid_y as f32, grid_start_pos.y),
                        ]);
                        let pos = V2f::from(ui.cursor_screen_pos());
                        let button_size = V2f {
                            x: imgui::Canvas::tile_size().x * OFF_BUTTON_SCALE,
                            y: imgui::Canvas::tile_size().y,
                        };
                        if ui.invisible_button("", button_size) {
                            slide_off_y = Some(grid_y);
                        }
                        let color = interactive_color(TILE_COLOR_EMPTY, ui);
                        draw_list
                            .add_rect(pos, pos + button_size, color)
                            .filled(true)
                            .build();
                        let size = V2f::from(ui.calc_text_size(text));
                        let text_pos = pos + (button_size - size) * 0.5;
                        draw_list.add_text(text_pos, TILE_COLOR_FILLED, text);
                    }
                    drop(off_buttons);
                };

                if matches!(editing_mode, GridEditingMode::MoveRight) {
                    off_buttons("<");
                }

                // NOTE: To prevent grid from "jumping" after every move we always apply offset
                // as if there was move-off-grid buttons
                ui.set_cursor_pos([
                    imgui::Canvas::tile_size()
                        .x
                        .mul_add(OFF_BUTTON_SCALE, grid_start_pos.x),
                    grid_start_pos.y,
                ]);

                let mut canvas =
                    imgui::Canvas::new(ui, &draw_list, ctx.large_font_id, &mut self.scratch_buffer);
                self.content.game.draw(&mut canvas);

                if matches!(editing_mode, GridEditingMode::MoveLeft) {
                    ui.set_cursor_pos([
                        (1.0 - OFF_BUTTON_SCALE).mul_add(
                            -imgui::Canvas::tile_size().x,
                            grid_start_pos.x
                                + imgui::Canvas::tile_position(
                                    self.content.game.grid().width() + 1,
                                    0,
                                )
                                .x,
                        ),
                        grid_start_pos.y,
                    ]);
                    off_buttons(">");
                }

                let mut move_target = None;

                if let Some((x, y)) = canvas.clicked_tile(self.content.game.grid()) {
                    match Edit::from(self.content.editing_mode.get()) {
                        Edit::Place(tile) => {
                            if self.content.game.grid().get(x, y) != tile {
                                self.content.game.grid_mut().set(x, y, tile);
                                is_dirty = true;
                            }
                        }
                        Edit::Move(player) => match self.content.initial_position {
                            None => {
                                if (matches!(player, Player::Left)
                                    && matches!(
                                        self.content.game.grid().get(x, y),
                                        Tile::LeftJumper | Tile::LeftSlipper
                                    ))
                                    || (matches!(player, Player::Right)
                                        && matches!(
                                            self.content.game.grid().get(x, y),
                                            Tile::RightJumper | Tile::RightSlipper
                                        ))
                                {
                                    self.content.initial_position = Some((x, y));
                                }
                            }
                            Some((start_x, start_y)) => {
                                if (start_x, start_y) == (x, y) {
                                    self.content.initial_position = None;
                                } else if matches!(self.content.game.grid().get(x, y), Tile::Empty)
                                {
                                    move_target = Some((x, y));
                                }
                            }
                        },
                    }
                }

                if let Some((start_x, start_y)) = self.content.initial_position {
                    canvas.highlight_tile(
                        imgui::Canvas::tile_position(start_x, start_y),
                        if matches!(editing_mode, GridEditingMode::MoveLeft) {
                            Color::BLUE
                        } else {
                            Color::RED
                        },
                    );

                    if move_target.is_some() || slide_off_y.is_some() {
                        let available_moves = if matches!(editing_mode, GridEditingMode::MoveLeft) {
                            self.content.game.available_left_moves()
                        } else {
                            self.content.game.available_right_moves()
                        };

                        if let Some(legal_move) =
                            available_moves.into_iter().find_map(|available_move| {
                                match available_move {
                                    Move::SlideOff {
                                        from: (move_x, move_y),
                                    } => ((move_x, move_y) == (start_x, start_y)
                                        && slide_off_y
                                            .is_some_and(|slide_off_y: u8| slide_off_y == move_y))
                                    .then_some(available_move),
                                    Move::Slide {
                                        from: (move_x, move_y),
                                        to_x: move_to_x,
                                    } => ((move_x, move_y) == (start_x, start_y)
                                        && move_target
                                            .is_some_and(|target| target == (move_to_x, move_y)))
                                    .then_some(available_move),
                                    Move::Jump {
                                        from: (move_x, move_y),
                                    } => ((move_x, move_y) == (start_x, start_y)
                                        && move_target
                                            .is_some_and(|target| target == (move_x, move_y + 2)))
                                    .then_some(available_move),
                                }
                            })
                        {
                            self.content.game = self.content.game.move_in(legal_move);
                            if self.content.alternating_moves {
                                if matches!(editing_mode, GridEditingMode::MoveLeft) {
                                    self.content.editing_mode =
                                        RawOf::new(GridEditingMode::MoveRight);
                                } else {
                                    self.content.editing_mode =
                                        RawOf::new(GridEditingMode::MoveLeft);
                                }
                            }
                            is_dirty = true;
                        };

                        self.content.initial_position = None;
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
                        "Ski Jumps",
                        Task::EvalSkiJumps(EvalTask {
                            window: self.window_id,
                            game: self.content.game.clone(),
                        }),
                    );
                }
            });
    }
}
