use crate::{
    SyncState,
    canvas::HtmlCanvas,
    reactive::{self, SelectOption, SelectOptionElement},
};
use cgt::{
    drawing::{Canvas, Color, Hits, Interactions},
    grid::{FiniteGrid, Grid as _, vec_grid::VecGrid},
    numeric::v2f::V2f,
    short::partizan::{Player, games::fission},
};
use cgt_py_messages::{GridBackendMessage, GridFrontendMessage, GridPreset, GridPresetFlag, Tile};
use futures_signals::{
    map_ref,
    signal::{Mutable, SignalExt},
};
use jupyter_rust_widget_frontend::{AnyWidgetModel, Context, WasmWidget};
use wasm_bindgen::{
    JsCast, JsValue,
    prelude::{ScopedClosure, wasm_bindgen},
};
use web_sys::{
    CanvasRenderingContext2d, Document, Element, HtmlButtonElement, HtmlCanvasElement,
    HtmlDivElement, HtmlElement, HtmlInputElement, HtmlLabelElement, MouseEvent,
};

#[derive(Clone, Copy)]
enum Edge {
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Clone, Copy)]
enum ResizeAction {
    Grow,
    Shrink,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditMode {
    FlipCell,
    PlaceObject(Tile),
    FissionMove(Player),
}

impl EditMode {
    fn opposite_player(self) -> Option<EditMode> {
        match self {
            EditMode::FlipCell => None,
            EditMode::PlaceObject(_) => None,
            EditMode::FissionMove(player) => Some(EditMode::FissionMove(player.opposite())),
        }
    }
}

const EDIT_OPTIONS: &[EditOption] = &[
    // Domineering
    EditOption {
        text: "Flip Tile",
        mode: EditMode::FlipCell,
        visible_presets: GridPresetFlag::Domineering,
    },
    // TODO: Domineering moves that hover over two tiles, kinda tricky
    // Generic
    EditOption {
        text: "Clear Tile",
        mode: EditMode::PlaceObject(Tile::Empty),
        visible_presets: GridPresetFlag::all(),
    },
    EditOption {
        text: "Fill Tile",
        mode: EditMode::PlaceObject(Tile::Taken),
        visible_presets: GridPresetFlag::all(),
    },
    // Fission
    EditOption {
        text: "Place Stone",
        mode: EditMode::PlaceObject(Tile::BlackStone),
        visible_presets: GridPresetFlag::from_slice(&[
            GridPresetFlag::Fission,
            GridPresetFlag::Amazons,
        ]),
    },
    EditOption {
        text: "Left Move",
        mode: EditMode::FissionMove(Player::Left),
        visible_presets: GridPresetFlag::Fission,
    },
    EditOption {
        text: "Right Move",
        mode: EditMode::FissionMove(Player::Right),
        visible_presets: GridPresetFlag::Fission,
    },
    // Amazons
    EditOption {
        text: "Place Left Queen",
        mode: EditMode::PlaceObject(Tile::BlueStone),
        visible_presets: GridPresetFlag::Amazons,
    },
    EditOption {
        text: "Place Right Queen",
        mode: EditMode::PlaceObject(Tile::RedStone),
        visible_presets: GridPresetFlag::Amazons,
    },
    // TODO: Amazons moves
];

struct GridWidget {
    preset: GridPreset,
    edit_option: Mutable<EditOption>,
    consecutive_moves: Mutable<bool>,
    grid: Mutable<SyncState<VecGrid<Tile>>>,

    /// What the pointer is doing, which every frame both reads and consumes
    interactions: Mutable<Interactions>,
}

impl GridWidget {
    fn new(preset: GridPreset) -> GridWidget {
        GridWidget {
            preset,
            edit_option: Mutable::new(
                *EDIT_OPTIONS
                    .iter()
                    .filter(|edit| preset.intersects(edit.visible_presets))
                    .next()
                    .unwrap(),
            ),
            consecutive_moves: Mutable::new(false),
            grid: Mutable::new(SyncState::uninitialized(FiniteGrid::zero_size())),
            interactions: Mutable::new(Interactions::new()),
        }
    }

    fn edge_buttons(
        grid: Mutable<SyncState<VecGrid<Tile>>>,
        document: &Document,
        edge: Edge,
    ) -> Result<HtmlDivElement, JsValue> {
        let (row, column, direction) = match edge {
            Edge::Top => ("1", "2", "row"),
            Edge::Bottom => ("3", "2", "row"),
            Edge::Left => ("2", "1", "column"),
            Edge::Right => ("2", "3", "column"),
        };

        let group = document
            .create_element("div")?
            .dyn_into::<HtmlDivElement>()?;
        group.style().set_property("display", "flex")?;
        group.style().set_property("gap", "4px")?;
        group.style().set_property("flex-direction", direction)?;
        group.style().set_property("grid-row", row)?;
        group.style().set_property("grid-column", column)?;

        let (axis, side) = match edge {
            Edge::Top => ("row", "top"),
            Edge::Bottom => ("row", "bottom"),
            Edge::Left => ("column", "left"),
            Edge::Right => ("column", "right"),
        };

        for (text, grow) in [("-", ResizeAction::Shrink), ("+", ResizeAction::Grow)] {
            let button = document
                .create_element("button")?
                .dyn_into::<HtmlButtonElement>()?;
            button.set_text_content(Some(text));
            button.set_title(&format!(
                "{} {} {} the {}",
                match grow {
                    ResizeAction::Grow => "Add",
                    ResizeAction::Shrink => "Remove",
                },
                axis,
                match grow {
                    ResizeAction::Grow => "at",
                    ResizeAction::Shrink => "from",
                },
                side,
            ));
            button.style().set_property("width", "24px")?;
            button.style().set_property("height", "24px")?;
            button.style().set_property("padding", "0")?;
            button.style().set_property("line-height", "1")?;

            let handler = ScopedClosure::<dyn FnMut() -> Result<(), JsValue>>::new({
                let grid = grid.clone();
                move || {
                    let Some(new_grid) = resize_edge(&grid.lock_ref().state, edge, grow) else {
                        return Ok(());
                    };
                    grid.set(SyncState::edited(new_grid));

                    Ok(())
                }
            });
            button.add_event_listener_with_callback("click", handler.as_ref().unchecked_ref())?;
            handler.forget();

            group.append_child(&button)?;
        }

        Ok(group)
    }
}

fn resize_edge(grid: &VecGrid<Tile>, edge: Edge, action: ResizeAction) -> Option<VecGrid<Tile>> {
    let (width, height) = (grid.width(), grid.height());

    let (new_width, new_height) = match edge {
        Edge::Left | Edge::Right => (checked_resize(width, action)?, height),
        Edge::Top | Edge::Bottom => (width, checked_resize(height, action)?),
    };

    // Where the old grid lands in the new one. Tiles falling outside of it are the ones removed.
    let (offset_x, offset_y) = match (edge, action) {
        (Edge::Left, ResizeAction::Grow) => (1, 0),
        (Edge::Left, ResizeAction::Shrink) => (-1, 0),
        (Edge::Top, ResizeAction::Grow) => (0, 1),
        (Edge::Top, ResizeAction::Shrink) => (0, -1),
        (Edge::Right | Edge::Bottom, _) => (0, 0),
    };

    let mut new_grid: VecGrid<Tile> = FiniteGrid::filled(new_width, new_height, Tile::Taken)?;
    for y in 0..height {
        for x in 0..width {
            let new_x = i16::from(x) + offset_x;
            let new_y = i16::from(y) + offset_y;
            if (0..i16::from(new_width)).contains(&new_x)
                && (0..i16::from(new_height)).contains(&new_y)
            {
                new_grid.set(new_x as u8, new_y as u8, grid.get(x, y));
            }
        }
    }

    Some(new_grid)
}

fn checked_resize(value: u8, action: ResizeAction) -> Option<u8> {
    match action {
        ResizeAction::Grow => value.checked_add(1),
        ResizeAction::Shrink => value.checked_sub(1),
    }
}

impl GridWidget {
    /// Paint the grid and report what the mouse did to its tiles
    fn draw(
        canvas: &HtmlCanvasElement,
        grid: &VecGrid<Tile>,
        interactions: &mut Interactions,
    ) -> Result<Hits<(u8, u8)>, JsValue> {
        let canvas_size = grid.canvas_size::<HtmlCanvas>().size();
        canvas.set_width(canvas_size.x as u32);
        canvas.set_height(canvas_size.y as u32);
        canvas
            .style()
            .set_property("width", &format!("{}px", canvas_size.x as u32))?;
        canvas
            .style()
            .set_property("height", &format!("{}px", canvas_size.y as u32))?;

        let context = canvas
            .get_context("2d")?
            .unwrap()
            .dyn_into::<CanvasRenderingContext2d>()?;

        let mut canvas = HtmlCanvas::new(context, interactions);
        Ok(canvas.frame(|canvas| {
            // The tiles do not cover the gaps between them, which the grid lines are drawn in
            canvas.rect(V2f::ZERO, canvas_size, Color::BLACK);
            grid.draw(canvas, Tile::drawing)
        }))
    }

    fn apply(
        grid: &Mutable<SyncState<VecGrid<Tile>>>,
        edit_option: &Mutable<EditOption>,
        consecutive_moves: &Mutable<bool>,
        preset: &GridPreset,
        hits: &Hits<(u8, u8)>,
    ) {
        let Some((x, y)) = hits.clicked else {
            return;
        };

        match edit_option.get().mode {
            EditMode::FlipCell => {
                let mut grid = grid.lock_mut();
                let flipped = match grid.state.get(x, y) {
                    Tile::Empty => Tile::Taken,
                    Tile::Taken => Tile::Empty,
                    Tile::BlueStone | Tile::RedStone | Tile::BlackStone => return,
                };
                grid.edit().set(x, y, flipped);
            }
            EditMode::PlaceObject(tile) => {
                // Placing what is already there is not an edit, and reporting it would
                // have python run its update callbacks over an unchanged grid
                let mut grid = grid.lock_mut();
                if grid.state.get(x, y) != tile {
                    grid.edit().set(x, y, tile);
                }
            }
            EditMode::FissionMove(player) => {
                let Ok(fission_grid) = grid
                    .lock_ref()
                    .state
                    .try_map(|t| fission::Tile::try_from(*t))
                else {
                    return;
                };

                let fission = fission::Fission::new(fission_grid);
                if !fission.available_moves(player).contains(&(x, y)) {
                    return;
                }

                grid.set(SyncState::edited(
                    fission.move_in(x, y, player).grid().map(|t| Tile::from(t)),
                ));
                if !consecutive_moves.get()
                    && let Some(new_mode) = edit_option.get().mode.opposite_player()
                    && let Some(new_option_idx) =
                        SelectOption::find_index(|o| o.mode == new_mode, preset, EDIT_OPTIONS)
                    && let Some(new_option) =
                        SelectOption::selected_value_idx(new_option_idx, preset, EDIT_OPTIONS)
                {
                    edit_option.set(new_option);
                }
            }
        }
    }

    fn update(
        canvas: &HtmlCanvasElement,
        grid: &Mutable<SyncState<VecGrid<Tile>>>,
        interactions: &Mutable<Interactions>,
        edit_option: &Mutable<EditOption>,
        consecutive_moves: &Mutable<bool>,
        preset: &GridPreset,
    ) -> Result<(), JsValue> {
        let hits = GridWidget::draw(canvas, &grid.lock_ref().state, &mut interactions.lock_mut())?;
        GridWidget::apply(grid, edit_option, consecutive_moves, preset, &hits);

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EditOption {
    text: &'static str,
    mode: EditMode,
    visible_presets: GridPresetFlag,
}

impl SelectOptionElement for EditOption {
    type Preset = GridPreset;

    fn text(&self) -> &str {
        self.text
    }

    fn is_visible(&self, preset: &Self::Preset) -> bool {
        preset.intersects(self.visible_presets)
    }
}

impl WasmWidget for GridWidget {
    type BackendMessage = GridBackendMessage;
    type FrontendMessage = GridFrontendMessage;

    fn handle_message(&mut self, message: Self::FrontendMessage) -> Result<(), JsValue> {
        match message {
            GridFrontendMessage::SetGrid(new_grid) => {
                // Python echoes back every grid it is told about
                // (since there may be multiple frontend instances)
                let mut grid = self.grid.lock_mut();
                if grid.state != new_grid {
                    *grid = SyncState::from_python(new_grid);
                }

                Ok(())
            }
        }
    }

    fn mount(
        &mut self,
        context: Context<GridBackendMessage>,
        element: Element,
    ) -> Result<(), JsValue> {
        let document = web_sys::window().unwrap().document().unwrap();

        let controls = document
            .create_element("div")?
            .dyn_into::<HtmlDivElement>()?;
        controls.style().set_property("display", "flex")?;
        controls.style().set_property("flex-direction", "column")?;
        controls.style().set_property("gap", "4px")?;
        controls.style().set_property("margin-bottom", "4px")?;

        let mode_box = document.create_element("div")?;

        let mode_select = SelectOption::create_element_reactive(
            &document,
            "Edit mode",
            self.preset,
            EDIT_OPTIONS,
            self.edit_option.clone(),
        )?;
        let consecutive_box = document
            .create_element("div")?
            .dyn_into::<HtmlDivElement>()?;

        // TODO: Auto-generate unique element id to link the label
        let consecutive_moves_checkbox = document
            .create_element("input")?
            .dyn_into::<HtmlInputElement>()?;
        consecutive_moves_checkbox.set_type("checkbox");
        let consecutive_moves = self.consecutive_moves.clone();
        reactive::checkbox(&consecutive_moves_checkbox, &consecutive_moves)?;

        let consecutive_label = document
            .create_element("label")?
            .dyn_into::<HtmlLabelElement>()?;
        consecutive_label.set_text_content(Some("Consecutive Moves"));
        consecutive_box.append_child(&consecutive_label)?;
        consecutive_box.append_child(&consecutive_moves_checkbox)?;

        reactive::style_set_property(
            HtmlElement::from(consecutive_box.clone()),
            "display",
            self.edit_option.signal().map(|option| {
                if option.mode.opposite_player().is_some() {
                    "block"
                } else {
                    "none"
                }
            }),
        )?;

        mode_box.append_child(&mode_select)?;
        mode_box.append_child(&consecutive_box)?;
        controls.append_child(&mode_box)?;

        element.append_child(&controls).unwrap();

        let canvas = document
            .create_element("canvas")?
            .dyn_into::<HtmlCanvasElement>()?;
        reactive::canvas_interactions(&canvas, &self.interactions)?;

        let board = document
            .create_element("div")?
            .dyn_into::<HtmlDivElement>()?;
        board.style().set_property("display", "grid")?;
        board.style().set_property("width", "fit-content")?;
        board.style().set_property("align-items", "center")?;
        board.style().set_property("justify-items", "center")?;
        board.style().set_property("gap", "4px")?;

        for edge in [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
            let buttons = Self::edge_buttons(self.grid.clone(), &document, edge)?;
            board.append_child(&buttons)?;
        }

        canvas.style().set_property("grid-row", "2")?;
        canvas.style().set_property("grid-column", "2")?;
        board.append_child(&canvas)?;
        element.append_child(&board).unwrap();

        wasm_bindgen_futures::spawn_local(
            map_ref! {
                let _grid = self.grid.signal_ref(|_| ()),
                let _pointer = self.interactions.signal_ref(Interactions::pointer).dedupe() => ()
            }
            .for_each({
                let canvas = canvas.clone();
                let grid = self.grid.clone();
                let interactions = self.interactions.clone();
                let edit_option = self.edit_option.clone();
                let consecutive_moves = self.consecutive_moves.clone();
                let preset = self.preset;
                move |()| {
                    // Nothing can be done about a frame that fails to paint
                    let _ = GridWidget::update(
                        &canvas,
                        &grid,
                        &interactions,
                        &edit_option,
                        &consecutive_moves,
                        &preset,
                    );

                    async {}
                }
            }),
        );

        wasm_bindgen_futures::spawn_local(self.grid.signal_cloned().for_each({
            let context = context.clone();
            move |grid| {
                if grid.provenance.should_report_to_python() {
                    context.send_message(&GridBackendMessage::SetGrid { grid: grid.state });
                }

                async {}
            }
        }));

        context.send_message(&GridBackendMessage::Initialized);

        Ok(())
    }
}

#[wasm_bindgen]
pub fn render_grid_widget_impl(
    model: AnyWidgetModel,
    el: Element,
    raw_preset: u32,
) -> Result<(), JsValue> {
    let preset = GridPreset::from_flag_bits(raw_preset).unwrap();
    let widget = GridWidget::new(preset);
    widget.render(model, el)
}
