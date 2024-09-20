use cgt::{
    grid::{small_bit_grid::SmallBitGrid, FiniteGrid, Grid},
    short::partizan::{
        games::domineering::Domineering, partizan_game::PartizanGame,
        transposition_table::ParallelTranspositionTable,
    },
};
use imgui::Condition;
use std::str::FromStr;

use crate::{widgets, Details, WindowId};

pub struct DomineeringWindow<'tt> {
    title: String,
    game: Domineering,
    is_open: bool,
    show_thermograph: bool,
    thermograph_scale: f32,
    details: Option<Details>,
    transposition_table: &'tt ParallelTranspositionTable<Domineering>,
}

impl<'tt> DomineeringWindow<'tt> {
    pub fn new(
        id: WindowId,
        domineering_tt: &'tt ParallelTranspositionTable<Domineering>,
    ) -> DomineeringWindow<'tt> {
        DomineeringWindow {
            game: Domineering::from_str(".#.##|...##|#....|#...#|###..").unwrap(),
            is_open: true,
            title: format!("Domineering##{}", id.0),
            show_thermograph: true,
            details: None,
            thermograph_scale: 50.0,
            transposition_table: &domineering_tt,
        }
    }

    pub fn draw(&mut self, ui: &imgui::Ui) {
        use cgt::short::partizan::games::domineering;

        if !self.is_open {
            return;
        }

        let width = self.game.grid().width();
        let height = self.game.grid().height();

        let mut new_width = width;
        let mut new_height = height;

        let mut is_dirty = false;

        ui.window(&self.title)
            .position([50.0, 50.0], Condition::Appearing)
            .size([700.0, 450.0], Condition::Appearing)
            .bring_to_front_on_focus(true)
            .menu_bar(true)
            .opened(&mut self.is_open)
            .build(|| {
                let draw_list = ui.get_window_draw_list();
                if let Some(_menu_bar) = ui.begin_menu_bar() {
                    if let Some(_new_menu) = ui.begin_menu("New") {
                        if ui.menu_item("Duplicate") {
                            // TODO
                        };
                    }
                }

                ui.columns(2, "Columns", true);

                widgets::grid_size_selector(ui, &mut new_width, &mut new_height);
                ui.spacing();
                is_dirty |= widgets::bit_grid(ui, &draw_list, self.game.grid_mut());

                if new_width != width || new_height != height {
                    is_dirty = true;
                    if let Some(mut new_grid) =
                        SmallBitGrid::filled(new_width, new_height, domineering::Tile::Taken)
                    {
                        for y in 0..height {
                            for x in 0..width {
                                new_grid.set(x, y, self.game.grid().get(x, y));
                            }
                        }
                        *self.game.grid_mut() = new_grid;
                    }
                }

                // Section: Right of grid
                ui.next_column();

                // SAFETY: We're fine because we're not pushing any style changes
                let pad_x = unsafe { ui.style().window_padding[0] };
                if is_dirty {
                    self.details = None;
                    ui.set_column_width(
                        0,
                        f32::max(
                            pad_x
                                + (widgets::DOMINEERING_TILE_SIZE + widgets::DOMINEERING_TILE_GAP)
                                    * new_width as f32,
                            ui.column_width(0),
                        ),
                    );
                }

                // TODO: Worker thread
                if self.details.is_none() {
                    let canonical_form = self.game.canonical_form(self.transposition_table);
                    self.details = Some(Details::from_canonical_form(canonical_form));
                }

                if let Some(details) = self.details.as_ref() {
                    ui.text_wrapped(&details.canonical_form_rendered);
                    ui.text_wrapped(&details.temperature_rendered);

                    ui.checkbox("Thermograph:", &mut self.show_thermograph);
                    if self.show_thermograph {
                        ui.align_text_to_frame_padding();
                        ui.text("Scale: ");
                        ui.same_line();
                        let short_slider = ui.push_item_width(200.0);
                        ui.slider("##1", 20.0, 150.0, &mut self.thermograph_scale);
                        short_slider.end();
                        widgets::thermograph(
                            ui,
                            &draw_list,
                            self.thermograph_scale,
                            &details.thermograph,
                        );
                    }
                }
            });
    }
}
