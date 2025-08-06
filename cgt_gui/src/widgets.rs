use crate::Details;
use ::imgui::{DrawListMut, FontId, ImColor32, Ui};
use cgt::{
    drawing::{imgui, svg, tiny_skia, Draw},
    numeric::v2f::V2f,
};

pub mod amazons;
pub mod canonical_form;
pub mod digraph_placement;
pub mod domineering;
pub mod fission;
pub mod konane;
pub mod ski_jumps;
pub mod snort;
pub mod toads_and_frogs;

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

pub fn grid_size_selector(ui: &Ui, new_width: &mut u8, new_height: &mut u8) {
    let short_inputs = ui.push_item_width(100.0);
    ui.input_scalar("Width", new_width).step(1).build();
    ui.input_scalar("Height", new_height).step(1).build();
    short_inputs.end();
}

pub fn game_details<'ui>(
    details: Option<&Details>,
    scratch_buffer: &mut String,
    ui: &'ui Ui,
    draw_list: &DrawListMut<'ui>,
    large_font_id: FontId,
) {
    if let Some(details) = details.as_ref() {
        ui.text_wrapped(&details.canonical_form_rendered);
        ui.text_wrapped(&details.temperature_rendered);

        let thermograph_size = details
            .thermograph
            .required_canvas_scaled::<imgui::Canvas>(1.0)
            .size();
        let [_, text_height] = ui.calc_text_size("1234567890()");
        let available_w = ui.current_column_width();
        let cursor_pos = V2f::from(ui.cursor_pos());
        let available_h = ui.window_size()[1] - cursor_pos.y - text_height;
        let scale_w = available_w / thermograph_size.x;
        let scale_h = available_h / thermograph_size.y;
        let thermograph_scale = f32::min(scale_w, scale_h);
        let thermograph_box = details
            .thermograph
            .required_canvas_scaled::<imgui::Canvas>(thermograph_scale);
        ui.set_cursor_pos(V2f {
            x: cursor_pos.x - thermograph_box.top_left.x,
            y: cursor_pos.y - thermograph_box.bottom_right.y,
        });
        let mut canvas = imgui::Canvas::new(ui, draw_list, large_font_id, scratch_buffer);
        details
            .thermograph
            .draw_scaled(&mut canvas, thermograph_scale);
    } else {
        ui.text("Evaluating...");
    }
}

pub fn save_button<Game>(ui: &Ui, game: &Game)
where
    Game: Draw,
{
    // TODO: Popups for file path
    if let Some(_save_menu) = ui.begin_menu("Save") {
        if ui.menu_item("as SVG") {
            let canvas_size = game.required_canvas::<svg::Canvas>();
            let mut canvas = svg::Canvas::new(canvas_size);
            game.draw(&mut canvas);
            eprintln!("{}", canvas.to_svg());
        };
        if ui.menu_item("as PNG") {
            let canvas_size = game.required_canvas::<tiny_skia::Canvas>();
            let mut canvas = tiny_skia::Canvas::new(canvas_size);
            game.draw(&mut canvas);
            let mut f = std::fs::File::create("out.png").unwrap();
            std::io::Write::write_all(&mut f, &canvas.to_png()).unwrap();
        };
    }
}
