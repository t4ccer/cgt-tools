use cgt::{
    grid::{small_bit_grid::SmallBitGrid, FiniteGrid, Grid},
    short::partizan::games::domineering::{Domineering, Tile},
};
use imgui::Condition;
use std::str::FromStr;

use crate::{
    impl_game_window, impl_titled_window,
    widgets::{self, canonical_form::CanonicalFormWindow},
    Context, DetailOptions, Details, EvalTask, IsCgtWindow, Task, TitledWindow, UpdateKind,
};

#[derive(Debug, Clone)]
pub struct DomineeringWindow {
    game: Domineering,
    details_options: DetailOptions,
    pub details: Option<Details>,
}

impl DomineeringWindow {
    pub fn new() -> DomineeringWindow {
        DomineeringWindow {
            game: Domineering::from_str(".#.##|...##|#....|#...#|###..").unwrap(),
            details_options: DetailOptions::new(),
            details: None,
        }
    }
}

impl IsCgtWindow for TitledWindow<DomineeringWindow> {
    impl_titled_window!("Domineering");
    impl_game_window!(EvalDomineering, DomineeringDetails);

    fn draw(&mut self, ui: &imgui::Ui, ctx: &mut Context) {
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
                is_dirty |= widgets::bit_grid(ui, &draw_list, self.content.game.grid_mut());

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
                    ctx.schedule_task(Task::EvalDomineering(EvalTask {
                        window: self.window_id,
                        game: self.content.game.clone(),
                    }));
                }
            });
    }
}
