use cgt::{
    graph,
    grid::{small_bit_grid::SmallBitGrid, FiniteGrid, Grid},
    numeric::dyadic_rational_number::DyadicRationalNumber,
    short::partizan::{
        canonical_form::CanonicalForm,
        games::{
            domineering::Domineering,
            snort::{self, Snort},
        },
        partizan_game::PartizanGame,
        thermograph::Thermograph,
        transposition_table::ParallelTranspositionTable,
    },
};
use imgui::{Condition, ImColor32, MouseButton, StyleColor};
use std::{borrow::Cow, f32::consts::PI, fmt::Write, marker::PhantomData, str::FromStr};

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

impl Details {
    pub fn from_canonical_form(canonical_form: CanonicalForm) -> Details {
        let canonical_form_rendered = format!("Canonical Form: {canonical_form}");
        let thermograph = canonical_form.thermograph();
        let temperature = thermograph.temperature();
        let temperature_rendered = format!("Temperature: {temperature}");
        Details {
            canonical_form,
            canonical_form_rendered,
            thermograph,
            temperature,
            temperature_rendered,
        }
    }
}

pub enum CgtWindow<'tt> {
    Domineering(DomineeringWindow<'tt>),
    CanonicalForm(CanonicalFormWindow),
    Snort(SnortWindow<'tt>),
}

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

                let [start_pos_x, start_pos_y] = ui.cursor_pos();

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

                ui.set_cursor_pos([start_pos_x, start_pos_y]);

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

                // Section: Right of grid
                ui.indent_by(
                    start_pos_x
                        + (widgets::DOMINEERING_TILE_SIZE + widgets::DOMINEERING_TILE_GAP)
                            * width as f32,
                );

                ui.next_column();

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

pub struct CanonicalFormWindow {
    title: String,
    is_open: bool,
    details: Details,
    value_input: String,
    input_error: bool,
}

impl CanonicalFormWindow {
    pub fn draw(&mut self, ui: &imgui::Ui) {
        if !self.is_open {
            return;
        }

        ui.window(&self.title)
            .position([50.0, 50.0], Condition::Appearing)
            .size([400.0, 450.0], Condition::Appearing)
            .bring_to_front_on_focus(true)
            .opened(&mut self.is_open)
            .build(|| {
                let draw_list = ui.get_window_draw_list();
                let short_inputs = ui.push_item_width(250.0);
                if ui.input_text("Value", &mut self.value_input).build() {
                    match CanonicalForm::from_str(&self.value_input) {
                        Err(_) => self.input_error = true,
                        Ok(cf) => {
                            self.input_error = false;
                            self.details = Details::from_canonical_form(cf);
                        }
                    }
                }
                short_inputs.end();

                if self.input_error {
                    ui.text_colored(
                        ImColor32::from_rgb(0xdd, 0x00, 0x00).to_rgba_f32s(),
                        "Invalid input",
                    );
                }
                ui.text_wrapped(&self.details.canonical_form_rendered);
                ui.text(&self.details.temperature_rendered);
                widgets::thermograph(ui, &draw_list, 50.0, &self.details.thermograph);
            });
    }
}

const SNORT_NODE_RADIUS: f32 = 16.0;

pub trait IsEnum {
    const LABELS: &'static [&'static str];

    fn to_usize(self) -> usize;
    fn from_usize(raw: usize) -> Self;
}

#[derive(Clone, Copy)]
pub struct RawOf<T> {
    pub value: usize,
    _ty: PhantomData<T>,
}

impl<T> RawOf<T>
where
    T: IsEnum,
{
    pub fn new(value: T) -> RawOf<T> {
        RawOf {
            value: value.to_usize(),
            _ty: PhantomData,
        }
    }

    pub fn as_enum(self) -> T {
        T::from_usize(self.value)
    }
}

macro_rules! imgui_enum {
    ($name:ident { $($variant:ident, $raw:expr, $pretty:expr,)*}) => {
        #[derive(Clone, Copy)]
        #[repr(usize)]
        pub enum $name {
            $($variant,)*
        }

        impl IsEnum for $name {
            const LABELS: &'static [&'static str] = &[$($pretty,)*];

            fn to_usize(self) -> usize {
                self as usize
            }

            fn from_usize(raw: usize) -> $name {
                match raw {
                    $($raw => $name::$variant,)*
                    _ => panic!("Invalid value: {raw}")
                }
            }
        }
    };
}

imgui_enum! {
    GraphEditingMode {
        DragNode, 0, "Drag vertex",
        TintNodeBlue, 1, "Tint vertex blue (left)",
        TintNodeRed, 2, "Tint vertex red (right)",
        TintNodeNone, 3, "Untint vertex",
        DeleteNode, 4, "Remove vertex",
        AddEdge, 5, "Add/Remove edge",
        AddNode, 6, "Add vertex",
    }
}

imgui_enum! {
    RepositionMode {
        Circle, 0, "Circle",
        FDP, 1, "FDP (Not Implemented Yet)",
    }
}

pub struct SnortWindow<'tt> {
    title: String,
    is_open: bool,
    game: Snort,
    reposition_option_selected: RawOf<RepositionMode>,

    #[allow(dead_code)]
    transposition_table: &'tt ParallelTranspositionTable<Snort>,
    node_positions: Vec<[f32; 2]>,
    editing_mode: RawOf<GraphEditingMode>,
    held_node_for_new_edge: Option<usize>,
    details: Option<Details>,
    show_thermograph: bool,
    thermograph_scale: f32,
}

impl<'tt> SnortWindow<'tt> {
    pub fn reposition_circle(&mut self) {
        let n = self.game.graph.size();
        let packing_circle_radius = SNORT_NODE_RADIUS * (self.game.graph.size() as f32 + 4.0) * 0.5;
        self.node_positions = Vec::with_capacity(n);
        for i in 0..n {
            let angle = (2.0 * PI * i as f32) / n as f32;
            let node_pos = [
                (packing_circle_radius - SNORT_NODE_RADIUS) * f32::cos(angle)
                    + packing_circle_radius,
                (packing_circle_radius - SNORT_NODE_RADIUS) * f32::sin(angle)
                    + packing_circle_radius,
            ];
            self.node_positions.push(node_pos);
        }
    }

    pub fn draw(&mut self, ui: &imgui::Ui) {
        if !self.is_open {
            return;
        }

        let mut should_reposition = false;
        let mut to_remove: Option<usize> = None;
        let mut is_dirty = false;
        let mut label_buf = String::new();

        ui.window(&self.title)
            .position([50.0, 50.0], Condition::Appearing)
            .size([750.0, 450.0], Condition::Appearing)
            .bring_to_front_on_focus(true)
            .opened(&mut self.is_open)
            .build(|| {
                let draw_list = ui.get_window_draw_list();

                ui.columns(2, "columns", true);

                let short_inputs = ui.push_item_width(200.0);
                ui.combo(
                    "##Reposition Mode",
                    &mut self.reposition_option_selected.value,
                    RepositionMode::LABELS,
                    |i| Cow::Borrowed(i),
                );
                ui.same_line();
                should_reposition = ui.button("Reposition");

                ui.combo(
                    "Edit Mode",
                    &mut self.editing_mode.value,
                    GraphEditingMode::LABELS,
                    |i| Cow::Borrowed(i),
                );
                short_inputs.end();

                let [pos_x, pos_y] = ui.cursor_screen_pos();
                let off_y = ui.cursor_pos()[1];

                let mut max_y = f32::NEG_INFINITY;
                let node_color = ui.style_color(StyleColor::Text);
                for this_vertex_idx in 0..self.game.graph.size() {
                    let [absolute_node_pos_x, absolute_node_pos_y] =
                        self.node_positions[this_vertex_idx];
                    let _node_id = ui.push_id_usize(this_vertex_idx as usize);
                    let node_pos @ [node_pos_x, node_pos_y] =
                        [pos_x + absolute_node_pos_x, pos_y + absolute_node_pos_y];
                    max_y = max_y.max(node_pos_y);
                    let button_pos @ [button_pos_x, button_pos_y] = [
                        node_pos_x - SNORT_NODE_RADIUS,
                        node_pos_y - SNORT_NODE_RADIUS,
                    ];
                    let button_size @ [button_size_width, button_size_height] =
                        [SNORT_NODE_RADIUS * 2.0, SNORT_NODE_RADIUS * 2.0];
                    ui.set_cursor_screen_pos(button_pos);

                    if ui.invisible_button("node", button_size) {
                        match self.editing_mode.as_enum() {
                            GraphEditingMode::DragNode => { /* NOOP */ }
                            GraphEditingMode::TintNodeNone => {
                                *self.game.vertices[this_vertex_idx].color_mut() =
                                    snort::VertexColor::Empty;
                                is_dirty = true;
                            }
                            GraphEditingMode::TintNodeBlue => {
                                *self.game.vertices[this_vertex_idx].color_mut() =
                                    snort::VertexColor::TintLeft;
                                is_dirty = true;
                            }
                            GraphEditingMode::TintNodeRed => {
                                *self.game.vertices[this_vertex_idx].color_mut() =
                                    snort::VertexColor::TintRight;
                                is_dirty = true;
                            }
                            GraphEditingMode::DeleteNode => {
                                // We don't remove it immediately because we're just iterating over
                                // vertices
                                to_remove = Some(this_vertex_idx);
                                is_dirty = true;
                            }
                            GraphEditingMode::AddEdge => { /* NOOP */ }
                            GraphEditingMode::AddNode => { /* NOOP */ }
                        }
                    };

                    if ui.is_item_activated()
                        && matches!(self.editing_mode.as_enum(), GraphEditingMode::AddEdge)
                    {
                        self.held_node_for_new_edge = Some(this_vertex_idx);
                    }

                    let [mouse_pos_x, mouse_pos_y] = ui.io().mouse_pos;
                    if !ui.io()[MouseButton::Left]
                        && mouse_pos_x >= button_pos_x
                        && mouse_pos_x <= (button_pos_x + button_size_width)
                        && mouse_pos_y >= button_pos_y
                        && mouse_pos_y <= (button_pos_y + button_size_height)
                    {
                        if let Some(held_node) = self.held_node_for_new_edge.take() {
                            if held_node != this_vertex_idx {
                                self.game.graph.connect(
                                    held_node,
                                    this_vertex_idx,
                                    !self.game.graph.are_adjacent(held_node, this_vertex_idx),
                                );
                                is_dirty = true;
                            }
                        }
                    }

                    if ui.is_item_active()
                        && matches!(self.editing_mode.as_enum(), GraphEditingMode::DragNode)
                    {
                        let [mouse_delta_x, mouse_delta_y] = ui.io().mouse_delta;
                        self.node_positions[this_vertex_idx] = [
                            absolute_node_pos_x + mouse_delta_x,
                            absolute_node_pos_y + mouse_delta_y,
                        ];
                    }

                    let (node_fill_color, should_fill) =
                        match self.game.vertices[this_vertex_idx].color() {
                            snort::VertexColor::Empty => (node_color, false),
                            snort::VertexColor::TintLeft => {
                                (ImColor32::from_bits(0xfffb4a4e).to_rgba_f32s(), true)
                            }
                            snort::VertexColor::TintRight => {
                                (ImColor32::from_bits(0xff7226f9).to_rgba_f32s(), true)
                            }
                            snort::VertexColor::Taken => {
                                (ImColor32::from_bits(0xff333333).to_rgba_f32s(), true)
                            }
                        };

                    draw_list
                        .add_circle(node_pos, SNORT_NODE_RADIUS, node_color)
                        .build();
                    if should_fill {
                        draw_list
                            .add_circle(node_pos, SNORT_NODE_RADIUS - 0.5, node_fill_color)
                            .filled(true)
                            .build();
                    }

                    label_buf.clear();
                    label_buf
                        .write_fmt(format_args!("{}", this_vertex_idx + 1))
                        .unwrap();
                    let off_x = ui.calc_text_size(&label_buf)[0];
                    draw_list.add_text(
                        [node_pos_x - off_x * 0.5, node_pos_y + SNORT_NODE_RADIUS],
                        node_color,
                        &label_buf,
                    );

                    for adjacent_vertex_idx in self.game.graph.adjacent_to(this_vertex_idx) {
                        if adjacent_vertex_idx < this_vertex_idx {
                            let [adjacent_pos_x, adjacent_pos_y] =
                                self.node_positions[adjacent_vertex_idx];
                            let adjacent_pos = [pos_x + adjacent_pos_x, pos_y + adjacent_pos_y];
                            draw_list
                                .add_line(node_pos, adjacent_pos, node_color)
                                .thickness(1.0)
                                .build();
                        }
                    }
                }

                if let Some(held_node) = self.held_node_for_new_edge {
                    let [held_node_pos_x, held_node_pos_y] = self.node_positions[held_node];
                    let held_node_pos = [pos_x + held_node_pos_x, pos_y + held_node_pos_y];
                    draw_list
                        .add_line(held_node_pos, ui.io().mouse_pos, ImColor32::BLACK)
                        .thickness(2.0)
                        .build();
                }

                ui.set_cursor_screen_pos([pos_x, pos_y]);
                if matches!(self.editing_mode.as_enum(), GraphEditingMode::AddNode)
                    && ui.invisible_button(
                        "Graph background",
                        [ui.current_column_width(), ui.window_size()[1] - off_y],
                    )
                {
                    self.game.graph.add_vertex();
                    self.game
                        .vertices
                        .push(snort::VertexKind::Single(snort::VertexColor::Empty));

                    let [mouse_x, mouse_y] = ui.io().mouse_pos;
                    self.node_positions.push([mouse_x - pos_x, mouse_y - pos_y]);
                    is_dirty = true;
                }

                ui.set_cursor_screen_pos([pos_x, max_y + SNORT_NODE_RADIUS]);
                ui.next_column();

                if let Some(to_remove) = to_remove.take() {
                    self.game.graph.remove_vertex(to_remove);
                    self.game.vertices.remove(to_remove);
                    self.node_positions.remove(to_remove);
                    is_dirty = true;
                }

                if is_dirty {
                    self.details = None;
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
                        ui.slider("##1", 5.0, 100.0, &mut self.thermograph_scale);
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

        if should_reposition {
            match self.reposition_option_selected.as_enum() {
                RepositionMode::Circle => self.reposition_circle(),
                RepositionMode::FDP => { /* TODO */ }
            }
        }

        if !ui.io()[MouseButton::Left] {
            self.held_node_for_new_edge = None;
        }
    }
}

fn main() {
    let mut next_id = WindowId(0);
    let mut windows = Vec::<CgtWindow>::new();

    let domineering_tt = ParallelTranspositionTable::new();
    let snort_tt = ParallelTranspositionTable::new();

    // must be a macro because borrow checker
    macro_rules! new_domineering {
        () => {{
            let d = DomineeringWindow {
                game: Domineering::from_str(".#.##|...##|#....|#...#|###..").unwrap(),
                is_open: true,
                title: format!("Domineering##{}", next_id.0),
                show_thermograph: true,
                details: None,
                thermograph_scale: 50.0,
                transposition_table: &domineering_tt,
            };
            next_id.0 += 1;
            windows.push(CgtWindow::Domineering(d));
        }};
    }

    macro_rules! new_canonical_form {
        () => {{
            let cf = CanonicalForm::from_str("{-1,{2|-2}|-5}").unwrap();
            let d = CanonicalFormWindow {
                value_input: cf.to_string(),
                details: Details::from_canonical_form(cf),
                is_open: true,
                title: format!("Canonical Form##{}", next_id.0),
                input_error: false,
            };
            next_id.0 += 1;
            windows.push(CgtWindow::CanonicalForm(d));
        }};
    }

    macro_rules! new_snort {
        () => {{
            let mut d = SnortWindow {
                title: format!("Snort##{}", next_id.0),
                is_open: true,
                // caterpillar C(4, 3, 4)
                game: Snort::new(graph::undirected::Graph::from_edges(
                    14,
                    &[
                        // left
                        (0, 4),
                        (1, 4),
                        (2, 4),
                        (3, 4),
                        // center
                        (6, 5),
                        (7, 5),
                        (8, 5),
                        // right
                        (10, 9),
                        (11, 9),
                        (12, 9),
                        (13, 9),
                        // main path
                        (4, 5),
                        (5, 9),
                    ],
                )),
                transposition_table: &snort_tt,
                node_positions: Vec::new(),
                reposition_option_selected: RawOf::new(RepositionMode::Circle),
                editing_mode: RawOf::new(GraphEditingMode::DragNode),
                held_node_for_new_edge: None,
                details: None,
                show_thermograph: true,
                thermograph_scale: 20.0,
            };
            next_id.0 += 1;
            d.reposition_circle();
            windows.push(CgtWindow::Snort(d));
        }};
    }

    // new_domineering!();
    new_snort!();

    let mut show_debug = false;

    imgui_sdl2_boilerplate::run("cgt-gui", |ui| {
        ui.dockspace_over_main_viewport();

        if show_debug {
            ui.show_demo_window(&mut show_debug);
        }

        if let Some(_main_menu) = ui.begin_main_menu_bar() {
            if let Some(_new_menu) = ui.begin_menu("New") {
                if ui.menu_item("Canonical Form") {
                    new_canonical_form!();
                }
                if ui.menu_item("Domineering") {
                    new_domineering!();
                }
                if ui.menu_item("Snort") {
                    new_snort!();
                }
            }
            if ui.menu_item("Debug") {
                show_debug = true;
            }
        }

        for d in windows.iter_mut() {
            match d {
                CgtWindow::Domineering(d) => d.draw(ui),
                CgtWindow::CanonicalForm(d) => d.draw(ui),
                CgtWindow::Snort(d) => d.draw(ui),
            }
        }
    });
}
