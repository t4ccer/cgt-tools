use crate::{
    Details, EvalTask, GuiContext, IsCgtWindow, Task, TitledWindow, impl_game_window,
    impl_titled_window,
    widgets::{self, canonical_form::CanonicalFormWindow},
};
use ::imgui::{Condition, Ui};
use cgt::{
    drawing::{Draw, imgui},
    grid::{BitTile, FiniteGrid, Grid, small_bit_grid::SmallBitGrid},
    short::partizan::games::domineering::{Domineering, Tile},
};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct DomineeringWindow {
    game: Domineering,
    pub details: Option<Details>,
}

impl DomineeringWindow {
    pub fn new() -> DomineeringWindow {
        DomineeringWindow {
            game: Domineering::from_str(".#.##|...##|#....|#...#|###..").unwrap(),
            details: None,
        }
    }
}

impl IsCgtWindow for TitledWindow<DomineeringWindow> {
    impl_titled_window!("Domineering");
    impl_game_window!("Domineering", EvalDomineering, DomineeringDetails);

    fn draw(&mut self, ui: &Ui, ctx: &mut GuiContext) {
        let width = self.content.game.grid().width();
        let height = self.content.game.grid().height();

        let mut new_width = width;
        let mut new_height = height;

        let mut is_dirty = false;

        ui.window(&self.title)
            .position(ui.io().mouse_pos, Condition::Appearing)
            .size([700.0, 450.0], Condition::Appearing)
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

                    if let Some(_new_menu) = ui.begin_menu("Save") {
                        if ui.menu_item("Dump Tikz") {
                            println!("{}", self.content.game.to_latex());
                        };
                    }
                }

                ui.columns(2, "Columns", true);

                widgets::grid_size_selector(ui, &mut new_width, &mut new_height);
                ui.spacing();

                let mut canvas =
                    imgui::Canvas::new(ui, &draw_list, ctx.large_font_id, &mut self.scratch_buffer);
                self.content.game.draw(&mut canvas);

                if let Some((x, y)) = canvas.clicked_tile(self.content.game.grid()) {
                    let flipped = self.content.game.grid().get(x, y).flip();
                    self.content.game.grid_mut().set(x, y, flipped);
                    is_dirty = true;
                };

                if new_width != width || new_height != height {
                    is_dirty = true;
                    if let Some(mut new_grid) =
                        SmallBitGrid::filled(new_width, new_height, Tile::Empty)
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
                        "Domineering",
                        Task::EvalDomineering(EvalTask {
                            window: self.window_id,
                            game: self.content.game,
                        }),
                    );
                }
            });
    }
}
