use cgt::{
    grid::{FiniteGrid, Grid, vec_grid::VecGrid},
    short::partizan::{Player, games::fission},
};
use cgt_py_messages::{GridBackendMessage, GridFrontendMessage, GridPreset, GridPresetFlag, Tile};
use core::f64;
use jupyter_rust_widget_frontend::{AnyWidgetModel, Context, WasmWidget};
use std::sync::{Arc, Mutex};
use wasm_bindgen::{
    JsCast, JsValue,
    prelude::{ScopedClosure, wasm_bindgen},
};
use web_sys::{
    CanvasRenderingContext2d, Document, Element, HtmlButtonElement, HtmlCanvasElement,
    HtmlDivElement, HtmlInputElement, HtmlLabelElement, HtmlSelectElement, MouseEvent,
};

use crate::{ActiveElement, SelectOption, SelectOptionElement};

struct HtmlState {
    canvas: HtmlCanvasElement,
    edit_mode: HtmlSelectElement,
}

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

#[derive(Clone, Copy, PartialEq, Eq)]
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

struct SharedState {
    preset: GridPreset,
    edit_mode: EditMode,
    consecutive_moves: bool,
    state: Option<HtmlState>,
    grid: VecGrid<Tile>,
    active_cell: ActiveElement<(u8, u8)>,
}

const EDIT_OPTIONS: &[EditOption] = &[
    // Domineering
    EditOption {
        text: "Flip Tile",
        mode: EditMode::FlipCell,
        visible_presets: GridPresetFlag::DOMINEERING,
    },
    // TODO: Domineering moves that hover over two tiles, kinda tricky
    // Generic
    EditOption {
        text: "Clear Tile",
        mode: EditMode::PlaceObject(Tile::Empty),
        visible_presets: GridPresetFlag::DOMINEERING
            .union(GridPresetFlag::FISSION)
            .union(GridPresetFlag::AMAZONS),
    },
    EditOption {
        text: "Fill Tile",
        mode: EditMode::PlaceObject(Tile::Taken),
        visible_presets: GridPresetFlag::DOMINEERING
            .union(GridPresetFlag::FISSION)
            .union(GridPresetFlag::AMAZONS),
    },
    // Fission
    EditOption {
        text: "Place Stone",
        mode: EditMode::PlaceObject(Tile::BlackStone),
        visible_presets: GridPresetFlag::FISSION.union(GridPresetFlag::AMAZONS),
    },
    EditOption {
        text: "Left Move",
        mode: EditMode::FissionMove(Player::Left),
        visible_presets: GridPresetFlag::FISSION,
    },
    EditOption {
        text: "Right Move",
        mode: EditMode::FissionMove(Player::Right),
        visible_presets: GridPresetFlag::FISSION,
    },
    // Amazons
    EditOption {
        text: "Place Left Queen",
        mode: EditMode::PlaceObject(Tile::BlueStone),
        visible_presets: GridPresetFlag::AMAZONS,
    },
    EditOption {
        text: "Place Right Queen",
        mode: EditMode::PlaceObject(Tile::RedStone),
        visible_presets: GridPresetFlag::AMAZONS,
    },
    // TODO: Amazons moves
];

struct GridWidget {
    preset: GridPreset,
    shared: Arc<Mutex<SharedState>>,
}

impl GridWidget {
    fn new(preset: GridPreset) -> GridWidget {
        GridWidget {
            preset,
            shared: Arc::new(Mutex::new(SharedState {
                preset,
                edit_mode: EDIT_OPTIONS
                    .iter()
                    .filter(|edit| preset.intersects(edit.visible_presets))
                    .next()
                    .unwrap()
                    .mode,
                consecutive_moves: true,
                state: None,
                grid: FiniteGrid::zero_size(),
                active_cell: ActiveElement::None,
            })),
        }
    }
}

const CELL_SIZE: f64 = 64.0f64;
const GAP_SIZE: f64 = 2.0f64;

fn screen_to_grid(click_x: f64, click_y: f64) -> Option<(u8, u8)> {
    let stride = CELL_SIZE + GAP_SIZE;

    if click_x < GAP_SIZE || click_y < GAP_SIZE {
        return None;
    }

    let col = (click_x - GAP_SIZE) / stride;
    let row = (click_y - GAP_SIZE) / stride;

    let cell_start_x = GAP_SIZE + (col * stride);
    let cell_start_y = GAP_SIZE + (row * stride);

    let hit_cell_x = click_x >= cell_start_x && click_x < (cell_start_x + CELL_SIZE);
    let hit_cell_y = click_y >= cell_start_y && click_y < (cell_start_y + CELL_SIZE);

    if hit_cell_x && hit_cell_y {
        Some((col as u8, row as u8))
    } else {
        None
    }
}

fn mouse_event_to_grid(event: &MouseEvent) -> Option<(u8, u8)> {
    let click_x = event.offset_x() as f64;
    let click_y = event.offset_y() as f64;
    screen_to_grid(click_x, click_y)
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

fn edge_buttons(
    document: &Document,
    shared: &Arc<Mutex<SharedState>>,
    context: &Context<GridBackendMessage>,
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
            let this = Arc::clone(shared);
            let context = context.clone();
            move || {
                let mut this = this.lock().unwrap();
                let Some(new_grid) = resize_edge(&this.grid, edge, grow) else {
                    return Ok(());
                };

                this.grid = new_grid;
                GridWidget::send_grid(&this, &context);
                GridWidget::draw_grid(&this)
            }
        });
        button.add_event_listener_with_callback("click", handler.as_ref().unchecked_ref())?;
        handler.forget();

        group.append_child(&button)?;
    }

    Ok(group)
}

impl GridWidget {
    // TODO: This whole mess should implement Canvas trait and delegate
    // actual grid/graph drawing there
    fn draw_grid(this: &SharedState) -> Result<(), JsValue> {
        let Some(state) = &this.state else {
            return Ok(());
        };

        let current_mode =
            SelectOption::selected_value(&state.edit_mode.value(), &this.preset, EDIT_OPTIONS);
        if current_mode.map(|o| o.mode) != Some(this.edit_mode)
            && let Some(mode) =
                SelectOption::find_index(|o| o.mode == this.edit_mode, &this.preset, EDIT_OPTIONS)
        {
            state.edit_mode.set_value(&mode.to_string());
        }

        let canvas_width =
            (this.grid.width() as f64 * CELL_SIZE) + ((this.grid.width() + 1) as f64 * GAP_SIZE);
        let canvas_height =
            (this.grid.height() as f64 * CELL_SIZE) + ((this.grid.height() + 1) as f64 * GAP_SIZE);
        state.canvas.set_width(canvas_width as u32);
        state.canvas.set_height(canvas_height as u32);
        state
            .canvas
            .style()
            .set_property("width", &format!("{}px", canvas_width as u32))?;
        state
            .canvas
            .style()
            .set_property("height", &format!("{}px", canvas_height as u32))?;

        let context = state
            .canvas
            .get_context("2d")?
            .unwrap()
            .dyn_into::<CanvasRenderingContext2d>()?;

        context.set_fill_style_str("#000000");
        context.fill_rect(0.0, 0.0, canvas_width, canvas_height);

        for grid_y in 0..this.grid.height() {
            for grid_x in 0..this.grid.width() {
                #[derive(Clone, Copy)]
                enum CellState {
                    None,
                    Hover,
                    Pressed,
                }

                let cell_state = match this.active_cell {
                    ActiveElement::Hover((x, y)) if x == grid_x && y == grid_y => CellState::Hover,
                    ActiveElement::Pressed((x, y)) if x == grid_x && y == grid_y => {
                        CellState::Pressed
                    }
                    _ => CellState::None,
                };

                let tile_color = match this.grid.get(grid_x, grid_y) {
                    Tile::Empty | Tile::BlueStone | Tile::RedStone | Tile::BlackStone => {
                        match cell_state {
                            CellState::None => "#cccccc",
                            CellState::Hover => "#b8b8b8",
                            CellState::Pressed => "#8f8f8f",
                        }
                    }
                    Tile::Taken => match cell_state {
                        CellState::None => "#444444",
                        CellState::Hover => "#3d3d3d",
                        CellState::Pressed => "#303030",
                    },
                };
                context.set_fill_style_str(tile_color);
                let pixel_x = (GAP_SIZE + grid_x as f64 * (CELL_SIZE + GAP_SIZE)) as f64;
                let pixel_y = (GAP_SIZE + grid_y as f64 * (CELL_SIZE + GAP_SIZE)) as f64;
                context.fill_rect(pixel_x, pixel_y, CELL_SIZE, CELL_SIZE);

                if let Some(stone_color) = match this.grid.get(grid_x, grid_y) {
                    Tile::Empty | Tile::Taken => None,
                    Tile::RedStone => Some(match cell_state {
                        CellState::None => "#f92672",
                        CellState::Hover => "#e02267",
                        CellState::Pressed => "#ae1b50",
                    }),
                    Tile::BlueStone => Some(match cell_state {
                        CellState::None => "#4e4afb",
                        CellState::Hover => "#4643e2",
                        CellState::Pressed => "#3734b0",
                    }),
                    Tile::BlackStone => Some(match cell_state {
                        CellState::None => "#444444",
                        CellState::Hover => "#3d3d3d",
                        CellState::Pressed => "#303030",
                    }),
                } {
                    const STONE_SCALE: f64 = 0.35;
                    context.begin_path();
                    context.arc(
                        pixel_x + CELL_SIZE * 0.5,
                        pixel_y + CELL_SIZE * 0.5,
                        CELL_SIZE * STONE_SCALE,
                        0.0,
                        2.0 * f64::consts::PI,
                    )?;
                    context.set_fill_style_str(stone_color);
                    context.fill();

                    context.set_line_width(2.0);
                    context.set_stroke_style_str("#000000");
                    context.stroke();
                }
            }
        }

        Ok(())
    }

    fn send_grid(this: &SharedState, context: &Context<GridBackendMessage>) {
        context.send_message(&GridBackendMessage::SetGrid {
            grid: this.grid.clone(),
        });
    }

    fn handle_edit(
        this: &mut SharedState,
        x: u8,
        y: u8,
        context: &Context<GridBackendMessage>,
    ) -> Result<(), JsValue> {
        match this.edit_mode {
            EditMode::FlipCell => {
                if let Some(flipped_tile) = match this.grid.get(x, y) {
                    Tile::Empty => Some(Tile::Taken),
                    Tile::Taken => Some(Tile::Empty),
                    _ => None,
                } {
                    this.grid.set(x, y, flipped_tile);
                    GridWidget::send_grid(this, context);
                }
            }
            EditMode::PlaceObject(tile) => {
                this.grid.set(x, y, tile);
                GridWidget::send_grid(this, context);
            }
            EditMode::FissionMove(player) => {
                if let Ok(grid) = this.grid.try_map(|t| fission::Tile::try_from(*t)) {
                    let fission = fission::Fission::new(grid);
                    if fission.available_moves(player).contains(&(x, y)) {
                        this.grid = fission.move_in(x, y, player).grid().map(|t| Tile::from(t));
                        GridWidget::send_grid(this, context);

                        if !this.consecutive_moves
                            && let Some(new_mode) = this.edit_mode.opposite_player()
                        {
                            this.edit_mode = new_mode;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

#[derive(Clone, Copy)]
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
                let mut this = self.shared.lock().unwrap();
                this.grid = new_grid;
                GridWidget::draw_grid(&this)
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

        let edit_mode = {
            let mode_box = document.create_element("div")?;

            let mode_select =
                SelectOption::create_element(&document, "Edit mode", &self.preset, EDIT_OPTIONS)?;
            let consecutive_box = document
                .create_element("div")?
                .dyn_into::<HtmlDivElement>()?;
            consecutive_box.style().set_property("display", "none")?;

            // TODO: Auto-generate unique element id to link the label
            let consecutive_moves = document
                .create_element("input")?
                .dyn_into::<HtmlInputElement>()?;
            consecutive_moves.set_type("checkbox");
            let consecutive_label = document
                .create_element("label")?
                .dyn_into::<HtmlLabelElement>()?;
            consecutive_label.set_text_content(Some("Consecutive Moves"));
            consecutive_box.append_child(&consecutive_label)?;
            consecutive_box.append_child(&consecutive_moves)?;

            let on_change_handler = ScopedClosure::<dyn FnMut() -> Result<(), JsValue>>::new({
                let preset = self.preset;
                let this = Arc::clone(&self.shared);
                let mode_select = mode_select.clone();
                let alternate_box = consecutive_box.clone();
                let alternate_checkbox = consecutive_moves.clone();
                move || {
                    if let Some(mode) =
                        SelectOption::selected_value(&mode_select.value(), &preset, EDIT_OPTIONS)
                    {
                        let display = if mode.mode.opposite_player().is_some() {
                            "block"
                        } else {
                            "none"
                        };
                        alternate_box.style().set_property("display", display)?;
                        this.lock().unwrap().edit_mode = mode.mode;
                    };

                    this.lock().unwrap().consecutive_moves = alternate_checkbox.checked();

                    Ok(())
                }
            });
            mode_select.add_event_listener_with_callback(
                "change",
                on_change_handler.as_ref().unchecked_ref(),
            )?;
            consecutive_box.add_event_listener_with_callback(
                "change",
                on_change_handler.as_ref().unchecked_ref(),
            )?;

            on_change_handler.forget();

            mode_box.append_child(&mode_select)?;
            mode_box.append_child(&consecutive_box)?;
            controls.append_child(&mode_box)?;

            mode_select
        };

        element.append_child(&controls).unwrap();

        let canvas = document
            .create_element("canvas")?
            .dyn_into::<HtmlCanvasElement>()?;

        let down_handler = ScopedClosure::<dyn FnMut(MouseEvent) -> Result<(), JsValue>>::new({
            let this = Arc::clone(&self.shared);
            move |event| {
                if let Some((grid_x, grid_y)) = mouse_event_to_grid(&event) {
                    let mut this = this.lock().unwrap();
                    match this.active_cell {
                        // Tiles are clicked, never dragged
                        ActiveElement::None
                        | ActiveElement::Pressed(_)
                        | ActiveElement::Dragging(..) => Ok(()),
                        ActiveElement::Hover(_) => {
                            this.active_cell = ActiveElement::Pressed((grid_x, grid_y));
                            GridWidget::draw_grid(&this)
                        }
                    }
                } else {
                    Ok(())
                }
            }
        });
        canvas
            .add_event_listener_with_callback("mousedown", down_handler.as_ref().unchecked_ref())
            .unwrap();
        down_handler.forget();

        let up_handler = ScopedClosure::<dyn FnMut(MouseEvent) -> Result<(), JsValue>>::new({
            let this = Arc::clone(&self.shared);
            let context = context.clone();
            move |event| {
                let mut this = this.lock().unwrap();
                if let Some((grid_x, grid_y)) = mouse_event_to_grid(&event) {
                    match this.active_cell {
                        ActiveElement::Pressed((x, y)) => {
                            if x == grid_x && y == grid_y {
                                GridWidget::handle_edit(&mut this, x, y, &context)?;
                            }
                        }
                        ActiveElement::None
                        | ActiveElement::Hover(_)
                        | ActiveElement::Dragging(..) => {}
                    }
                    this.active_cell = ActiveElement::Hover((grid_x, grid_y));
                    GridWidget::draw_grid(&this)
                } else {
                    this.active_cell = ActiveElement::None;
                    GridWidget::draw_grid(&this)
                }
            }
        });
        canvas
            .add_event_listener_with_callback("mouseup", up_handler.as_ref().unchecked_ref())
            .unwrap();
        up_handler.forget();

        let move_handler = ScopedClosure::<dyn FnMut(MouseEvent) -> Result<(), JsValue>>::new({
            let this = Arc::clone(&self.shared);
            move |event| {
                let new_hover = mouse_event_to_grid(&event);
                let mut this = this.lock().unwrap();

                match (this.active_cell, new_hover) {
                    (ActiveElement::None, None)
                    | (ActiveElement::Pressed(_) | ActiveElement::Dragging(..), _) => Ok(()),
                    (ActiveElement::Hover(_), None) => {
                        this.active_cell = ActiveElement::None;
                        GridWidget::draw_grid(&this)
                    }
                    (ActiveElement::None | ActiveElement::Hover(_), Some((x, y))) => {
                        this.active_cell = ActiveElement::Hover((x, y));
                        GridWidget::draw_grid(&this)
                    }
                }
            }
        });
        canvas
            .add_event_listener_with_callback("mousemove", move_handler.as_ref().unchecked_ref())
            .unwrap();
        move_handler.forget();

        let leave_handler = ScopedClosure::<dyn FnMut() -> Result<(), JsValue>>::new({
            let this = Arc::clone(&self.shared);
            move || {
                let mut this = this.lock().unwrap();
                if matches!(
                    this.active_cell,
                    ActiveElement::Hover(_) | ActiveElement::Pressed(_)
                ) {
                    this.active_cell = ActiveElement::None;
                    GridWidget::draw_grid(&this)
                } else {
                    Ok(())
                }
            }
        });
        canvas
            .add_event_listener_with_callback("mouseleave", leave_handler.as_ref().unchecked_ref())
            .unwrap();
        leave_handler.forget();

        let board = document
            .create_element("div")?
            .dyn_into::<HtmlDivElement>()?;
        board.style().set_property("display", "grid")?;
        board.style().set_property("width", "fit-content")?;
        board.style().set_property("align-items", "center")?;
        board.style().set_property("justify-items", "center")?;
        board.style().set_property("gap", "4px")?;

        for edge in [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
            let buttons = edge_buttons(&document, &self.shared, &context, edge)?;
            board.append_child(&buttons)?;
        }

        canvas.style().set_property("grid-row", "2")?;
        canvas.style().set_property("grid-column", "2")?;
        board.append_child(&canvas)?;
        element.append_child(&board).unwrap();

        let mut this = self.shared.lock().unwrap();
        this.state = Some(HtmlState { canvas, edit_mode });
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
