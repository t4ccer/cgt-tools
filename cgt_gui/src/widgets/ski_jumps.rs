use cgt::{
    grid::{vec_grid::VecGrid, FiniteGrid, Grid},
    numeric::v2f::V2f,
    short::partizan::games::ski_jumps::{Move, SkiJumps, Tile},
};
use imgui::{ComboBoxFlags, Condition};
use std::str::FromStr;

use crate::{
    imgui_enum, impl_game_window, impl_titled_window,
    widgets::{
        self, canonical_form::CanonicalFormWindow, interactive_color, GridEditorAction, COLOR_BLUE,
        COLOR_RED, TILE_COLOR_EMPTY, TILE_COLOR_FILLED, TILE_SIZE, TILE_SPACING,
    },
    DetailOptions, Details, EvalTask, GuiContext, IsCgtWindow, RawOf, Task, TitledWindow,
};

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

#[derive(Debug, Clone)]
pub struct SkiJumpsWindow {
    game: SkiJumps,
    details_options: DetailOptions,
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
            details_options: DetailOptions::new(),
            editing_mode: RawOf::new(GridEditingMode::ClearTile),
            alternating_moves: true,
            initial_position: None,
            details: None,
        }
    }
}

impl IsCgtWindow for TitledWindow<SkiJumpsWindow> {
    impl_titled_window!("Ski Jumps");
    impl_game_window!(EvalSkiJumps, SkiJumpsDetails);

    fn draw(&mut self, ui: &imgui::Ui, ctx: &mut GuiContext) {
        let width = self.content.game.grid().width();
        let height = self.content.game.grid().height();

        let mut new_width = width;
        let mut new_height = height;

        let mut is_dirty = false;
        const OFF_BUTTON_SCALE: f32 = 0.3;

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
                    self.content.editing_mode.as_enum(),
                    GridEditingMode::MoveLeft | GridEditingMode::MoveRight
                ) {
                    ui.same_line();
                    ui.checkbox("Alternating", &mut self.content.alternating_moves);
                }

                widgets::grid_size_selector(ui, &mut new_width, &mut new_height);
                ui.spacing();
                let grid_start_pos = V2f::from(ui.cursor_pos());

                let mut slide_off_y = None;

                let editing_mode = self.content.editing_mode.as_enum();

                macro_rules! off_buttons {
                    ($text:expr, $start_x:expr) => {{
                        let grid_start_pos = V2f::from(ui.cursor_pos());
                        let off_buttons = ui.push_id("off_buttons");
                        for grid_y in 0..height {
                            let _y_id = ui.push_id_usize(grid_y as usize);
                            ui.set_cursor_pos([
                                grid_start_pos.x + (TILE_SIZE + TILE_SPACING) * $start_x as f32,
                                grid_start_pos.y + (TILE_SIZE + TILE_SPACING) * grid_y as f32,
                            ]);
                            let pos = V2f::from(ui.cursor_screen_pos());
                            let button_size = V2f {
                                x: TILE_SIZE * OFF_BUTTON_SCALE,
                                y: TILE_SIZE,
                            };
                            if ui.invisible_button("", button_size) {
                                slide_off_y = Some(grid_y);
                            }
                            let color = interactive_color(TILE_COLOR_EMPTY, ui);
                            draw_list
                                .add_rect(pos, pos + button_size, color)
                                .filled(true)
                                .build();
                            let size = V2f::from(ui.calc_text_size($text));
                            let text_pos = pos + (button_size - size) * 0.5;
                            draw_list.add_text(text_pos, TILE_COLOR_FILLED, $text);
                        }
                        ui.set_cursor_pos([
                            grid_start_pos.x,
                            grid_start_pos.y
                                + (TILE_SIZE + TILE_SPACING + TILE_SPACING) * height as f32,
                        ]);
                        drop(off_buttons);
                    }};
                }

                if matches!(editing_mode, GridEditingMode::MoveRight) {
                    off_buttons!("<", 0);
                }

                // NOTE: To prevent grid from "jumping" after every move we always apply offset
                // as if there was move-off-grid buttons
                ui.set_cursor_pos([
                    grid_start_pos.x + TILE_SIZE * OFF_BUTTON_SCALE + TILE_SPACING,
                    grid_start_pos.y,
                ]);

                let large_font = ui.push_font(ctx.large_font_id);
                let action = widgets::grid(
                    ui,
                    &draw_list,
                    self.content.game.grid(),
                    |screen_pos, (x, y), tile, draw_list| {
                        let color = interactive_color(TILE_COLOR_EMPTY, ui);
                        draw_list
                            .add_rect(
                                screen_pos,
                                screen_pos
                                    + V2f {
                                        x: TILE_SIZE,
                                        y: TILE_SIZE,
                                    },
                                color,
                            )
                            .filled(true)
                            .build();

                        if self
                            .content
                            .initial_position
                            .is_some_and(|initial| initial == (x, y))
                        {
                            match self.content.editing_mode.as_enum() {
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

                        macro_rules! draw_text {
                            ($text:expr) => {
                                let size = V2f::from(ui.calc_text_size($text));
                                let text_pos = screen_pos
                                    + (V2f {
                                        x: TILE_SIZE,
                                        y: TILE_SIZE,
                                    } - size)
                                        * 0.5;
                                draw_list.add_text(text_pos, TILE_COLOR_FILLED, $text);
                            };
                        }
                        match tile {
                            Tile::Empty => {}
                            Tile::LeftJumper => {
                                draw_text!("L");
                            }
                            Tile::LeftSlipper => {
                                draw_text!("l");
                            }
                            Tile::RightJumper => {
                                draw_text!("R");
                            }
                            Tile::RightSlipper => {
                                draw_text!("r");
                            }
                        }
                    },
                );
                drop(large_font);

                if matches!(editing_mode, GridEditingMode::MoveLeft) {
                    let current_pos = V2f::from(ui.cursor_pos());
                    ui.set_cursor_pos([current_pos.x, grid_start_pos.y]);
                    off_buttons!(">", width);
                }

                let mut move_target = None;
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

                        match editing_mode {
                            GridEditingMode::ClearTile => {
                                place_tile!(Empty);
                            }
                            GridEditingMode::LeftJumper => {
                                place_tile!(LeftJumper);
                            }
                            GridEditingMode::LeftSlipper => {
                                place_tile!(LeftSlipper);
                            }
                            GridEditingMode::RightJumper => {
                                place_tile!(RightJumper);
                            }
                            GridEditingMode::RightSlipper => {
                                place_tile!(RightSlipper);
                            }
                            GridEditingMode::MoveLeft | GridEditingMode::MoveRight => {
                                match self.content.initial_position {
                                    None => {
                                        if matches!(editing_mode, GridEditingMode::MoveLeft)
                                            && matches!(
                                                self.content.game.grid().get(x, y),
                                                Tile::LeftJumper | Tile::LeftSlipper
                                            )
                                        {
                                            self.content.initial_position = Some((x, y));
                                        } else if matches!(editing_mode, GridEditingMode::MoveRight)
                                            && matches!(
                                                self.content.game.grid().get(x, y),
                                                Tile::RightJumper | Tile::RightSlipper
                                            )
                                        {
                                            self.content.initial_position = Some((x, y));
                                        }
                                    }
                                    Some((start_x, start_y)) => {
                                        if (start_x, start_y) == (x, y) {
                                            self.content.initial_position = None;
                                        } else if matches!(
                                            self.content.game.grid().get(x, y),
                                            Tile::Empty
                                        ) {
                                            move_target = Some((x, y));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if let Some((start_x, start_y)) = self.content.initial_position {
                    if move_target.is_some() || slide_off_y.is_some() {
                        let available_moves = if matches!(editing_mode, GridEditingMode::MoveLeft) {
                            self.content.game.available_left_moves()
                        } else {
                            self.content.game.available_right_moves()
                        };

                        macro_rules! make_move {
                            ($move:expr) => {{
                                self.content.game = self.content.game.move_in($move);
                                self.content.initial_position = None;
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
                                break;
                            }};
                        }

                        for available_move in available_moves {
                            match available_move {
                                Move::SlideOff {
                                    from: (move_x, move_y),
                                } => {
                                    if (move_x, move_y) == (start_x, start_y)
                                        && slide_off_y
                                            .is_some_and(|slide_off_y| slide_off_y == move_y)
                                    {
                                        make_move!(available_move);
                                    }
                                }
                                Move::Slide {
                                    from: (move_x, move_y),
                                    to_x: move_to_x,
                                } => {
                                    if (move_x, move_y) == (start_x, start_y)
                                        && move_target
                                            .is_some_and(|target| target == (move_to_x, move_y))
                                    {
                                        make_move!(available_move);
                                    }
                                }
                                Move::Jump {
                                    from: (move_x, move_y),
                                } => {
                                    if (move_x, move_y) == (start_x, start_y)
                                        && move_target
                                            .is_some_and(|target| target == (move_x, move_y + 2))
                                    {
                                        make_move!(available_move);
                                    }
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
                    ctx.schedule_task(Task::EvalSkiJumps(EvalTask {
                        window: self.window_id,
                        game: self.content.game.clone(),
                    }));
                }
            });
    }
}
