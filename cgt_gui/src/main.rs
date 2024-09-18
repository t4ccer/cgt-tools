use cgt::{
    grid::{small_bit_grid::SmallBitGrid, FiniteGrid, Grid},
    numeric::dyadic_rational_number::DyadicRationalNumber,
    short::partizan::{
        canonical_form::CanonicalForm, games::domineering::Domineering,
        partizan_game::PartizanGame, thermograph::Thermograph,
        transposition_table::ParallelTranspositionTable,
    },
};
use imgui::Condition;
use std::str::FromStr;

mod imgui_sdl2_boilerplate;
mod widgets;

fn fade(mut color: [f32; 4], alpha: f32) -> [f32; 4] {
    let alpha = alpha.clamp(0.0, 1.0);
    color[3] *= alpha;
    color
}

fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start + t * (end - start)
}

#[derive(Clone, Copy)]
pub struct WindowId(usize);

#[allow(dead_code)]
pub struct Details {
    canonical_form: CanonicalForm,
    canonical_form_rendered: String,
    thermograph: Thermograph,
    temperature: DyadicRationalNumber,
    temperature_rendered: String,
}

pub struct DomineeringWindow<'tt> {
    title: String,
    game: Domineering,
    is_open: bool,
    show_thermograph: bool,
    details: Option<Details>,
    transposition_table: &'tt ParallelTranspositionTable<Domineering>,
}

impl<'tt> DomineeringWindow<'tt> {
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
            .size([700.0, 575.0], Condition::Appearing)
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

                let [start_pos_x, start_pos_y] = ui.cursor_pos();

                widgets::grid_size_selector(ui, &mut new_width, &mut new_height);
                ui.spacing();
                is_dirty |= widgets::bit_grid(ui, &draw_list, self.game.grid_mut());

                // Section: Right of grid
                ui.set_cursor_pos([start_pos_x, start_pos_y]);
                ui.indent_by(
                    start_pos_x
                        + (widgets::DOMINEERING_TILE_SIZE + widgets::DOMINEERING_TILE_GAP)
                            * width as f32,
                );

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

                if is_dirty {
                    self.details = None;
                }

                // TODO: Worker thread
                if self.details.is_none() {
                    let canonical_form = self.game.canonical_form(self.transposition_table);
                    let canonical_form_rendered = format!("Canonical Form: {canonical_form}");
                    let thremograph = canonical_form.thermograph();
                    let temperature = thremograph.temperature();
                    let temperature_rendered = format!("Temperature: {temperature}");
                    self.details = Some(Details {
                        canonical_form,
                        canonical_form_rendered,
                        thermograph: thremograph,
                        temperature,
                        temperature_rendered,
                    });
                }

                if let Some(details) = self.details.as_ref() {
                    ui.text_wrapped(&details.canonical_form_rendered);
                    ui.text_wrapped(&details.temperature_rendered);

                    ui.checkbox("Thermograph:", &mut self.show_thermograph);
                    if self.show_thermograph {
                        widgets::thermograph(ui, &draw_list, &details.thermograph);
                    }
                }
            });
    }
}

fn main() {
    let mut next_id = WindowId(0);
    let mut windows = Vec::new();

    let domineering_tt = ParallelTranspositionTable::new();

    // must be a macro because borrow checker
    macro_rules! new_domineering {
        () => {{
            let d = DomineeringWindow {
                game: Domineering::from_str("..#..|#...#|....#|##...|##.#.").unwrap(),
                is_open: true,
                title: format!("Domineering##{}", next_id.0),
                show_thermograph: true,
                details: None,
                transposition_table: &domineering_tt,
            };
            next_id.0 += 1;
            windows.push(d);
        }};
    }

    new_domineering!();

    let mut show_debug = false;

    imgui_sdl2_boilerplate::run("cgt-gui", |ui| {
        ui.dockspace_over_main_viewport();

        if show_debug {
            ui.show_demo_window(&mut show_debug);
        }

        if let Some(_main_menu) = ui.begin_main_menu_bar() {
            if let Some(_new_menu) = ui.begin_menu("New") {
                if ui.menu_item("Domineering") {
                    new_domineering!();
                }
                if ui.menu_item("Snort") {
                    // TODO
                }
            }
            if ui.menu_item("Debug") {
                show_debug = true;
            }
        }

        for d in windows.iter_mut() {
            d.draw(ui);
        }
    });
}
