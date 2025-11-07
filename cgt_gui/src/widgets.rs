use crate::Details;
use ::imgui::{DrawListMut, FontId, ImColor32, Ui};
use cgt::{
    drawing::{Canvas, Color, Draw, imgui, svg, tiny_skia},
    graph::{Graph, VertexIndex},
    has::Has,
    numeric::v2f::V2f,
    short::partizan::thermograph::Thermograph,
};
use std::{ops::DerefMut, time::SystemTime};

pub mod amazons;
pub mod canonical_form;
pub mod digraph_placement;
pub mod domineering;
pub mod fission;
pub mod graph_editor;
pub mod konane;
pub mod resolving_set;
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
        ui.set_cursor_pos(cursor_pos - thermograph_box.top_left);
        let mut canvas = imgui::Canvas::new(ui, draw_list, large_font_id, scratch_buffer);
        details
            .thermograph
            .draw_scaled(&mut canvas, thermograph_scale);
    } else {
        ui.text("Evaluating...");
    }
}

pub fn save_button<Game>(ui: &Ui, prefix: &str, game: &Game, thermograph: Option<&Thermograph>)
where
    Game: Draw,
{
    // TODO: Popups for file path

    macro_rules! save_using_canvas {
        ($canvas:ident, $extension:literal, canvas $(. $finalizer:ident ())*) => {
            let now = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map_or(0, |duration| duration.as_secs());

            let canvas_size = game.required_canvas::<$canvas::Canvas>();
            let mut canvas = $canvas::Canvas::new(canvas_size);
            game.draw(&mut canvas);
            let mut f =
                std::fs::File::create(format!("{}_{}.{}", prefix, now, $extension)).unwrap();
            std::io::Write::write_all(&mut f, canvas$(. $finalizer ())*).unwrap();

            if let Some(thermograph) = thermograph {
                let canvas_size = thermograph.required_canvas::<$canvas::Canvas>();
                let mut canvas = $canvas::Canvas::new(canvas_size);
                thermograph.draw(&mut canvas);
                let mut f =
                    std::fs::File::create(format!("{}_thermograph_{}.{}", prefix, now, $extension))
                        .unwrap();
                std::io::Write::write_all(&mut f, canvas$(. $finalizer ())*).unwrap();
            }
        };
    }

    if let Some(_save_menu) = ui.begin_menu("Save") {
        if ui.menu_item("as SVG") {
            save_using_canvas!(svg, "svg", canvas.to_svg().as_bytes());
        }
        if ui.menu_item("as PNG") {
            save_using_canvas!(tiny_skia, "png", canvas.to_png().as_ref());
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AddEdgeMode {
    pub edge_start_vertex: Option<VertexIndex>,
    pub edge_creates_vertex: bool,
}

impl AddEdgeMode {
    pub const fn new() -> AddEdgeMode {
        AddEdgeMode {
            edge_creates_vertex: true,
            edge_start_vertex: None,
        }
    }

    pub fn handle_update<G, V>(
        &mut self,
        mouse_pos: V2f,
        graph_area_position: V2f,
        canvas: &mut imgui::Canvas<'_>,
        graph: &mut impl DerefMut<Target = G>,
        mk_new_vertex: impl FnOnce(V2f) -> V,
    ) where
        G: Graph<V>,
        V: Has<V2f>,
    {
        let mouse_position = mouse_pos - graph_area_position;
        if let Some(pressed) = canvas.pressed_vertex() {
            self.edge_start_vertex = Some(pressed);
            let pressed_position: V2f = *graph.get_vertex(pressed).get_inner();
            canvas.line(
                pressed_position,
                mouse_position,
                imgui::Canvas::thin_line_weight(),
                Color::BLACK,
            );
        } else if let Some(start) = self.edge_start_vertex.take() {
            let graph_ref: &G = &*graph;
            if let Some(end) = canvas.vertex_at_position(mouse_position, graph_ref)
                && start != end
            {
                let should_connect = !graph.are_adjacent(start, end);
                graph.connect(start, end, should_connect);
            } else if self.edge_creates_vertex {
                let end = graph.add_vertex(mk_new_vertex(mouse_position));
                graph.connect(start, end, true);
            }
        }
    }
}

crate::imgui_enum! {
    #[derive(Debug, Clone, Copy)]
    RepositionMode {
        SpringEmbedder, "Spring Embedder",
        Circle, "Circle",
    }
}
