use cgt::{
    graph::{Graph, VertexIndex},
    grid::{BitTile, FiniteGrid, Grid},
    has::Has,
    numeric::{rational::Rational, v2f::V2f},
    short::partizan::{thermograph::Thermograph, trajectory::Trajectory},
};
use imgui::{DrawListMut, ImColor32, MouseButton, StyleColor, Ui};
use std::fmt::Write;

pub mod canonical_form;
pub mod digraph_placement;
pub mod domineering;
pub mod snort;

pub const THERMOGRAPH_TOP_MAST_LEN: f32 = 1.0;
pub const THERMOGRAPH_AXIS_PAD: f32 = 0.5;
pub const THERMOGRAPH_ARROW_SIZE: f32 = 0.15;
pub const THERMOGRAPH_TRAJECTORY_THICKNESS: f32 = 2.0;
pub const THERMOGRAPH_AXIS_THICKNESS: f32 = 1.0;

pub const DOMINEERING_TILE_SIZE: f32 = 64.0;
pub const DOMINEERING_TILE_GAP: f32 = 4.0;
pub const DOMINEERING_EMPTY_COLOR: ImColor32 = ImColor32::from_rgb(0xcc, 0xcc, 0xcc);
pub const DOMINEERING_FILLED_COLOR: ImColor32 = ImColor32::from_rgb(0x44, 0x44, 0x44);

pub const COLOR_BLUE: ImColor32 = ImColor32::from_bits(0xfffb4a4e);
pub const COLOR_RED: ImColor32 = ImColor32::from_bits(0xff7226f9);

const VERTEX_RADIUS: f32 = 16.0;
const ARROW_HEAD_SIZE: f32 = 4.0;

fn fade(mut color: [f32; 4], alpha: f32) -> [f32; 4] {
    let alpha = alpha.clamp(0.0, 1.0);
    color[3] *= alpha;
    color
}

fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start + t * (end - start)
}

// FIXME: Height is too large if Y axis is not visible
fn thermograph_size(thermograph: &Thermograph) -> V2f {
    let left_x = thermograph.left_wall.value_at(Rational::from(-1));
    let right_x = thermograph.right_wall.value_at(Rational::from(-1));
    let x_len = (left_x - right_x).as_f32().unwrap();
    let y_top_above_x_axis = thermograph
        .left_wall
        .critical_points
        .first()
        .copied()
        .and_then(Rational::as_f32)
        .unwrap_or(0.0);

    V2f {
        x: x_len + THERMOGRAPH_AXIS_PAD * 2.0,
        y: y_top_above_x_axis + 1.0 + THERMOGRAPH_TOP_MAST_LEN + THERMOGRAPH_AXIS_PAD * 2.0, // +1.0 to go up to -1 below y axis,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ScafoldPlayer {
    Left,
    Right,
}

pub fn thermograph<'ui>(
    ui: &'ui imgui::Ui,
    draw_list: &'ui imgui::DrawListMut<'ui>,
    thermograph_scale: f32,
    scratch_buffer: &mut String,
    thermograph: &Thermograph,
) {
    let [pos_x, pos_y] = ui.cursor_screen_pos();

    let left_x = thermograph.left_wall.value_at(Rational::from(-1));
    let right_x = thermograph.right_wall.value_at(Rational::from(-1));
    let x_len = (left_x - right_x).as_f32().unwrap();

    let y_top_above_x_axis = thermograph
        .left_wall
        .critical_points
        .first()
        .copied()
        .and_then(Rational::as_f32)
        .unwrap_or(0.0);

    let axis_color = ui.style_color(StyleColor::TextDisabled);

    // x axis
    draw_list
        .add_line(
            [
                pos_x,
                pos_y
                    + (y_top_above_x_axis + THERMOGRAPH_TOP_MAST_LEN + THERMOGRAPH_AXIS_PAD)
                        * thermograph_scale,
            ],
            [
                pos_x + thermograph_scale * (x_len + THERMOGRAPH_AXIS_PAD * 2.0),
                pos_y
                    + (y_top_above_x_axis + THERMOGRAPH_TOP_MAST_LEN + THERMOGRAPH_AXIS_PAD)
                        * thermograph_scale,
            ],
            axis_color,
        )
        .thickness(THERMOGRAPH_AXIS_THICKNESS)
        .build();

    let left_x = left_x.as_f32().unwrap();
    let right_x = right_x.as_f32().unwrap();

    // y axis
    // Don't draw vertical axis if it's not visible
    if left_x >= 0.0 && right_x <= 0.0 {
        let y_axis_loc = lerp(left_x, right_x, 0.0);

        draw_list
            .add_line(
                [
                    pos_x + (THERMOGRAPH_AXIS_PAD + y_axis_loc) * thermograph_scale,
                    pos_y,
                ],
                [
                    pos_x + (THERMOGRAPH_AXIS_PAD + y_axis_loc) * thermograph_scale,
                    pos_y
                        + (y_top_above_x_axis
                            + 1.0 // We need to go up to -1
                            + THERMOGRAPH_TOP_MAST_LEN
                            + THERMOGRAPH_AXIS_PAD * 2.0)
                            * thermograph_scale,
                ],
                axis_color,
            )
            .thickness(THERMOGRAPH_AXIS_THICKNESS)
            .build();
    }

    let x_offset = thermograph
        .left_wall
        .value_at(Rational::from(-1))
        .as_f32()
        .unwrap();

    draw_trajectory(
        ui,
        &draw_list,
        [pos_x, pos_y],
        x_offset,
        thermograph_scale,
        scratch_buffer,
        &thermograph.left_wall,
        ScafoldPlayer::Left,
    );
    draw_trajectory(
        ui,
        &draw_list,
        [pos_x, pos_y],
        x_offset,
        thermograph_scale,
        scratch_buffer,
        &thermograph.right_wall,
        ScafoldPlayer::Right,
    );
}

fn draw_trajectory<'ui>(
    ui: &'ui imgui::Ui,
    draw_list: &'ui imgui::DrawListMut<'ui>,
    [pos_x, pos_y]: [f32; 2],
    x_offset: f32,
    thermograph_scale: f32,
    scratch_buffer: &mut String,
    trajectory: &Trajectory,
    side: ScafoldPlayer,
) {
    let y_top_above_x_axis = trajectory
        .critical_points
        .first()
        .copied()
        .and_then(Rational::as_f32)
        .unwrap_or(0.0);

    // We start drawing from the top so we initialize prev_{x,y} as top of the mast arrow
    let mut prev_x = trajectory.mast_x_intercept().as_f32().unwrap();
    let mut prev_y = y_top_above_x_axis + THERMOGRAPH_TOP_MAST_LEN;

    let arrow_top_point = [
        pos_x + (THERMOGRAPH_AXIS_PAD + x_offset - prev_x) * thermograph_scale,
        pos_y
            + (THERMOGRAPH_AXIS_PAD + y_top_above_x_axis + THERMOGRAPH_TOP_MAST_LEN - prev_y)
                * thermograph_scale,
    ];
    let arrow_left_point = [
        pos_x
            + (THERMOGRAPH_AXIS_PAD + x_offset - prev_x - THERMOGRAPH_ARROW_SIZE)
                * thermograph_scale,
        pos_y
            + (THERMOGRAPH_AXIS_PAD + y_top_above_x_axis + THERMOGRAPH_TOP_MAST_LEN - prev_y
                + THERMOGRAPH_ARROW_SIZE)
                * thermograph_scale,
    ];
    let arrow_right_point = [
        pos_x
            + (THERMOGRAPH_AXIS_PAD + x_offset - prev_x + THERMOGRAPH_ARROW_SIZE)
                * thermograph_scale,
        pos_y
            + (THERMOGRAPH_AXIS_PAD + y_top_above_x_axis + THERMOGRAPH_TOP_MAST_LEN - prev_y
                + THERMOGRAPH_ARROW_SIZE)
                * thermograph_scale,
    ];

    let trajectory_color = ui.style_color(StyleColor::Text);

    draw_list
        .add_line(arrow_top_point, arrow_left_point, trajectory_color)
        .thickness(THERMOGRAPH_TRAJECTORY_THICKNESS)
        .build();
    draw_list
        .add_line(arrow_top_point, arrow_right_point, trajectory_color)
        .thickness(THERMOGRAPH_TRAJECTORY_THICKNESS)
        .build();

    for (point_idx, this_y_r) in trajectory
        .critical_points
        .iter()
        .copied()
        .chain(std::iter::once(Rational::from(-1)))
        .enumerate()
    {
        let this_x_r = trajectory.value_at(this_y_r);

        let this_y = this_y_r.as_f32().unwrap();
        let this_x = this_x_r.as_f32().unwrap();

        let prev_point = [
            pos_x + (THERMOGRAPH_AXIS_PAD + x_offset - prev_x) * thermograph_scale,
            pos_y
                + (THERMOGRAPH_AXIS_PAD + y_top_above_x_axis + THERMOGRAPH_TOP_MAST_LEN - prev_y)
                    * thermograph_scale,
        ];
        let this_point = [
            pos_x + (THERMOGRAPH_AXIS_PAD + x_offset - this_x) * thermograph_scale,
            pos_y
                + (THERMOGRAPH_AXIS_PAD + y_top_above_x_axis + THERMOGRAPH_TOP_MAST_LEN - this_y)
                    * thermograph_scale,
        ];

        // Skip highest point when drawing left side - point is the same as that is the
        // intersection but label is in different place as we offset right label to the left
        // to fit within thermograph bounds
        if point_idx > 0 || matches!(side, ScafoldPlayer::Right) {
            scratch_buffer.clear();
            scratch_buffer
                .write_fmt(format_args!("({this_x_r}, {this_y_r})"))
                .unwrap();
            if matches!(side, ScafoldPlayer::Right) {
                let [point_label_width, _] = ui.calc_text_size(&scratch_buffer);
                ui.set_cursor_screen_pos([this_point[0] - point_label_width, this_point[1]]);
            } else {
                ui.set_cursor_screen_pos(this_point);
            }
            ui.text(&scratch_buffer);
        }

        draw_list
            .add_line(prev_point, this_point, trajectory_color)
            .thickness(THERMOGRAPH_TRAJECTORY_THICKNESS)
            .build();

        prev_x = this_x;
        prev_y = this_y;
    }
}

pub fn grid_size_selector(ui: &imgui::Ui, new_width: &mut u8, new_height: &mut u8) {
    let short_inputs = ui.push_item_width(100.0);
    ui.input_scalar("Width", new_width).step(1).build();
    ui.input_scalar("Height", new_height).step(1).build();
    short_inputs.end();
}

pub fn bit_grid<'ui, G>(ui: &'ui imgui::Ui, draw_list: &'ui DrawListMut<'ui>, grid: &mut G) -> bool
where
    G: Grid + FiniteGrid,
    G::Item: BitTile,
{
    let mut is_dirty = false;

    let width = grid.width();
    let height = grid.height();

    let [grid_start_pos_x, grid_start_pos_y] = ui.cursor_pos();

    for grid_y in 0..height {
        let _y_id = ui.push_id_usize(grid_y as usize);

        for grid_x in 0..width {
            let _x_id = ui.push_id_usize(grid_x as usize);

            ui.set_cursor_pos([
                grid_start_pos_x + (DOMINEERING_TILE_SIZE + DOMINEERING_TILE_GAP) * grid_x as f32,
                grid_start_pos_y + (DOMINEERING_TILE_SIZE + DOMINEERING_TILE_GAP) * grid_y as f32,
            ]);

            let [pos_x, pos_y] = ui.cursor_screen_pos();
            if ui.invisible_button("", [DOMINEERING_TILE_SIZE, DOMINEERING_TILE_SIZE]) {
                is_dirty = true;
                let flipped = grid.get(grid_x, grid_y).flip();
                grid.set(grid_x, grid_y, flipped);
            }

            let color = match grid.get(grid_x, grid_y).tile_to_bool() {
                false => DOMINEERING_EMPTY_COLOR,
                true => DOMINEERING_FILLED_COLOR,
            };
            let color = color.to_rgba_f32s();

            let color = if ui.is_item_active() {
                fade(color, 0.6)
            } else if ui.is_item_hovered() {
                fade(color, 0.8)
            } else {
                color
            };

            draw_list
                .add_rect(
                    [pos_x, pos_y],
                    [pos_x + DOMINEERING_TILE_SIZE, pos_y + DOMINEERING_TILE_SIZE],
                    color,
                )
                .filled(true)
                .build();
        }
    }

    ui.set_cursor_pos([
        grid_start_pos_x,
        grid_start_pos_y + (DOMINEERING_TILE_SIZE + DOMINEERING_TILE_GAP) * height as f32,
    ]);

    is_dirty
}

macro_rules! game_details {
    ($self:expr, $ui:expr, $draw_list:expr) => {{
        if let Some(details) = $self.content.details.as_ref() {
            $ui.text_wrapped(&details.canonical_form_rendered);
            $ui.text_wrapped(&details.temperature_rendered);

            if $self.content.details_options.thermograph_fit {
                let thermograph_size = $crate::widgets::thermograph_size(&details.thermograph);
                let [_, text_height] = $ui.calc_text_size("1234567890()");
                let available_w = $ui.current_column_width();
                let pos_y = $ui.cursor_pos()[1];
                let mut available_h = $ui.window_size()[1] - pos_y - text_height;

                // Slider is about to appear so we need to shrink available height
                if !$self.content.details_options.thermograph_fit {
                    let style = unsafe { $ui.style() };
                    available_h -= style.scrollbar_size + style.item_spacing[1] * 2.0;
                }

                let scale_w = available_w / thermograph_size.x;
                let scale_h = available_h / thermograph_size.y;

                $self.content.details_options.thermograph_scale = f32::min(scale_w, scale_h);
            } else {
                // NOTE: When adding widgets here, account for extra height above
                $ui.align_text_to_frame_padding();
                $ui.text("Scale: ");
                $ui.same_line();
                let short_slider = $ui.push_item_width(200.0);
                $ui.slider(
                    "##Thermograph scale",
                    5.0,
                    100.0,
                    &mut $self.content.details_options.thermograph_scale,
                );
                short_slider.end();
            }

            $crate::widgets::thermograph(
                $ui,
                &$draw_list,
                $self.content.details_options.thermograph_scale,
                &mut $self.scratch_buffer,
                &details.thermograph,
            );
        } else {
            $ui.text("Evaluating...");
        }
    }};
}

pub(crate) use game_details;

#[derive(Debug, Clone, Copy)]
enum GraphEditorAction {
    None,
    VertexClick(VertexIndex),
    NewVertex(V2f, Option<VertexIndex>),
}

trait VertexFillColor {
    fn fill_color(&self) -> ImColor32;
}

#[derive(Debug, Clone)]
struct GraphEditor {
    new_edge_starting_vertex: Option<VertexIndex>,
    graph_panel_size: V2f,
}

impl GraphEditor {
    fn new() -> GraphEditor {
        GraphEditor {
            new_edge_starting_vertex: None,
            graph_panel_size: V2f::ZERO,
        }
    }

    fn draw<'ui, V>(
        &mut self,
        ui: &'ui Ui,
        draw_list: &'ui DrawListMut<'ui>,
        new_edge_mode: bool,
        new_vertex_mode: bool,
        drag_mode: bool,
        edge_creates_vertex: bool,
        scratch_buffer: &mut String,
        graph: &mut impl Graph<V>,
    ) -> GraphEditorAction
    where
        V: Has<V2f> + VertexFillColor,
    {
        let mut action = GraphEditorAction::None;
        let graph_region_start = V2f::from(ui.cursor_screen_pos());
        let control_panel_height = ui.cursor_pos()[1];

        let mut max_y = f32::NEG_INFINITY;
        let vertex_border_color = ui.style_color(StyleColor::Text);
        for this_vertex_idx in graph.vertex_indices() {
            let absolute_vertex_pos = *graph.get_vertex(this_vertex_idx).get_inner();
            let _vertex_id = ui.push_id_usize(this_vertex_idx.index);
            let this_vertex_pos = graph_region_start + absolute_vertex_pos;
            max_y = max_y.max(this_vertex_pos.y);
            let button_pos = this_vertex_pos - VERTEX_RADIUS;
            let button_size = V2f {
                x: VERTEX_RADIUS * 2.0,
                y: VERTEX_RADIUS * 2.0,
            };
            ui.set_cursor_screen_pos(button_pos);

            if ui.invisible_button("vertex", button_size) && !new_edge_mode {
                action = GraphEditorAction::VertexClick(this_vertex_idx);
            };

            if ui.is_item_activated() && new_edge_mode {
                self.new_edge_starting_vertex = Some(this_vertex_idx);
            }

            let mouse_pos = V2f::from(ui.io().mouse_pos);
            if !ui.io()[MouseButton::Left]
                && mouse_pos.x >= button_pos.x
                && mouse_pos.x <= (button_pos.x + button_size.x)
                && mouse_pos.y >= button_pos.y
                && mouse_pos.y <= (button_pos.y + button_size.y)
            {
                if let Some(starting_vertex) = self.new_edge_starting_vertex.take() {
                    if starting_vertex != this_vertex_idx {
                        graph.connect(
                            starting_vertex,
                            this_vertex_idx,
                            !graph.are_adjacent(starting_vertex, this_vertex_idx),
                        );
                    }
                }
            }

            if ui.is_item_active() && drag_mode {
                let mouse_delta = V2f::from(ui.io().mouse_delta);
                *graph.get_vertex_mut(this_vertex_idx).get_inner_mut() = V2f {
                    x: f32::max(VERTEX_RADIUS, absolute_vertex_pos.x + mouse_delta.x),
                    y: f32::max(VERTEX_RADIUS, absolute_vertex_pos.y + mouse_delta.y),
                };
            }

            let vertex_fill_color: ImColor32 = graph.get_vertex(this_vertex_idx).fill_color();

            draw_list
                .add_circle(this_vertex_pos, VERTEX_RADIUS, vertex_border_color)
                .filled(false)
                .build();
            draw_list
                .add_circle(this_vertex_pos, VERTEX_RADIUS - 0.5, vertex_fill_color)
                .filled(true)
                .build();

            scratch_buffer.clear();
            scratch_buffer
                .write_fmt(format_args!("{}", this_vertex_idx.index + 1))
                .unwrap();
            let off_x = ui.calc_text_size(&scratch_buffer)[0];
            draw_list.add_text(
                [
                    this_vertex_pos.x - off_x * 0.5,
                    this_vertex_pos.y + VERTEX_RADIUS,
                ],
                vertex_border_color,
                &scratch_buffer,
            );

            for adjacent_vertex_idx in graph.adjacent_to(this_vertex_idx) {
                let both_ways = graph.are_adjacent(adjacent_vertex_idx, this_vertex_idx);

                if !both_ways || this_vertex_idx < adjacent_vertex_idx {
                    let adjacent_relative_pos = *graph.get_vertex(adjacent_vertex_idx).get_inner();

                    let adjacent_vertex_pos = graph_region_start + adjacent_relative_pos;
                    let direction = V2f::direction(this_vertex_pos, adjacent_vertex_pos);
                    let edge_start_pos = this_vertex_pos + direction * VERTEX_RADIUS;
                    let edge_end_pos = adjacent_vertex_pos - direction * VERTEX_RADIUS;
                    let distance_between_vertices = adjacent_vertex_pos - this_vertex_pos;

                    if distance_between_vertices.x.abs() < 2.0 * VERTEX_RADIUS
                        && distance_between_vertices.y.abs() < 2.0 * VERTEX_RADIUS
                    {
                        continue;
                    }

                    draw_list
                        .add_line(edge_start_pos, edge_end_pos, vertex_border_color)
                        .thickness(1.0)
                        .build();

                    // If connection is both ways then we do not draw arrow heads
                    if !both_ways {
                        draw_list
                            .add_triangle(
                                edge_end_pos,
                                V2f {
                                    x: edge_end_pos.x - direction.x * ARROW_HEAD_SIZE
                                        + direction.y * ARROW_HEAD_SIZE,
                                    y: edge_end_pos.y
                                        - direction.y * ARROW_HEAD_SIZE
                                        - direction.x * ARROW_HEAD_SIZE,
                                },
                                V2f {
                                    x: edge_end_pos.x
                                        - direction.x * ARROW_HEAD_SIZE
                                        - direction.y * ARROW_HEAD_SIZE,
                                    y: edge_end_pos.y - direction.y * ARROW_HEAD_SIZE
                                        + direction.x * ARROW_HEAD_SIZE,
                                },
                                vertex_border_color,
                            )
                            .filled(true)
                            .build();
                    }
                }
            }
        }
        if let Some(starting_vertex) = self.new_edge_starting_vertex {
            let held_vertex_relative_pos: V2f = *graph.get_vertex(starting_vertex).get_inner();
            let held_vertex_pos = graph_region_start + held_vertex_relative_pos;
            draw_list
                .add_line(held_vertex_pos, ui.io().mouse_pos, vertex_border_color)
                .thickness(2.0)
                .build();
        }

        ui.set_cursor_screen_pos(graph_region_start);
        let item_spacing_y = unsafe { ui.style().item_spacing[1] };
        self.graph_panel_size = V2f {
            x: ui.current_column_width(),
            y: ui.window_size()[1] - control_panel_height - item_spacing_y * 2.0,
        };
        if new_vertex_mode && ui.invisible_button("Add vertex", self.graph_panel_size) {
            let mouse_pos = V2f::from(ui.io().mouse_pos);
            action = GraphEditorAction::NewVertex(mouse_pos - graph_region_start, None);
        }

        if !ui.io()[MouseButton::Left] {
            if let Some(edge_start) = self.new_edge_starting_vertex.take() {
                if edge_creates_vertex {
                    let mouse_pos = V2f::from(ui.io().mouse_pos);
                    action = GraphEditorAction::NewVertex(
                        V2f {
                            x: f32::max(VERTEX_RADIUS, mouse_pos.x - graph_region_start.x),
                            y: f32::max(VERTEX_RADIUS, mouse_pos.y - graph_region_start.y),
                        },
                        Some(edge_start),
                    );
                }
            }
        }

        ui.set_cursor_screen_pos([graph_region_start.x, max_y + VERTEX_RADIUS]);

        action
    }
}
