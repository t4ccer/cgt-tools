use cgt::{
    drawing::{Button, Canvas, Color, Hits, Interactions},
    grid::{FiniteGrid, Grid, vec_grid::VecGrid},
    numeric::v2f::V2f,
    short::partizan::{Player, games::fission},
};
use cgt_py_messages::{GridBackendMessage, GridFrontendMessage, GridPreset, GridPresetFlag, Tile};
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

use crate::{SelectOption, SelectOptionElement, canvas::HtmlCanvas};

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
    interactions: Interactions,
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
                interactions: Interactions::new(),
            })),
        }
    }
}

fn mouse_event_to_canvas(event: &MouseEvent) -> V2f {
    V2f {
        x: event.offset_x() as f32,
        y: event.offset_y() as f32,
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
                GridWidget::draw(&mut this)?;

                Ok(())
            }
        });
        button.add_event_listener_with_callback("click", handler.as_ref().unchecked_ref())?;
        handler.forget();

        group.append_child(&button)?;
    }

    Ok(group)
}

impl GridWidget {
    /// Keep the mode dropdown showing the mode that is actually in effect, which changes on
    /// its own when a move is made in a game that alternates players
    fn sync_edit_mode(this: &SharedState) {
        let Some(state) = &this.state else {
            return;
        };

        let current_mode =
            SelectOption::selected_value(&state.edit_mode.value(), &this.preset, EDIT_OPTIONS);
        if current_mode.map(|o| o.mode) != Some(this.edit_mode)
            && let Some(mode) =
                SelectOption::find_index(|o| o.mode == this.edit_mode, &this.preset, EDIT_OPTIONS)
        {
            state.edit_mode.set_value(&mode.to_string());
        }
    }

    /// Paint the grid and report what the mouse did to its tiles
    fn draw(this: &mut SharedState) -> Result<Hits<(u8, u8)>, JsValue> {
        GridWidget::sync_edit_mode(this);

        let SharedState {
            state,
            grid,
            interactions,
            ..
        } = this;

        let Some(state) = state else {
            return Ok(Hits::new());
        };

        let canvas_size = grid.canvas_size::<HtmlCanvas>().size();
        state.canvas.set_width(canvas_size.x as u32);
        state.canvas.set_height(canvas_size.y as u32);
        state
            .canvas
            .style()
            .set_property("width", &format!("{}px", canvas_size.x as u32))?;
        state
            .canvas
            .style()
            .set_property("height", &format!("{}px", canvas_size.y as u32))?;

        let context = state
            .canvas
            .get_context("2d")?
            .unwrap()
            .dyn_into::<CanvasRenderingContext2d>()?;

        let grid: &VecGrid<Tile> = grid;
        let mut canvas = HtmlCanvas::new(context, interactions);
        Ok(canvas.frame(|canvas| {
            // The tiles do not cover the gaps between them, which the grid lines are drawn in
            canvas.rect(V2f::ZERO, canvas_size, Color::BLACK);
            grid.draw(canvas, Tile::drawing)
        }))
    }

    fn send_grid(this: &SharedState, context: &Context<GridBackendMessage>) {
        context.send_message(&GridBackendMessage::SetGrid {
            grid: this.grid.clone(),
        });
    }

    /// Apply what the mouse did to the grid, reporting whether it changed. Tiles are
    /// clicked, never dragged
    fn apply(this: &mut SharedState, hits: &Hits<(u8, u8)>) -> bool {
        let Some((x, y)) = hits.clicked else {
            return false;
        };

        match this.edit_mode {
            EditMode::FlipCell => match this.grid.get(x, y) {
                Tile::Empty => {
                    this.grid.set(x, y, Tile::Taken);
                    true
                }
                Tile::Taken => {
                    this.grid.set(x, y, Tile::Empty);
                    true
                }
                Tile::BlueStone | Tile::RedStone | Tile::BlackStone => false,
            },
            EditMode::PlaceObject(tile) => {
                this.grid.set(x, y, tile);
                true
            }
            EditMode::FissionMove(player) => {
                let Ok(grid) = this.grid.try_map(|t| fission::Tile::try_from(*t)) else {
                    return false;
                };

                let fission = fission::Fission::new(grid);
                if !fission.available_moves(player).contains(&(x, y)) {
                    return false;
                }

                this.grid = fission.move_in(x, y, player).grid().map(|t| Tile::from(t));
                if !this.consecutive_moves
                    && let Some(new_mode) = this.edit_mode.opposite_player()
                {
                    this.edit_mode = new_mode;
                }
                true
            }
        }
    }

    /// Paint a frame, apply what the mouse did to it, and paint it again if that changed
    /// anything
    fn update(
        this: &mut SharedState,
        context: &Context<GridBackendMessage>,
    ) -> Result<(), JsValue> {
        // Clicks are reported by a single frame and consumed by it, so the shading of
        // whatever was pressed has to be painted over even when nothing else changed
        let settling = matches!(this.interactions.pointer().button, Button::Released(_));

        let hits = GridWidget::draw(this)?;
        let changed = GridWidget::apply(this, &hits);
        if changed {
            GridWidget::send_grid(this, context);
        }
        if changed || settling {
            GridWidget::draw(this)?;
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
                GridWidget::draw(&mut this)?;

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
            let context = context.clone();
            move |event: MouseEvent| {
                if event.button() != 0 {
                    return Ok(());
                }

                let mut this = this.lock().unwrap();
                this.interactions
                    .pointer_pressed(mouse_event_to_canvas(&event));
                GridWidget::update(&mut this, &context)
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
                this.interactions
                    .pointer_released(mouse_event_to_canvas(&event));
                GridWidget::update(&mut this, &context)
            }
        });
        canvas
            .add_event_listener_with_callback("mouseup", up_handler.as_ref().unchecked_ref())
            .unwrap();
        up_handler.forget();

        let move_handler = ScopedClosure::<dyn FnMut(MouseEvent) -> Result<(), JsValue>>::new({
            let this = Arc::clone(&self.shared);
            let context = context.clone();
            move |event| {
                let mut this = this.lock().unwrap();
                this.interactions
                    .pointer_moved(mouse_event_to_canvas(&event));
                GridWidget::update(&mut this, &context)
            }
        });
        canvas
            .add_event_listener_with_callback("mousemove", move_handler.as_ref().unchecked_ref())
            .unwrap();
        move_handler.forget();

        let leave_handler = ScopedClosure::<dyn FnMut() -> Result<(), JsValue>>::new({
            let this = Arc::clone(&self.shared);
            let context = context.clone();
            move || {
                let mut this = this.lock().unwrap();
                this.interactions.pointer_left();
                GridWidget::update(&mut this, &context)
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
