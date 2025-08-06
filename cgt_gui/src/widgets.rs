use cgt::{
    numeric::{rational::Rational, v2f::V2f},
    short::partizan::{thermograph::Thermograph, trajectory::Trajectory},
};
use imgui::{ImColor32, StyleColor, Ui};
use std::fmt::Write;

pub mod amazons;
pub mod canonical_form;
pub mod digraph_placement;
pub mod domineering;
pub mod fission;
pub mod konane;
pub mod ski_jumps;
pub mod snort;
pub mod toads_and_frogs;

pub const THERMOGRAPH_TOP_MAST_LEN: f32 = 1.0;
pub const THERMOGRAPH_AXIS_PAD: f32 = 0.5;
pub const THERMOGRAPH_ARROW_SIZE: f32 = 0.15;
pub const THERMOGRAPH_TRAJECTORY_THICKNESS: f32 = 2.0;
pub const THERMOGRAPH_AXIS_THICKNESS: f32 = 1.0;
const TRAJECTORY_COLOR: imgui::StyleColor = StyleColor::Text;

pub const TILE_SIZE: f32 = 64.0;
pub const TILE_SPACING: f32 = 4.0;
pub const TILE_COLOR_EMPTY: ImColor32 = ImColor32::from_rgb(0xcc, 0xcc, 0xcc);
pub const TILE_COLOR_FILLED: ImColor32 = ImColor32::from_rgb(0x44, 0x44, 0x44);

fn fade(mut color: [f32; 4], alpha: f32) -> [f32; 4] {
    let alpha = alpha.clamp(0.0, 1.0);
    color[3] *= alpha;
    color
}

fn fade_color(color: ImColor32, alpha: f32) -> ImColor32 {
    ImColor32::from(fade(color.to_rgba_f32s(), alpha))
}

fn interactive_color(color: ImColor32, ui: &Ui) -> ImColor32 {
    if ui.is_item_active() {
        fade_color(color, 0.6)
    } else if ui.is_item_hovered() {
        fade_color(color, 0.8)
    } else {
        color
    }
}

fn lerp(start: f32, end: f32, t: f32) -> f32 {
    t.mul_add(end - start, start)
}

// FIXME: Height is too large if Y axis is not visible
fn thermograph_size(thermograph: &Thermograph) -> V2f {
    let left_x = thermograph.left_wall.value_at(Rational::from(-1));
    let right_x = thermograph.right_wall.value_at(Rational::from(-1));
    let x_len = (left_x - right_x).as_f32().unwrap();
    let y_top_above_x_axis_l = thermograph
        .left_wall
        .critical_points
        .first()
        .copied()
        .and_then(Rational::as_f32)
        .unwrap_or(0.0);
    let y_top_above_x_axis_r = thermograph
        .right_wall
        .critical_points
        .first()
        .copied()
        .and_then(Rational::as_f32)
        .unwrap_or(0.0);
    let y_top_above_x_axis = y_top_above_x_axis_l.max(y_top_above_x_axis_r);

    V2f {
        x: THERMOGRAPH_AXIS_PAD.mul_add(2.0, x_len),
        y: THERMOGRAPH_AXIS_PAD.mul_add(2.0, y_top_above_x_axis + 1.0 + THERMOGRAPH_TOP_MAST_LEN), // +1.0 to go up to -1 below y axis,
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

    let y_top_above_x_axis_l = thermograph
        .left_wall
        .critical_points
        .first()
        .copied()
        .and_then(Rational::as_f32)
        .unwrap_or(0.0);
    let y_top_above_x_axis_r = thermograph
        .right_wall
        .critical_points
        .first()
        .copied()
        .and_then(Rational::as_f32)
        .unwrap_or(0.0);
    let y_offset = y_top_above_x_axis_l.max(y_top_above_x_axis_r);

    let axis_color = ui.style_color(StyleColor::TextDisabled);

    // x axis
    draw_list
        .add_line(
            [
                pos_x,
                (y_offset + THERMOGRAPH_TOP_MAST_LEN + THERMOGRAPH_AXIS_PAD)
                    .mul_add(thermograph_scale, pos_y),
            ],
            [
                thermograph_scale.mul_add(THERMOGRAPH_AXIS_PAD.mul_add(2.0, x_len), pos_x),
                (y_offset + THERMOGRAPH_TOP_MAST_LEN + THERMOGRAPH_AXIS_PAD)
                    .mul_add(thermograph_scale, pos_y),
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
                    (THERMOGRAPH_AXIS_PAD + y_axis_loc).mul_add(thermograph_scale, pos_x),
                    pos_y,
                ],
                [
                    (THERMOGRAPH_AXIS_PAD + y_axis_loc).mul_add(thermograph_scale, pos_x),
                    THERMOGRAPH_AXIS_PAD
                        .mul_add(2.0, y_offset + 1.0 + THERMOGRAPH_TOP_MAST_LEN)
                        .mul_add(thermograph_scale, pos_y),
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

    // thermograph arrow head
    {
        let trajectory_color = ui.style_color(TRAJECTORY_COLOR);

        debug_assert_eq!(
            thermograph.left_wall.mast_x_intercept(),
            thermograph.right_wall.mast_x_intercept()
        );
        let prev_x = thermograph.left_wall.mast_x_intercept().as_f32().unwrap();
        let prev_y = y_offset + THERMOGRAPH_TOP_MAST_LEN;

        let arrow_top_point = [
            (THERMOGRAPH_AXIS_PAD + x_offset - prev_x).mul_add(thermograph_scale, pos_x),
            (THERMOGRAPH_AXIS_PAD + y_offset + THERMOGRAPH_TOP_MAST_LEN - prev_y)
                .mul_add(thermograph_scale, pos_y),
        ];
        let arrow_left_point = [
            (THERMOGRAPH_AXIS_PAD + x_offset - prev_x - THERMOGRAPH_ARROW_SIZE)
                .mul_add(thermograph_scale, pos_x),
            (THERMOGRAPH_AXIS_PAD + y_offset + THERMOGRAPH_TOP_MAST_LEN - prev_y
                + THERMOGRAPH_ARROW_SIZE)
                .mul_add(thermograph_scale, pos_y),
        ];
        let arrow_right_point = [
            (THERMOGRAPH_AXIS_PAD + x_offset - prev_x + THERMOGRAPH_ARROW_SIZE)
                .mul_add(thermograph_scale, pos_x),
            (THERMOGRAPH_AXIS_PAD + y_offset + THERMOGRAPH_TOP_MAST_LEN - prev_y
                + THERMOGRAPH_ARROW_SIZE)
                .mul_add(thermograph_scale, pos_y),
        ];

        draw_list
            .add_line(arrow_top_point, arrow_left_point, trajectory_color)
            .thickness(THERMOGRAPH_TRAJECTORY_THICKNESS)
            .build();
        draw_list
            .add_line(arrow_top_point, arrow_right_point, trajectory_color)
            .thickness(THERMOGRAPH_TRAJECTORY_THICKNESS)
            .build();
    }

    draw_trajectory(
        ui,
        draw_list,
        [pos_x, pos_y],
        x_offset,
        y_offset,
        thermograph_scale,
        scratch_buffer,
        &thermograph.left_wall,
        ScafoldPlayer::Left,
    );
    draw_trajectory(
        ui,
        draw_list,
        [pos_x, pos_y],
        x_offset,
        y_offset,
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
    y_offset: f32,
    thermograph_scale: f32,
    scratch_buffer: &mut String,
    trajectory: &Trajectory,
    side: ScafoldPlayer,
) {
    let trajectory_color = ui.style_color(TRAJECTORY_COLOR);
    let y_top_above_x_axis = trajectory
        .critical_points
        .first()
        .copied()
        .and_then(Rational::as_f32)
        .unwrap_or(0.0);

    // We start drawing from the top so we initialize prev_{x,y} as top of the mast arrow
    let mut prev_x = trajectory.mast_x_intercept().as_f32().unwrap();
    let mut prev_y = y_top_above_x_axis + THERMOGRAPH_TOP_MAST_LEN;

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
            (THERMOGRAPH_AXIS_PAD - prev_x + x_offset).mul_add(thermograph_scale, pos_x),
            (THERMOGRAPH_AXIS_PAD - prev_y + y_offset + 1.0).mul_add(thermograph_scale, pos_y),
        ];
        let this_point = [
            (THERMOGRAPH_AXIS_PAD - this_x + x_offset).mul_add(thermograph_scale, pos_x),
            (THERMOGRAPH_AXIS_PAD - this_y + y_offset + 1.0).mul_add(thermograph_scale, pos_y),
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

macro_rules! game_details {
    ($self:expr, $ui:expr, $draw_list:expr) => {{
        if let Some(details) = $self.content.details.as_ref() {
            $ui.text_wrapped(&details.canonical_form_rendered);
            $ui.text_wrapped(&details.temperature_rendered);

            let thermograph_size = $crate::widgets::thermograph_size(&details.thermograph);
            let [_, text_height] = $ui.calc_text_size("1234567890()");
            let available_w = $ui.current_column_width();
            let pos_y = $ui.cursor_pos()[1];
            let available_h = $ui.window_size()[1] - pos_y - text_height;
            let scale_w = available_w / thermograph_size.x;
            let scale_h = available_h / thermograph_size.y;
            let thermograph_scale = f32::min(scale_w, scale_h);

            $crate::widgets::thermograph(
                $ui,
                &$draw_list,
                thermograph_scale,
                &mut $self.scratch_buffer,
                &details.thermograph,
            );
        } else {
            $ui.text("Evaluating...");
        }
    }};
}

pub(crate) use game_details;
