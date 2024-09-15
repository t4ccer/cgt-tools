use std::collections::BTreeMap;

use cgt::{
    grid::{small_bit_grid::SmallBitGrid, BitTile, FiniteGrid, Grid},
    numeric::dyadic_rational_number::DyadicRationalNumber,
    short::partizan::{
        canonical_form::CanonicalForm,
        games::domineering::{self, Domineering},
        partizan_game::PartizanGame,
        transposition_table::ParallelTranspositionTable,
    },
};
use raylib::prelude::*;

use text_rectangle_bounds::draw_text_boxed;

mod text_rectangle_bounds;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PanelId {
    value: u64,
}

#[derive(Debug, Clone)]
pub struct Panels {
    next_id: PanelId,
    panels: BTreeMap<PanelId, Panel>,
    panel_chain: Vec<PanelId>,
}

impl Panels {
    pub fn new() -> Panels {
        Panels {
            next_id: PanelId { value: 0 },
            panels: BTreeMap::new(),
            panel_chain: Vec::new(),
        }
    }

    pub fn add_panel(&mut self, panel: Panel) {
        let id = self.next_id;
        self.next_id.value += 1;
        self.add_panel_with_id(panel, id);
    }

    pub fn add_panel_with_id(&mut self, panel: Panel, id: PanelId) {
        self.panel_chain.push(id);
        self.panels.insert(id, panel);
    }

    pub fn move_to_top(&mut self, id: PanelId) {
        if self.panel_chain.last().is_some_and(|last| *last == id) {
            // We are already at the top so nothing more to do
            return;
        }

        let idx = self
            .panel_chain
            .iter()
            .copied()
            .enumerate()
            .find_map(|(idx, panel)| (id == panel).then_some(idx))
            .unwrap();
        self.panel_chain.remove(idx);
        self.panel_chain.push(id);
    }
}

#[derive(Debug, Clone)]
pub struct DomineeringContent {
    domineering: Domineering,
    canonical_form: Option<(CanonicalForm, String)>,
    temperature: Option<(DyadicRationalNumber, String)>,
}

impl DomineeringContent {
    pub fn new(domineering: Domineering) -> DomineeringContent {
        DomineeringContent {
            domineering,
            canonical_form: None,
            temperature: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum PanelContent {
    Domineering(DomineeringContent),
}

#[derive(Debug, Clone)]
pub struct Panel {
    position: Vector2,
    size: Vector2,
    content: PanelContent,

    /// Its own content depends on the content of the parent. If `true` both this panel and
    /// parent panel are immutable
    has_parent: bool,

    sensible_left: Option<Vec<PanelId>>,
    sensible_right: Option<Vec<PanelId>>,
}

impl Panel {
    fn view_rect(&self) -> Rectangle {
        Rectangle {
            x: self.position.x,
            y: self.position.y,
            width: self.size.x,
            height: self.size.y,
        }
    }

    fn full_rect(&self) -> Rectangle {
        Rectangle {
            x: self.position.x,
            y: self.position.y - PANEL_TOP_BAR_HEIGHT,
            width: self.size.x,
            height: self.size.y + PANEL_TOP_BAR_HEIGHT,
        }
    }

    fn title(&self) -> &str {
        match self.content {
            PanelContent::Domineering(_) if self.has_parent || self.has_children() => {
                "Domineering (locked)"
            }
            PanelContent::Domineering(_) => "Domineering",
        }
    }

    fn has_children(&self) -> bool {
        self.sensible_left.is_some() || self.sensible_right.is_some()
    }

    fn copy_standalone(&self) -> Panel {
        Panel {
            position: self.position + Vector2 { x: 50.0, y: -50.0 },
            size: self.size,
            content: self.content.clone(),
            has_parent: false,
            sensible_left: None,
            sensible_right: None,
        }
    }
}

// From https://www.color-hex.com/color-palette/90884
const GUI_DARK_BLUE: Color = Color {
    r: 68,
    g: 82,
    b: 113,
    a: 255,
};
const GUI_GRAY: Color = Color {
    r: 78,
    g: 71,
    b: 89,
    a: 255,
};
const GUI_GREEN: Color = Color {
    r: 86,
    g: 113,
    b: 124,
    a: 255,
};
const GUI_LIGHT_BLUE: Color = Color {
    r: 106,
    g: 137,
    b: 156,
    a: 255,
};
const GUI_RED: Color = Color {
    r: 157,
    g: 113,
    b: 106,
    a: 255,
};

const PANEL_TOP_BAR_HEIGHT: f32 = 20.0;
const PANEL_BORDER_WEIGHT: f32 = 1.0;
const PANEL_FONT_SIZE: i32 = 12;
const PANEL_LEFT_PAD: f32 = 8.0;
const PANEL_TOP_PAD: f32 = 8.0;

const DOMINEERING_TILE_SIZE: f32 = 48.0;
const DOMINEERING_TILE_GAP: f32 = 1.0;

const ZOOM_CAP_LOW: f32 = 0.1;
const ZOOM_SCROLL_SENSITIVITY: f32 = 0.17;

fn main() {
    let tt_domineering = ParallelTranspositionTable::<Domineering>::new();

    let (mut rl, thread) = raylib::init()
        .size(640, 480)
        .title("cgt-gui")
        .resizable()
        .build();

    rl.set_exit_key(None); // don't quit on ESC
    rl.set_target_fps(60);

    let mut cam = Camera2D {
        offset: Vector2::zero(),
        target: Vector2::zero(),
        rotation: 0.0,
        zoom: 1.0,
    };

    let mut panels = Panels::new();
    panels.add_panel(Panel {
        position: Vector2 { x: 200.0, y: 300.0 },
        size: Vector2 { x: 500.0, y: 350.0 },
        content: PanelContent::Domineering(DomineeringContent::new(Domineering::new(
            SmallBitGrid::empty(4, 4).unwrap(),
        ))),
        has_parent: false,
        sensible_left: None,
        sensible_right: None,
    });

    let mut dragged_window: Option<PanelId> = None;
    let mut window_to_focus: Option<PanelId> = None;
    let mut panels_to_add: Vec<(Panel, PanelId)> = Vec::new();
    let mut panels_to_delete: Vec<PanelId> = Vec::new();

    let font = rl
        .load_font_from_memory(&thread, ".ttf", include_bytes!("Inconsolata.ttf"), 48, None)
        .unwrap();

    let mut cursor;

    while !rl.window_should_close() {
        let mut d = rl.begin_drawing(&thread);

        cursor = MouseCursor::MOUSE_CURSOR_DEFAULT;

        d.clear_background(GUI_LIGHT_BLUE);

        let mouse_pos = d.get_screen_to_world2D(d.get_mouse_position(), cam);

        #[cfg(debug_assertions)]
        d.draw_fps(8, 8);

        {
            let mut d = d.begin_mode2D(cam);

            // TODO: This is (nlogn)^2 @perf
            for (panel_idx, panel_id) in panels.panel_chain.iter().copied().enumerate() {
                // If there is any window above current window ignore mouse events
                let is_click_valid =
                    panels.panel_chain[panel_idx + 1..]
                        .iter()
                        .all(|higher_panel_id| {
                            !check_collision_point_rec(
                                mouse_pos,
                                panels.panels.get(higher_panel_id).unwrap().full_rect(),
                            )
                        });

                let panel = panels.panels.get_mut(&panel_id).unwrap();

                macro_rules! button {
                    ($rec:expr, $rec_color:expr, $text:expr $(,)?) => {{
                        let rec = $rec;

                        if is_click_valid && check_collision_point_rec(mouse_pos, $rec) {
                            let c = d.gui_fade($rec_color, 0.7);
                            d.draw_rectangle_rec(rec, c);
                            cursor = MouseCursor::MOUSE_CURSOR_POINTING_HAND;
                        } else {
                            d.draw_rectangle_rec(rec, $rec_color);
                        }

                        d.draw_rectangle_lines_ex(rec, 1.0, Color::BLACK);
                        d.draw_text_ex(
                            &font,
                            $text,
                            Vector2 {
                                x: rec.x + 5.0,
                                y: rec.y + rec.height * 0.5 - (PANEL_FONT_SIZE as f32) * 0.5,
                            },
                            PANEL_FONT_SIZE as f32,
                            1.0,
                            Color::BLACK,
                        );

                        is_click_valid
                            && check_collision_point_rec(mouse_pos, rec)
                            && d.is_mouse_button_released(MouseButton::MOUSE_BUTTON_LEFT)
                    }};
                }

                if is_click_valid
                    && d.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT)
                    && check_collision_point_rec(mouse_pos, panel.full_rect())
                {
                    window_to_focus = Some(panel_id);
                }

                let top_bar = Rectangle {
                    x: panel.position.x,
                    y: panel.position.y - PANEL_TOP_BAR_HEIGHT,
                    width: panel.size.x,
                    height: PANEL_TOP_BAR_HEIGHT,
                };

                let game_panel_start = Vector2 {
                    x: panel.position.x + PANEL_LEFT_PAD,
                    y: panel.position.y + PANEL_TOP_PAD,
                };

                let game_panel_size = match &panel.content {
                    PanelContent::Domineering(content) => Vector2 {
                        x: content.domineering.grid().width() as f32
                            * (DOMINEERING_TILE_SIZE + DOMINEERING_TILE_GAP)
                            - DOMINEERING_TILE_GAP,

                        y: content.domineering.grid().width() as f32
                            * (DOMINEERING_TILE_SIZE + DOMINEERING_TILE_GAP)
                            - DOMINEERING_TILE_GAP,
                    },
                };

                let panel_view_rect = panel.view_rect();

                d.draw_rectangle_rec(
                    Rectangle {
                        x: top_bar.x - PANEL_BORDER_WEIGHT,
                        y: top_bar.y - PANEL_BORDER_WEIGHT,
                        width: top_bar.width + PANEL_BORDER_WEIGHT * 2.0,
                        height: top_bar.height
                            + panel.view_rect().height
                            + PANEL_BORDER_WEIGHT * 2.0,
                    },
                    Color::BLACK,
                );

                d.draw_rectangle_rec(top_bar, GUI_DARK_BLUE);
                d.draw_rectangle_rec(panel.view_rect(), Color::RAYWHITE);

                d.draw_text_ex(
                    &font,
                    panel.title(),
                    Vector2 {
                        x: game_panel_start.x,
                        y: top_bar.y + PANEL_FONT_SIZE as f32 * 0.4,
                    },
                    PANEL_FONT_SIZE as f32,
                    1.0,
                    Color::BLACK,
                );

                let panel_has_children = panel.has_children();
                match &mut panel.content {
                    PanelContent::Domineering(content) => {
                        let mut is_dirty = false;

                        for y in 0..content.domineering.grid().height() {
                            for x in 0..content.domineering.grid().width() {
                                let mut tile_color = match content.domineering.grid().get(x, y) {
                                    domineering::Tile::Empty => Color::get_color(0xccccccff),
                                    domineering::Tile::Taken => Color::get_color(0x444444ff),
                                };

                                let tile_pos = game_panel_start
                                    + Vector2 {
                                        x: x as f32
                                            * (DOMINEERING_TILE_SIZE + DOMINEERING_TILE_GAP),
                                        y: y as f32
                                            * (DOMINEERING_TILE_SIZE + DOMINEERING_TILE_GAP),
                                    };
                                let tile_size = Vector2 {
                                    x: DOMINEERING_TILE_SIZE,
                                    y: DOMINEERING_TILE_SIZE,
                                };

                                if is_click_valid
                                    && check_collision_point_rec_v(mouse_pos, tile_pos, tile_size)
                                    && !panel.has_parent
                                    && !panel_has_children
                                {
                                    tile_color = d.gui_fade(tile_color, 0.8);
                                    cursor = MouseCursor::MOUSE_CURSOR_POINTING_HAND;
                                    if d.is_mouse_button_released(MouseButton::MOUSE_BUTTON_LEFT) {
                                        let new_tile = content.domineering.grid().get(x, y).flip();
                                        content.domineering.grid_mut().set(x, y, new_tile);
                                        is_dirty = true;
                                    }
                                }

                                d.draw_rectangle_v(tile_pos, tile_size, tile_color);
                            }
                        }

                        if is_dirty {
                            content.canonical_form = None;
                            content.temperature = None;
                        }

                        // TODO: Move to worker thread @perf
                        if content.canonical_form.is_none()
                            || content.temperature.is_none()
                            || is_dirty
                        {
                            let cf = content.domineering.canonical_form(&tt_domineering);
                            let temp = cf.temperature();
                            let cf_text = format!("Position: {cf}");
                            let temp_text = format!("Temperature: {temp}");
                            content.canonical_form = Some((cf, cf_text));
                            content.temperature = Some((temp, temp_text));
                        }

                        let text_x = (game_panel_start + game_panel_size).x + PANEL_LEFT_PAD;
                        let text_y = game_panel_start.y + PANEL_TOP_PAD;
                        let prev_text = draw_text_boxed(
                            &mut d,
                            &font,
                            match &content.canonical_form {
                                Some((_, v)) => &v,
                                None => "Position: ???",
                            },
                            Rectangle {
                                x: text_x,
                                y: text_y,
                                width: panel_view_rect.width - text_x + panel_view_rect.x,
                                height: panel_view_rect.height - text_y + panel_view_rect.y,
                            },
                            PANEL_FONT_SIZE as f32,
                            1.0,
                            false,
                            Color::BLACK,
                        );

                        let _prev_text = draw_text_boxed(
                            &mut d,
                            &font,
                            match &content.temperature {
                                Some((_, v)) => &v,
                                None => "Temperature: ???",
                            },
                            Rectangle {
                                x: text_x,
                                y: prev_text.y,
                                width: panel_view_rect.width - text_x + panel_view_rect.x,
                                height: panel_view_rect.height - prev_text.y + panel_view_rect.y,
                            },
                            PANEL_FONT_SIZE as f32,
                            1.0,
                            false,
                            Color::BLACK,
                        );

                        const PANEL_BUTTON_WIDTH: f32 = 110.0;
                        const PANEL_BUTTON_HEIGHT: f32 = 25.0;
                        const PANEL_BUTTON_GAP: f32 = 10.0;
                        const PANEL_BUTTON_PADDING: f32 = 10.0;

                        if panel.sensible_left.is_none()
                            && button!(
                                Rectangle {
                                    x: panel_view_rect.x
                                        + PANEL_BUTTON_PADDING
                                        + (PANEL_BUTTON_WIDTH + PANEL_BUTTON_GAP) * 0.0,
                                    y: panel_view_rect.y + panel_view_rect.height
                                        - PANEL_BUTTON_PADDING
                                        - PANEL_BUTTON_HEIGHT,
                                    width: PANEL_BUTTON_WIDTH,
                                    height: PANEL_BUTTON_HEIGHT,
                                },
                                GUI_GREEN,
                                "Sensible Left",
                            )
                        {
                            let mut sensible_left = Vec::new();
                            for (idx, left_move) in content
                                .domineering
                                .sensible_left_moves(&tt_domineering)
                                .into_iter()
                                .enumerate()
                            {
                                let panel = Panel {
                                    position: Vector2 {
                                        x: panel_view_rect.x
                                            - (panel_view_rect.width + 10.0) * (idx as f32 + 0.6),
                                        y: panel_view_rect.y + panel_view_rect.height + 200.0,
                                    },
                                    size: Vector2 { x: 500.0, y: 350.0 },
                                    content: PanelContent::Domineering(DomineeringContent {
                                        domineering: left_move,
                                        canonical_form: None,
                                        temperature: None,
                                    }),
                                    has_parent: true,
                                    sensible_left: None,
                                    sensible_right: None,
                                };
                                let id = panels.next_id;
                                panels.next_id.value += 1;
                                panels_to_add.push((panel, id));
                                sensible_left.push(id);
                            }
                            panel.sensible_left = Some(sensible_left);
                        }

                        if panel.sensible_right.is_none()
                            && button!(
                                Rectangle {
                                    x: panel_view_rect.x
                                        + PANEL_BUTTON_PADDING
                                        + (PANEL_BUTTON_WIDTH + PANEL_BUTTON_GAP) * 1.0,
                                    y: panel_view_rect.y + panel_view_rect.height
                                        - PANEL_BUTTON_PADDING
                                        - PANEL_BUTTON_HEIGHT,
                                    width: PANEL_BUTTON_WIDTH,
                                    height: PANEL_BUTTON_HEIGHT,
                                },
                                GUI_GREEN,
                                "Sensible Right",
                            )
                        {
                            let mut sensible_right = Vec::new();
                            for (idx, right_move) in content
                                .domineering
                                .sensible_right_moves(&tt_domineering)
                                .into_iter()
                                .enumerate()
                            {
                                let panel = Panel {
                                    position: Vector2 {
                                        x: panel_view_rect.x
                                            + (panel_view_rect.width + 10.0) * (idx as f32 + 0.6),
                                        y: panel_view_rect.y + panel_view_rect.height + 200.0,
                                    },
                                    size: Vector2 { x: 500.0, y: 350.0 },
                                    content: PanelContent::Domineering(DomineeringContent {
                                        domineering: right_move,
                                        canonical_form: None,
                                        temperature: None,
                                    }),
                                    has_parent: true,
                                    sensible_left: None,
                                    sensible_right: None,
                                };
                                let id = panels.next_id;
                                panels.next_id.value += 1;
                                panels_to_add.push((panel, id));
                                sensible_right.push(id);
                            }
                            panel.sensible_right = Some(sensible_right);
                        }

                        if button!(
                            Rectangle {
                                x: panel_view_rect.x
                                    + PANEL_BUTTON_PADDING
                                    + (PANEL_BUTTON_WIDTH + PANEL_BUTTON_GAP) * 0.0,
                                y: panel_view_rect.y + panel_view_rect.height
                                    - (PANEL_BUTTON_PADDING + PANEL_BUTTON_HEIGHT) * 2.0,
                                width: PANEL_BUTTON_WIDTH,
                                height: PANEL_BUTTON_HEIGHT,
                            },
                            GUI_GREEN,
                            "Duplicate",
                        ) {
                            let id = panels.next_id;
                            panels.next_id.value += 1;
                            panels_to_add.push((panel.copy_standalone(), id));
                        }

                        if !panel.has_parent
                            && button!(
                                Rectangle {
                                    x: panel_view_rect.x + panel_view_rect.width
                                        - PANEL_BUTTON_PADDING
                                        - (PANEL_BUTTON_WIDTH),
                                    y: panel_view_rect.y + panel_view_rect.height
                                        - (PANEL_BUTTON_PADDING + PANEL_BUTTON_HEIGHT) * 1.0,
                                    width: PANEL_BUTTON_WIDTH,
                                    height: PANEL_BUTTON_HEIGHT,
                                },
                                GUI_RED,
                                "Delete",
                            )
                        {
                            panels_to_delete.push(panel_id);
                        }

                        if !panel.has_parent
                            && button!(
                                Rectangle {
                                    x: panel_view_rect.x + panel_view_rect.width
                                        - PANEL_BUTTON_PADDING
                                        - (PANEL_BUTTON_WIDTH),
                                    y: panel_view_rect.y + panel_view_rect.height
                                        - (PANEL_BUTTON_PADDING + PANEL_BUTTON_HEIGHT) * 2.0,
                                    width: PANEL_BUTTON_WIDTH,
                                    height: PANEL_BUTTON_HEIGHT,
                                },
                                GUI_RED,
                                "Delete Recursive",
                            )
                        {
                            panels_to_delete.push(panel_id);
                            panels_to_delete.extend(
                                panel
                                    .sensible_left
                                    .iter()
                                    .flatten()
                                    .chain(panel.sensible_right.iter().flatten()),
                            );
                        }
                    }
                }

                if is_click_valid
                    && check_collision_point_rec(mouse_pos, top_bar)
                    && dragged_window.is_none()
                {
                    cursor = MouseCursor::MOUSE_CURSOR_RESIZE_ALL;

                    if d.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
                        dragged_window = Some(panel_id);
                    }
                } else if !d.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT) {
                    dragged_window = None;
                }

                // NOTE: We don't check `is_click_valid` here because window is already
                // grabbed so we want to move it over new windows
                if d.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT)
                    && dragged_window == Some(panel_id)
                {
                    cursor = MouseCursor::MOUSE_CURSOR_RESIZE_ALL;
                    let delta = d.get_mouse_delta() * (1.0 / cam.zoom);
                    panel.position.x += delta.x;
                    panel.position.y += delta.y;
                }

                let panel = panels.panels.get(&panel_id).unwrap();
                if let Some(sensible_lefts) = &panel.sensible_left {
                    for sensible_left_id in sensible_lefts.iter() {
                        // NOTE: It'll be None on first iteration before we add everything from
                        // `panels_to_add`
                        if let Some(sensible_left) = panels.panels.get(sensible_left_id) {
                            let panel_rect = panel.full_rect();
                            let child_rect = sensible_left.full_rect();
                            d.draw_line_v(
                                Vector2 {
                                    x: panel_rect.x + panel_rect.width * 0.3,
                                    y: panel_rect.y + panel_rect.height,
                                },
                                Vector2 {
                                    x: child_rect.x + child_rect.width * 0.5,
                                    y: child_rect.y,
                                },
                                Color::BLUE,
                            );
                        }
                    }
                }
                if let Some(sensible_rights) = &panel.sensible_right {
                    for sensible_right_id in sensible_rights.iter() {
                        // NOTE: It'll be None on first iteration before we add everything from
                        // `panels_to_add`
                        if let Some(sensible_right) = panels.panels.get(sensible_right_id) {
                            let panel_rect = panel.full_rect();
                            let child_rect = sensible_right.full_rect();
                            d.draw_line_v(
                                Vector2 {
                                    x: panel_rect.x + panel_rect.width * 0.7,
                                    y: panel_rect.y + panel_rect.height,
                                },
                                Vector2 {
                                    x: child_rect.x + child_rect.width * 0.5,
                                    y: child_rect.y,
                                },
                                Color::RED,
                            );
                        }
                    }
                }
            }

            if let Some(w) = window_to_focus.take() {
                panels.move_to_top(w);
            }

            for (panel, id) in panels_to_add.drain(..) {
                panels.add_panel_with_id(panel, id);
            }

            for panel_id in panels_to_delete.drain(..) {
                if let Some(panel) = panels.panels.remove(&panel_id) {
                    for child_id in panel
                        .sensible_left
                        .iter()
                        .flatten()
                        .chain(panel.sensible_right.iter().flatten())
                    {
                        if let Some(child) = panels.panels.get_mut(&child_id) {
                            child.has_parent = false;
                        }
                    }

                    if let Some(panel_idx) = panels
                        .panel_chain
                        .iter()
                        .enumerate()
                        .find_map(|(idx, id)| (*id == panel_id).then_some(idx))
                    {
                        panels.panel_chain.remove(panel_idx);
                    }
                }
            }

            if d.is_mouse_button_down(MouseButton::MOUSE_BUTTON_MIDDLE) {
                cursor = MouseCursor::MOUSE_CURSOR_RESIZE_ALL;
                cam.offset += d.get_mouse_delta();
            }

            let scroll = d.get_mouse_wheel_move_v().y;
            if scroll != 0.0 {
                let mut scale_factor = 1.0 + (scroll.abs() * ZOOM_SCROLL_SENSITIVITY);
                if scroll < 0.0 {
                    scale_factor = 1.0 / scale_factor;
                }
                cam.offset = d.get_mouse_position();
                cam.target = mouse_pos;

                cam.zoom = cam.zoom * scale_factor;

                if cam.zoom <= ZOOM_CAP_LOW {
                    cam.zoom = ZOOM_CAP_LOW;
                }
            }

            d.set_mouse_cursor(cursor);
        }

        if d.gui_button(
            Rectangle {
                x: 8.0,
                y: 30.0,
                width: 120.0,
                height: 24.0,
            },
            Some(rstr!("Domineering")),
        ) {
            panels.add_panel(Panel {
                position: mouse_pos,
                size: Vector2 { x: 500.0, y: 350.0 },
                content: PanelContent::Domineering(DomineeringContent::new(Domineering::new(
                    SmallBitGrid::empty(4, 4).unwrap(),
                ))),
                has_parent: false,
                sensible_left: None,
                sensible_right: None,
            });
        }
    }
}

fn check_collision_point_rec(point: Vector2, rec: Rectangle) -> bool {
    (point.x >= rec.x)
        && (point.x <= (rec.x + rec.width))
        && (point.y >= rec.y)
        && (point.y <= (rec.y + rec.height))
}

fn check_collision_point_rec_v(point: Vector2, pos: Vector2, size: Vector2) -> bool {
    (point.x >= pos.x)
        && (point.x <= (pos.x + size.x))
        && (point.y >= pos.y)
        && (point.y <= (pos.y + size.y))
}
